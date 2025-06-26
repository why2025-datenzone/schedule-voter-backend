use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Event::Table)
                    .col(pk_auto(Event::Id))
                    .col(text(Event::Title))
                    .col(text_uniq(Event::Slug))
                    .col(boolean(Event::Voting))
                    .col(json_binary(Event::Content))
                    .col(integer(Event::Version))
                    .to_owned(),
            )
            .await?;
        manager.create_table(
            Table::create()
                .table(Vote::Table)
                .col(text(Vote::Client))
                .col(integer(Vote::Sequence))
                .col(integer(Vote::Event))
                .foreign_key(ForeignKey::create()
                    .name("fk-vote-event")
                    .from(Vote::Table, Vote::Event)
                    .to(Event::Table, Event::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade))
                .col(array(Vote::Shown, ColumnType::Text))
                .col(array(Vote::Expanded, ColumnType::Text))
                .col(array(Vote::Up, ColumnType::Text))
                .col(array(Vote::Down, ColumnType::Text))
                .primary_key(
                    Index::create().col(Vote::Client).col(Vote::Event)
                )
                .to_owned(),
            ).await?;
        // manager.create_index(
        //     Index::create()
        //         .table(Vote::Table)
        //         .name("uq-vote-event-client") // A descriptive name for the unique constraint
        //         .unique()
        //         .col(Vote::Event) // First column of the composite key
        //         .col(Vote::Client) // Second column of the composite key
        //         .to_owned()
        //     ).await?;
        manager.create_table(
            Table::create()
                .table(OidcProvider::Table)
                .col(
                    ColumnDef::new(OidcProvider::Name) // Use ColumnDef::new for more control
                        .text() // Or .text() if you prefer TEXT type. .string() often maps to VARCHAR.
                        .not_null() // Primary keys must be not null
                        .primary_key() // This makes 'name' the primary key
                )
                .col(
                    ColumnDef::new(OidcProvider::Metadata)
                        .json_binary() // Use json_binary() if you want to store it as a binary JSON type
                        .not_null() // Assuming metadata should always be present
                )
                .col(string(OidcProvider::Authurl))
                .to_owned()
        ).await?;
        manager.create_table(
            Table::create()
                .table(User::Table)
                .col(pk_auto(User::Id))
                .col(text(User::Name))
                .col(text_uniq(User::Email))
                .col(text_null(User::Password))
                .col(text_null(User::Provider))
                .col(text_null(User::Account))
                .col(boolean(User::CanCreate))
                .to_owned()
        ).await?;
        manager.create_index(
            Index::create()
                .table(User::Table)
                .name("unique-user-provider-account")
                .unique()
                .col(User::Provider)
                .col(User::Account)
                .to_owned()
        ).await?;
        manager.create_table(
            Table::create()
                .table(Token::Table)
                .col(text(Token::Value))
                .col(integer(Token::User))
                .col(integer(Token::Creator))
                .col(
                    ColumnDef::new(Token::Created)
                        .timestamp()
                        .default(Expr::current_timestamp())
                        .not_null()
                        
                )
                .primary_key(
                    Index::create().col(Token::Value)
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk-token-user")
                    .from(Token::Table, Token::User)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        manager.create_table(
            Table::create()
                .table(UserEvent::Table)
                .col(
                    ColumnDef::new(UserEvent::User)
                    .integer()
                    .not_null()
                )
                .col(ColumnDef::new(UserEvent::Event)
                    .integer()
                    .not_null()
                )
                .col(text(UserEvent::Perm))
                .foreign_key(ForeignKey::create()
                    .name("fk-userevent-user")
                    .from(UserEvent::Table, UserEvent::User)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(ForeignKey::create()
                    .name("fk-userevent-event")
                    .from(UserEvent::Table, UserEvent::Event)
                    .to(Event::Table, Event::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .primary_key(
                    Index::create().col(UserEvent::Event).col(UserEvent::User)
                )
            .to_owned()
        ).await?;
        manager.create_table(
            Table::create()
                .table(Source::Table)
                .col(pk_auto(Source::Id))
                .col(integer(Source::Event))
                .foreign_key(ForeignKey::create()
                    .name("fk-source-event")
                    .from(UserEvent::Table, Source::Event)
                    .to(Event::Table, Event::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .col(text(Source::Slug))
                .col(text_null(Source::Url))
                .col(text_null(Source::EventSlug))
                .col(text(Source::Updatekey))
                .col(integer_null(Source::Interval))
                .col(text_null(Source::Apikey))
                .col(integer_null(Source::Filter))
                .col(
                    ColumnDef::new(Source::Autoupdate)
                    .boolean()
                    .not_null()
                    .default(false)
                )
                .col(timestamp_null(Source::LastUpdate))
                .col(timestamp_null(Source::LastAttemp))
            .to_owned()
        ).await?;
        manager.create_index(
            Index::create()
                .table(Source::Table)
                .name("unique-event-slug")
                .unique()
                .col(Source::Event)
                .col(Source::Slug)
                .to_owned()
        ).await?;
        manager.create_table(
            Table::create()
                .table(Submission::Table)
                .col(text(Submission::Code))
                .col(text(Submission::Title))
                .col(text(Submission::Abstract))
                .col(text(Submission::Html))
                .col(integer(Submission::Source))
                .col(timestamp_null(Submission::Start))
                .col(timestamp_null(Submission::End))
                .col(
                    ColumnDef::new(Submission::Created)
                        .timestamp()
                        .default(Expr::current_timestamp())
                        .not_null()
                )
                .col(
                    ColumnDef::new(Submission::Updated)
                        .timestamp()
                        .default(Expr::current_timestamp())
                        .not_null()
                )
                .foreign_key(ForeignKey::create()
                    .name("fk-submission-source")
                    .from(Submission::Table, Submission::Source)
                    .to(Source::Table, Source::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .primary_key(
                    Index::create().col(Submission::Source).col(Submission::Code)
                )
                .to_owned()
        ).await

        // manager.create_index(
        //     Index::create()
        //         .table(UserEvent::Table)
        //         .name("unique-user-event") // A descriptive name for the unique constraint
        //         .unique()
        //         .col(UserEvent::User) // First column of the composite key
        //         .col(UserEvent::Event) // Second column of the composite key
        //         .to_owned()
        // ).await

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Source::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Vote::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OidcProvider::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Token::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserEvent::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
        
    }
}

#[derive(DeriveIden)]
enum Event {
    Table,
    Id,
    Title,
    Slug,
    Voting,
    Content,
    Version,
}

#[derive(DeriveIden)]
enum Vote {
    Table,
    Client,
    Event,
    Sequence,
    Shown,
    Expanded,
    Up,
    Down,
}

#[derive(DeriveIden)]
enum OidcProvider {
    Table,
    Name,
    Metadata,
    Authurl,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Name,
    Email,
    Password,
    Provider,
    Account,
    CanCreate,
}

#[derive(DeriveIden)]
enum UserEvent {
    Table,
    User,
    Event,
    Perm,
}

#[derive(DeriveIden)]
enum Token {
    Table,
    User,
    Created,
    Value,
    Creator,
}

#[derive(DeriveIden)]
enum  Source {
    Table,
    Id,
    Event,
    Slug,
    Url,
    EventSlug,
    Interval,
    Apikey,
    Filter,
    Autoupdate,
    LastUpdate,
    LastAttemp,
    Updatekey,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    Source,
    Title,
    Abstract,
    Code,
    Html,
    Start,
    End,
    Created,
    Updated,
}