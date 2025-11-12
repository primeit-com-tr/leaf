use crate::{
    entities::{DeploymentActiveModel, DeploymentColumn, DeploymentModel, DeploymentsEntity},
    types::DeploymentStatus,
};
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect,
};

pub struct DeploymentRepository {
    db: DatabaseConnection,
}

impl DeploymentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_all(&self) -> Result<Vec<DeploymentModel>> {
        DeploymentsEntity::find()
            .all(&self.db)
            .await
            .context("Failed to get all deployments.")
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<DeploymentModel>> {
        DeploymentsEntity::find_by_id(id)
            .one(&self.db)
            .await
            .context(format!("Failed to find deployment by id: {}", id))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<DeploymentModel> {
        DeploymentsEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Deployment with ID {} not found", id))
    }

    pub async fn get_by_plan_id(&self, plan_id: i32) -> Result<Vec<DeploymentModel>> {
        DeploymentsEntity::find()
            .filter(DeploymentColumn::PlanId.eq(plan_id))
            .all(&self.db)
            .await
            .context(format!(
                "Failed to get all deployments for plan {}",
                plan_id
            ))
    }

    pub async fn fetch_deployments(
        &self,
        plan_id: Option<i32>,
        limit: Option<u32>,
        order: Option<String>,
    ) -> Result<Vec<DeploymentModel>> {
        let mut query = DeploymentsEntity::find();

        if let Some(plan_id) = plan_id {
            query = query.filter(DeploymentColumn::PlanId.eq(plan_id));
        }

        if let Some(limit) = limit {
            query = query.limit(Some(limit.into()));
        }

        let order = order.unwrap_or_else(|| "desc".to_string());

        if order == "desc" {
            query = query.order_by_desc(DeploymentColumn::CreatedAt);
        } else {
            query = query.order_by_asc(DeploymentColumn::CreatedAt);
        }

        query
            .all(&self.db)
            .await
            .context("Failed to find deployments")
    }

    pub async fn delete(&self, id: i32) -> Result<u64, sea_orm::DbErr> {
        let res = DeploymentsEntity::delete_by_id(id).exec(&self.db).await?;
        Ok(res.rows_affected)
    }

    pub async fn delete_all(&self) -> Result<u64> {
        Ok(DeploymentsEntity::delete_many()
            .exec(&self.db)
            .await?
            .rows_affected)
    }

    pub async fn find_last_by_plan_id(&self, plan_id: i32) -> Result<Option<DeploymentModel>> {
        DeploymentsEntity::find()
            .filter(DeploymentColumn::PlanId.eq(plan_id))
            .order_by(DeploymentColumn::CreatedAt, Order::Desc)
            .one(&self.db)
            .await
            .context(format!(
                "Failed to find last deployment for plan {}",
                plan_id
            ))
    }

    pub async fn find_last_successful_by_plan_id(
        &self,
        plan_id: i32,
    ) -> Result<Option<DeploymentModel>> {
        DeploymentsEntity::find()
            .filter(DeploymentColumn::PlanId.eq(plan_id))
            .filter(DeploymentColumn::Status.eq(DeploymentStatus::Success))
            .order_by(DeploymentColumn::CreatedAt, Order::Desc)
            .one(&self.db)
            .await
            .context(format!(
                "Failed to find last successful deployment for plan {}",
                plan_id
            ))
    }

    pub async fn create(
        &self,
        plan_id: i32,
        cutoff_date: NaiveDateTime,
        payload: String,
        started_at: NaiveDateTime,
    ) -> Result<DeploymentModel> {
        let active_model = DeploymentActiveModel {
            id: NotSet,
            plan_id: Set(plan_id),
            cutoff_date: Set(cutoff_date),
            payload: Set(payload),
            started_at: Set(Some(started_at)),
            ..Default::default()
        };
        let saved = active_model.save(&self.db).await?;
        Ok(saved.try_into()?)
    }

    pub async fn save_deployment(
        &self,
        deployment: DeploymentActiveModel,
    ) -> Result<DeploymentModel> {
        let model = deployment.save(&self.db).await?;
        Ok(model.try_into()?)
    }

    pub async fn set_status(&self, id: i32, status: DeploymentStatus) -> Result<DeploymentModel> {
        let deployment = self
            .get_by_id(id)
            .await
            .context(format!("Deployment with ID {} not found", id))?;

        let mut active: DeploymentActiveModel = deployment.into();
        active.set_status(status);

        active
            .update(&self.db)
            .await
            .context(format!("Failed to update status for deployment {}", id))?;

        self.get_by_id(id)
            .await
            .context("Deployment was updated but could not be retrieved")
    }
}
