use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Deployments::Table)
                    .if_not_exists()
                    .col(pk_auto(Deployments::Id))
                    .col(integer(Deployments::PlanId).not_null())
                    .col(timestamp(Deployments::CutoffDate).not_null())
                    .col(text(Deployments::Payload).not_null())
                    .col(string(Deployments::Status).not_null().default("IDLE"))
                    .col(text_null(Deployments::Errors))
                    .col(
                        timestamp(Deployments::CreatedAt)
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(timestamp_null(Deployments::UpdatedAt))
                    .col(timestamp_null(Deployments::StartedAt))
                    .col(timestamp_null(Deployments::EndedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Deployments::Table, Deployments::PlanId)
                            .to(Plans::Table, Plans::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Deployments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Deployments {
    Table,
    Id,
    PlanId,
    CutoffDate,
    Payload,
    Status,
    Errors,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
}

#[derive(DeriveIden)]
enum Plans {
    Table,
    Id,
}
