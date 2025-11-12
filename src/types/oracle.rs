use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct Object {
    pub owner: String,
    pub object_name: String,
    pub object_type: String,
    pub last_ddl_time: NaiveDateTime,
    pub ddl: Option<String>,
}
