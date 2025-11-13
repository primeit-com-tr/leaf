use crate::{
    entities::{
        ChangeModel, ChangeRelation, ChangesEntity, ChangesetColumn, ChangesetsEntity,
        RollbackActiveModel, RollbackColumn, RollbackModel, RollbackRelation, RollbacksEntity,
    },
    types::RollbackStatus,
};

use anyhow::{Context, Result};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, JoinType, ModelTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};

pub struct RollbackRepository {
    db: DatabaseConnection,
}

impl RollbackRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_by_deployment_id(&self, deployment_id: i32) -> Result<Vec<RollbackModel>> {
        let rollbacks = RollbacksEntity::find()
            .join(sea_orm::JoinType::InnerJoin, RollbackRelation::Change.def())
            .join(
                sea_orm::JoinType::InnerJoin,
                ChangeRelation::Changeset.def(),
            )
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .order_by_asc(RollbackColumn::Id)
            .all(&self.db)
            .await?;

        Ok(rollbacks)
    }

    pub async fn get_rollbacks_with_changes_and_changesets(
        &self,
        deployment_id: i32,
    ) -> Result<Option<Vec<(RollbackModel, ChangeModel, crate::entities::ChangesetModel)>>> {
        let rollbacks = RollbacksEntity::find()
            .join(JoinType::InnerJoin, RollbackRelation::Change.def())
            .join(JoinType::InnerJoin, ChangeRelation::Changeset.def())
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .order_by_asc(RollbackColumn::Id)
            .all(&self.db)
            .await?;

        if rollbacks.is_empty() {
            return Ok(None);
        }

        let mut results = Vec::new();

        for rollback in rollbacks {
            if let Some(change) = rollback.find_related(ChangesEntity).one(&self.db).await? {
                if let Some(changeset) = change.find_related(ChangesetsEntity).one(&self.db).await?
                {
                    results.push((rollback, change, changeset));
                }
            }
        }

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results))
        }
    }

    pub async fn get_by_change_id(&self, change_id: i32) -> Result<Vec<RollbackModel>> {
        let rollbacks = RollbacksEntity::find()
            .filter(RollbackColumn::ChangeId.eq(change_id))
            .order_by_asc(RollbackColumn::Id)
            .all(&self.db)
            .await?;

        Ok(rollbacks)
    }

    pub async fn create(&self, change_id: i32, script: String) -> Result<RollbackModel> {
        let active_model = RollbackActiveModel {
            id: NotSet,
            change_id: Set(change_id),
            script: Set(script),
            status: Set(RollbackStatus::default()),
            ..Default::default()
        };
        let result = RollbacksEntity::insert(active_model)
            .exec(&self.db)
            .await
            .context(format!("Failed to insert rollback for '{}'", change_id))?;

        self.get_by_id(result.last_insert_id)
            .await
            .context("Rollback was created but could not be retrieved")
    }

    pub async fn get_by_id(&self, id: i32) -> Result<RollbackModel> {
        RollbacksEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Rollback with ID {} not found", id))
    }

    pub async fn set_status(&self, id: i32, status: RollbackStatus) -> Result<RollbackModel> {
        let rollback = self.get_by_id(id).await?;
        let mut active: RollbackActiveModel = rollback.into();
        active.set_status(status);

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for rollback {}", id))?;

        self.get_by_id(id)
            .await
            .context("Rollback was updated but could not be retrieved")
    }

    pub async fn set_error(&self, id: i32, error: String) -> Result<RollbackModel> {
        let mut active: RollbackActiveModel = self
            .get_by_id(id)
            .await
            .context("Failed to get rollback")?
            .into_active_model();

        active.status = Set(RollbackStatus::Error);
        active.error = Set(Some(error));

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for rollback {}", id))?;

        self.get_by_id(id)
            .await
            .context("Rollback was updated but could not be retrieved")
    }
}
