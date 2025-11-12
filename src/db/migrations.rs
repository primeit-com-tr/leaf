use migration::MigratorTrait;
use sea_orm::DatabaseConnection;
use tracing::{error, info};

pub async fn up(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    info!("Running migrations");
    match migration::Migrator::up(db, None).await {
        Ok(_) => {
            info!("✅ Migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to run migrations: {}", e);
            Err(e)
        }
    }
}

pub async fn down(db: &DatabaseConnection, steps: u32) -> Result<(), sea_orm::DbErr> {
    info!("Rolling back {} migration(s)", steps);
    match migration::Migrator::down(db, Some(steps)).await {
        Ok(_) => {
            info!("✅ Migrations rolled back successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to rollback migrations: {}", e);
            Err(e)
        }
    }
}
