use sea_orm::{ActiveValue::Set, entity::prelude::*};
use serde::{Deserialize, Serialize};

use crate::types::{PlanStatus, StringList};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel, Default)]
#[sea_orm(table_name = "plans")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub name: String,

    pub source_connection_id: i32,

    pub target_connection_id: i32,

    #[sea_orm(column_type = "Text")]
    pub schemas: StringList,

    #[sea_orm(column_type = "Text")]
    pub exclude_object_types: Option<StringList>,

    #[sea_orm(column_type = "Text")]
    pub exclude_object_names: Option<StringList>,

    #[sea_orm(column_type = "Text")]
    pub disabled_drop_types: Option<StringList>,

    #[sea_orm(column_type = "Integer")]
    pub fail_fast: bool,

    #[sea_orm(default_value = "IDLE")]
    pub status: PlanStatus,

    #[sea_orm(default = "chrono::Utc::now().naive_utc()")]
    pub created_at: DateTime,
}

impl Model {
    pub fn get_schemas(&self) -> Vec<String> {
        self.schemas.0.clone()
    }

    /// Deserialize excluded object types from JSON string
    pub fn get_exclude_object_types(&self) -> Option<Vec<String>> {
        self.exclude_object_types.as_ref().map(|e| e.0.clone())
    }

    /// Deserialize excluded object names from JSON string
    pub fn get_exclude_object_names(&self) -> Option<Vec<String>> {
        self.exclude_object_names.as_ref().map(|e| e.0.clone())
    }

    /// Update status (mutates the model)
    pub fn set_status(&mut self, status: PlanStatus) {
        self.status = status;
    }

    pub fn set_running(&mut self) {
        self.set_status(PlanStatus::Running);
    }

    pub fn set_rolling_back(&mut self) {
        self.set_status(PlanStatus::RollingBack);
    }

    pub fn set_rolled_back(&mut self) {
        self.set_status(PlanStatus::RolledBack);
    }

    pub fn set_rollback_error(&mut self) {
        self.set_status(PlanStatus::RollbackError);
    }

    pub fn as_payload(&self) -> serde_json::Value {
        // reserved for future use
        todo!()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::connection::Entity",
        from = "Column::SourceConnectionId",
        to = "super::connection::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    SourceConnection,

    #[sea_orm(
        belongs_to = "super::connection::Entity",
        from = "Column::TargetConnectionId",
        to = "super::connection::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TargetConnection,
}

impl Related<super::connection::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SourceConnection.def()
    }
}

impl Entity {
    /// Find the source connection for this plan
    pub async fn find_source_connection(
        plan_id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<super::connection::Model>, DbErr> {
        let plan = Self::find_by_id(plan_id).one(db).await?;

        if let Some(plan) = plan {
            super::connection::Entity::find_by_id(plan.source_connection_id)
                .one(db)
                .await
        } else {
            Ok(None)
        }
    }

    /// Find the target connection for this plan
    pub async fn find_target_connection(
        plan_id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<super::connection::Model>, DbErr> {
        let plan = Self::find_by_id(plan_id).one(db).await?;

        if let Some(plan) = plan {
            super::connection::Entity::find_by_id(plan.target_connection_id)
                .one(db)
                .await
        } else {
            Ok(None)
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    /// Create a new ActiveModel for insertion
    pub fn new(
        name: String,
        source_connection_id: i32,
        target_connection_id: i32,
        schemas: Vec<String>,
        exclude_object_types: Option<Vec<String>>,
        exclude_object_names: Option<Vec<String>>,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self {
            name: Set(name),
            source_connection_id: Set(source_connection_id),
            target_connection_id: Set(target_connection_id),
            schemas: Set(StringList(schemas)),
            exclude_object_types: Set(exclude_object_types.map(StringList)),
            exclude_object_names: Set(exclude_object_names.map(StringList)),
            created_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        })
    }
}
