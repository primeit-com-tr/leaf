mod commands;

use clap::Parser;
use colored::Colorize;

use crate::cli::commands::{
    Commands, cmd_connections, cmd_db, cmd_deployments, cmd_init, cmd_plans, cmd_version,
};
use crate::config::Settings;
use crate::services::AppServices;

pub struct Context<'a> {
    pub settings: &'a Settings,
    pub services: &'a AppServices,
}

#[derive(Parser, Debug)]
#[command(
    name = "leaf",
    about = "Leaf CLI application",
    long_about = format!(
r#"{} - {}
by {} - {}"#,
"LEAF".green().bold(),
"Simple and powerful database deployment tool.",
"primeit".blue(), "https://primeit.com.tr".on_bright_black()
))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub async fn execute(&self, ctx: &Context<'_>) {
        match &self.command {
            Some(Commands::Db { action }) => cmd_db::execute(action, ctx.settings).await,
            Some(Commands::Connections { action }) => cmd_connections::execute(action, ctx).await,
            Some(Commands::Plans { action }) => cmd_plans::execute(action, ctx).await,
            Some(Commands::Deployments { action }) => cmd_deployments::execute(action, ctx).await,
            Some(Commands::Deploy(args)) => {
                cmd_plans::execute(&cmd_plans::PlanCommands::Run(args.clone()), ctx).await
            }
            Some(Commands::Init { action }) => cmd_init::execute(action, ctx).await,
            Some(Commands::Version(action)) => cmd_version::execute(action, ctx.settings).await,
            None => {
                // No command means start server - handled in main
            }
        }
    }

    pub fn should_run_main(&self) -> bool {
        self.command.is_none()
    }
}
