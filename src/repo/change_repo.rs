use crate::{
    entities::{
        ChangeActiveModel, ChangeColumn, ChangeModel, ChangesEntity, ChangesetColumn,
        ChangesetsEntity,
    },
    types::ChangeStatus,
};

use anyhow::{Context, Result};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, PaginatorTrait, QueryFilter,
    QuerySelect,
};

pub struct ChangeRepository {
    db: DatabaseConnection,
}

impl ChangeRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_deployment_id(&self, deployment_id: i32) -> Result<Vec<ChangeModel>> {
        let results = ChangesEntity::find()
            .join(
                JoinType::InnerJoin,
                ChangesEntity::belongs_to(ChangesetsEntity)
                    .from(ChangeColumn::ChangesetId)
                    .to(ChangesetColumn::Id)
                    .into(),
            )
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .all(&self.db)
            .await
            .context(format!(
                "Failed to find changes for deployment_id = {}",
                deployment_id
            ))?;

        Ok(results)
    }

    pub async fn get_all(&self) -> Result<Vec<ChangeModel>> {
        ChangesEntity::find()
            .all(&self.db)
            .await
            .context(format!("Failed to get all changesets."))
    }

    pub async fn get_by_changeset_id(&self, changeset_id: i32) -> Result<Vec<ChangeModel>> {
        ChangesEntity::find()
            .filter(ChangeColumn::ChangesetId.eq(changeset_id))
            .all(&self.db)
            .await
            .context(format!(
                "Failed to find changes by changeset id: {}",
                changeset_id
            ))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ChangeModel>> {
        ChangesEntity::find_by_id(id)
            .one(&self.db)
            .await
            .context(format!("Failed to find changes by id: {}", id))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<ChangeModel> {
        ChangesEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Change with ID {} not found", id))
    }

    pub async fn create(
        &self,
        changeset_id: i32,
        script: &str,
        rollback_script: &str,
    ) -> Result<ChangeModel> {
        let active_model = ChangeActiveModel {
            id: NotSet,
            changeset_id: Set(changeset_id),
            script: Set(script.to_string()),
            rollback_script: Set(rollback_script.to_string()),
            ..Default::default()
        };

        let saved = active_model.save(&self.db).await?;
        Ok(saved.try_into()?)
    }

    pub async fn save_change(&self, change: &ChangeActiveModel) -> Result<ChangeModel> {
        let saved = change.clone().save(&self.db).await?;
        Ok(saved.try_into()?)
    }

    pub async fn delete(&self, id: i32) -> Result<u64> {
        let res = ChangesEntity::delete_by_id(id).exec(&self.db).await?;
        Ok(res.rows_affected)
    }

    pub async fn delete_all(&self) -> Result<u64> {
        Ok(ChangesEntity::delete_many()
            .exec(&self.db)
            .await?
            .rows_affected)
    }

    pub async fn get_count_by_deployment_id(&self, deployment_id: i32) -> Result<u64> {
        ChangesEntity::find()
            .join(
                JoinType::InnerJoin,
                ChangesEntity::belongs_to(ChangesetsEntity)
                    .from(ChangeColumn::ChangesetId)
                    .to(ChangesetColumn::Id)
                    .into(),
            )
            .filter(ChangesetColumn::DeploymentId.eq(deployment_id))
            .count(&self.db)
            .await
            .context(format!(
                "Failed to count changes by deployment id: {}",
                deployment_id
            ))
    }

    pub async fn set_status(&self, id: i32, status: ChangeStatus) -> Result<ChangeModel> {
        let change = self.get_by_id(id).await?;
        let mut active: ChangeActiveModel = change.into();
        active.set_status(status);

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for change {}", id))?;

        self.get_by_id(id)
            .await
            .context("Change was updated but could not be retrieved")
    }
}
