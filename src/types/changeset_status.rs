use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use colored::*;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    Default,
    DeriveActiveEnum,
    EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum ChangesetStatus {
    #[sea_orm(string_value = "IDLE")]
    #[default]
    Idle,

    #[sea_orm(string_value = "RUNNING")]
    Running,

    #[sea_orm(string_value = "SUCCESS")]
    Success,

    #[sea_orm(string_value = "ERROR")]
    Error,

    #[sea_orm(string_value = "WARNING")]
    Warning,

    #[sea_orm(string_value = "ROLLING_BACK")]
    RollingBack,

    #[sea_orm(string_value = "ROLLED_BACK")]
    RolledBack,

    #[sea_orm(string_value = "ROLLBACK_ERROR")]
    RollbackError,
}

impl ChangesetStatus {
    pub fn to_colored_string(&self) -> String {
        match self {
            ChangesetStatus::Idle => "IDLE".bright_black().to_string(),
            ChangesetStatus::Running => "RUNNING".blue().bold().to_string(),
            ChangesetStatus::Success => "SUCCESS".green().bold().to_string(),
            ChangesetStatus::Error => "ERROR".red().bold().to_string(),
            ChangesetStatus::Warning => "WARNING".yellow().bold().to_string(),
            ChangesetStatus::RollingBack => "ROLLING_BACK".blue().bold().to_string(),
            ChangesetStatus::RolledBack => "ROLLED_BACK".green().bold().to_string(),
            ChangesetStatus::RollbackError => "ROLLBACK_ERROR".red().bold().to_string(),
        }
    }
}
