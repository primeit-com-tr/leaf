use crate::{
    delta::{delta::with_disabled_drop_types_excluded, find_deltas},
    entities::{
        ChangeActiveModel, ChangeModel, ChangesetActiveModel, ChangesetModel,
        DeploymentActiveModel, DeploymentModel, PlanModel, RollbackModel,
    },
    errors::{DeployError, SchemaValidationError},
    oracle::OracleClient,
    repo::{
        ChangeRepository, ChangesetRepository, ConnectionRepository, DeploymentRepository,
        PlanRepository, rollback_repo::RollbackRepository,
    },
    types::{
        ChangeStatus, Delta, DeploymentItem, DeploymentResultDetails, DeploymentResultType,
        DeploymentStatus, DryDeployment, PlanStatus, RollbackStatus, StringList,
    },
    utils::ProgressReporter,
};
use anyhow::{Context, Result, anyhow};
use chrono::{NaiveDateTime, Utc};
use sea_orm::IntoActiveModel;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tokio::try_join;
use tracing::{debug, warn};
/// Service layer for Deployment business logic
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

    async fn check_if_plan_is_runnable(
        &self,
        plan: &PlanModel,
        progress: &ProgressReporter,
    ) -> Result<()> {
        progress.report(format!("Checking if plan '{}' is runnable...", plan.name));
        if self.plan_repo.is_running(plan.id).await? {
            return Err(anyhow!("Plan is already running"));
        }
        progress.report(format!(
            "Checking if source or target connection is in use..."
        ));
        let (source_in_use, target_in_use) = try_join!(
            self.plan_repo
                .is_connection_in_use(plan.source_connection_id),
            self.plan_repo
                .is_connection_in_use(plan.target_connection_id)
        )?;

        if source_in_use {
            return Err(anyhow!("Source connection is in use"));
        }
        if target_in_use {
            return Err(anyhow!("Target connection is in use"));
        }
        Ok(())
    }

    pub async fn run(
        &self,
        plan: PlanModel,
        cutoff_date: NaiveDateTime,
        is_dry_run: Option<bool>,
        progress: ProgressReporter,
    ) -> Result<DeploymentResultType> {
        let plan_id = plan.id;

        self.check_if_plan_is_runnable(&plan, &progress).await?;

        progress.report(format!("Setting plan '{}' status to RUNNING...", plan.name));
        self.plan_repo
            .set_status(plan_id, PlanStatus::Running)
            .await?;

        let result = async {
            let is_dry_run = is_dry_run.unwrap_or(false);

            let source_client = self.get_client(plan.source_connection_id).await?;
            let target_client = self.get_client(plan.target_connection_id).await?;
            let schemas = plan.get_schemas();

            progress.report(format!("Validating schemas..."));
            self.validate_schemas(&source_client, &schemas).await?;
            self.validate_schemas(&target_client, &schemas).await?;

            let exclude_object_types = plan.get_exclude_object_types();
            let exclude_object_names = plan.get_exclude_object_names();

            let start_time = Utc::now().naive_utc();

            progress.report(format!("Fetching source objects..."));
            let sources = source_client
                .get_objects_with_ddls(
                    schemas.clone(),
                    Some(cutoff_date),
                    exclude_object_types.clone(),
                    exclude_object_names.clone(),
                )
                .await?;
            progress.report(format!("Fetched {} source objects", sources.len()));

            progress.report(format!("Fetching target objects..."));
            let targets = target_client
                .get_objects_with_ddls(schemas, None, exclude_object_types, exclude_object_names)
                .await?;
            progress.report(format!("Fetched {} target objects", targets.len()));

            progress.report(format!("Finding deltas..."));
            let deltas = find_deltas(sources, targets);

            let disabled_drop_types = plan.disabled_drop_types.clone().map(|sl| sl.0);
            let deltas = with_disabled_drop_types_excluded(deltas, disabled_drop_types.clone());

            if is_dry_run {
                return Ok(DeploymentResultType::DryDeployment(DryDeployment {
                    plan: plan.clone(),
                    deltas,
                }));
            }

            progress.report(format!("Creating deployment..."));
            let deployment_model = self
                .create_deployment(&plan, cutoff_date, start_time)
                .await?;

            let mut deployment: DeploymentActiveModel = deployment_model.into_active_model();
            let deployment_id = deployment.id.clone().unwrap();

            if deltas.is_empty() {
                progress.report(format!("No changes found for plan '{}'", plan.name));
                warn!("No changes found for plan {}", plan.name);
                deployment.end(None);
                return Ok(DeploymentResultType::Deployment(
                    self.repo.save_deployment(deployment).await?,
                ));
            }

            progress.report(format!("Creating changesets..."));
            self.create_changesets(deployment_id, deltas).await?;

            deployment.start();
            let mut deployment = self
                .repo
                .save_deployment(deployment)
                .await?
                .into_active_model();

            let res = self
                .deploy_by_id(deployment_id, plan.fail_fast, progress)
                .await;

            deployment.end(
                res.as_ref()
                    .err()
                    .map(|e| anyhow::Error::msg(e.to_string())),
            );
            if res.is_err() {
                return Err(res.err().unwrap());
            }

            Ok(DeploymentResultType::Deployment(
                self.repo.save_deployment(deployment).await?,
            ))
        }
        .await;

        let final_status = if result.is_ok() {
            PlanStatus::Success
        } else {
            PlanStatus::Error
        };

        let _ = self.plan_repo.set_status(plan_id, final_status).await;

        result
    }

    async fn create_deployment(
        &self,
        plan: &PlanModel,
        cutoff_date: NaiveDateTime,
        start_time: NaiveDateTime,
    ) -> Result<DeploymentModel> {
        self.repo
            .create(
                plan.id,
                cutoff_date,
                serde_json::to_string(&plan)?,
                start_time,
            )
            .await
    }

    async fn create_changesets(&self, deployment_id: i32, deltas: Vec<Delta>) -> Result<()> {
        for delta in deltas {
            if delta.source_ddl == delta.target_ddl {
                debug!(
                    "Skipping changeset for {}.{} because source and target DDLs are the same",
                    delta.object_owner, delta.object_name
                );
                continue;
            }

            let changeset = self
                .changeset_repo
                .create(
                    deployment_id,
                    &delta.object_type,
                    &delta.object_name,
                    &delta.object_owner,
                    delta.source_ddl.as_deref(),
                    delta.target_ddl.as_deref(),
                )
                .await?;

            let scripts = delta.scripts;
            let rollback_scripts = delta.rollback_scripts;

            for (script, rollback) in scripts.into_iter().zip(rollback_scripts.into_iter()) {
                self.change_repo
                    .create(changeset.id, script.as_str(), rollback.as_str())
                    .await?;
            }
        }
        Ok(())
    }

    async fn deploy_by_id(
        &self,
        deployment_id: i32,
        fail_fast: bool,
        progress: ProgressReporter,
    ) -> Result<()> {
        progress.report(format!("Applying changes ..."));

        let deployment = self.repo.get_by_id(deployment_id).await?;
        let plan_id = deployment.plan_id;

        let changesets_with_changes = self
            .changeset_repo
            .get_by_deployment_id_with_changes(deployment_id)
            .await?;

        if changesets_with_changes.is_empty() {
            warn!("No changesets found for deployment {}", deployment_id);
            return Ok(());
        }

        let plan = self.plan_repo.get_by_id(plan_id).await?;
        let client = self.get_client(plan.target_connection_id).await?;

        let mut errors: Vec<String> = Vec::new();

        for (changeset, changes) in changesets_with_changes {
            if changes.is_empty() {
                continue;
            }

            let object_type = changeset.object_type.clone();
            let object_owner = changeset.object_owner.clone();
            let object_name = changeset.object_name.clone();

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

                progress.report(format!(
                    "Executing change for '{} {}.{}'",
                    object_type, object_owner, object_name
                ));
                let result = match client.execute(&script).await {
                    Ok(_) => {
                        change_active.end(None);
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Change {} ({}): {}", change_id, object_name, e);

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

    pub async fn get_deployment_details_from_result(
        &self,
        result: DeploymentResultType,
    ) -> Result<DeploymentResultDetails> {
        match result {
            DeploymentResultType::Deployment(deployment) => {
                let plan = self.plan_repo.get_by_id(deployment.plan_id).await?;
                let source_connection = self
                    .connection_repo
                    .get_by_id(plan.source_connection_id)
                    .await?;
                let target_connection = self
                    .connection_repo
                    .get_by_id(plan.target_connection_id)
                    .await?;

                let changesets_with_changes = self
                    .changeset_repo
                    .get_by_deployment_id_with_changes(deployment.id)
                    .await?;

                let mut items = Vec::new();

                for (changeset, changes) in changesets_with_changes {
                    let mut item = DeploymentItem {
                        object_type: changeset.object_type,
                        object_name: changeset.object_name,
                        object_owner: changeset.object_owner,
                        source_ddl: changeset.source_ddl,
                        target_ddl: changeset.target_ddl,
                        scripts: Vec::new(),
                        rollback_scripts: Vec::new(),
                        status: Some(changeset.status),
                        errors: changeset.errors.map(|sl| sl.into_inner()),
                    };

                    for change in changes {
                        let script = change.script.clone();
                        let rollback_script = change.rollback_script.clone();
                        item.scripts.push(script);
                        item.rollback_scripts.push(rollback_script);
                    }

                    items.push(item);
                }

                Ok(DeploymentResultDetails {
                    is_dry_run: false,
                    id: Some(deployment.id),
                    plan_id: deployment.plan_id,
                    plan_name: plan.name,
                    source_connection_id: plan.source_connection_id,
                    source_connection_name: source_connection.name,
                    target_connection_id: plan.target_connection_id,
                    target_connection_name: target_connection.name,
                    status: Some(deployment.status),
                    started_at: deployment.started_at,
                    ended_at: deployment.ended_at,
                    items,
                })
            }
            DeploymentResultType::DryDeployment(dry) => {
                let plan = self.plan_repo.get_by_id(dry.plan.id).await?;
                let source_connection = self
                    .connection_repo
                    .get_by_id(plan.source_connection_id)
                    .await?;
                let target_connection = self
                    .connection_repo
                    .get_by_id(plan.target_connection_id)
                    .await?;

                let mut items = Vec::new();
                for delta in &dry.deltas {
                    let mut item = DeploymentItem {
                        object_type: delta.object_type.clone(),
                        object_name: delta.object_name.clone(),
                        object_owner: delta.object_owner.clone(),
                        scripts: Vec::new(),
                        rollback_scripts: Vec::new(),
                        ..Default::default()
                    };

                    for script in &delta.scripts {
                        item.scripts.push(script.clone());
                    }

                    for script in &delta.rollback_scripts {
                        item.rollback_scripts.push(script.clone());
                    }

                    items.push(item);
                }

                Ok(DeploymentResultDetails {
                    is_dry_run: true,
                    id: None,
                    plan_id: dry.plan.id,
                    plan_name: plan.name,
                    source_connection_id: plan.source_connection_id,
                    source_connection_name: source_connection.name,
                    target_connection_id: plan.target_connection_id,
                    target_connection_name: target_connection.name,
                    status: None,
                    started_at: None,
                    ended_at: None,
                    items: items,
                })
            }
        }
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
    ) -> Result<Option<BTreeMap<ChangesetModel, Vec<(ChangeModel, RollbackModel)>>>> {
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
        let mut rollbacks: BTreeMap<ChangesetModel, Vec<(ChangeModel, RollbackModel)>> =
            BTreeMap::new();

        // first create rollbacks
        progress.report(format!("Creating rollback actions..."));
        for (changeset, changes) in changesets_with_changes.unwrap().into_iter().rev() {
            let changeset_group = rollbacks.entry(changeset.clone()).or_default();

            for change in changes {
                progress.report(format!(
                    "Creating rollback for '{} {}.{}'",
                    changeset.object_type, changeset.object_owner, changeset.object_name
                ));

                let rollback = self
                    .rollback_repo
                    .create(change.id, change.rollback_script.clone())
                    .await?;

                changeset_group.push((change.clone(), rollback));
            }
        }

        Ok(Some(rollbacks))
    }
    async fn execute_rollbacks(
        &self,
        deployment_id: i32,
        rollbacks: BTreeMap<ChangesetModel, Vec<(ChangeModel, RollbackModel)>>,
        progress: &ProgressReporter,
    ) -> Result<()> {
        let deployment = self.repo.get_by_id(deployment_id).await?;
        let plan = self.plan_repo.get_by_id(deployment.plan_id).await?;

        self.plan_repo
            .set_status(plan.id, PlanStatus::RollingBack)
            .await?;

        self.repo
            .set_status(deployment_id, DeploymentStatus::RollingBack)
            .await?;

        let result: Result<()> = async {
            let client = self.get_client(plan.target_connection_id).await?;

            for (changeset, change_rollback_vec) in &rollbacks {
                for (i, (change, rollback)) in change_rollback_vec.iter().enumerate() {
                    progress.report(format!(
                        "Executing rollback {} of {} for '{} {}.{}'",
                        i + 1,
                        change_rollback_vec.len(),
                        changeset.object_type,
                        changeset.object_owner,
                        changeset.object_name
                    ));
                    println!(
                        "Executing rollback {} of {} for '{} {}.{}'",
                        i + 1,
                        change_rollback_vec.len(),
                        changeset.object_type,
                        changeset.object_owner,
                        changeset.object_name
                    );

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
            }

            Ok(())
        }
        .await;

        // Handle the result and set appropriate final statuses
        match result {
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
        progress: ProgressReporter,
    ) -> Result<()> {
        match self.prepare_rollback(deployment_id, &progress).await? {
            Some(rollbacks) => {
                self.execute_rollbacks(deployment_id, rollbacks, &progress)
                    .await
            }
            None => {
                progress.report("No changes to rollback".to_string());
                Ok(())
            }
        }
    }

    pub async fn rollback(&self, plan_id: i32, progress: ProgressReporter) -> Result<()> {
        let deployment = self
            .repo
            .find_last_successful_by_plan_id(plan_id)
            .await?
            .ok_or_else(|| anyhow!("No successful deployment found for plan {}", plan_id))?;

        self.rollback_by_deployment_id(deployment.id, progress)
            .await
    }
}
