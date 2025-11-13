pub mod cmd_connections;
pub mod cmd_db;
pub mod cmd_deployments;
pub mod cmd_init;
pub mod cmd_plans;
pub mod cmd_version;

use clap::Subcommand;

use crate::cli::commands::{
    cmd_connections::ConnectionCommands,
    cmd_db::DbCommands,
    cmd_deployments::DeploymentCommands,
    cmd_init::InitCommands,
    cmd_plans::{PlanCommands, PlansRunArgs},
    cmd_version::VersionCommand,
};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage repository database
    Db {
        #[command(subcommand)]
        action: DbCommands,
    },

    /// Manage connections
    Connections {
        #[command(subcommand)]
        action: ConnectionCommands,
    },

    /// Plan and run database deployments
    Plans {
        #[command(subcommand)]
        action: PlanCommands,
    },

    /// Deployment commands
    Deployments {
        #[command(subcommand)]
        action: DeploymentCommands,
    },

    /// Deploy a plan, alias for `plans run`
    Deploy(PlansRunArgs),

    /// Initialize application
    Init {
        #[command(subcommand)]
        action: Option<InitCommands>,
    },

    /// Print version
    Version(VersionCommand),
}

pub trait ExitOnErr<T> {
    fn exit_on_err(self, msg: &str) -> T;
}

impl<T, E: std::fmt::Display> ExitOnErr<T> for Result<T, E> {
    fn exit_on_err(self, msg: &str) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                eprintln!("❌ {}: {}", msg, e);
                std::process::exit(1);
            }
        }
    }
}
