use std::sync::Arc;

use crate::entities::ConnectionModel;
use crate::oracle::OracleClient;
use crate::repo::ConnectionRepository;
use anyhow::{Context, Result, ensure};

pub struct ConnectionService {
    repo: Arc<ConnectionRepository>,
}

impl ConnectionService {
    pub fn new(repo: Arc<ConnectionRepository>) -> Self {
        Self { repo }
    }

    pub fn get_repo(&self) -> Arc<ConnectionRepository> {
        self.repo.clone()
    }

    pub async fn delete_by_name(&self, name: &str) -> Result<ConnectionModel> {
        let connection = self
            .repo
            .find_by_name(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connection '{}' not found", name))?;

        let res = self.repo.delete(connection.id).await?;
        ensure!(res > 0, "Connection '{}' could not be deleted", name);

        Ok(connection)
    }

    pub async fn prune(&self) -> Result<u64> {
        self.repo.delete_all().await
    }

    pub async fn create(
        &self,
        name: &str,
        username: &str,
        password: &str,
        connection_string: &str,
    ) -> Result<ConnectionModel> {
        if self.repo.exists_by_name(name).await? {
            anyhow::bail!(
                "Connection with name '{}' already exists. Connection names are case-insensitive and must be unique.",
                name
            );
        }

        self.repo
            .create(name, username, password, connection_string)
            .await
    }

    pub async fn get_all(&self) -> Result<Vec<ConnectionModel>> {
        self.repo.get_all().await
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ConnectionModel>> {
        self.repo.find_by_id(id).await
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<ConnectionModel>> {
        self.repo.find_by_name(name).await
    }

    pub async fn get_by_id(&self, id: i32) -> Result<ConnectionModel> {
        self.repo.get_by_id(id).await
    }

    pub async fn ping(&self, name: &str) -> Result<ConnectionModel> {
        let connection = self
            .repo
            .find_by_name(name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .ok_or_else(|| anyhow::anyhow!("Connection '{}' not found", name))?;

        self.test(
            &connection.username,
            &connection.password,
            &connection.connection_string,
        )
        .await?;

        Ok(connection)
    }

    pub async fn test(
        &self,
        username: &str,
        password: &str,
        connection_string: &str,
    ) -> Result<()> {
        let client = OracleClient::connect(username, password, connection_string)
            .context("Failed to connect to Oracle database")?;

        client
            .conn
            .query_row("select 1 from dual", &[])
            .context("Failed to execute test query on Oracle database")?;
        Ok(())
    }
}
