use crate::{
    entities::{PlanActiveModel, PlanColumn, PlanModel, PlansEntity},
    types::{PlanStatus, StringList},
};
use anyhow::{Context, Result};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter,
};
use sea_orm::{PaginatorTrait, sea_query::Expr};

/// Repository for Plan entity operations
pub struct PlanRepository {
    db: DatabaseConnection,
}

impl PlanRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Find a plan by its unique name, case-insensitive
    pub async fn find_by_name(&self, name: &str) -> Result<Option<PlanModel>> {
        PlansEntity::find()
            .filter(Condition::all().add(Expr::col(PlanColumn::Name).like(name)))
            .one(&self.db)
            .await
            .context("Failed to query plan by name")
    }

    /// Find a plan by its ID
    pub async fn get_by_id(&self, id: i32) -> Result<PlanModel> {
        PlansEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Plan with ID {} not found", id))
    }

    /// Get all plans
    pub async fn get_all(&self) -> Result<Vec<PlanModel>> {
        PlansEntity::find()
            .all(&self.db)
            .await
            .context("Failed to query all plans")
    }

    /// Get plans by status
    pub async fn get_by_status(&self, status: PlanStatus) -> Result<Vec<PlanModel>> {
        PlansEntity::find()
            .filter(PlanColumn::Status.eq(status.to_string()))
            .all(&self.db)
            .await
            .context(format!("Failed to query plans with status: {}", status))
    }

    /// Create a new plan
    pub async fn create(
        &self,
        name: &str,
        source_connection_id: i32,
        target_connection_id: i32,
        schemas: StringList,
        exclude_object_types: Option<StringList>,
        exclude_object_names: Option<StringList>,
        disabled_drop_types: Option<StringList>,
        fail_fast: bool,
    ) -> Result<PlanModel> {
        let active_model = PlanActiveModel {
            id: NotSet,
            name: Set(name.to_string()),
            source_connection_id: Set(source_connection_id),
            target_connection_id: Set(target_connection_id),
            schemas: Set(schemas),
            exclude_object_types: Set(exclude_object_types),
            exclude_object_names: Set(exclude_object_names),
            status: Set(PlanStatus::default()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            disabled_drop_types: Set(disabled_drop_types),
            fail_fast: Set(fail_fast),
            ..Default::default()
        };

        let result = PlansEntity::insert(active_model)
            .exec(&self.db)
            .await
            .context(format!("Failed to insert plan '{}'", name))?;

        self.get_by_id(result.last_insert_id)
            .await
            .context("Plan was created but could not be retrieved")
    }

    /// Update a plan's status
    pub async fn set_status(&self, id: i32, status: PlanStatus) -> Result<PlanModel> {
        let plan = self
            .get_by_id(id)
            .await
            .context(format!("Plan with ID {} not found", id))?;

        let mut active: PlanActiveModel = plan.into();
        active.status = Set(status);

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for plan {}", id))?;

        self.get_by_id(id)
            .await
            .context("Plan was updated but could not be retrieved")
    }

    /// Delete a plan by ID
    pub async fn delete(&self, id: i32) -> Result<()> {
        let result = PlansEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .context(format!("Failed to delete plan with id {}", id))?;

        if result.rows_affected == 0 {
            anyhow::bail!("Plan with id {} not found", id);
        }

        Ok(())
    }

    pub async fn delete_all(&self) -> Result<u64> {
        Ok(PlansEntity::delete_many()
            .exec(&self.db)
            .await?
            .rows_affected)
    }

    /// Check if a plan with the given name already exists
    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        Ok(self.find_by_name(name).await?.is_some())
    }

    /// Get plans associated with a specific connection (source or target)
    pub async fn get_by_connection_id(&self, connection_id: i32) -> Result<Vec<PlanModel>> {
        PlansEntity::find()
            .filter(
                Condition::any()
                    .add(PlanColumn::SourceConnectionId.eq(connection_id))
                    .add(PlanColumn::TargetConnectionId.eq(connection_id)),
            )
            .all(&self.db)
            .await
            .context(format!(
                "Failed to query plans for connection {}",
                connection_id
            ))
    }

    pub async fn is_running(&self, id: i32) -> Result<bool> {
        let plan = self.get_by_id(id).await?;
        Ok(plan.status == PlanStatus::Running)
    }

    /// Check if a connection is in use by any running plan
    pub async fn is_connection_in_use(&self, source_connection_id: i32) -> Result<bool> {
        let count = PlansEntity::find()
            .filter(
                Condition::any()
                    .add(PlanColumn::SourceConnectionId.eq(source_connection_id))
                    .add(PlanColumn::TargetConnectionId.eq(source_connection_id)),
            )
            .filter(PlanColumn::Status.eq(PlanStatus::Running.to_string()))
            .count(&self.db)
            .await?;

        Ok(count > 0)
    }
}
