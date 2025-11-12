use clap::{Parser, Subcommand};
use colored::Colorize;
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, Style, object::Rows},
};

use crate::{
    cli::{Context, commands::ExitOnErr},
    utils::format_duration,
};

#[derive(Subcommand, Debug)]
pub enum ShowSubcommand {
    /// Show a deployment
    Deployment {
        #[arg(long, short, required = true)]
        deployment_id: i32,
    },

    /// List all changed objects for a deployment
    Objects {
        #[arg(long, short, required = true)]
        deployment_id: i32,
    },

    /// List all changes for a deployment
    Changes {
        #[arg(long, short, required = true)]
        deployment_id: i32,
    },
}

#[derive(Parser, Debug)]
pub struct ShowCommand {
    #[command(subcommand)]
    pub subcommand: Option<ShowSubcommand>,
}

#[derive(Subcommand, Debug)]
pub enum DeploymentCommands {
    /// List deployments
    List {
        #[arg(short, long)]
        plan: Option<String>,

        #[arg(short, long)]
        limit: Option<u32>,

        #[arg(short, default_value = "desc")]
        order: Option<String>,
    },
    Show(ShowCommand),
}

#[derive(Tabled)]
struct DeploymentRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "ID")]
    id: String,

    #[tabled(rename = "Plan")]
    plan_name: String,

    #[tabled(rename = "Source")]
    source_connection: String,

    #[tabled(rename = "Target")]
    target_connection: String,

    #[tabled(rename = "Cutoff Date")]
    cutoff_date: String,

    #[tabled(rename = "Schemas")]
    schema_count: String,

    #[tabled(rename = "Objects")]
    object_count: String,

    #[tabled(rename = "Changes")]
    change_count: String,

    #[tabled(rename = "Started At")]
    started_at: String,

    #[tabled(rename = "Duration")]
    duration: String,

    #[tabled(rename = "Status")]
    status: String,
}

#[derive(Tabled)]
struct KeyValueRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "Attribute")]
    key: String,

    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct DeploymentObjectRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "Type")]
    object_type: String,

    #[tabled(rename = "Owner")]
    object_owner: String,

    #[tabled(rename = "Name")]
    object_name: String,
}

#[derive(Tabled)]
struct DeploymentChangeRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "ID")]
    id: String,

    #[tabled(rename = "Owner")]
    object_owner: String,

    #[tabled(rename = "Name")]
    object_name: String,

    #[tabled(rename = "Script")]
    script: String,

    #[tabled(rename = "Line Count")]
    line_count: String,

    #[tabled(rename = "Status")]
    status: String,
}

pub async fn execute(action: &DeploymentCommands, ctx: &Context<'_>) {
    match action {
        DeploymentCommands::List { plan, limit, order } => {
            list_deployments(plan.clone(), *limit, order.clone(), ctx).await
        }
        DeploymentCommands::Show(show_cmd) => match &show_cmd.subcommand {
            Some(sub) => match sub {
                ShowSubcommand::Deployment { deployment_id } => {
                    show_deployment(*deployment_id, ctx).await
                }
                ShowSubcommand::Objects { deployment_id } => {
                    show_deployment_objects(*deployment_id, ctx).await
                }
                ShowSubcommand::Changes { deployment_id } => {
                    show_deployment_changes(*deployment_id, ctx).await
                }
            },
            None => show_deployment(0, ctx).await, // default behavior when no subcommand is given
        },
    }
}

async fn show_deployment(deployment_id: i32, ctx: &Context<'_>) {
    let deployment = ctx
        .services
        .deployment_service
        .get_by_id(deployment_id)
        .await
        .exit_on_err(&format!(
            "❌ Failed to find deployment by id {}",
            deployment_id
        ));

    let mut table_data = Vec::new();

    let mut index = 1;

    let id = deployment.id.to_string();
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "ID".to_string(),
        value: id.clone(),
    });

    index += 1;
    let plan_id = deployment.plan_id;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Plan ID".to_string(),
        value: plan_id.to_string(),
    });
    let plan = ctx
        .services
        .plan_service
        .get_by_id(plan_id)
        .await
        .exit_on_err(&format!("❌ Failed to find plan by id {}", plan_id));

    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Plan Name".to_string(),
        value: plan.name.clone(),
    });

    let source_connection = ctx
        .services
        .connection_service
        .get_by_id(plan.source_connection_id)
        .await
        .exit_on_err(&format!(
            "❌ Failed to find source connection for plan id {}",
            plan_id
        ));
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Source Connection".to_string(),
        value: source_connection.name.clone(),
    });

    let target_connection = ctx
        .services
        .connection_service
        .get_by_id(plan.target_connection_id)
        .await
        .exit_on_err(&format!(
            "❌ Failed to find target connection for plan id {}",
            plan_id
        ));
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Target Connection".to_string(),
        value: target_connection.name.clone(),
    });

    let cutoff_date = deployment
        .cutoff_date
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Cutoff Date".to_string(),
        value: cutoff_date,
    });

    let schema_count = plan.schemas.0.len();
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "# of Schemas".to_string(),
        value: schema_count.to_string(),
    });

    let object_count = ctx
        .services
        .deployment_service
        .get_changeset_count_by_deployment_id(deployment.id)
        .await
        .exit_on_err("❌ Failed to get changeset count");
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "# of Objects".to_string(),
        value: object_count.to_string(),
    });

    let change_count = ctx
        .services
        .deployment_service
        .get_change_count_by_deployment_id(deployment.id)
        .await
        .exit_on_err("❌ Failed to get change count");
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "# of Changes".to_string(),
        value: change_count.to_string(),
    });

    let started_at = deployment
        .started_at
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "N/A".to_string());
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Started At".to_string(),
        value: started_at,
    });

    let ended_at = deployment
        .ended_at
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "N/A".to_string());
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Ended At".to_string(),
        value: ended_at,
    });

    let duration = format_duration(
        deployment.started_at.map(|dt| dt.and_utc()),
        deployment.ended_at.map(|dt| dt.and_utc()),
    );
    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Duration".to_string(),
        value: duration,
    });

    index += 1;
    table_data.push(KeyValueRow {
        index: index.to_string().bright_black().to_string(),
        key: "Status".to_string(),
        value: deployment.status.to_colored_string(),
    });

    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();
    println!("{}", table);
}

async fn show_deployment_objects(deployment_id: i32, ctx: &Context<'_>) {
    let changesets = ctx
        .services
        .deployment_service
        .find_changesets_by_deployment_id(deployment_id)
        .await
        .exit_on_err("❌ Failed to fetch deployments");

    let mut table_data = Vec::new();

    let mut index = 1;
    for changeset in changesets {
        let object_type = changeset.object_type.clone();
        let object_owner = changeset.object_owner.clone();
        let object_name = changeset.object_name.clone();

        table_data.push(DeploymentObjectRow {
            index: index.to_string().bright_black().to_string(),
            object_type,
            object_owner,
            object_name,
        });
        index += 1;
    }

    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();
    println!("{}", table);
}
async fn show_deployment_changes(deployment_id: i32, ctx: &Context<'_>) {
    let changesets_with_changes = ctx
        .services
        .deployment_service
        .find_changesets_with_changes_by_deployment_id(deployment_id)
        .await
        .exit_on_err("❌ Failed to fetch changesets with changes");

    let mut table_data = Vec::new();
    if changesets_with_changes.is_none() {
        println!("✅ No changes found");
        return;
    }
    let changesets_with_changes = changesets_with_changes.unwrap();

    for (i, (changeset, changes)) in changesets_with_changes.iter().enumerate() {
        for change in changes {
            let id = change.id.to_string();
            let object_owner = changeset.object_owner.clone();
            let object_name = changeset.object_name.clone();

            let line_count = change.script.lines().count();
            let first_line = change
                .script
                .trim()
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            let status = change.status.to_colored_string();

            table_data.push(DeploymentChangeRow {
                index: i.to_string().bright_black().to_string(),
                id,
                object_owner,
                object_name,
                script: if line_count > 1 {
                    format!("{}...", first_line)
                } else {
                    first_line
                },
                line_count: line_count.to_string(),
                status,
            });
        }
    }

    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();
    println!("{}", table);
}

async fn list_deployments(
    plan: Option<String>,
    limit: Option<u32>,
    order: Option<String>,
    ctx: &Context<'_>,
) {
    let plan_model = match plan {
        Some(name) => ctx
            .services
            .plan_service
            .find_by_name(&name)
            .await
            .exit_on_err(&format!("❌ Failed to find plan '{}'", name)),
        None => None,
    };
    let plan_id = plan_model.map(|p| p.id);

    let deployments = ctx
        .services
        .deployment_service
        .fetch_deployments(plan_id, limit, order)
        .await
        .exit_on_err("❌ Failed to fetch deployments");

    println!("{}", "=== Deployments ===".blue());
    if deployments.is_empty() {
        println!("✅ No deployments found");
        return;
    }

    let mut table_data = Vec::new();

    for deployment in deployments {
        let plan = ctx
            .services
            .plan_service
            .get_by_id(deployment.plan_id)
            .await
            .exit_on_err(&format!(
                "❌ Failed to find plan by id {}",
                deployment.plan_id
            ));

        let source_connection = ctx
            .services
            .connection_service
            .get_by_id(plan.source_connection_id)
            .await
            .exit_on_err(&format!(
                "❌ Failed to find source connection for plan id {}",
                deployment.plan_id
            ));

        let target_connection = ctx
            .services
            .connection_service
            .get_by_id(plan.target_connection_id)
            .await
            .exit_on_err(&format!(
                "❌ Failed to find target connection for plan id {}",
                deployment.plan_id
            ));

        let cutoff_date = deployment
            .cutoff_date
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let changeset_count = ctx
            .services
            .deployment_service
            .get_changeset_count_by_deployment_id(deployment.id)
            .await
            .exit_on_err("❌ Failed to get changeset count");

        let change_count = ctx
            .services
            .deployment_service
            .get_change_count_by_deployment_id(deployment.id)
            .await
            .exit_on_err("❌ Failed to get change count");

        let started_at = deployment
            .started_at
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let duration = format_duration(
            deployment.started_at.map(|dt| dt.and_utc()),
            deployment.ended_at.map(|dt| dt.and_utc()),
        );

        table_data.push(DeploymentRow {
            id: deployment.id.to_string().blue().to_string(),
            index: deployment.id.to_string(),
            plan_name: plan.name.clone(),
            source_connection: source_connection.name.clone(),
            target_connection: target_connection.name.clone(),
            cutoff_date,
            schema_count: plan.schemas.0.len().to_string().cyan().to_string(),
            object_count: changeset_count.to_string().cyan().to_string(),
            change_count: change_count.to_string().cyan().to_string(),
            started_at,
            duration,
            status: deployment.status.to_colored_string(),
        });
    }
    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();

    println!("{}", table);
}
