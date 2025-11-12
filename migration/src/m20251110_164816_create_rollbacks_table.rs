use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Rollbacks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Rollbacks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Rollbacks::ChangeId).integer().not_null())
                    .col(ColumnDef::new(Rollbacks::Script).text().not_null())
                    .col(
                        ColumnDef::new(Rollbacks::Status)
                            .string_len(255)
                            .not_null()
                            .default("IDLE"),
                    )
                    .col(ColumnDef::new(Rollbacks::Error).text())
                    .col(
                        ColumnDef::new(Rollbacks::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Rollbacks::UpdatedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Rollbacks::Table, Rollbacks::ChangeId)
                            .to(Changes::Table, Changes::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Rollbacks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Rollbacks {
    Table,
    Id,
    ChangeId,
    Script,
    Status,
    Error,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Changes {
    Table,
    Id,
}
