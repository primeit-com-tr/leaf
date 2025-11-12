use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Changes::Table)
                    .if_not_exists()
                    .col(pk_auto(Changes::Id))
                    .col(integer(Changes::ChangesetId).not_null())
                    .col(text(Changes::Script).not_null())
                    .col(text(Changes::RollbackScript))
                    .col(string(Changes::Status).not_null().default("IDLE"))
                    .col(text_null(Changes::Error))
                    .col(
                        timestamp(Changes::CreatedAt)
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(timestamp_null(Changes::UpdatedAt))
                    .col(timestamp_null(Changes::StartedAt))
                    .col(timestamp_null(Changes::EndedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Changes::Table, Changes::ChangesetId)
                            .to(Changesets::Table, Changesets::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Changes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Changes {
    Table,
    Id,
    ChangesetId,
    Script,
    RollbackScript,
    Status,
    Error,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
}

#[derive(DeriveIden)]
enum Changesets {
    Table,
    Id,
}
