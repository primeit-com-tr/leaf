pub use sea_orm_migration::prelude::*;

mod m20251019_194941_create_connections_table;
mod m20251025_205133_create_plans_table;
mod m20251025_220607_create_changesets_table;
mod m20251103_185312_create_deployments_table;
mod m20251105_220916_create_changes_table;
mod m20251110_164816_create_rollbacks_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251019_194941_create_connections_table::Migration),
            Box::new(m20251025_205133_create_plans_table::Migration),
            Box::new(m20251025_220607_create_changesets_table::Migration),
            Box::new(m20251103_185312_create_deployments_table::Migration),
            Box::new(m20251105_220916_create_changes_table::Migration),
            Box::new(m20251110_164816_create_rollbacks_table::Migration),
        ]
    }
}
