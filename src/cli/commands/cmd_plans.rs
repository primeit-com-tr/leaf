use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Confirm;
use tabled::{Table, Tabled, builder::Builder};
use tokio::sync::mpsc;
use tracing::error;

use crate::{
    cli::{
        Context,
        commands::{ExitOnErr, get_cut_off_date_or_bail, new_spinner},
    },
    types::PlanStatus,
    utils::{
        DeploymentContext, ProgressReporter, deployment_context::DeploymentContextOptions,
        parsers::parse_cutoff_date,
    },
};
use tabled::{settings::Alignment, settings::Modify, settings::Style, settings::object::Rows};

#[derive(Parser, Debug, Clone)]
pub struct PlansRunArgs {
    /// Plan name, case insensitive
    #[arg(required = true)]
    name: String,

    #[arg(long, value_parser = parse_cutoff_date, long_help = "Cutoff date — deploy everything changed after this date.\n\
        If not specified, the last successful deployment start date will be used.\n\
        If not found, then the app will exit.\n\n\
        Example formats:\n\
        - 2023.01.01\n\
        - 2023.01.01:00.00.00\n\
        - 2023.01.01:23.59.59"
    )]
    cutoff_date: Option<NaiveDateTime>,

    /// Fail fast mode
    #[arg(long)]
    fail_fast: Option<bool>,

    /// Dry run mode, this will not apply changes to the database
    #[arg(short, long)]
    dry: bool,

    /// Show report after running the plan
    #[arg(short, long, default_value_t = false)]
    show_report: bool,
}

#[derive(Subcommand, Debug)]
pub enum ListSubcommand {
    /// List all plans (default)
    All,
    /// List schemas
    Schemas {
        #[arg(long, short, required = true)]
        plan: String,
    },
    /// List excluded object types for a plan
    ExcludedObjectTypes {
        #[arg(long, short, required = true)]
        plan: String,
    },

    /// List excluded object names for a plan
    ExcludedObjectNames {
        #[arg(long, short, required = true)]
        plan: String,
    },

    /// List disabled object types for a plan
    DisabledDropTypes {
        #[arg(long, short, required = true)]
        plan: String,
    },
}

#[derive(Parser, Debug)]
pub struct ListCommand {
    #[command(subcommand)]
    pub subcommand: Option<ListSubcommand>,
}

#[derive(Subcommand, Debug)]
pub enum PlanCommands {
    /// Add a plan
    Add {
        /// Name of the plan
        #[arg(long, required = true)]
        name: String,

        /// Source connection name
        #[arg(long, required = true)]
        source: String,

        /// Target connection name
        #[arg(long, required = true)]
        target: String,

        /// Comma-separated list of schemas to include in the plan
        #[arg(long, required = true, value_delimiter = ',')]
        schemas: Vec<String>,

        /// Comma-separated list of object types to exclude from the plan
        #[arg(long, value_delimiter = ',')]
        exclude_object_types: Vec<String>,

        /// Comma-separated list of object names to exclude from the plan
        #[arg(long, value_delimiter = ',')]
        exclude_object_names: Vec<String>,

        /// Comma-separated list of disabled object types do drop
        /// (e.g., TABLE, VIEW, PROCEDURE, FUNCTION, TRIGGER, etc.)
        #[arg(long, value_delimiter = ',')]
        disabled_drop_types: Vec<String>,

        /// Fail fast mode
        #[arg(long)]
        fail_fast: bool,
    },
    /// List plans, schemas, excluded object types
    List(ListCommand),

    /// Remove a plan
    Remove {
        #[arg(required = true)]
        name: String,
    },

    /// Remove all plans
    Prune {
        #[arg(short, long)]
        yes: bool,
    },

    /// Reset plan status to IDLE
    Reset {
        #[arg(long, short, required = true)]
        plan: String,

        #[arg(long, short, required = false)]
        yes: bool,
    },

    /// Run a plan
    Run(PlansRunArgs),

    /// Rollback a plan
    Rollback {
        #[arg(long, short, required = true)]
        plan: String,
    },
}

#[derive(Tabled)]
struct PlanRow {
    #[tabled(rename = "#")]
    index: String,

    #[tabled(rename = "Name")]
    name: String,

    #[tabled(rename = "Source")]
    source: String,

    #[tabled(rename = "Target")]
    target: String,

    #[tabled(rename = "Schemas")]
    schema_count: String,

    #[tabled(rename = "Status")]
    status: String,
}

pub async fn execute(action: &PlanCommands, ctx: &Context<'_>) {
    match action {
        PlanCommands::Add {
            name,
            source,
            target,
            schemas,
            exclude_object_types,
            exclude_object_names,
            disabled_drop_types,
            fail_fast,
        } => {
            add(
                name,
                source,
                target,
                schemas,
                exclude_object_types,
                exclude_object_names,
                disabled_drop_types,
                *fail_fast,
                ctx,
            )
            .await
        }
        PlanCommands::List(list_cmd) => match &list_cmd.subcommand {
            Some(sub) => match sub {
                ListSubcommand::All => list_plans(ctx).await,
                ListSubcommand::Schemas { plan } => list_plan_field(plan, "schemas", ctx).await,
                ListSubcommand::ExcludedObjectTypes { plan } => {
                    list_plan_field(plan, "excluded_object_types", ctx).await
                }
                ListSubcommand::ExcludedObjectNames { plan } => {
                    list_plan_field(plan, "excluded_object_names", ctx).await
                }
                ListSubcommand::DisabledDropTypes { plan } => {
                    list_plan_field(plan, "disabled_drop_types", ctx).await
                }
            },
            None => list_plans(ctx).await, // default behavior when no subcommand is given
        },
        PlanCommands::Remove { name } => remove(name, ctx).await,
        PlanCommands::Prune { yes } => prune(yes, ctx).await,
        PlanCommands::Run(args) => {
            run(
                &args.name,
                &args.dry,
                args.cutoff_date,
                args.fail_fast,
                args.show_report,
                ctx,
            )
            .await
        }
        PlanCommands::Rollback { plan } => rollback(&plan, ctx).await,
        PlanCommands::Reset { plan, yes } => reset(&plan, *yes, ctx).await,
    }
}

pub async fn add(
    name: &str,
    source: &str,
    target: &str,
    schemas: &Vec<String>,
    exclude_object_types: &Vec<String>,
    exclude_object_names: &Vec<String>,
    excluded_drop_types: &Vec<String>,
    fail_fast: bool,
    ctx: &Context<'_>,
) {
    ctx.services
        .plan_service
        .create(
            name,
            source,
            target,
            schemas,
            Some(exclude_object_types.to_vec()),
            Some(exclude_object_names.to_vec()),
            Some(excluded_drop_types.to_vec()),
            fail_fast,
        )
        .await
        .exit_on_err(&format!("❌ Plan creation failed for '{}'", name));

    println!("✅ Plan created successfully for '{}'", name);
}

pub async fn list_plans(ctx: &Context<'_>) {
    let plans = ctx
        .services
        .plan_service
        .get_all()
        .await
        .exit_on_err("Failed to get all plans");

    println!("{}", "=== Plans ===".blue());

    if plans.is_empty() {
        println!("✅ No plans found");
        return;
    }

    let mut table_data = Vec::new();

    for (i, plan) in plans.into_iter().enumerate() {
        let source_connection = ctx
            .services
            .connection_service
            .get_by_id(plan.source_connection_id)
            .await
            .exit_on_err(&format!(
                "❌ Failed to retrieve source connection for plan '{}'",
                plan.name
            ));

        let target_connection = ctx
            .services
            .connection_service
            .get_by_id(plan.target_connection_id)
            .await
            .exit_on_err(&format!(
                "❌ Failed to retrieve target connection for plan '{}'",
                plan.name
            ));

        table_data.push(PlanRow {
            index: (i + 1).to_string(),
            name: plan.name.green().to_string(),
            source: source_connection.name.blue().to_string(),
            target: target_connection.name.purple().to_string(),
            schema_count: plan.schemas.0.len().to_string().cyan().to_string(),
            status: plan.status.to_colored_string(),
        });
    }

    let table = Table::new(table_data)
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();

    println!("{}", table);
}

pub async fn list_plan_field(plan_name: &str, field: &str, ctx: &Context<'_>) {
    let plan = ctx
        .services
        .plan_service
        .find_by_name(plan_name)
        .await
        .exit_on_err(&format!("❌ Failed to find plan '{}'", plan_name));

    if plan.is_none() {
        println!("❌ Plan '{}' not found", plan_name);
        return;
    }
    let plan = plan.unwrap();

    let field_value: Vec<String> = match field {
        "schemas" => plan.schemas.0,
        "excluded_object_types" => plan.exclude_object_types.map_or_else(Vec::new, |sl| sl.0),
        "excluded_object_names" => plan.exclude_object_names.map_or_else(Vec::new, |sl| sl.0),
        "disabled_drop_types" => plan.disabled_drop_types.map_or_else(Vec::new, |sl| sl.0),
        _ => {
            eprintln!("❌ Invalid field name '{}'", field);
            std::process::exit(1);
        }
    };

    if field_value.is_empty() {
        println!("✅ No {} found for plan '{}'", field, plan_name);
        return;
    }

    let mut builder = Builder::default();
    builder.push_record(vec![field]);

    for value in &field_value {
        builder.push_record(vec![value]);
    }

    let table = builder.build().with(Style::rounded()).to_string();

    println!("{}", table);
}

pub async fn remove(name: &str, ctx: &Context<'_>) {
    let plan = ctx
        .services
        .plan_service
        .delete_by_name(name)
        .await
        .exit_on_err(&format!("❌ Failed to delete plan '{}'", name));
    println!("✅ Plan '{}' removed", plan.name);
}

pub async fn prune(yes: &bool, ctx: &Context<'_>) {
    let proceed = *yes
        || Confirm::new("This will delete all plans. Continue?")
            .with_default(false)
            .prompt()
            .unwrap_or(false);

    if !proceed {
        println!("✅ Aborted");
        return;
    }

    let count = ctx
        .services
        .plan_service
        .prune()
        .await
        .exit_on_err("❌ Failed to delete all plans");

    if count == 0 {
        println!("✅ No plans to delete");
        return;
    }

    println!("✅ Deleted all {} plans", count);
}

pub async fn run(
    name: &str,
    dry: &bool,
    cutoff_date: Option<NaiveDateTime>,
    fail_fast: Option<bool>,
    show_report: bool,
    ctx: &Context<'_>,
) {
    let (spinner, tx) = new_spinner();

    let plan = ctx
        .services
        .plan_service
        .find_by_name(name)
        .await
        .exit_on_err(format!("❌ Failed to find plan '{}'", name).as_str())
        .unwrap_or_else(|| {
            eprintln!("❌ Plan '{}' not found", name);
            std::process::exit(1);
        });

    let cutoff_date = get_cut_off_date_or_bail(cutoff_date, plan.id, ctx).await;

    let mut sink = DeploymentContext::new(Some(DeploymentContextOptions::new(
        *dry,
        false,
        None,
        None,
        Some(tx),
    )))
    .exit_on_err("Failed to initialize deployment sink"); // TODO: Add more info

    let res = ctx
        .services
        .deployment_service
        .prepare_and_run(plan.id, fail_fast.unwrap_or(false), cutoff_date, &mut sink)
        .await;

    if res.is_err() {
        error!("Failed to run plan: {:?}", res.as_ref().err());
        std::process::exit(1);
    }

    if sink.is_dry_run() {
        sink.print_summary("✅ Dry run completed successfully");
    }

    spinner.finish_and_clear();

    if show_report {
        let _ = res.unwrap();
        todo!();
    } else {
        println!("✅ Deployment for plan '{}' completed successfully", name);
    }
}

async fn rollback(plan_name: &str, ctx: &Context<'_>) {
    let plan = ctx
        .services
        .plan_service
        .find_by_name(&plan_name)
        .await
        .exit_on_err(&format!("❌ Failed to find plan '{}'", plan_name));

    if plan.is_none() {
        eprintln!("❌ Plan '{}' not found", plan_name);
        std::process::exit(1);
    }
    let plan = plan.unwrap();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message(format!(
        "Rolling back deployment for plan '{}'...",
        plan_name
    ));

    let (tx, mut rx) = mpsc::unbounded_channel();
    let progress = ProgressReporter::new(Some(tx));

    let spinner_clone = spinner.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            spinner_clone.set_message(msg);
        }
    });

    let res = ctx
        .services
        .deployment_service
        .rollback(plan.id, progress)
        .await;

    if res.is_err() {
        spinner.finish_and_clear();
        error!("Failed to rollback deployment: {:?}", res.as_ref().err());
        std::process::exit(1);
    }
    spinner.finish_and_clear();

    println!(
        "✅ Deployment for plan '{}' rolled back successfully",
        plan_name
    );
}

async fn reset(plan_name: &str, yes: bool, ctx: &Context<'_>) {
    let plan = ctx
        .services
        .plan_service
        .find_by_name(plan_name)
        .await
        .exit_on_err(&format!("❌ Failed to find plan '{}'", plan_name));

    if plan.is_none() {
        eprintln!("❌ Plan '{}' not found", plan_name);
        std::process::exit(1);
    }
    let plan = plan.unwrap();

    if plan.status != PlanStatus::Running {
        eprintln!("⚠️ Only running plans can be reset.");
        std::process::exit(1);
    }

    let proceed = yes
        || Confirm::new("This will reset the status of the plan. Continue?")
            .with_default(false)
            .prompt()
            .unwrap_or(false);

    if !proceed {
        println!("✅ Aborted");
        return;
    }

    let res = ctx.services.plan_service.reset_status_by_id(plan.id).await;

    if res.is_err() {
        error!("Failed to reset plan status: {:?}", res.as_ref().err());
        std::process::exit(1);
    }

    println!("✅ Plan '{}' status reset successfully", plan_name);
}
