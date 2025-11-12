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
pub enum RollbackStatus {
    #[sea_orm(string_value = "IDLE")]
    #[default]
    Idle,

    #[sea_orm(string_value = "RUNNING")]
    Running,

    #[sea_orm(string_value = "SUCCESS")]
    Success,

    #[sea_orm(string_value = "ERROR")]
    Error,
}

impl RollbackStatus {
    pub fn to_colored_string(&self) -> String {
        match self {
            RollbackStatus::Idle => "IDLE".bright_black().to_string(),
            RollbackStatus::Running => "RUNNING".blue().bold().to_string(),
            RollbackStatus::Success => "SUCCESS".green().bold().to_string(),
            RollbackStatus::Error => "ERROR".red().bold().to_string(),
        }
    }
}
