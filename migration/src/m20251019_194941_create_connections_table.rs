use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Connections::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Connections::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(Connections::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Connections::Username).string().not_null())
                    .col(ColumnDef::new(Connections::Password).string().not_null())
                    .col(
                        ColumnDef::new(Connections::ConnectionString)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Connections::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Connections::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Connections {
    Table,
    Id,
    Name,
    Username,
    Password,
    ConnectionString,
    CreatedAt,
}
