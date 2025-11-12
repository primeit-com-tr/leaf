pub mod connection_service;
pub mod deployment_service;
pub mod plan_service;

use anyhow::{Context, Result};
use std::sync::Arc;

pub use connection_service::ConnectionService;
pub use deployment_service::DeploymentService;
pub use plan_service::PlanService;

use crate::{
    config::Settings,
    db::init_db,
    repo::{
        ChangeRepository, ChangesetRepository, ConnectionRepository, DeploymentRepository,
        PlanRepository, RollbackRepository,
    },
};

pub struct AppServices {
    pub plan_service: PlanService,
    pub deployment_service: DeploymentService,
    pub connection_service: ConnectionService,
}

impl AppServices {
    pub async fn new(settings: &Settings) -> Result<Self> {
        let connection_repo = Arc::new(ConnectionRepository::new(
            init_db(&settings)
                .await
                .context("Failed to initialize database for ConnectionRepository")?,
        ));
        let plan_repo = Arc::new(PlanRepository::new(
            init_db(&settings)
                .await
                .expect("Failed to initialize database for PlanRepository"),
        ));

        let deployment_repo =
            Arc::new(DeploymentRepository::new(init_db(&settings).await.expect(
                "Failed to initialize database for DeploymentRepository",
            )));

        let changeset_repo = Arc::new(ChangesetRepository::new(
            init_db(&settings)
                .await
                .expect("Failed to initialize database for ChangesetRepository"),
        ));

        let change_repo = Arc::new(ChangeRepository::new(
            init_db(&settings)
                .await
                .expect("Failed to initialize database for ChangeRepository"),
        ));

        let rollback_repo = Arc::new(RollbackRepository::new(
            init_db(&settings)
                .await
                .expect("Failed to initialize database for RollbackRepository"),
        ));

        Ok(Self {
            plan_service: PlanService::new(
                settings.clone(),
                plan_repo.clone(),
                connection_repo.clone(),
            ),
            connection_service: ConnectionService::new(connection_repo.clone()),
            deployment_service: DeploymentService::new(
                deployment_repo,
                plan_repo.clone(),
                connection_repo,
                changeset_repo,
                change_repo,
                rollback_repo,
            ),
        })
    }
}
