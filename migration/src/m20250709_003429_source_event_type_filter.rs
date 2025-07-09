use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum  Source {
    Table,
    SubmissionTypes,
    TypeFilter,
    
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Source::Table)
                    .add_column(
                        ColumnDef::new(Source::SubmissionTypes)
                        .null()
                        .json_binary()
                    )
                    .add_column(
                        ColumnDef::new(Source::TypeFilter)
                        .not_null()
                        .array(ColumnType::Integer)
                        .default(Expr::cust("'{}'::integer[]"))
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Source::Table)
                    .drop_column(Source::SubmissionTypes)
                    .drop_column(Source::TypeFilter)
                    .to_owned()
            ).await
        
    }
}
