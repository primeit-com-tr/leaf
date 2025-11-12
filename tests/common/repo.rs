use anyhow::{Context, Result};
use leaf::db::migrations;
use sea_orm::DatabaseConnection;

pub async fn init_repo(db: &DatabaseConnection) -> Result<()> {
    migrations::up(&db)
        .await
        .context("Failed to init repository objects.")
}
