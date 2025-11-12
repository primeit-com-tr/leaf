use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    // Set DATABASE_URL from LEAF__DATABASE__URL if it exists
    if let Ok(db_url) = std::env::var("LEAF__DATABASE__URL") {
        std::env::set_var("DATABASE_URL", db_url);
    }

    cli::run_cli(migration::Migrator).await;
}
