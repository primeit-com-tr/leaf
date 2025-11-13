use sea_orm::{ActiveValue::Set, entity::prelude::*};
use serde::{Deserialize, Serialize};

use crate::types::RollbackStatus;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel, Default)]
#[sea_orm(table_name = "rollbacks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub change_id: i32,

    pub script: String,

    #[sea_orm(default_value = "IDLE")]
    pub status: RollbackStatus,

    #[sea_orm(column_type = "Text")]
    pub error: Option<String>,

    #[sea_orm(default = "chrono::Utc::now().naive_utc()")]
    pub created_at: DateTime,

    pub updated_at: Option<DateTime>,
}

impl Model {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::change::Entity",
        from = "Column::ChangeId",
        to = "super::change::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Change,
}

impl Related<super::change::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Change.def()
    }
}

impl Entity {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    /// Create a new ActiveModel for insertion
    pub fn new(change_id: i32, script: String) -> Self {
        Self {
            change_id: Set(change_id),
            script: Set(script),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
    }
    pub fn set_status(&mut self, status: RollbackStatus) {
        self.status = Set(status);
        self.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
    }

    pub fn start(&mut self) {
        self.set_status(RollbackStatus::Running);
    }

    pub fn end(&mut self, error: Option<String>) {
        if let Some(error) = error {
            self.set_status(RollbackStatus::Error);
            self.error = Set(Some(error));
            return;
        }
        self.set_status(RollbackStatus::Success);
    }
}
