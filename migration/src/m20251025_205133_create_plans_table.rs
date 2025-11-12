use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Plans::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Plans::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Plans::Name).string().not_null().unique_key())
                    .col(
                        ColumnDef::new(Plans::SourceConnectionId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Plans::TargetConnectionId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Plans::Schemas).text().not_null())
                    .col(ColumnDef::new(Plans::ExcludeObjectTypes).text())
                    .col(ColumnDef::new(Plans::ExcludeObjectNames).text())
                    .col(ColumnDef::new(Plans::DisabledDropTypes).text())
                    .col(
                        ColumnDef::new(Plans::FailFast)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Plans::Status)
                            .string()
                            .not_null()
                            .default("IDLE"),
                    )
                    .col(
                        ColumnDef::new(Plans::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // 🔗 Foreign keys
                    .foreign_key(
                        ForeignKey::create()
                            .from(Plans::Table, Plans::SourceConnectionId)
                            .to(Connections::Table, Connections::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Plans::Table, Plans::TargetConnectionId)
                            .to(Connections::Table, Connections::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Plans::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Plans {
    Table,
    Id,
    Name,
    SourceConnectionId,
    TargetConnectionId,
    Schemas,
    ExcludeObjectTypes,
    ExcludeObjectNames,
    DisabledDropTypes,
    FailFast,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Connections {
    Table,
    Id,
}
