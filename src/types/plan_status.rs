use colored::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString}; // for terminal colors

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
pub enum PlanStatus {
    #[sea_orm(string_value = "IDLE")]
    #[default]
    Idle,

    #[sea_orm(string_value = "RUNNING")]
    Running,

    #[sea_orm(string_value = "ERROR")]
    Error,

    #[sea_orm(string_value = "SUCCESS")]
    Success,

    #[sea_orm(string_value = "ROLLING_BACK")]
    RollingBack,

    #[sea_orm(string_value = "ROLLED_BACK")]
    RolledBack,

    #[sea_orm(string_value = "ROLLBACK_ERROR")]
    RollbackError,
}

impl PlanStatus {
    pub fn to_colored_string(&self) -> String {
        match self {
            PlanStatus::Idle => "IDLE".bright_black().to_string(),
            PlanStatus::Running => "RUNNING".blue().bold().to_string(),
            PlanStatus::Error => "ERROR".red().bold().to_string(),
            PlanStatus::Success => "SUCCESS".green().bold().to_string(),
            PlanStatus::RollingBack => "ROLLING_BACK".blue().bold().to_string(),
            PlanStatus::RolledBack => "ROLLED_BACK".green().bold().to_string(),
            PlanStatus::RollbackError => "ROLLBACK_ERROR".red().bold().to_string(),
        }
    }
}
