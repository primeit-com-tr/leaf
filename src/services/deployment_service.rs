use crate::{
    delta::{delta::with_disabled_drop_types_excluded, find_deltas},
    entities::{
        ChangeActiveModel, ChangeModel, ChangesetActiveModel, ChangesetModel, DeploymentModel,
        PlanModel,
    },
    errors::{DeployError, PlanIsNotRunnableError, SchemaValidationError},
    oracle::OracleClient,
    repo::{
        ChangeRepository, ChangesetRepository, ConnectionRepository, DeploymentRepository,
        PlanRepository, rollback_repo::RollbackRepository,
    },
    types::{ChangeStatus, Delta, DeploymentStatus, PlanStatus, RollbackStatus, StringList},
    utils::{DeploymentContext, ProgressReporter},
};
use anyhow::{Context, Result, anyhow};
use chrono::NaiveDateTime;
use sea_orm::IntoActiveModel;
use std::sync::Arc;
use tokio::try_join;
use tracing::warn;

pub struct DeploymentService {
    repo: Arc<DeploymentRepository>,
    plan_repo: Arc<PlanRepository>,
    connection_repo: Arc<ConnectionRepository>,
    changeset_repo: Arc<ChangesetRepository>,
    change_repo: Arc<ChangeRepository>,
    rollback_repo: Arc<RollbackRepository>,
}

impl DeploymentService {
    pub fn new(
        repo: Arc<DeploymentRepository>,
        plan_repo: Arc<PlanRepository>,
        connection_repo: Arc<ConnectionRepository>,
        changeset_repo: Arc<ChangesetRepository>,
        change_repo: Arc<ChangeRepository>,
        rollback_repo: Arc<RollbackRepository>,
    ) -> Self {
        Self {
            repo,
            plan_repo,
            connection_repo,
            changeset_repo,
            change_repo,
            rollback_repo,
        }
    }

    pub async fn get_by_id(&self, id: i32) -> Result<DeploymentModel> {
        self.repo.get_by_id(id).await
    }

    async fn create_deployment(
        &self,
        plan: &PlanModel,
        cutoff_date: NaiveDateTime,
        disable_hooks: Option<bool>,
    ) -> Result<DeploymentModel> {
        self.repo
            .create(
                plan.id,
                cutoff_date,
                serde_json::to_string(&plan)?,
                disable_hooks.unwrap_or(plan.disable_hooks),
                plan.get_hooks()?.clone(),
            )
            .await
    }

    pub async fn validate_schemas(&self, client: &OracleClient, schemas: &[String]) -> Result<()> {
        let all_schemas = client.get_all_users().await?;
        let missing = schemas
            .iter()
            .filter(|s| !all_schemas.contains(s))
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(anyhow::Error::new(SchemaValidationError::from_vec(missing)))
        }
    }

    async fn get_client(&self, connection_id: i32) -> Result<OracleClient> {
        let connection = self
            .connection_repo
            .get_by_id(connection_id)
            .await
            .context(format!(
                "Failed to find connection for id '{}'",
                connection_id
            ))?;
        OracleClient::connect(
            &connection.username,
            &connection.password,
            &connection.connection_string,
        )
        .context("Failed to connect to Oracle database")
    }

    async fn validate_plan_is_runnable(
        &self,
        plan: &PlanModel,
        ctx: &mut DeploymentContext,
    ) -> Result<()> {
        ctx.progress(format!("Checking if plan '{}' is runnable...", plan.name));
        if self.plan_repo.is_running(plan.id).await? {
            return Err(PlanIsNotRunnableError::PlanIsAlreadyRunning)
                .context(format!("Plan '{}' is already running", plan.name));
        }
        ctx.progress(format!(
            "Checking if source or target connection is in use..."
        ));
        let (source_in_use, target_in_use) = try_join!(
            self.plan_repo
                .is_connection_in_use(plan.source_connection_id),
            self.plan_repo
                .is_connection_in_use(plan.target_connection_id)
        )?;

        if source_in_use {
            return Err(PlanIsNotRunnableError::SourceConnectionInUse).context(format!(
                "Source connection '{}' is in use",
                plan.source_connection_id
            ));
        }
        if target_in_use {
            return Err(PlanIsNotRunnableError::TargetConnectionInUse).context(format!(
                "Target connection '{}' is in use",
                plan.target_connection_id
            ));
        }
        Ok(())
    }

    async fn create_changesets(
        &self,
        deployment_model: Option<DeploymentModel>,
        deltas: &Vec<Delta>,
        ctx: &mut DeploymentContext,
    ) -> Result<()> {
        for (i, delta) in deltas.into_iter().enumerate() {
            if delta.source_ddl == delta.target_ddl {
                ctx.progress(format!(
                    "Skipping changeset for {}.{} because source and target DDLs are the same",
                    delta.object_owner, delta.object_name,
                ));
                continue;
            }
            ctx.progress(format!(
                "Creating changeset {} of {} for '{} {}.{}'",
                i + 1,
                deltas.len(),
                delta.object_type,
                delta.object_owner,
                delta.object_name
            ));

            let changeset: Option<ChangesetModel> = if ctx.is_dry_run() {
                Ok(None)
            } else {
                let deployment_id = deployment_model.as_ref().map(|d| d.id).unwrap();
                match self
                    .changeset_repo
                    .create(
                        deployment_id,
                        &delta.object_type,
                        &delta.object_name,
                        &delta.object_owner,
                        delta.source_ddl.as_deref(),
                        delta.target_ddl.as_deref(),
                    )
                    .await
                {
                    Ok(model) => Ok(Some(model)),
                    Err(e) => {
                        ctx.progress(format!("Failed to create changeset: {}", e));
                        Err(e)
                    }
                }
            }?;

            let scripts = &delta.scripts;
            let rollback_scripts = &delta.rollback_scripts;

            for (script, rollback) in scripts.into_iter().zip(rollback_scripts.into_iter()) {
                if ctx.is_dry_run() {
                    ctx.write_script(script.as_str())?;
                    ctx.write_rollback_script(rollback.as_str())?;
                } else {
                    self.change_repo
                        .create(
                            changeset.as_ref().unwrap().id,
                            script.as_str(),
                            rollback.as_str(),
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }

    pub async fn run(
        &self,
        plan_id: i32,
        fail_fast: bool,
        cutoff_date: NaiveDateTime,
        disable_hooks: Option<bool>,
        ctx: &mut DeploymentContext,
    ) -> Result<Option<i32>> {
        match self.prepare(plan_id, cutoff_date, disable_hooks, ctx).await {
            Ok(result) => match result {
                Some(deployment_id) => {
                    match self
                        .apply(deployment_id, fail_fast, disable_hooks, ctx)
                        .await
                    {
                        Ok(_) => {
                            // Success - update both statuses
                            self.repo
                                .set_status(deployment_id, DeploymentStatus::Success)
                                .await?;
                            self.plan_repo
                                .set_status(plan_id, PlanStatus::Success)
                                .await?;
                            ctx.progress("✅ Deployment completed successfully");
                            Ok(Some(deployment_id))
                        }
                        Err(e) => {
                            let (count, errors) = e
                                .downcast_ref::<DeployError>()
                                .map(|DeployError::Errors(count, errors)| (*count, errors.clone()))
                                .unwrap_or_else(|| (1, vec![e.to_string()]));

                            ctx.progress(format!("❌ Deployment failed with {} error(s)", count));
                            self.plan_repo
                                .set_status(plan_id, PlanStatus::Error)
                                .await?;
                            self.repo.set_error(deployment_id, &errors).await?;
                            Err(e)
                        }
                    }
                }
                // Ifd it's a dry mode.
                None => Ok(None),
            },
            Err(e) => {
                // Failure during preparation
                self.plan_repo
                    .set_status(plan_id, PlanStatus::Error)
                    .await?;
                ctx.progress(format!("❌ Failed to prepare deployment: {}", e));
                Err(e)
            }
        }
    }

    pub async fn prepare(
        &self,
        plan_id: i32,
        cutoff_date: NaiveDateTime,
        disable_hooks: Option<bool>,
        ctx: &mut DeploymentContext,
    ) -> Result<Option<i32>> {
        let plan = self.plan_repo.get_by_id(plan_id).await?;
        let source_client = self.get_client(plan.source_connection_id).await?;
        let target_client = self.get_client(plan.target_connection_id).await?;

        plan.run_pre_prepare_hooks(disable_hooks, &source_client, ctx)
            .await?;

        let plan_id = plan.id;

        ctx.progress(format!("Preparing deployment for plan '{}' ...", plan.name));

        let result = async {
            let schemas = plan.get_schemas();

            ctx.progress(format!("Validating schemas..."));
            self.validate_schemas(&source_client, &schemas).await?;
            self.validate_schemas(&target_client, &schemas).await?;

            let exclude_object_types = plan.get_exclude_object_types();
            let exclude_object_names = plan.get_exclude_object_names();

            ctx.progress(format!("Fetching source objects..."));
            let sources = source_client
                .get_objects_with_ddls(
                    schemas.clone(),
                    Some(cutoff_date),
                    exclude_object_types.clone(),
                    exclude_object_names.clone(),
                )
                .await?;
            ctx.progress(format!("Fetched {} source objects", sources.len()));

            ctx.progress(format!("Fetching target objects..."));

            let targets = target_client
                .get_objects_with_ddls(schemas, None, exclude_object_types, exclude_object_names)
                .await?;
            ctx.progress(format!("Fetched {} target objects", targets.len()));

            ctx.progress(format!("Finding deltas..."));

            let deltas = find_deltas(sources, targets, plan.disable_all_drops);
            let deltas = if !plan.disable_all_drops {
                let disabled_drop_types = plan.disabled_drop_types.clone().map(|sl| sl.0);
                with_disabled_drop_types_excluded(deltas, disabled_drop_types.clone())
            } else {
                deltas
            };

            ctx.progress(format!("Creating deployment..."));
            let deployment_model: Option<DeploymentModel> = if ctx.is_dry_run() {
                Ok(None)
            } else {
                match self
                    .create_deployment(&plan, cutoff_date, disable_hooks)
                    .await
                {
                    Ok(model) => Ok(Some(model)),
                    Err(e) => {
                        ctx.progress(format!("Failed to create deployment: {}", e));
                        Err(e)
                    }
                }
            }?;

            if deltas.is_empty() {
                ctx.progress(format!("No changes found for plan '{}'", plan.name));
                warn!("No changes found for plan {}", plan.name);
                return Ok(None);
            }
            let deployment_id: Option<i32> = deployment_model.as_ref().map(|d| d.id);
            self.create_changesets(deployment_model, &deltas, ctx)
                .await?;

            Ok(deployment_id)
        }
        .await;

        let final_status = if result.is_ok() {
            PlanStatus::Success
        } else {
            PlanStatus::Error
        };

        self.plan_repo.set_status(plan_id, final_status).await?;

        plan.run_post_prepare_hooks(disable_hooks, &source_client, ctx)
            .await?;

        result
    }

    pub async fn apply(
        &self,
        deployment_id: i32,
        fail_fast: bool,
        disable_hooks: Option<bool>,
        ctx: &mut DeploymentContext,
    ) -> Result<()> {
        ctx.progress(format!("Applying changes ..."));

        let deployment = self.repo.get_by_id(deployment_id).await?;
        let plan_id = deployment.plan_id;

        let plan = self.plan_repo.get_by_id(plan_id).await?;
        self.validate_plan_is_runnable(&plan, ctx).await?;

        ctx.progress(format!("Getting target client for plan '{}'...", plan.name));
        let client = self.get_client(plan.target_connection_id).await?;

        plan.run_pre_apply_hooks(disable_hooks, &client, ctx)
            .await?;

        let result = async {
            ctx.progress(format!("Setting plan '{}' status to RUNNING...", plan.name));
            self.plan_repo
                .set_status(plan_id, PlanStatus::Running)
                .await?;

            ctx.progress(format!(
                "Setting deployment '{}' status to RUNNING...",
                deployment_id
            ));
            self.repo
                .set_status(deployment_id, DeploymentStatus::Running)
                .await?;

            ctx.progress(format!(
                "Retrieving changesets for deployment ID('{}')...",
                deployment_id
            ));
            let changesets_with_changes = self
                .changeset_repo
                .get_by_deployment_id_with_changes(deployment_id)
                .await?;

            if changesets_with_changes.is_empty() {
                ctx.progress(format!(
                    "No changesets found for deployment {}. Skipping deployment...",
                    deployment_id
                ));
                return Ok(());
            }

            let mut errors: Vec<String> = Vec::new();
            for (changeset, changes) in changesets_with_changes {
                let object_type = changeset.object_type.clone();
                let object_owner = changeset.object_owner.clone();
                let object_name = changeset.object_name.clone();

                if changes.is_empty() {
                    ctx.progress(format!(
                        "Skipping changeset for '{} {}.{}' because no changes were found",
                        object_type, object_owner, object_name
                    ));
                    continue;
                }

                let mut changeset_active: ChangesetActiveModel = changeset.into_active_model();
                changeset_active.start();

                self.changeset_repo
                    .save_changeset(&changeset_active)
                    .await?;

                let mut changeset_errors = Vec::new();

                for change in changes {
                    let change_id = change.id;
                    let script = change.script.clone();

                    let mut change_active: ChangeActiveModel = change.into_active_model();
                    change_active.start();

                    self.change_repo.save_change(&change_active).await?;

                    ctx.progress(format!(
                        "Executing change for '{} {}.{}'",
                        object_type, object_owner, object_name
                    ));
                    let result = match client.execute(&script).await {
                        Ok(_) => {
                            change_active.end(None);
                            Ok(())
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Change {} ({}): {}", change_id, object_name, e);

                            // Update change status with error message
                            change_active.end(Some(e.to_string()));

                            // Collect errors for later reporting
                            changeset_errors.push(error_msg.clone());
                            errors.push(error_msg);

                            Err(e)
                        }
                    };

                    self.change_repo.save_change(&change_active).await?;

                    if result.is_err() && fail_fast {
                        return Err(DeployError::Errors(1, errors).into());
                    }
                }

                if changeset_errors.is_empty() {
                    changeset_active.end(None);
                } else {
                    changeset_active.end(Some(StringList(changeset_errors)));
                }

                self.changeset_repo
                    .save_changeset(&changeset_active)
                    .await?;
            }

            if !errors.is_empty() {
                return Err(DeployError::Errors(errors.len(), errors).into());
            }
            Ok(())
        }
        .await;

        plan.run_post_apply_hooks(disable_hooks, &client, ctx)
            .await?;

        result
    }

    pub async fn find_last_deployment_by_plan_id(
        &self,
        plan_id: i32,
    ) -> Result<Option<DeploymentModel>> {
        self.repo.find_last_by_plan_id(plan_id).await
    }

    pub async fn find_last_successful_deployment_by_plan_id(
        &self,
        plan_id: i32,
    ) -> Result<Option<DeploymentModel>> {
        self.repo.find_last_successful_by_plan_id(plan_id).await
    }

    pub async fn fetch_deployments(
        &self,
        plan_id: Option<i32>,
        limit: Option<u32>,
        order: Option<String>,
    ) -> Result<Vec<DeploymentModel>> {
        self.repo.fetch_deployments(plan_id, limit, order).await
    }

    pub async fn find_changes_by_deployment_id(
        &self,
        deployment_id: i32,
    ) -> Result<Vec<ChangeModel>> {
        self.change_repo.find_by_deployment_id(deployment_id).await
    }

    pub async fn get_changeset_count_by_deployment_id(&self, deployment_id: i32) -> Result<u64> {
        self.changeset_repo
            .get_count_by_deployment_id(deployment_id)
            .await
    }

    pub async fn get_change_count_by_deployment_id(&self, deployment_id: i32) -> Result<u64> {
        self.change_repo
            .get_count_by_deployment_id(deployment_id)
            .await
    }

    pub async fn find_changesets_by_deployment_id(
        &self,
        deployment_id: i32,
    ) -> Result<Vec<ChangesetModel>> {
        self.changeset_repo
            .find_by_deployment_id(deployment_id)
            .await
    }

    pub async fn find_changesets_with_changes_by_deployment_id(
        &self,
        deployment_id: i32,
    ) -> Result<Option<Vec<(ChangesetModel, Vec<ChangeModel>)>>> {
        self.changeset_repo
            .find_by_deployment_id_with_changes(deployment_id)
            .await
    }

    async fn prepare_rollback(
        &self,
        deployment_id: i32,
        progress: &ProgressReporter,
    ) -> Result<Option<u64>> {
        let plan = self.plan_repo.get_by_id(deployment_id).await?;
        progress.report(format!(
            "Preparing rollback for deployment {} for plan '{}' ...",
            deployment_id, plan.name
        ));
        let changesets_with_changes = self
            .changeset_repo
            .find_by_deployment_id_with_changes(deployment_id)
            .await?;
        if changesets_with_changes.is_none() {
            progress.report(format!("No changes found for deployment {}", deployment_id));
            return Ok(None);
        }
        // first create rollbacks
        progress.report(format!("Creating rollback actions..."));

        let mut change_count = 0;
        // rollbacks are saved in reverse change order to keep dependencies.
        for (changeset, changes) in changesets_with_changes.unwrap().into_iter().rev() {
            for change in &changes {
                // Add & here to borrow instead of move
                progress.report(format!(
                    "Creating rollback for '{} {}.{}'",
                    changeset.object_type, changeset.object_owner, changeset.object_name
                ));

                self.rollback_repo
                    .create(change.id, change.rollback_script.clone())
                    .await?;
            }
            change_count += changes.len() as u64;
        }

        Ok(Some(change_count))
    }

    async fn execute_rollbacks(
        &self,
        deployment_id: i32,
        disable_hooks: Option<bool>,
        progress: &ProgressReporter,
    ) -> Result<()> {
        let deployment = self.repo.get_by_id(deployment_id).await?;
        let plan = self.plan_repo.get_by_id(deployment.plan_id).await?;
        let client = self.get_client(plan.target_connection_id).await?;

        plan.run_pre_rollback_hooks(disable_hooks, &client, progress)
            .await?;

        let rollbacks = self
            .rollback_repo
            .get_rollbacks_with_changes_and_changesets(deployment_id)
            .await?;

        if rollbacks.is_none() {
            progress.report("No changes to rollback".to_string());
            return Ok(());
        }

        let rollbacks = rollbacks.unwrap();

        self.plan_repo
            .set_status(plan.id, PlanStatus::RollingBack)
            .await?;

        self.repo
            .set_status(deployment_id, DeploymentStatus::RollingBack)
            .await?;

        let result: Result<()> = async {
            for (i, (rollback, change, changeset)) in rollbacks.iter().enumerate() {
                progress.report(format!(
                    "Executing rollback {} of {} for '{} {}.{}'",
                    i + 1,
                    rollbacks.len(),
                    changeset.object_type,
                    changeset.object_owner,
                    changeset.object_name
                ));

                self.rollback_repo
                    .set_status(rollback.id, RollbackStatus::Running)
                    .await?;

                match client.execute(&rollback.script).await {
                    Ok(_) => {
                        self.change_repo
                            .set_status(change.id, ChangeStatus::RolledBack)
                            .await?;
                        self.rollback_repo
                            .set_status(rollback.id, RollbackStatus::Success)
                            .await?;
                    }
                    Err(e) => {
                        self.change_repo
                            .set_status(change.id, ChangeStatus::RollbackError)
                            .await?;
                        self.rollback_repo
                            .set_error(rollback.id, e.to_string())
                            .await?;

                        // Set final statuses before returning error
                        self.repo
                            .set_status(deployment_id, DeploymentStatus::RollbackError)
                            .await?;
                        self.plan_repo
                            .set_status(plan.id, PlanStatus::RollbackError)
                            .await?;

                        return Err(e);
                    }
                }
            }

            Ok(())
        }
        .await;

        let rollback_result = plan
            .run_post_rollback_hooks(disable_hooks, &client, progress)
            .await;

        // Handle the result and set appropriate final statuses
        match rollback_result.and(result) {
            Ok(_) => {
                self.repo
                    .set_status(deployment_id, DeploymentStatus::RolledBack)
                    .await?;
                self.plan_repo
                    .set_status(plan.id, PlanStatus::RolledBack)
                    .await?;
                progress.report("Rollback completed successfully".to_string());
                Ok(())
            }
            Err(e) => {
                // Error statuses already set in the loop above
                progress.report(format!("Rollback failed: {}", e));
                Err(e)
            }
        }
    }

    async fn rollback_by_deployment_id(
        &self,
        deployment_id: i32,
        disable_hooks: Option<bool>,
        progress: ProgressReporter,
    ) -> Result<()> {
        match self.prepare_rollback(deployment_id, &progress).await? {
            Some(change_count) if change_count > 0 => {
                self.execute_rollbacks(deployment_id, disable_hooks, &progress)
                    .await?;
                Ok(())
            }
            Some(_) | None => {
                progress.report("No changes to rollback".to_string());
                Ok(())
            }
        }
    }

    pub async fn rollback(
        &self,
        plan_id: i32,
        disable_hooks: Option<bool>,
        progress: ProgressReporter,
    ) -> Result<()> {
        let deployment = self
            .repo
            .find_last_successful_by_plan_id(plan_id)
            .await?
            .ok_or_else(|| anyhow!("No successful deployment found for plan {}", plan_id))?;

        self.rollback_by_deployment_id(deployment.id, disable_hooks, progress)
            .await
    }
}
