use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use tracing::{debug, info};

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base58::ToBase58;
use entity::{oidc_provider, source, submission, token, user, user_event};
use ::entity::{event, vote}; use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sea_orm::sqlx::types::chrono::Utc;
use sea_orm::ActiveValue::NotSet;
// Entities
use sea_orm::{
    ActiveModelBehavior, Condition, ConnectionTrait, DatabaseTransaction, DbConn, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set, Statement as SeaOrmStatement, TransactionTrait // For ActiveModel in seed_event
};
use serde_json::Value;
use serde::{Deserialize, Serialize};
use sea_orm::sea_query::{
    Expr, InsertStatement, IntoIden, OnConflict, PostgresQueryBuilder, Query // For converting Column enums to Iden
};

use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use serde_repr::Serialize_repr;
use slug::slugify;

use crate::{RoleInfo, SourceFilter};

pub struct Mutation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Votes {
    pub shown: Vec<String>,
    pub expanded: Vec<String>,
    pub up: Vec<String>,
    pub down: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateVoteRequest {
    pub sequence: u32,
    pub votes: Votes,
    pub event: String,
    pub client: String,
}


#[derive(Deserialize)]
pub struct DeleteVoteRequest {
    #[serde(rename = "clientIds")]
    pub client_ids: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub voting_enabled: Option<bool>,
}

#[derive(Clone,Debug,Deserialize)]
pub struct SubmissionStartEnd {
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
}

#[derive(Debug,Deserialize)]
pub struct SubmissionSyncItem {
    pub title: String,
    pub r#abstract: String,
    pub code: String,
    pub schedule_time: Option<SubmissionStartEnd>,
}

pub struct SyncSourceRequest {
    pub source_id: i32,
    pub submission_types: Option<HashMap<i32, String>>,
    pub submissions: HashMap<String, SubmissionSyncItem>,
}


#[derive(Serialize)]
struct ContentItem {
    code: String,
    title: String,
    r#abstract: String,
}

#[derive(Serialize)]
#[serde(transparent)]
struct ContentValue {
    items: HashMap<String, ContentItem>,
}

#[derive(Serialize,Deserialize)]
#[serde(transparent)]
pub struct SubmissionTypesField {
    pub items: HashMap<i32, String>,
}


// impl From<UpdateEventRequest> for event::ActiveModel {
//     fn from(value: UpdateEventRequest) -> Self {
//         let mut model = Self::new()
//         if let Some(name) = value.name {
//             model.title = Set(name)
//         }
//         if let Some(ve) = value.voting_enabled {
//             model.voting = Set(ve)
//         }
//     }
// }

#[derive(Serialize_repr)]
#[repr(i32)]
pub enum TokenCreator {
    OpenID = 0,
    UserPassword = 1,
}


#[derive(Deserialize)]
pub struct UpdateSourceRequest {
    autoupdate: Option<bool>,
    filter: Option<SourceFilter>,
    interval: Option<i32>,
    url: Option<String>,
    #[serde(rename="eventSlug")]
    event_slug: Option<String>,
    apikey: Option<String>,
    #[serde(rename="typeFilter")]
    type_filter: Option<Vec<i32>>,
}

#[derive(Deserialize)]
pub struct CreateEventRequest {
    name: String,
    slug: String,
}

impl Mutation {


    pub fn gentoken() -> String {
        let mut rng = ChaCha20Rng::from_os_rng();
        //let mut respondToken: [u8;64];
        //rng.fill_bytes(&mut respondToken);
        let mut response_token = [0u8; 16];
        rng.fill(&mut response_token);
        let response_string = response_token.to_base58();
        response_string
    }


    pub async fn update_source_attemp(db: &DbConn, source: i32) -> Result<(), DbErr> {
        source::Entity::update_many()
            .col_expr(source::Column::LastAttemp, Expr::current_timestamp().into())
            .filter(source::Column::Id.eq(source))
            .exec(db)
            .await?;
        Ok(())
    }

    pub fn update_submission_time(code: &String, source_id: i32, old: &submission::Model, schedule_time: &Option<&SubmissionStartEnd>) -> Option<submission::ActiveModel> {
        match schedule_time.as_ref() {
                        Some(scheduled_time) => {
                            if (old.start == None) || (old.end == None) {
                                Some(submission::ActiveModel {
                                    code: Set(code.clone()),
                                    title: NotSet,
                                    r#abstract: NotSet,
                                    html: NotSet,
                                    source: Set(source_id),
                                    start: Set(schedule_time.as_ref().map(|t| t.start.naive_utc())),
                                    end: Set(schedule_time.as_ref().map(|t| t.end.naive_utc())),
                                    created: NotSet,
                                    updated: NotSet,
                                })
                            } else {
                                let startdiff = old.start.unwrap().and_utc() - scheduled_time.start;
                                let enddiff = old.end.unwrap().and_utc() - scheduled_time.end.to_utc();
                                if (startdiff.num_milliseconds() > 500) || (enddiff.num_milliseconds() > 500) {
                                    Some(submission::ActiveModel {
                                        code: Set(code.clone()),
                                        title: NotSet,
                                        r#abstract: NotSet,
                                        html: NotSet,
                                        source: Set(source_id),
                                        start: Set(schedule_time.as_ref().map(|t| t.start.naive_utc())),
                                        end: Set(schedule_time.as_ref().map(|t| t.end.naive_utc())),
                                        created: NotSet,
                                        updated: NotSet,
                                    })
                                } else {
                                    None
                                }
                            }
                            
                        },
                        None => {
                            if !((old.start == None) || (old.end == None)) {
                                Some(submission::ActiveModel {
                                    code: Set(code.clone()),
                                    title: NotSet,
                                    r#abstract: NotSet,
                                    html: NotSet,
                                    source: Set(source_id),
                                    start: Set(None),
                                    end: Set(None),
                                    created: NotSet,
                                    updated: NotSet,
                                })
                            } else {
                                None
                            }
                        }
                    }
    }

    pub async fn sync_schedule(db: &DbConn, source_id: i32, schedule: HashMap<String, SubmissionStartEnd>) -> Result<(), DbErr> {
        debug!("Syncing schedule for source {}", source_id);
        db.transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                // We use it only for locking
                let _current_source = source::Entity::find_by_id(source_id)
                    .lock_exclusive()
                    .one(txn)
                    .await?
                    .ok_or_else(|| DbErr::RecordNotFound("Could not find source".to_string()))?;
                    
                let current_submissions = submission::Entity::find()
                    .filter(submission::Column::Source.eq(source_id))
                    .all(txn)
                    .await?;

                for m in current_submissions.into_iter() {
                    let to_update = Self::update_submission_time(&m.code, source_id, &m, &schedule.get(&m.code));
                    match to_update {
                        Some(m) => {m.update(txn).await?;},
                        None => (),
                    }
                }
                Ok(())
        })}).await.map_err(|_e| DbErr::RecordNotInserted)?;
        Ok(())
    }

    pub async fn sync_source(db: &DbConn, req: SyncSourceRequest) -> Result<(), DbErr> {
        // Get the current submissions
        db.transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let current_source = source::Entity::find_by_id(req.source_id)
                    .lock_exclusive()
                    .one(txn)
                    .await?
                    .ok_or_else(|| DbErr::RecordNotFound("Could not find source".to_string()))?;
                    
                let current_submissions = submission::Entity::find()
                    .filter(submission::Column::Source.eq(req.source_id))
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(|m| (m.code.clone(), m))
                    .collect::<HashMap<String, submission::Model>>();
                
                let current_codes = current_submissions.keys().cloned().collect::<HashSet<_>>();
                let new_codes = req.submissions.keys().cloned().collect::<HashSet<_>>();
                let to_delete_codes = current_codes.difference(&new_codes).cloned().collect::<Vec<_>>();
                let to_create = new_codes.difference(&current_codes).cloned().collect::<Vec<_>>();
                let to_potential_update = current_codes.intersection(&new_codes);
                let mut real_update = false;
                let updates = to_potential_update.filter_map(|code| {
                    let old: &submission::Model = current_submissions.get(code).unwrap();
                    let new: &SubmissionSyncItem = req.submissions.get(code).unwrap();
                    if (old.r#abstract != new.r#abstract) || (old.title != new.title) {
                        real_update = true;
                        eprintln!("Preparing real update: old {:?} new {:?}", old, new);
                        return Some(submission::ActiveModel {
                            code: Set(code.clone()),
                            title: Set(new.title.clone()),
                            r#abstract: Set(new.r#abstract.clone()),
                            html: Set(markdown::to_html(&new.r#abstract)),
                            source: Set(req.source_id),
                            start: Set(new.schedule_time.as_ref().map(|t| t.start.naive_utc())),
                            end: Set(new.schedule_time.as_ref().map(|t| t.end.naive_utc())),
                            created: NotSet,
                            updated: Set(Utc::now().naive_utc()),
                        })
                    }
                    Self::update_submission_time(code, req.source_id, old, &new.schedule_time.as_ref())
                }).collect::<Vec<_>>();
                if (!to_delete_codes.is_empty()) || (!to_create.is_empty()) {
                    real_update = true;
                }
                
                let mut force_update = false;
                if let Some(ref submission_types) = req.submission_types {
                        if let Some(old_types) = &current_source.submission_types {
                            if let Ok(old_types_map) = serde_json::from_value::<SubmissionTypesField>(old_types.clone()) {
                                if !(old_types_map.items == *submission_types) {
                                    force_update = true;
                                }
                            } else {
                                force_update = true;
                            }
                        } else {
                            force_update = true;
                        }
                        
                    }
                    
                info!("Status: to_delete: {:?} to_create: {:?} updates: {:?}", to_delete_codes, to_create, updates);
                if (!to_delete_codes.is_empty()) || (!to_create.is_empty()) || (!updates.is_empty()) || force_update {
                    
                    for code in to_delete_codes.into_iter() {
                        submission::Entity::delete_many()
                            .filter(submission::Column::Source.eq(req.source_id))
                            .filter(submission::Column::Code.eq(code))
                            .exec(txn)
                            .await?;
                    }
                    for new_code in to_create.into_iter() {
                        let template = req.submissions.get(&new_code).unwrap().to_owned();
                        submission::ActiveModel {
                            code: Set(new_code.clone()),
                            title: Set(template.title.clone()),
                            html: Set(markdown::to_html(&template.r#abstract)),
                            r#abstract: Set(template.r#abstract.clone()),
                            source: Set(req.source_id),
                            start: Set(template.schedule_time.as_ref().map(|t| t.start.naive_utc())),
                            end: Set(template.schedule_time.as_ref().map(|t| t.end.naive_utc())),
                            created: Set(Utc::now().naive_utc()),
                            updated: Set(Utc::now().naive_utc()),
                        }.insert(txn).await?;
                    }
                    for model in updates {
                        model.update(txn).await?;
                    }
                    let event = current_source.event;
                    let mut am = current_source.into_active_model();
                    am.set(source::Column::LastUpdate, Utc::now().naive_utc().into());
                    if let Some(submission_types) = req.submission_types {
                        let stf = SubmissionTypesField{items: submission_types};
                        // info!("Setting submission types: {}", stf);
                        let v = serde_json::to_value(stf).unwrap();
                        info!("v is {}", v);
                        am.set(source::Column::SubmissionTypes, v.into());
                    }
                    am.update(txn).await?;
                    if real_update {
                        Self::update_event_schedule(txn, event).await?;
                    }
                }
            Ok(())
        })}).await.map_err(|_e| DbErr::RecordNotInserted)?;
        Ok(())
    }

    pub async fn update_event_schedule(txn: &DatabaseTransaction, event: i32) -> Result<(), DbErr>{
        let event_model = event::Entity::find_by_id(event).lock_exclusive().one(txn).await?.ok_or_else(|| DbErr::RecordNotFound("Invalid event id".to_string()))?;
        let submissions = source::Entity::find()
            .find_with_related(submission::Entity)
            .filter(source::Column::Event.eq(event))
            .all(txn)
            .await?;
        let res = submissions.into_iter().flat_map(|(source, submission_vec)| {
            submission_vec.into_iter().map(move |submission| {
                (
                    format!("{}/{}", &source.slug, &submission.code), 
                    ContentItem { 
                        code: submission.code, 
                        title: submission.title, 
                        r#abstract: submission.html
                    }
                )
            })
        }).collect::<HashMap<String, ContentItem>>();
        let value = serde_json::to_value(ContentValue{items: res}).unwrap();
        let newver = event_model.version+1;
        let mut am = event_model.into_active_model();
        am.set(event::Column::Content, value.into());
        am.set(event::Column::Version, newver.into());
        am.update(txn).await?;
        Ok(())
    }

    pub async fn create_event(db: &DbConn, req: CreateEventRequest, user: i32) -> Result<(), DbErr> {
        if (req.slug.eq("")) || (req.slug.eq("admin")) || (req.slug.eq("static")) || (req.slug.eq("api")) || (req.slug != slugify(&req.slug)) {
            return Err(DbErr::Custom(format!("Invalid slug: {}", &req.slug)))
        }
        db.transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
            let model = event::ActiveModel {
                title: Set(req.name),
                slug: Set(req.slug),
                voting: Set(false),
                content: Set(serde_json::json!("{}")),
                version: Set(0),
                ..Default::default()
            };
            // let event_insert_result = event::ActiveModel::insert(model, txn).await?;
            let res = model.insert(txn).await?;
            let even_id = res.id;
            user_event::ActiveModel {
                event: Set(even_id),
                user: Set(user),
                perm: Set(RoleInfo::Admin.into())
            }.insert(txn).await?;
            Ok(())
        })}).await.map_err(|_e| DbErr::RecordNotInserted)?;

        return Ok(())
    }

    pub async fn create_user(db: &DbConn, name: String, email: String, password: String, can_create: bool) -> Result<(), DbErr> {
        let salt = SaltString::generate(&mut OsRng);

        let hash = Argon2::default()
            .hash_password(
                password.as_bytes(), 
                &salt
            )
            .map_err(
                |_e| DbErr::Custom("Could not hash password".to_string())
            )?.to_string();

        user::ActiveModel {
            name: Set(name),
            email: Set(email),
            password: Set(Some(hash)),
            can_create: Set(can_create),
            ..Default::default()
        }.insert(db).await?;
        Ok(())
    }

    pub async fn login(db: &DbConn, email: String, password: String) -> Result<String, DbErr> {
        let maybe_user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await?;
        if let Some(user) = maybe_user {
            if let Some(password_hash) = user.password {
                Argon2::default()
                    .verify_password(
                        password.as_bytes(), 
                        &PasswordHash::new(&password_hash)
                            .map_err(|_e| DbErr::Custom("Could not parse password hash".to_string()))?
                    ).map_err(|_e| DbErr::Custom("Invalid password".to_string()))?;
                let token = Self::gentoken();
                Self::insert_token(db, user.id, token.clone(), TokenCreator::UserPassword).await?;
                return Ok(token)
            }
        }
        Err(DbErr::Custom("Login failed".to_string()))
    }

    pub async fn insert_token(db: &DbConn, user: i32, token: String, creator: TokenCreator) -> Result<(), DbErr> {
        let x = token::ActiveModel {
            value: Set(token),
            user: Set(user),
            creator: Set(creator as i32),
            ..Default::default()
        };
        x.insert(db).await?;
        Ok(())
    }

    pub async fn get_or_create_oid_user(db: &DbConn, provider: String, name: String, email: String, account: String, can_create: bool) -> Result<Option<i32>, DbErr> {

        let on_conflict_provider_clause = OnConflict::columns(
            [
                user::Column::Provider, 
                user::Column::Account, 
            ]
        )
        .update_columns(
            [
                user::Column::Name,
                user::Column::Email,
            ]
        ).to_owned();

        // let on_conflict_email_clause = OnConflict::columns(
        //     [
        //         user::Column::Email,
        //     ]
        // ).update_columns(
        //     [
        //         user::Column::Name,
        //     ]
        // )
        // .action_and_where(
        //     user::Column::Provider.eq(&provider).and(user::Column::Account.eq(&account))
        // ).to_owned();
        
        let insert_statement = InsertStatement::new()
            .into_table(user::Entity)
            .columns(
                [
                    user::Column::Provider, 
                    user::Column::Account, 
                    user::Column::Name, 
                    user::Column::Email, 
                    user::Column::CanCreate,
                ]
            )
            .values(
                [
                    provider.into(),
                    account.into(),
                    name.into(),
                    email.into(),
                    can_create.into(),
                ]
            )
            .unwrap()
            .returning(
                Query::returning().column(user::Column::Id)
            )
            .on_conflict(on_conflict_provider_clause)
            .to_owned();
        
        let (sql_str, sql_values) = insert_statement.build(PostgresQueryBuilder);

        // For debugging:
        println!("Executing SQL: {}", sql_str);
        println!("With Values: {:?}", sql_values);

        let sea_orm_stmt = SeaOrmStatement::from_sql_and_values(db.get_database_backend(), sql_str, sql_values);

        // 5. Execute the single, combined SQL statement
        let res = db.query_one(sea_orm_stmt).await?;
        match res {
            Some(v) => {
                let id: i32 = v.try_get("", "id")?;
                Ok(Some(id))
            },
            None => Ok(None),
        }
    }


    pub async fn seed_event(db: &DbConn) -> Result<(), DbErr> {
        let submissions = r#"{"SUB001":{"title":"Initial Submission 1","code":"abc123","abstract":"Abstract 1..."},"SUB002":{"title":"Another Submission","code":"abc123","abstract":"Abstract 2..."},"SUB003":{"title":"Will Be Deleted","code":"bad23","abstract":"Abstract 3..."},"SUB004":{"title":"Stable Submission","code":"c55","abstract":"Abstract 4..."}}"#;
        let data: Value = serde_json::from_str(submissions).unwrap();
        event::ActiveModel{
            slug: Set("test-event".into()),
            title: Set("Test Event".into()),
            // id: Set(1),
            voting: Set(true),
            version: Set(1),
            content: Set(data),
            ..Default::default()
        }.save(db).await?;
        Ok(())
    }

    pub async fn update_source(db: &DbConn, event: i32, source: impl Into<String>, update: UpdateSourceRequest) -> Result<(), DbErr> {
        let slug = source.into();
        if slug.eq("") || (slugify(&slug) != slug) {
            return Err(DbErr::Custom(format!("invalid slug: {}", &slug)))
        }
        let mut rng = ChaCha20Rng::from_os_rng();
        let mut response_token = [0u8; 16];
        rng.fill(&mut response_token);
        let updatekey = response_token.to_base58();

        let mut model = source::ActiveModel::new();
        model.slug = Set(slug);
        model.event = Set(event);
        model.updatekey = Set(updatekey);
        
        let mut to_update = Vec::new();
        if let Some(interval) = update.interval {
            if (interval < 60) || (interval > 7200) {
                return Err(DbErr::Custom(format!("Invalid interval: {}", interval)))
            }
            model.interval = Set(Some(interval));
            to_update.push(source::Column::Interval);
        }
        if let Some(autoupdate) = update.autoupdate {
            model.autoupdate = Set(autoupdate);
            to_update.push(source::Column::Autoupdate);
        }
        if let Some(filter) = update.filter {
            model.filter = Set(Some(filter as i32));
            to_update.push(source::Column::Filter);
        }
        if let Some(type_filter) = update.type_filter {
            let hs = type_filter.into_iter().collect::<HashSet<i32>>();
            model.type_filter = Set(hs.into_iter().sorted().collect());
            to_update.push(source::Column::TypeFilter);
        }
        if let Some(apikey) = update.apikey {
            model.apikey = Set(Some(apikey));
            to_update.push(source::Column::Apikey);
            
            if let Some(url) = update.url {
                url::Url::parse(&url).map_err(|e| DbErr::Custom(format!("Invalid URL: {} {}", url, e)))?;
                model.url = Set(Some(url));
                to_update.push(source::Column::Url);
            }
            if let Some(event_slug) = update.event_slug {
                model.event_slug = Set(Some(event_slug));
                to_update.push(source::Column::EventSlug);
            }
        }
        source::Entity::insert(model)
            .on_conflict(sea_orm::sea_query::OnConflict::columns([source::Column::Slug, source::Column::Event])
            .update_columns(to_update)
            .to_owned()
        )
        .exec(db)
        .await?;
        Ok(())
    }

    
    pub async fn delete_source(db: &DbConn, event: i32, source: impl Into<String>) -> Result<(), DbErr> {
        
        source::Entity::delete_many()
            .filter(source::Column::Event.eq(event))
            .filter(source::Column::Slug.eq(source.into()))
        .exec(db)
        .await?;
        Ok(())
    }

    
    
    pub async fn edit_user(db: &DbConn, conference: i32, user: i32, role: Option<RoleInfo>) -> Result<(), DbErr> {
        if let Some(r) = role  {
            let model = user_event::ActiveModel {
                event: Set(conference),
                user: Set(user),
                perm: Set(r.into()),
            };
            user_event::Entity::insert(model)
                .on_conflict(OnConflict::columns(
                    [
                        user_event::Column::Event,
                        user_event::Column::User
                    ]
                    ).to_owned()
                .update_columns(
                    [
                        user_event::Column::Perm
                    ]
                    ).to_owned()
                )
                .exec(db)
                .await?;
        } else {
            user_event::Entity::delete_many()
                .filter(user_event::Column::Event.eq(conference))
                .filter(user_event::Column::User.eq(user))
                .exec(db)
                .await?;
        }
        Ok(())
    }


    pub async fn update_event(db: &DbConn, event: i32, update: UpdateEventRequest) -> Result<(), DbErr> {
        let mut model = event::ActiveModel::new();
        if let Some(name) = update.name {
            model.title = Set(name)
        }
        if let Some(ve) = update.voting_enabled {
            model.voting = Set(ve)
        }
        if model.is_changed() {
            event::Entity::update_many()
                .set(model)
                .filter(event::Column::Id.eq(event))
                .exec(db)
                .await?;
        }
        Ok(())
    }

    pub async fn update_oicd_provider(db: &DbConn, name: impl Into<String>, authurl: String, provider: serde_json::Value) -> Result<(), DbErr> {
        let m = oidc_provider::ActiveModel {
            name: Set(name.into()),
            metadata: Set(provider),
            authurl: Set(authurl,)
        };
        oidc_provider::Entity::insert(m)
            .on_conflict(OnConflict::column(oidc_provider::Column::Name)
            .update_columns([oidc_provider::Column::Metadata, oidc_provider::Column::Authurl])
            .to_owned()
        )
        .exec(db)
        .await?;
        Ok(())
    }

    pub async fn delete_votes(db: &DbConn, delete_vote_request: DeleteVoteRequest) -> Result<(), DbErr> {
        // delete_vote_request contains mapping from slugs to clientids.
        // Complete this method
        let client_event_pairs = delete_vote_request.client_ids;

        // If the input map is empty, there's nothing to delete.
        if client_event_pairs.is_empty() {
            return Ok(());
        }

        // Build a combined WHERE condition for the EXISTS subquery.
        // This will look like:
        // ( (event.slug = 'slug1' AND vote.client = 'client1') OR (event.slug = 'slug2' AND vote.client = 'client2') OR ... )
        let mut subquery_conditions = Condition::any();

        for (slug, client_id) in client_event_pairs.into_iter() {
            // For each (slug, client_id) pair, create an AND condition:
            // (event.slug = 'some_slug' AND vote.client = 'some_client_id')
            // `.to_owned()` is used as `eq` expects an owned value for comparison.
            subquery_conditions = subquery_conditions.add(
                Condition::all() 
                    .add(event::Column::Slug.eq(slug.to_owned())) 
                    .add(vote::Column::Client.eq(client_id.to_owned())) 
            );
        }

        // Construct the SELECT statement for the WHERE EXISTS subquery.
        // This subquery will select 1 if a matching event exists that corresponds
        // to a vote row from the outer query, based on the `subquery_conditions`.
        let mut subquery = Query::select();
        subquery
            .expr(Expr::val(1)) // Select a constant value (1) as we only care about existence
            .from(event::Entity) // The subquery's primary table is 'event'
            .and_where(
                // This is the correlated condition: links the event.id from the subquery
                // to the vote.event (foreign key) from the outer DELETE statement.
                Expr::col((event::Entity, event::Column::Id)).eq(Expr::col((vote::Entity, vote::Column::Event)))
            )
            .and_where(subquery_conditions.into()); // Apply the OR-grouped slug and client ID conditions

        // Execute the DELETE operation using the WHERE EXISTS subquery.
        // SeaORM's `filter` method for `delete_many` directly accepts a `SelectStatement`
        // for `WHERE EXISTS` clauses.
        vote::Entity::delete_many()
            .filter(Expr::exists(subquery))
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn update_votes(db: &DbConn, update_vote_request: UpdateVoteRequest) -> Result<(), DbErr> {        
        let client_val = update_vote_request.client;
        let sequence_val = update_vote_request.sequence as i32; // Cast to i32 for DB
        let votes_shown_val = update_vote_request.votes.shown;
        let votes_expanded_val = update_vote_request.votes.expanded;
        let votes_up_val = update_vote_request.votes.up;
        let votes_down_val = update_vote_request.votes.down;
        let event_slug_val = update_vote_request.event;

        // 1. Construct the SELECT part of the "INSERT ... SELECT ..."
        // This subquery attempts to find the event by its slug and prepares all values
        // for the 'vote' table row. If the event doesn't exist, this select yields no rows.
        // The order of `expr()` calls here must match the order of columns in `target_vote_columns`.
        let mut select_query = Query::select();
        select_query
            .expr(Expr::val(client_val)) // Value for vote.client
            .from(event::Entity) // The table to select `event.id` and filter by `event.slug`
            .expr(Expr::col((event::Entity, event::Column::Id))) // Value for vote.event (the event_id)
            .expr(Expr::val(votes_shown_val))    // Value for vote.shown
            .expr(Expr::val(votes_expanded_val)) // Value for vote.expanded
            .expr(Expr::val(votes_up_val))        // Value for vote.up
            .expr(Expr::val(votes_down_val))      // Value for vote.down
            .expr(Expr::val(sequence_val))       // Value for vote.sequence
            .and_where(Expr::col((event::Entity, event::Column::Slug)).eq(event_slug_val)); // Filter by event slug

        // Define the target columns in the 'vote' table for the INSERT.
        // The order must match the SELECT expressions above.
        let target_vote_columns = vec![
            vote::Column::Client.into_iden(),
            vote::Column::Event.into_iden(),
            vote::Column::Shown.into_iden(),
            vote::Column::Expanded.into_iden(),
            vote::Column::Up.into_iden(),
            vote::Column::Down.into_iden(),
            vote::Column::Sequence.into_iden(),
        ];

        // 2. Construct the main INSERT statement using sea-query.
        let mut insert_statement = Query::insert();
        insert_statement
            .into_table(vote::Entity)
            .columns(target_vote_columns) // Specify target columns for the insert
            .select_from(select_query).unwrap(); // Specify the SELECT subquery

        // 3. Add the ON CONFLICT clause
        let on_conflict_clause = OnConflict::columns([vote::Column::Event, vote::Column::Client])
            .update_columns([ // Specifies which columns to update using EXCLUDED.* values
                vote::Column::Shown,
                vote::Column::Expanded,
                vote::Column::Up,
                vote::Column::Down,
                vote::Column::Sequence,
            ])
            .action_and_where( // Corrected: Use action_where for the WHERE clause on DO UPDATE
                Expr::col((vote::Entity, vote::Column::Sequence)) // Refers to the existing row's sequence
                .lt(Expr::val(sequence_val))// Compare with the new sequence value from the request (EXCLUDED.sequence)
            )
            .to_owned(); // Finalize the OnConflict builder

        insert_statement.on_conflict(on_conflict_clause);


        // 4. Build the SQL string and values for SeaORM execution
        // The 'build' method expects a concrete type implementing QueryBuilder,
        // so we use PostgresQueryBuilder directly since we know the backend.
        let (sql_str, sql_values) = insert_statement.build(PostgresQueryBuilder);

        // For debugging:
        println!("Executing SQL: {}", sql_str);
        println!("With Values: {:?}", sql_values);

        let sea_orm_stmt = SeaOrmStatement::from_sql_and_values(db.get_database_backend(), sql_str, sql_values);

        // 5. Execute the single, combined SQL statement
        db.execute(sea_orm_stmt).await?;

        Ok(())
    }
    
}