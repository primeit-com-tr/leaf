use crate::entities::{
    ConnectionActiveModel, ConnectionColumn, ConnectionModel, ConnectionsEntity,
};
use anyhow::{Context, Result};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Condition, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter,
    sea_query::Expr,
};

pub struct ConnectionRepository {
    db: DatabaseConnection,
}

impl ConnectionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_db(&self) -> DatabaseConnection {
        self.db.clone()
    }

    pub async fn get_all(&self) -> Result<Vec<ConnectionModel>> {
        ConnectionsEntity::find()
            .all(&self.db)
            .await
            .context(format!("failed to get all connections."))
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<ConnectionModel>> {
        ConnectionsEntity::find()
            .filter(Condition::all().add(Expr::col(ConnectionColumn::Name).like(name)))
            .one(&self.db)
            .await
            .context(format!("Failed to find connection by name: {}", name))
    }

    /// Check if a connection with the given name already exists
    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        Ok(self.find_by_name(name).await?.is_some())
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ConnectionModel>> {
        ConnectionsEntity::find_by_id(id)
            .one(&self.db)
            .await
            .context(format!("Failed to find connection by id: {}", id))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<ConnectionModel> {
        ConnectionsEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(format!("Connection with ID {} not found", id))
    }

    pub async fn create(
        &self,
        name: &str,
        username: &str,
        password: &str,
        connection_string: &str,
    ) -> Result<ConnectionModel> {
        let active_model = ConnectionActiveModel {
            id: NotSet,
            name: Set(name.to_string()),
            username: Set(username.to_string()),
            password: Set(password.to_string()),
            connection_string: Set(connection_string.to_string()),
            ..Default::default()
        };

        let res = ConnectionsEntity::insert(active_model)
            .exec(&self.db)
            .await?;
        Ok(ConnectionsEntity::find_by_id(res.last_insert_id)
            .one(&self.db)
            .await?
            .unwrap())
    }

    pub async fn delete(&self, id: i32) -> Result<u64, sea_orm::DbErr> {
        let res = ConnectionsEntity::delete_by_id(id).exec(&self.db).await?;
        Ok(res.rows_affected)
    }

    pub async fn delete_all(&self) -> Result<u64> {
        Ok(ConnectionsEntity::delete_many()
            .exec(&self.db)
            .await?
            .rows_affected)
    }
}
