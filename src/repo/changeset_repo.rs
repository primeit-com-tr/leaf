use crate::{
    entities::{
        ChangeModel, ChangesEntity, ChangesetActiveModel, ChangesetColumn, ChangesetModel,
        ChangesetsEntity,
    },
    types::ChangesetStatus,
};
use anyhow::{Context, Result};
use migration::Expr;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

pub struct ChangesetRepository {
    db: DatabaseConnection,
}

impl ChangesetRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_all(&self) -> Result<Vec<ChangesetModel>> {
        ChangesetsEntity::find()
            .all(&self.db)
            .await
            .context(format!("failed to get all changesets."))
    }

    pub async fn get_by_deployment_id(&self, deployment_id: i32) -> Result<Vec<ChangesetModel>> {
        ChangesetsEntity::find()
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .all(&self.db)
            .await
            .context(format!("failed to get all changesets."))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ChangesetModel>> {
        ChangesetsEntity::find_by_id(id)
            .one(&self.db)
            .await
            .context(format!("Failed to find changeset by id: {}", id))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<ChangesetModel> {
        ChangesetsEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Connection with ID {} not found", id))
    }

    pub async fn find_by_deployment_id_with_changes(
        &self,
        deployment_id: i32,
    ) -> Result<Option<Vec<(ChangesetModel, Vec<ChangeModel>)>>> {
        let results = ChangesetsEntity::find()
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .order_by_asc(Expr::cust(
                "CASE LOWER(object_type)
                    WHEN 'table' THEN 100
                    WHEN 'sequence' THEN 200
                    WHEN 'view' THEN 300
                    WHEN 'package' THEN 400
                    WHEN 'package body' THEN 500
                    WHEN 'procedure' THEN 600
                    WHEN 'function' THEN 700
                    WHEN 'index' THEN 800
                    WHEN 'trigger' THEN 900
                    ELSE 1000
                END * case when target_ddl is null then 1 else -1 end", // Reverse order drop statements
            ))
            .find_with_related(ChangesEntity)
            .all(&self.db)
            .await
            .context("Failed to fetch changesets with changes")?;

        Ok((!results.is_empty()).then_some(results))
    }

    pub async fn get_by_deployment_id_with_changes(
        &self,
        deployment_id: i32,
    ) -> Result<Vec<(ChangesetModel, Vec<ChangeModel>)>> {
        ChangesetsEntity::find()
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .order_by_asc(Expr::cust(
                "CASE LOWER(object_type)
                    WHEN 'table' THEN 100
                    WHEN 'sequence' THEN 200
                    WHEN 'view' THEN 300
                    WHEN 'package' THEN 400
                    WHEN 'package body' THEN 500
                    WHEN 'procedure' THEN 600
                    WHEN 'function' THEN 700
                    WHEN 'index' THEN 800
                    WHEN 'trigger' THEN 900
                    ELSE 1000
                END * case when target_ddl is null then 1 else -1 end", // Reverse order drop statements
            ))
            .find_with_related(ChangesEntity)
            .all(&self.db)
            .await
            .context("Failed to fetch changesets with changes")
    }

    pub async fn create(
        &self,
        deployment_id: i32,
        object_type: &str,
        object_name: &str,
        object_owner: &str,
        source_ddl: Option<&str>,
        target_ddl: Option<&str>,
    ) -> Result<ChangesetModel> {
        let active_model = ChangesetActiveModel {
            id: NotSet,
            deployment_id: Set(deployment_id),
            object_type: Set(object_type.to_string()),
            object_name: Set(object_name.to_string()),
            object_owner: Set(object_owner.to_string()),
            source_ddl: Set(source_ddl.map(|s| s.to_string())),
            target_ddl: Set(target_ddl.map(|s| s.to_string())),
            ..Default::default()
        };

        let saved = active_model.save(&self.db).await?;
        Ok(saved.try_into()?)
    }

    pub async fn save_changeset(&self, changeset: &ChangesetActiveModel) -> Result<ChangesetModel> {
        let saved = changeset.clone().save(&self.db).await?;
        Ok(saved.try_into()?)
    }

    pub async fn delete(&self, id: i32) -> Result<u64> {
        let res = ChangesetsEntity::delete_by_id(id).exec(&self.db).await?;
        Ok(res.rows_affected)
    }

    pub async fn delete_all(&self) -> Result<u64> {
        Ok(ChangesetsEntity::delete_many()
            .exec(&self.db)
            .await?
            .rows_affected)
    }

    pub async fn get_count_by_deployment_id(&self, deployment_id: i32) -> Result<u64> {
        ChangesetsEntity::find()
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .count(&self.db)
            .await
            .context(format!(
                "Failed to count changesets by deployment id: {}",
                deployment_id
            ))
    }

    pub async fn find_by_deployment_id(&self, deployment_id: i32) -> Result<Vec<ChangesetModel>> {
        ChangesetsEntity::find()
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .all(&self.db)
            .await
            .context(format!(
                "Failed to find changesets by deployment id: {}",
                deployment_id
            ))
    }

    pub async fn set_status(&self, id: i32, status: ChangesetStatus) -> Result<ChangesetModel> {
        let changeset = self.get_by_id(id).await?;
        let mut active: ChangesetActiveModel = changeset.into();
        active.set_status(status);

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for changeset {}", id))?;

        self.get_by_id(id)
            .await
            .context("Changeset was updated but could not be retrieved")
    }
}
