use std::sync::Arc;

use crate::{
    config::Settings,
    entities::plan::Model as PlanModel,
    repo::{ConnectionRepository, DeploymentRepository, plan_repo::PlanRepository},
    types::{PlanStatus, StringList},
};
use anyhow::{Context, Result, anyhow, ensure};
use chrono::NaiveDateTime;

/// Service layer for Plan business logic
pub struct PlanService {
    settings: Settings,
    repo: Arc<PlanRepository>,
    deployment_repo: Arc<DeploymentRepository>,
    connection_repo: Arc<ConnectionRepository>,
}

impl PlanService {
    pub fn new(
        settings: Settings,
        repo: Arc<PlanRepository>,
        deployment_repo: Arc<DeploymentRepository>,
        connection_repo: Arc<ConnectionRepository>,
    ) -> Self {
        Self {
            settings,
            repo,
            deployment_repo,
            connection_repo,
        }
    }

    /// Create a new plan with business validation
    pub async fn create(
        &self,
        name: &str,
        source: &str,
        target: &str,
        schemas: &[String],
        exclude_object_types: Option<Vec<String>>,
        exclude_object_names: Option<Vec<String>>,
        disabled_drop_types: Option<Vec<String>>,
        fail_fast: bool,
    ) -> Result<PlanModel> {
        if self.repo.exists_by_name(name).await? {
            anyhow::bail!(
                "Plan with name '{}' already exists. Plan names are case-insensitive and must be unique.",
                name
            );
        }

        // Business validation: Source connection must exist
        let source_connection = self
            .connection_repo
            .find_by_name(source)
            .await?
            .ok_or_else(|| anyhow!("Source connection '{}' not found", source))?;

        // Business validation: Target connection must exist
        let target_connection = self
            .connection_repo
            .find_by_name(target)
            .await?
            .ok_or_else(|| anyhow!("Target connection '{}' not found", target))?;

        if source_connection.id == target_connection.id {
            anyhow::bail!("Source and target connections cannot be the same");
        }

        if schemas.is_empty() {
            anyhow::bail!("At least one schema must be specified");
        }

        let combined_exclude_object_types = self
            .settings
            .rules
            .combined_exclude_object_types(exclude_object_types)
            .map(StringList);

        let combined_exclude_object_names = self
            .settings
            .rules
            .combined_exclude_object_names(exclude_object_names)
            .map(StringList);

        let combined_disabled_drop_types = self
            .settings
            .rules
            .combined_disabled_drop_types(disabled_drop_types)
            .map(StringList);

        self.repo
            .create(
                name,
                source_connection.id,
                target_connection.id,
                StringList(schemas.to_vec()),
                combined_exclude_object_types,
                combined_exclude_object_names,
                combined_disabled_drop_types,
                fail_fast,
            )
            .await
            .context("Failed to create plan")
    }

    /// Update plan status by name
    pub async fn set_status_by_name(&self, name: &str, status: PlanStatus) -> Result<PlanModel> {
        let plan = self.repo.find_by_name(name).await?;
        ensure!(plan.is_some(), "Plan '{}' not found", name);

        self.repo
            .set_status(plan.unwrap().id, status)
            .await
            .context(format!("Failed to update status for plan '{}'", name))
    }

    /// Delete a plan by name
    pub async fn delete_by_name(&self, name: &str) -> Result<PlanModel> {
        let plan = self
            .repo
            .find_by_name(name)
            .await?
            .ok_or_else(|| anyhow!("Plan '{}' not found", name))?;

        self.repo
            .delete(plan.id)
            .await
            .context(format!("Failed to delete plan '{}'", name))?;

        Ok(plan)
    }

    /// Delete all plans
    pub async fn prune(&self) -> Result<u64> {
        self.repo.delete_all().await
    }

    /// Get a plan by name
    pub async fn find_by_name(&self, name: &str) -> Result<Option<PlanModel>> {
        self.repo.find_by_name(name).await
    }

    /// Get a plan by ID
    pub async fn get_by_id(&self, id: i32) -> Result<PlanModel> {
        self.repo.get_by_id(id).await
    }

    /// Get all plans
    pub async fn get_all(&self) -> Result<Vec<PlanModel>> {
        self.repo.get_all().await
    }

    /// Get plans by status
    pub async fn get_by_status(&self, status: PlanStatus) -> Result<Vec<PlanModel>> {
        self.repo.get_by_status(status).await
    }

    pub async fn reset_status_by_id(&self, id: i32) -> Result<PlanModel> {
        let plan = self.repo.get_by_id(id).await?;
        self.repo
            .set_status(id, PlanStatus::Idle)
            .await
            .context(format!("Failed to reset status for plan '{}'", plan.name))?;

        Ok(plan)
    }

    pub async fn get_last_cutoff_date(&self, plan_id: i32) -> Result<Option<NaiveDateTime>> {
        let deployment = self
            .deployment_repo
            .find_last_successful_by_plan_id(plan_id)
            .await?;
        Ok(deployment.map(|d| d.cutoff_date))
    }
}
