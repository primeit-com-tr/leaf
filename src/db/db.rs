use crate::config::Settings;
use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};

pub async fn init_db(settings: &Settings) -> Result<DatabaseConnection> {
    let db = Database::connect(&settings.database.url).await?;
    Ok(db)
}
