use sea_orm::{ActiveValue::Set, entity::prelude::*};

use crate::types::ChangeStatus;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "changes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub changeset_id: i32,

    /// Scripts to apply to source database, serialized as JSON
    #[sea_orm(column_type = "Text")]
    pub script: String,

    /// Scripts to rollback to source database, serialized as JSON
    #[sea_orm(column_type = "Text")]
    pub rollback_script: String,

    #[sea_orm(default_value = "IDLE")]
    pub status: ChangeStatus,

    #[sea_orm(column_type = "Text")]
    pub error: Option<String>,

    #[sea_orm(default = "chrono::Utc::now().naive_utc()")]
    pub created_at: DateTime,

    pub updated_at: Option<DateTime>,

    pub started_at: Option<DateTime>,

    pub ended_at: Option<DateTime>,
}

impl ActiveModel {
    pub fn set_status(&mut self, status: ChangeStatus) {
        self.status = Set(status);
    }
    pub fn start(&mut self) {
        self.set_status(ChangeStatus::Running);
        self.started_at = Set(Some(chrono::Utc::now().naive_utc()));
    }

    pub fn end(&mut self, error: Option<String>) {
        if let Some(error) = error {
            self.set_status(ChangeStatus::Error);
            self.error = Set(Some(error));
            return;
        }
        self.set_status(ChangeStatus::Success);
        self.ended_at = Set(Some(chrono::Utc::now().naive_utc()));
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::changeset::Entity",
        from = "Column::ChangesetId",
        to = "super::changeset::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Changeset,
}

impl Related<super::changeset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Changeset.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    // Before insert - set both created_at and updated_at
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let now = chrono::Utc::now().naive_utc();

        if insert {
            // On insert, set both timestamps
            self.created_at = Set(now);
        } else {
            // On update, only set updated_at
            self.updated_at = Set(Some(now));
        }

        Ok(self)
    }
}
