use std::collections::HashMap;
use itertools::Itertools;

use sea_orm::{prelude::Expr, sea_query::ExprTrait, *};
use ::entity::{event, oidc_provider, source, submission, token, user, user_event, vote};
use serde::{Deserialize, Serialize};
use url::Url;

use bitvec::prelude::*;

pub struct Query;

pub struct SourceToUpdate {
    pub update_url: String,
    pub event_slug: String,
    pub apikey: String,
    pub source_id: i32,
    pub event_id: i32,
    pub filter: SourceFilter,
}

#[derive(Serialize)]
pub struct EventRatingItem {
    up: i64,
    down: i64,
    views: i64,
    expanded: i64,
}

struct ConflictState {
    fullcode_a: String,
    fullcode_b: String,
    index_a: usize,
    index_b: usize,
    count_joint: u64,
    count_total: u64,
}

#[derive(FromQueryResult)]
struct SubmissionConflictQueryItem {
    fullcode_a: String,
    fullcode_b: String,
}

#[derive(FromQueryResult)]
struct EvenRatingQueryItem {
    code: String,
    up: i64,
    down: i64,
    views: i64,
    expanded: i64,
}

#[derive(Serialize)]
pub struct EventConflictItem {
    a: String,
    b: String,
    correlation: f64,
}

#[derive(PartialEq)]
pub enum ConflcitMode {
    Up,
    Expanded,
    ExpandedWithoutDown,
}

impl TryFrom<&str> for ConflcitMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "up" => Ok(Self::Up),
            "expanded" => Ok(Self::Expanded),
            "expanded-without-down" => Ok(Self::ExpandedWithoutDown),
            _ => Err(format!("Invalid value: {}", value))
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct EventConflictResponse {
    res: Vec<EventConflictItem>,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct EventRatingsResponse {
    response: HashMap<String, EventRatingItem>,
}

#[derive(Serialize)]
pub struct EventContent {
    version: i32,
    submissions: serde_json::Value,
}

#[derive(Deserialize)]
pub struct ExportRequest {
    pub events: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct ExportResult {
    events: HashMap<String, VoteExport>
}

#[derive(Serialize)]
pub struct VoteExport {
    client: String,
    seq: u32,
    up: Vec<String>,
    down: Vec<String>,
    shown: Vec<String>,
    expanded: Vec<String>,
}

#[derive(Debug, FromQueryResult)]
struct VoteAndSlug {
    client: String,
    sequence: i32,
    shown: Vec<String>,
    expanded: Vec<String>,
    up: Vec<String>,
    down: Vec<String>,
    slug: String, 
}


#[derive(Serialize,Deserialize,PartialEq)]
pub enum RoleInfo {
    #[serde(rename="admin")]
    Admin,
    #[serde(rename="update")]
    Update,
    #[serde(rename="view")]
    View,
}

impl Into<String> for RoleInfo {
    fn into(self) -> String {
        match self {
            RoleInfo::Admin => "admin".to_string(),
            RoleInfo::Update => "update".to_string(),
            RoleInfo::View => "view".to_string(),
        }
    }
}

impl TryFrom<&str> for RoleInfo {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "admin" => Ok(RoleInfo::Admin),
            "update" => Ok(RoleInfo::Update),
            "view" => Ok(RoleInfo::View),
            _ => Err(format!("Invalid value: {}", value)),
        }
    }
}

#[derive(Serialize)]
pub struct PermissionsInfo {
    view: bool,
    configure: bool,
    update: bool,
    role: RoleInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    name: String,
    email: String,
}

#[derive(Serialize)]
pub struct EventInfo {
    name: String,
    slug: String,
    voting_enabled: bool,
    permissions: PermissionsInfo,
}


#[derive(Serialize)]
pub struct EventReseponse {
    can_create: bool,
    user: UserInfo,
    events: HashMap<String, EventInfo>,
}

#[derive(Serialize)]
pub struct EventOverview {
    pub sources: i32,
    pub submissions: i32,
    pub votes: i32,
}

pub struct EventWithPerm {
    pub event: i32,
    pub perm: RoleInfo,
}

#[derive(Serialize)]
pub struct ScheduledTime {
    start: u32,
    end: u32,
}

#[derive(Serialize)]
pub struct SubmissionsResponseItem {
    pub code: String,
    pub lastupdate: u32,
    pub title: String,
    pub r#abstract: String,
    pub time: Option<ScheduledTime>,
}


#[derive(Serialize)]
pub struct SubmissionsResponse {
    submissions: HashMap<String,SubmissionsResponseItem>,
}

#[derive(Serialize,Deserialize)]
pub enum SourceFilter {
    #[serde(rename="accepted")]
    Accepted,
    #[serde(rename="confirmed")]
    Confirmed,
}

impl TryFrom<i32> for SourceFilter {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Accepted),
            1 => Ok(Self::Confirmed),
            _ => Err(format!("Invalid value for SourceFilter: {}", value)),
        }
    }
}

impl TryFrom<&str> for SourceFilter {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "accepted" => Ok(Self::Accepted),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(format!("Invalid value for SourceFilter: {}", value)),
        }
    }
}

#[derive(Serialize)]
pub struct SourceResponseItem {
    autoupdate: bool,
    url: Option<String>,
    #[serde(rename="eventSlug")]
    event_slug: Option<String>,
    interval: Option<i32>,
    filter: Option<SourceFilter>

}

#[derive(Serialize)]
#[serde(transparent)]
pub struct SoureceResponse {
    sources: HashMap<String, SourceResponseItem>
}

#[derive(Serialize)]
pub struct ConferenceUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub permissions: ConferenceUserRole,
}

#[derive(Serialize)]
pub enum ConferenceUserRole {
    #[serde(rename="view")]
    View,
    #[serde(rename="update")]
    Update,
    #[serde(rename="admin")]
    Admin,
    #[serde(rename="self")]
    Own,
}

impl From<RoleInfo> for ConferenceUserRole {
    fn from(value: RoleInfo) -> Self {
        match value {
            RoleInfo::Admin => ConferenceUserRole::Admin,
            RoleInfo::Update => ConferenceUserRole::Update,
            RoleInfo::View => ConferenceUserRole::View,
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]

pub struct ConferenceUserList {
    users: Vec<ConferenceUser>,
}

#[derive(Serialize)]
pub struct SearchUser {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct SearchUserList {
    users: Vec<SearchUser>,
}

#[derive(Serialize)]
pub struct EventUrlItem {
    submission_update_url: String,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct EventUrlsResponse {
    urls: HashMap<String, EventUrlItem>,
}

impl Query {

    pub async fn get_sources(db: &DbConn, event: i32) -> Result<SoureceResponse, DbErr> {
        let sources = source::Entity::find()
            .filter(source::Column::Event.eq(event))
            .all(db)
            .await?;
        let res = sources
            .into_iter()
            .map(|m| {
                let f = m.filter
                    .map(SourceFilter::try_from)
                    .transpose()
                    .map_err(|e| DbErr::Custom(format!("Can't convert filter: {}", e)))?;
                Ok::<(std::string::String, SourceResponseItem), DbErr>((m.slug, SourceResponseItem {
                    autoupdate: m.autoupdate,
                    url: m.url,
                    event_slug: m.event_slug,
                    interval: m.interval,
                    filter: f,
                }))
            })
            .collect::<Result<HashMap<_,_>,_>>()?;
        return Ok(SoureceResponse {sources: res})
    }
    pub async fn get_event_overview(db: &DbConn, event: i32) -> Result<Option<EventOverview>, DbErr> {
            let sources = source::Entity::find()
                .filter(source::Column::Event.eq(event))
                .count(db)
                .await?;
            let submissions = source::Entity::find()
                .filter(source::Column::Event.eq(event))
                .join(
                    JoinType::InnerJoin,
                    source::Relation::Submission.def(),
                )
                .count(db)
                .await?;
            let votes = vote::Entity::find()
                .filter(vote::Column::Event.eq(event))
                .count(db)
                .await?;
            let res = EventOverview {
                sources: sources as i32,
                submissions: submissions as i32,
                votes: votes as i32,
            };
            Ok(Some(res))
        }

    pub async fn get_event_perm_for_user(db: &DbConn, user: i32, slug: impl Into<String>) -> Result<Option<EventWithPerm>, DbErr> {
        let maybe_perm = user_event::Entity::find()
        .select_only()
        .column(event::Column::Id)
        .column(user_event::Column::Perm)
        .join(
            JoinType::InnerJoin,
            user_event::Relation::Event.def(),
        )
        .filter(event::Column::Slug.eq(slug.into()))
        .filter(user_event::Column::User.eq(user))
        .into_tuple::<(i32, String)>()
        .one(db)
        .await?;
        if let Some((event, perm)) = maybe_perm {
            return Ok(
                Some(EventWithPerm{
                    event: event,
                    perm: RoleInfo::try_from(perm.as_str())
                        .map_err(|e| DbErr::Custom(e))?,
                }
            ));
        } else {
            return Ok(None);
        }
    }

    pub async fn get_user_from_token(db: &DbConn, token: impl Into<String>) -> Result<Option<i32>, DbErr> {
        let token: Option<(i32,)> = token::Entity::find().filter(token::Column::Value.eq(token.into())).select_only().column(token::Column::User).into_tuple().one(db).await?;
        Ok(token.map(|t| t.0))
    }

    pub async fn user_can_create(db: &DbConn, user: i32) -> Result<bool, DbErr> {
        let write: Option<bool> = user::Entity::find().filter(user::Column::Id.eq(user)).select_only().column(user::Column::CanCreate).into_tuple().one(db).await?;
        return write.ok_or_else(|| DbErr::Custom("User not found".to_string()));
    }

    pub async fn get_conference_users(db: &DbConn, conference: i32, user: i32) -> Result<ConferenceUserList, DbErr> {
        // eprintln!("Conference is {}", conference);
        let users = user_event::Entity::find()
            .find_with_related(user::Entity)
            .filter(user_event::Column::Event.eq(conference))
            .all(db)
            .await?;
        let resp = users
            .into_iter()
            .map(|(ue, ulist)| {
                ulist.into_iter().map(move |u| {
                    Ok::<ConferenceUser, String>(ConferenceUser {
                        id: format!("{}", u.id),
                        name: u.name,
                        email: u.email,
                        permissions: if u.id == user { 
                            ConferenceUserRole::Own 
                        } else { 
                            ConferenceUserRole::from(
                                RoleInfo::try_from(ue.perm.as_str())?
                            )
                        },
                    })
                })
            })
            .flatten()
            .collect::<Result<Vec<ConferenceUser>,_>>()
            .map_err(
                |e| DbErr::Custom(format!("Could not get role info: {}", e))
            )?;
        return Ok(ConferenceUserList { users: resp })
    }

    pub async fn search_users(db: &DbConn, conference: i32, query: String, limit: u64) -> Result<SearchUserList, DbErr> {

        let users_in_event = user_event::Entity::find()
            .select_only()
            .column(user_event::Column::User)
            .filter(user_event::Column::Event.eq(conference))
            .into_query();
        let mut cond = Condition::all()
                .add(user::Column::Id.not_in_subquery(users_in_event));
        if query != "" {
            let query_cond = Condition::any()
                .add(user::Column::Name.contains(query.clone()))
                .add(user::Column::Email.contains(query));
            cond = cond.add(query_cond);
        }

        let users = user::Entity::find()
            .filter(cond)
            .limit(limit)
            .all(db)
            .await?;

        let res = users
            .into_iter()
            .map(|u| {
                SearchUser {
                    id: format!("{}", u.id),
                    name: u.name,
                    email: u.email,
                }
            })
            .collect();
        return Ok(SearchUserList { users: res })

    }

    pub async fn get_submissions(db: &DbConn, conference: i32) -> Result<Option<SubmissionsResponse>, DbErr> {
        let submission = source::Entity::find()
        .find_with_related(submission::Entity)
        .filter(source::Column::Event.eq(conference))
        .all(db)
        .await?;
        let resp = submission
        .into_iter()
        .flat_map(|(so, su_vec)| {
            su_vec
            .into_iter()
            .map(move |su| {
                (format!("{}/{}",so.slug, su.code.clone()), 
                SubmissionsResponseItem {
                    code: su.code,
                    lastupdate: su.updated.and_utc().timestamp() as u32,
                    title: su.title,
                    r#abstract: su.r#abstract,
                    time: match (su.start, su.end) {
                        (Some(start), Some(end)) => Some(ScheduledTime { 
                            start: start.and_utc().timestamp() as u32,
                            end: end.and_utc().timestamp() as u32 
                        }),
                        _ => None
                    },
                })
            })
        })
        .collect::<HashMap<_,_>>();
        Ok(Some(SubmissionsResponse {
            submissions: resp,
        }))

    }

    pub async fn get_events(db: &DbConn, user: i32) -> Result<Option<EventReseponse>, DbErr> {
        let user_result =user::Entity::find_by_id(user).one(db).await?;
        if let Some(user) = user_result {
            let events = user.find_related(user_event::Entity).find_also_related(event::Entity).all(db).await?;
            let map = events.into_iter().filter_map(
                |(ue, maybe_event)| {
                    match maybe_event {
                        Some(event) => {
                            let pi = 
                                match ue.perm.as_str() {
                                    "admin" => PermissionsInfo {
                                        view: true,
                                        configure: true,
                                        update: true,
                                        role: RoleInfo::Admin,
                                    },
                                    "view" => PermissionsInfo { view: true, configure: false, update: false, role: RoleInfo::View },
                                    _ => return None,

                                };
                            
                            let ei = EventInfo {
                                name: event.title,
                                slug: event.slug.clone(),
                                voting_enabled: event.voting,
                                permissions: pi,
                            };
                            Some((event.slug, ei))
                        },
                        None => None,
                    }
                }).collect::<HashMap<_,_>>();
                let ui = UserInfo {
                    name: user.name,
                    email: user.email
                };
                let resp = EventReseponse {
                    can_create: user.can_create,
                    user: ui,
                    events: map,
                };
                return Ok(Some(resp));
        } else {
            return Ok(None);
        }
        
    }

    pub async fn get_active_event_title(db: &DbConn, slug: impl Into<String>) -> Result<Option<String>,DbErr> {
        let res = event::Entity::find()
            .filter(
                event::Column::Slug.eq(slug.into())
            )
            .filter(
                event::Column::Voting.into_simple_expr()
            )
            .one(db)
            .await?;
        Ok(res.map(|model| model.title))
    }

    pub async fn get_provider_authurl(db: &DbConn, provider_name: impl Into<String>) -> Result<Option<String>, DbErr> {
        let res: Option<(String,)> = oidc_provider::Entity::find()
            .filter(oidc_provider::Column::Name.eq(provider_name.into()))
            .select_only()
            .column(oidc_provider::Column::Authurl)
            .into_tuple()
            .one(db)
            .await?;
        Ok(res.map(|r| r.0))
    }

    pub async fn get_sources_for_updates(db: &DbConn) -> Result<Vec<SourceToUpdate>,DbErr> {
        let sources = source::Entity::find()
            .filter(source::Column::Autoupdate.eq(true))
            .filter(source::Column::Apikey.is_not_null())
            .filter(source::Column::Url.is_not_null())
            .filter(source::Column::EventSlug.is_not_null())
            .filter(source::Column::Interval.is_not_null())
            .filter(source::Column::Filter.is_not_null())
            .filter(
                Condition::any()
                    .add(source::Column::LastAttemp.is_null())
                    .add(
                        Expr::col(source::Column::LastAttemp)
                        .add(
                            Expr::col(source::Column::Interval)
                                .mul(Expr::cust("INTERVAL '1 second'")
                            )
                        )
                        .lt(Expr::current_timestamp())
                    )
            )
            .all(db)
            .await?;
        Ok(
            sources.into_iter().map(|m| {
                SourceToUpdate {
                    update_url: m.url.unwrap(),
                    event_slug: m.event_slug.unwrap(),
                    apikey: m.apikey.unwrap(),
                    source_id: m.id,
                    event_id: m.event,
                    filter: SourceFilter::try_from(m.filter.unwrap()).unwrap(),
                }
            }
        ).collect())
            
    }

    pub async fn verify_update_token(db: &DbConn, event_slug: String, source_slug: String, token: String) -> Result<i32, DbErr> {
        let source_model = source::Entity::find()
            .find_with_related(event::Entity)
            .filter(event::Column::Slug.eq(event_slug))
            .filter(source::Column::Slug.eq(source_slug))
            .filter(source::Column::Updatekey.eq(token))
            .all(db)
            .await?;
        if source_model.len() == 1 {
            return Ok(source_model.get(0).unwrap().0.id)
        } else {
            return Err(DbErr::RecordNotFound(String::from("Token not found")))
        }
    }

    pub async fn get_update_urls(db: &DbConn, event: i32, prefix: Url) -> Result<EventUrlsResponse, DbErr> {
        let sources = source::Entity::find()
            .filter(source::Column::Event.eq(event))
            .all(db)
            .await?;
        let all = sources.into_iter().map(|m| {
            let url = prefix.join(
                format!("{}/{}", &m.slug, &m.updatekey).as_str()
                )
                .map_err(
                    |e| DbErr::Custom(format!("Could not parse url parts: {}", e))
                )?
                .to_string();
            Ok::<(String, EventUrlItem),DbErr>(
                (
                    m.slug, 
                    EventUrlItem {
                        submission_update_url: url
                    }
                )
            )
        }).collect::<Result<HashMap<_,_>,_>>()?;
        return Ok(EventUrlsResponse { urls: all })

    }

    pub async fn export_for_events(db: &DbConn, client_ids: ExportRequest) -> Result<ExportResult, DbErr> {
        if client_ids.events.is_empty() {
            return Ok(ExportResult { events: HashMap::new() });
        }
        let mut conditions = Condition::any(); 

        for (slug, client_id) in client_ids.events.iter() {
            conditions = conditions.add(
                Condition::all() 
                    .add(event::Column::Slug.eq(slug.to_owned())) 
                    .add(vote::Column::Client.eq(client_id.to_owned())) 
            );
        }

        let votes_with_events = vote::Entity::find()
            .join(
                JoinType::InnerJoin,
                vote::Relation::Event.def()
            )
            .filter(conditions)
            .select_only() 
            .column(vote::Column::Client)
            .column(vote::Column::Sequence)
            .column(vote::Column::Shown)
            .column(vote::Column::Expanded)
            .column(vote::Column::Up)
            .column(vote::Column::Down)
            .column(event::Column::Slug) 
            .into_model::<VoteAndSlug>() 
            .all(db) 
            .await?;

        let mut result_map: HashMap<String, VoteExport> = HashMap::new();
        for vote_record in votes_with_events {
            let vote_export = VoteExport {
                seq: vote_record.sequence as u32, 
                client: vote_record.client,
                up: vote_record.up,
                down: vote_record.down,
                shown: vote_record.shown,
                expanded: vote_record.expanded,
            };
            result_map.insert(vote_record.slug, vote_export);
        }

        Ok(ExportResult { events: result_map })

    }

    pub async fn get_provider_metadata(db: &DbConn, name: impl Into<String>) -> Result<Option<serde_json::Value>, DbErr> { 
        let res = oidc_provider::Entity::find_by_id(name.into()).one(db).await?;
        Ok(res.map(|v| v.metadata))
    }

    pub async fn get_event_conflicts(db: &DbConn, event: i32, mode: ConflcitMode, limit: u32) -> Result<EventConflictResponse, DbErr> {
        let submission_conflicts = SubmissionConflictQueryItem::find_by_statement(
            Statement::from_sql_and_values(
                DatabaseBackend::Postgres, 
                r#"WITH submission_with_event AS (
                    -- Step 1: Join submission with source to get the event ID and slug.
                    -- We also create the new custom identifier here.
                    SELECT
                        src.event AS event_id,
                        src.slug || '/' || s.code AS submission_identifier, -- New concatenated identifier
                        s.start,
                        s.end,
                        -- We still need the original primary key for the self-join condition
                        s.source,
                        s.code
                    FROM
                        submission s
                    JOIN
                        source src ON s.source = src.id
                    WHERE
                        s.start IS NOT NULL AND s.end IS NOT NULL and src.event = $1
                    )
                    -- Step 2: Join this view to itself to find the overlapping pairs.
                    SELECT
                    s1.submission_identifier AS fullcode_a,
                    s2.submission_identifier AS fullcode_b
                    FROM
                    submission_with_event s1
                    JOIN
                    submission_with_event s2 ON s1.event_id = s2.event_id -- Must be the same event
                    WHERE
                    -- The time intervals must overlap
                    (s1.start, s1.end) OVERLAPS (s2.start, s2.end)
                    -- Ensure we don't match a submission with itself and get each pair only once
                    AND (s1.source, s1.code) < (s2.source, s2.code);"#,
                     [event.into()]
                    )
                )
                .all(db)
                .await?;
            let mut all_conflict_fullcodes: HashMap<String, usize> = HashMap::new();
            let mut index:usize = 0;
            for fullcode in submission_conflicts.iter() {
                if !all_conflict_fullcodes.contains_key(&fullcode.fullcode_a) {
                    all_conflict_fullcodes.insert(fullcode.fullcode_a.clone(), index);
                    index +=1;
                }
                if !all_conflict_fullcodes.contains_key(&fullcode.fullcode_b) {
                    all_conflict_fullcodes.insert(fullcode.fullcode_b.clone(), index);
                    index +=1;
                }
            }
            let mut state = submission_conflicts.into_iter().map(|i| ConflictState {
                index_a: all_conflict_fullcodes.get(&i.fullcode_a).unwrap().clone(),
                index_b: all_conflict_fullcodes.get(&i.fullcode_b).unwrap().clone(),
                fullcode_a: i.fullcode_a,
                fullcode_b: i.fullcode_b,
                count_joint: 0,
                count_total: 0,
            }).collect::<Vec<ConflictState>>();
            let maxsize = index;
            let mut exists = bitvec![0;maxsize];
            match mode {
                ConflcitMode::ExpandedWithoutDown => {
                    let votes: Vec<(Vec<String>,Vec<String>)> = vote::Entity::find()
                        .filter(vote::Column::Event.eq(event))
                        .select_only()
                        .columns([vote::Column::Expanded, vote::Column::Down])
                        .into_tuple()
                        .all(db)
                        .await?;
                
                    for (expanded, down) in votes.iter() {
                        exists.set_elements(0);
                        
                        for code in expanded.iter() {
                            match all_conflict_fullcodes.get(code) {
                                Some(idx) => exists.set(*idx,true),
                                None => ()
                            }
                        }
                        for code in down.iter() {
                            match all_conflict_fullcodes.get(code) {
                                Some(idx) => exists.set(*idx,false),
                                None => ()
                            }
                        }
                        for s in state.iter_mut() {
                            let a_in = exists[s.index_a];
                            let b_in = exists[s.index_b];
                            if (a_in)||(b_in) {
                                s.count_total += 1;
                            }
                            if (a_in)&&(b_in) {
                                s.count_joint += 1;
                            }
                        }
                    }
                },
                _ => {
                    let votes: Vec<(Vec<String>,)> = vote::Entity::find()
                        .filter(vote::Column::Event.eq(event))
                        .select_only()
                        .column(if mode == ConflcitMode::Up {vote::Column::Up} else { vote::Column::Expanded})
                        .into_tuple()
                        .all(db)
                        .await?;
                
                    for v in votes.iter() {
                        exists.set_elements(0);
                        for code in v.0.iter() {
                            match all_conflict_fullcodes.get(code) {
                                Some(idx) => exists.set(*idx,true),
                                None => ()
                            }
                        }
                        for s in state.iter_mut() {
                            let a_in = exists[s.index_a];
                            let b_in = exists[s.index_b];
                            if (a_in)||(b_in) {
                                s.count_total += 1;
                            }
                            if (a_in)&&(b_in) {
                                s.count_joint += 1;
                            }
                        }
                    }

                }
            }
            
            let results = state.into_iter().map(|s| EventConflictItem {
                a: s.fullcode_a,
                b: s.fullcode_b,
                correlation: if s.count_total > 0 { (s.count_joint as f64)/(s.count_total as f64)} else {0.0},
            })
            .sorted_by(|a,b| b.correlation.total_cmp(&a.correlation))
            .take(limit as usize)
            .collect();

            return Ok(EventConflictResponse{res: results});
            }

    pub async fn get_event_ratings(db: &DbConn, event: i32) -> Result<EventRatingsResponse, DbErr> {
        let res = EvenRatingQueryItem::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"WITH fullcodes_for_event AS (
            -- This CTE generates the fullcode and gets the title for each submission
            SELECT 
                source.slug || '/' || submission.code AS fullcode,
                submission.title
            FROM 
                source
            JOIN 
                submission ON submission.source = source.id
            WHERE 
                source.event = $1
        ),
        shown_counts AS (
            SELECT unnest(shown) AS fullcode, count(*) as ct
            FROM vote WHERE event = $1 GROUP BY 1
        ),
        expanded_counts AS (
            SELECT unnest(expanded) AS fullcode, count(*) as ct
            FROM vote WHERE event = $1 GROUP BY 1
        ),
        up_counts AS (
            SELECT unnest(up) AS fullcode, count(*) as ct
            FROM vote WHERE event = $1 GROUP BY 1
        ),
        down_counts AS (
            SELECT unnest(down) AS fullcode, count(*) as ct
            FROM vote WHERE event = $1 GROUP BY 1
        )
        SELECT
            fc.fullcode as code,
            -- Renamed the columns as requested using aliases
            COALESCE(s.ct, 0) AS views,
            COALESCE(e.ct, 0) AS expanded,
            COALESCE(u.ct, 0) AS up,
            COALESCE(d.ct, 0) AS down
        FROM
            fullcodes_for_event fc
        LEFT JOIN shown_counts s ON fc.fullcode = s.fullcode
        LEFT JOIN expanded_counts e ON fc.fullcode = e.fullcode
        LEFT JOIN up_counts u ON fc.fullcode = u.fullcode
        LEFT JOIN down_counts d ON fc.fullcode = d.fullcode
        "#,
                [event.into()],
            ))
            .all(db)
            .await?;
        return Ok(EventRatingsResponse { response: res.into_iter().map(|m| (m.code, EventRatingItem { up: m.up, down: m.down, views: m.views, expanded: m.expanded})).collect::<HashMap<String, EventRatingItem>>() })
    }

    pub async fn get_conditional_content(db: &DbConn, slug: impl Into<String>, version: Option<i32>) -> Result<EventContent, DbErr> {
        let base = event::Entity::find()
            .filter(
                event::Column::Slug.eq(slug.into())
            )
            .filter(
                event::Column::Voting.into_simple_expr()
            );
        let filtered = match version {
            Some(v) => base.filter(event::Column::Version.lt(v)),
            None => base,
        };
        let res = filtered.one(db).await?;
        
        Ok(
            match res {
                Some(model) => EventContent { 
                    version: model.version, 
                    submissions: model.content},
                None => EventContent { 
                    version: version.unwrap_or(0), 
                    submissions: serde_json::from_str("{}").unwrap()
                },
            }
        )
    }
}