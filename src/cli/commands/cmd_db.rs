use crate::cli::commands::ExitOnErr;
use crate::db::init_db;
use crate::{config::Settings, db::migrations};
use clap::Subcommand;
use tracing::info;

#[derive(Subcommand, Debug)]
pub enum MigrateCommands {
    /// Run pending migrations
    Up,

    /// Rollback migrations
    Down {
        #[arg(long, default_value = "1")]
        steps: u32,
    },
}

#[derive(Subcommand, Debug)]
pub enum DbCommands {
    /// Run database migrations
    Migrate {
        #[command(subcommand)]
        action: MigrateCommands,
    },
}

pub async fn execute(action: &DbCommands, settings: &Settings) {
    match action {
        DbCommands::Migrate { action } => match action {
            MigrateCommands::Up => up(settings).await,
            MigrateCommands::Down { steps } => down(settings, *steps).await,
        },
    }
}

pub async fn up(settings: &Settings) {
    let db = init_db(settings)
        .await
        .exit_on_err("Failed to connect to database");

    info!("Running migrations");
    migrations::up(&db)
        .await
        .exit_on_err("Failed to run migrations");
}

pub async fn down(settings: &Settings, steps: u32) {
    let db = init_db(settings)
        .await
        .exit_on_err("Failed to connect to database");

    info!("Rolling back {} migration(s)", steps);
    migrations::down(&db, steps)
        .await
        .exit_on_err("Failed to rollback migrations");
}
