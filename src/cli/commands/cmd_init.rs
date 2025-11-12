use std::path::Path;

use clap::Subcommand;
use inquire::Confirm;
use tracing::info;

use crate::{
    cli::{Context, commands::ExitOnErr},
    db::{init_db, migrations},
    utils::init::get_env_file_with_defaults,
};

#[derive(Subcommand, Debug)]
pub enum InitCommands {
    /// initialize .env file and database using defaults
    All {
        /// Overwrite existing .env file
        #[arg(short, long, default_value_t = false)]
        overwrite: bool,
    },

    /// initialize .env file
    Env {
        /// Overwrite existing .env file
        #[arg(short, long, default_value_t = false)]
        overwrite: bool,
    },

    /// initializes database. Run this command after initializing .env file
    /// Runs migrations.
    Db,
}

pub async fn execute(action: &InitCommands, ctx: &Context<'_>) {
    match action {
        InitCommands::All { overwrite } => {
            init_env_file(*overwrite).await;
            init_database(ctx).await;
        }
        InitCommands::Env { overwrite } => init_env_file(*overwrite).await,
        InitCommands::Db => init_database(ctx).await,
    }
}

async fn init_env_file(overwrite: bool) {
    // Changed from Option<bool> to bool
    let env_file =
        get_env_file_with_defaults("env.default.jinja").exit_on_err("Failed to get env file");

    // check if .env file exists in the current directory
    if Path::new(".env").exists() && !overwrite {
        let should_overwrite =
            Confirm::new("A .env file already exists. Do you want to overwrite it?")
                .with_default(false)
                .prompt()
                .unwrap_or(false);

        if !should_overwrite {
            println!("Exiting...");
            return;
        }
    }

    std::fs::write(".env", env_file).exit_on_err("Failed to create .env file");
    println!("✅ Successfully created .env file.");
}

async fn init_database(ctx: &Context<'_>) {
    let db = init_db(&ctx.settings)
        .await
        .exit_on_err("Failed to connect to database");

    info!("Running migrations");
    migrations::up(&db)
        .await
        .exit_on_err("Failed to run migrations");
}
