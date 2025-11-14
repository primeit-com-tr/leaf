use anyhow::Error;
use chrono::Utc;
use sea_orm::{ActiveValue::Set, JsonValue, entity::prelude::*};
use serde::{Deserialize, Serialize};

use crate::types::{DeploymentStatus, Hooks, StringList};
use sea_orm::ActiveModelBehavior;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "deployments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub plan_id: i32,

    /// Deploy all objects modified after this date
    pub cutoff_date: DateTime,

    #[sea_orm(column_type = "Text")]
    pub payload: String,

    #[sea_orm(column_type = "Integer")]
    pub disable_hooks: bool,

    #[sea_orm(column_type = "Json", nullable)]
    pub hooks: Option<JsonValue>,

    #[sea_orm(default_value = "IDLE")]
    pub status: DeploymentStatus,

    #[sea_orm(column_type = "Text")]
    pub errors: Option<StringList>,

    #[sea_orm(default = "chrono::Utc::now().naive_utc()")]
    pub created_at: DateTime,

    #[sea_orm(default)]
    pub updated_at: Option<DateTime>,

    #[sea_orm(default)]
    pub started_at: Option<DateTime>,

    pub ended_at: Option<DateTime>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: 0,
            plan_id: 0,
            cutoff_date: Utc::now().naive_utc(),
            payload: String::new(),
            disable_hooks: false,
            hooks: None,
            status: DeploymentStatus::default(),
            errors: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Some(Utc::now().naive_utc()),
            started_at: Some(Utc::now().naive_utc()),
            ended_at: Some(Utc::now().naive_utc()),
        }
    }
}
#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let now = chrono::Utc::now().naive_utc();

        if insert {
            self.created_at = Set(now);
        } else {
            self.updated_at = Set(Some(now));
        }

        Ok(self)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::plan::Entity",
        from = "Column::PlanId",
        to = "super::plan::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Plan,
}

impl Related<super::plan::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Plan.def()
    }
}

impl Entity {
    /// Find plan by deployment ID
    pub async fn find_plan(
        id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<super::plan::Model>, DbErr> {
        let deployment = Self::find_by_id(id).one(db).await?;
        if let Some(deployment) = deployment {
            super::plan::Entity::find_by_id(deployment.plan_id)
                .one(db)
                .await
        } else {
            Ok(None)
        }
    }
}

impl Model {
    pub fn get_hooks(&self) -> Option<Result<Hooks, serde_json::Error>> {
        self.hooks
            .as_ref()
            .map(|h| serde_json::from_value(h.clone()))
    }

    /// Set hooks from a Hooks struct (mutates the model)
    pub fn set_hooks(&mut self, hooks: Option<Hooks>) -> Result<(), serde_json::Error> {
        self.hooks = hooks.map(|h| serde_json::to_value(h)).transpose()?;
        Ok(())
    }
}

impl ActiveModel {
    pub fn new(plan_id: i32, payload: String) -> Self {
        Self {
            plan_id: Set(plan_id),
            payload: Set(payload),
            ..Default::default()
        }
    }
    /// Update status (mutates the model)
    pub fn set_status(&mut self, status: DeploymentStatus) {
        match status {
            DeploymentStatus::Running => {
                self.started_at = Set(Some(Utc::now().naive_utc()));
            }
            DeploymentStatus::Success | DeploymentStatus::Error => {
                self.ended_at = Set(Some(Utc::now().naive_utc()));
            }
            _ => {}
        }
        self.status = Set(status);
        self.updated_at = Set(Some(Utc::now().naive_utc()));
    }
    // Helper method to start deployment
    pub fn start(&mut self) {
        self.started_at = Set(Some(Utc::now().naive_utc()));
        self.status = Set(DeploymentStatus::Running);
    }

    // Helper method to end deployment
    pub fn end(&mut self, error: Option<Error>) {
        self.ended_at = Set(Some(Utc::now().naive_utc()));
        if let Some(error) = error {
            self.errors = Set(Some(StringList(vec![error.to_string()])));
            self.status = Set(DeploymentStatus::Error);
            return;
        }
        self.errors = Set(None);
        self.status = Set(DeploymentStatus::Success);
    }

    pub fn set_running(&mut self) {
        self.set_status(DeploymentStatus::Running);
    }

    pub fn set_rolling_back(&mut self) {
        self.set_status(DeploymentStatus::RollingBack);
    }

    pub fn set_rolled_back(&mut self) {
        self.set_status(DeploymentStatus::RolledBack);
    }

    pub fn set_rollback_error(&mut self) {
        self.set_status(DeploymentStatus::RollbackError);
    }

    pub fn set_hooks(&mut self, hooks: Option<Hooks>) -> Result<(), serde_json::Error> {
        self.hooks = Set(hooks.map(|h| serde_json::to_value(h)).transpose()?);
        Ok(())
    }
}
