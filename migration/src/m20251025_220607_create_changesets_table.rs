use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Changesets::Table)
                    .if_not_exists()
                    .col(pk_auto(Changesets::Id))
                    .col(integer(Changesets::DeploymentId).not_null())
                    .col(string(Changesets::ObjectType).not_null())
                    .col(string(Changesets::ObjectName).not_null())
                    .col(string(Changesets::ObjectOwner).not_null())
                    .col(timestamp_null(Changesets::SourceDdlTime))
                    .col(text_null(Changesets::SourceDdl))
                    .col(timestamp_null(Changesets::TargetDdlTime))
                    .col(text_null(Changesets::TargetDdl))
                    .col(string(Changesets::Status).not_null().default("IDLE"))
                    .col(text_null(Changesets::Errors))
                    .col(
                        timestamp(Changesets::CreatedAt)
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(timestamp_null(Changesets::UpdatedAt))
                    .col(timestamp_null(Changesets::StartedAt))
                    .col(timestamp_null(Changesets::EndedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Changesets::Table, Changesets::DeploymentId)
                            .to(Deployments::Table, Deployments::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on deployment_id for faster queries
        manager
            .create_index(
                Index::create()
                    .name("idx_changes_deployment_id")
                    .table(Changesets::Table)
                    .col(Changesets::DeploymentId)
                    .to_owned(),
            )
            .await?;

        // Create index on status for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_changes_status")
                    .table(Changesets::Table)
                    .col(Changesets::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_changes_status")
                    .table(Changesets::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_changes_plan_id")
                    .table(Changesets::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Changesets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Changesets {
    Table,
    Id,
    DeploymentId,
    ObjectType,
    ObjectName,
    ObjectOwner,
    SourceDdlTime,
    SourceDdl,
    TargetDdlTime,
    TargetDdl,
    Status,
    Errors,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
}

#[derive(DeriveIden)]
enum Deployments {
    Table,
    Id,
}
