use colored::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

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
pub enum ChangeStatus {
    #[sea_orm(string_value = "IDLE")]
    #[default]
    Idle,

    #[sea_orm(string_value = "RUNNING")]
    Running,

    #[sea_orm(string_value = "SUCCESS")]
    Success,

    #[sea_orm(string_value = "ERROR")]
    Error,

    #[sea_orm(string_value = "ROLLING_BACK")]
    RollingBack,

    #[sea_orm(string_value = "ROLLED_BACK")]
    RolledBack,

    #[sea_orm(string_value = "ROLLBACK_ERROR")]
    RollbackError,
}

impl ChangeStatus {
    pub fn to_colored_string(&self) -> String {
        match self {
            ChangeStatus::Idle => "IDLE".bright_black().to_string(),
            ChangeStatus::Running => "RUNNING".blue().bold().to_string(),
            ChangeStatus::Success => "SUCCESS".green().bold().to_string(),
            ChangeStatus::Error => "ERROR".red().bold().to_string(),
            ChangeStatus::RollingBack => "ROLLING BACK".blue().bold().to_string(),
            ChangeStatus::RolledBack => "ROLLED BACK".yellow().bold().to_string(),
            ChangeStatus::RollbackError => "ROLLBACK ERROR".red().bold().to_string(),
        }
    }
}
