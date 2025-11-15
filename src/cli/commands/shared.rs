use chrono::NaiveDateTime;

use crate::cli::{Context, commands::ExitOnErr};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::mpsc;

pub async fn get_cut_off_date_or_bail(
    cutoff_date: Option<NaiveDateTime>,
    plan_id: i32,
    ctx: &Context<'_>,
) -> NaiveDateTime {
    let plan = ctx
        .services
        .plan_service
        .get_by_id(plan_id)
        .await
        .exit_on_err(&format!("❌ Failed to find plan by id '{}'", plan_id));

    if let Some(date) = cutoff_date {
        Some(date)
    } else {
        println!("⚠️ No cutoff date provided, using the last deployment start date");
        ctx.services
            .plan_service
            .get_last_cutoff_date(plan.id)
            .await
            .exit_on_err("Failed to get last deployment cutoff date")
    }
    .unwrap_or_else(|| {
        eprintln!(
            "❌ Failed to determine deployment cutoff date for plan '{}'",
            plan.name
        );
        std::process::exit(1);
    })
}

pub fn new_spinner() -> (ProgressBar, mpsc::UnboundedSender<String>) {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let (tx, mut rx) = mpsc::unbounded_channel();
    let pb_clone = spinner.clone();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            pb_clone.set_message(msg);
        }
    });

    (spinner, tx)
}
