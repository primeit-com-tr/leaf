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
    EnumString,
    Default,
    Display,
    DeriveActiveEnum,
    EnumIter,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DeploymentStatus {
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

impl DeploymentStatus {
    pub fn to_colored_string(&self) -> String {
        match self {
            DeploymentStatus::Idle => "IDLE".bright_black().to_string(),
            DeploymentStatus::Running => "RUNNING".blue().bold().to_string(),
            DeploymentStatus::Error => "ERROR".red().bold().to_string(),
            DeploymentStatus::Success => "SUCCESS".green().bold().to_string(),
            DeploymentStatus::RollingBack => "ROLLING_BACK".blue().bold().to_string(),
            DeploymentStatus::RolledBack => "ROLLED_BACK".green().bold().to_string(),
            DeploymentStatus::RollbackError => "ROLLBACK_ERROR".red().bold().to_string(),
        }
    }
}
