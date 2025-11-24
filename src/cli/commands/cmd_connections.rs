use crate::cli::{Context, commands::ExitOnErr};
use clap::Subcommand;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Confirm;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, Style, object::Rows},
};

#[derive(Subcommand, Debug)]
pub enum ConnectionCommands {
    /// Create a new connection
    Add {
        /// Connection name (unique)
        #[arg(long, required = true)]
        name: String,

        /// Username
        #[arg(long, required = true)]
        username: String,

        /// Password
        #[arg(long, required = true)]
        password: String,

        /// Connection string
        #[arg(long, required = true)]
        connection_string: String,
    },

    /// Test a connection
    Test {
        #[arg(long, required = true)]
        username: String,

        #[arg(long, required = true)]
        password: String,

        #[arg(long, required = true)]
        connection_string: String,
    },

    /// Same as test, but for saved connections
    Ping {
        #[arg(required = true)]
        name: String,
    },

    /// Delete a connection
    Remove {
        #[arg(required = true)]
        name: String,
    },

    /// Remove all connections
    Prune {
        #[arg(short, long)]
        yes: bool,
    },

    /// List all connections
    List,
}

#[derive(Tabled)]
struct ConnectionRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "Name")]
    name: String,

    #[tabled(rename = "Username")]
    username: String,

    #[tabled(rename = "Connection String")]
    connection_string: String,
}

pub async fn execute(action: &ConnectionCommands, ctx: &Context<'_>) {
    match action {
        ConnectionCommands::Add {
            name,
            username,
            password,
            connection_string,
        } => add(ctx, name, username, password, connection_string).await,
        ConnectionCommands::Remove { name, .. } => remove(name, ctx).await,
        ConnectionCommands::Prune { yes } => prune(yes, ctx).await,
        ConnectionCommands::List => list(ctx).await,
        ConnectionCommands::Ping { name } => ping(name, ctx).await,
        ConnectionCommands::Test {
            username,
            password,
            connection_string,
        } => test(username, password, connection_string, ctx).await,
    }
}

pub async fn add(
    ctx: &Context<'_>,
    name: &str,
    username: &str,
    password: &str,
    connection_string: &str,
) {
    ctx.services
        .connection_service
        .create(name, username, password, connection_string)
        .await
        .exit_on_err(&format!("Connection creation failed for '{}'", name));

    println!("✅ Connection created successfully for '{}'", name);
}

pub async fn remove(name: &str, ctx: &Context<'_>) {
    ctx.services
        .connection_service
        .delete_by_name(&name)
        .await
        .exit_on_err(&format!("Failed to delete connection '{}'", name));

    println!("✅ Connection '{}' deleted", name);
}

pub async fn prune(yes: &bool, ctx: &Context<'_>) {
    let proceed = *yes
        || Confirm::new("This will delete all connections. Continue?")
            .with_default(false)
            .prompt()
            .unwrap_or(false);

    if !proceed {
        println!("✅ Aborted");
        return;
    }

    let count = ctx
        .services
        .connection_service
        .prune()
        .await
        .exit_on_err("Failed to delete all connections");

    if count == 0 {
        println!("✅ No connections to delete");
        return;
    }
    println!("✅ Deleted all {} connections", count);
}

pub async fn list(ctx: &Context<'_>) {
    let connections = ctx
        .services
        .connection_service
        .get_all()
        .await
        .exit_on_err("Failed to list connections");

    println!("{}", "=== Connections ===".blue());

    if connections.is_empty() {
        println!("✅ No connections found");
        return;
    }
    let table_data: Vec<ConnectionRow> = connections
        .into_iter()
        .enumerate()
        .map(|(i, c)| ConnectionRow {
            index: (i + 1).to_string().bright_black().to_string(),
            name: c.name.green().to_string(),
            username: c.username.blue().to_string(),
            connection_string: c.connection_string.bright_cyan().to_string(),
        })
        .collect();

    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();

    println!("{}", table);
}

pub async fn ping(name: &str, ctx: &Context<'_>) {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Testing connection '{}'...", name));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let result = ctx.services.connection_service.ping(name).await;

    spinner.finish_and_clear();

    match result {
        Ok(_) => println!("✅ Connection test passed for '{}'", name),
        Err(e) => eprintln!("❌ Connection test failed for '{}': {:?}", name, e),
    }
}

pub async fn test(username: &str, password: &str, connection_string: &str, ctx: &Context<'_>) {
    let result = ctx
        .services
        .connection_service
        .test(&username, &password, &connection_string)
        .await;

    match result {
        Ok(_) => println!("✅ Connection test passed for '{}'", username),
        Err(e) => println!("❌ Connection test failed for '{}': {:?}", username, e),
    }
}
