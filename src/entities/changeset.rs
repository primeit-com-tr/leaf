use std::hash::{Hash, Hasher};

use sea_orm::{ActiveValue::Set, entity::prelude::*};

use crate::types::{ChangesetStatus, StringList};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Default)]
#[sea_orm(table_name = "changesets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub deployment_id: i32,

    pub object_type: String,

    pub object_name: String,

    pub object_owner: String,

    pub source_ddl_time: Option<DateTime>,

    /// DDL for source database
    #[sea_orm(column_type = "Text")]
    pub source_ddl: Option<String>,

    pub target_ddl_time: Option<DateTime>,

    /// DDL for target database
    #[sea_orm(column_type = "Text")]
    pub target_ddl: Option<String>,

    #[sea_orm(default_value = "IDLE")]
    pub status: ChangesetStatus,

    #[sea_orm(column_type = "Text")]
    pub errors: Option<StringList>,

    #[sea_orm(default = "chrono::Utc::now().naive_utc()")]
    pub created_at: DateTime,

    pub updated_at: Option<DateTime>,

    pub started_at: Option<DateTime>,

    pub ended_at: Option<DateTime>,
}

impl Hash for Model {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Only call the hash function on the unique primary key 'id'
        self.id.hash(state);
        // Do not call hash on any other fields
    }
}

impl ActiveModel {
    /// Add an error to the errors list
    pub fn add_error(&mut self, error: String) {
        match &mut self.errors {
            Set(Some(errors)) => errors.push(error),
            Set(None) => {
                self.errors = Set(Some(StringList(vec![error])));
            }
            _ => {
                self.errors = Set(Some(StringList(vec![error])));
            }
        }
    }

    pub fn set_status(&mut self, status: ChangesetStatus) {
        self.status = Set(status);
        self.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
    }

    pub fn start(&mut self) {
        self.set_status(ChangesetStatus::Running);
        self.started_at = Set(Some(chrono::Utc::now().naive_utc()));
    }

    pub fn end(&mut self, errors: Option<StringList>) {
        if let Some(errors) = errors {
            self.set_status(ChangesetStatus::Error);
            self.errors = Set(Some(errors));
            return;
        }
        self.set_status(ChangesetStatus::Success);
        self.ended_at = Set(Some(chrono::Utc::now().naive_utc()));
    }

    pub fn set_running(&mut self) {
        self.set_status(ChangesetStatus::Running);
    }

    pub fn set_rolling_back(&mut self) {
        self.set_status(ChangesetStatus::RollingBack);
    }

    pub fn set_rolled_back(&mut self) {
        self.set_status(ChangesetStatus::RolledBack);
    }

    pub fn set_rollback_error(&mut self) {
        self.set_status(ChangesetStatus::RollbackError);
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::deployment::Entity",
        from = "Column::DeploymentId",
        to = "super::deployment::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Deployment,

    #[sea_orm(has_many = "super::change::Entity")]
    Changes,
}

impl Related<super::deployment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Deployment.def()
    }
}
impl Related<super::change::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Changes.def()
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
