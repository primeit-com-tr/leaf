use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Delta {
    pub object_type: String,
    pub object_name: String,
    pub object_owner: String,
    pub source_ddl_time: Option<NaiveDateTime>,
    pub source_ddl: Option<String>,
    pub target_ddl_time: Option<NaiveDateTime>,
    pub target_ddl: Option<String>,
    pub scripts: Vec<String>,
    pub rollback_scripts: Vec<String>,
}
