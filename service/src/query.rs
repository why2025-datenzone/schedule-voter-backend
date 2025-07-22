use std::collections::HashMap;

use sea_orm::{prelude::Expr, sea_query::ExprTrait, *};
use ::entity::{event, oidc_provider, source, submission, token, user, user_event, vote};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::vote_metric::Similarity;
use crate::{vote_metric::{self, CosineVoteMetric, ExpandedVoteSelector, ExpandedWithoutDownVoteSelector, JaccardVoteMetric, LiftVoteMetric, SupportVoteMetric, UpVoteSelector, WeightedSupportMetric}, SubmissionTypesField};

pub struct Query;

enum VoteMetricVariant {
    JaccardUp(vote_metric::SimilarityStruct::<JaccardVoteMetric<UpVoteSelector>>),
    JaccardExpanded(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedVoteSelector>>),
    JaccardExpandedWithoutDown(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedWithoutDownVoteSelector>>),
    SupportUp(vote_metric::SimilarityStruct::<SupportVoteMetric<UpVoteSelector>>),
    SupportExpanded(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedVoteSelector>>),
    SupportExpandedWithoutDown(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedWithoutDownVoteSelector>>),
    SupportWeightedUp(vote_metric::SimilarityStruct::<WeightedSupportMetric<UpVoteSelector>>),
    SupportWeightedExpanded(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedVoteSelector>>),
    SupportWeightedExpandedWithoutDown(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedWithoutDownVoteSelector>>),
    LiftUp(vote_metric::SimilarityStruct::<LiftVoteMetric<UpVoteSelector>>),
    LiftExpanded(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedVoteSelector>>),
    LiftExpandedWithoutDown(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedWithoutDownVoteSelector>>),
    Cosine(vote_metric::SimilarityStruct::<CosineVoteMetric>),
}


impl VoteMetricVariant {
    fn get_columns(&self) -> Vec<vote::Column> {
        match self {
            VoteMetricVariant::JaccardUp(vm) => vm.get_columns(),
            VoteMetricVariant::JaccardExpanded(vm) => vm.get_columns(),
            VoteMetricVariant::JaccardExpandedWithoutDown(vm) => vm.get_columns(),
            VoteMetricVariant::SupportUp(vm) => vm.get_columns(),
            VoteMetricVariant::SupportExpanded(vm) => vm.get_columns(),
            VoteMetricVariant::SupportExpandedWithoutDown(vm) => vm.get_columns(),
            VoteMetricVariant::SupportWeightedUp(vm) => vm.get_columns(),
            VoteMetricVariant::SupportWeightedExpanded(vm) => vm.get_columns(),
            VoteMetricVariant::SupportWeightedExpandedWithoutDown(vm) => vm.get_columns(),
            VoteMetricVariant::LiftUp(vm) => vm.get_columns(),
            VoteMetricVariant::LiftExpanded(vm) => vm.get_columns(),
            VoteMetricVariant::LiftExpandedWithoutDown(vm) => vm.get_columns(),
            VoteMetricVariant::Cosine(vm) => vm.get_columns(),
        }
    }

    fn process_votes(&mut self, votes: Vec<Vec<Vec<String>>>) {
        match self {
            VoteMetricVariant::JaccardUp(vm) => vm.process_votes(votes),
            VoteMetricVariant::JaccardExpanded(vm) => vm.process_votes(votes),
            VoteMetricVariant::JaccardExpandedWithoutDown(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportUp(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportExpanded(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportExpandedWithoutDown(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportWeightedUp(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportWeightedExpanded(vm) => vm.process_votes(votes),
            VoteMetricVariant::SupportWeightedExpandedWithoutDown(vm) => vm.process_votes(votes),
            VoteMetricVariant::LiftUp(vm) => vm.process_votes(votes),
            VoteMetricVariant::LiftExpanded(vm) => vm.process_votes(votes),
            VoteMetricVariant::LiftExpandedWithoutDown(vm) => vm.process_votes(votes),
            VoteMetricVariant::Cosine(vm) => vm.process_votes(votes),
        }
    }

    fn get_item_results(self, limit: usize) -> Vec<SubmissionSimilarityItem> {
        match self {
            VoteMetricVariant::JaccardUp(vm) => vm.get_item_results(limit),
            VoteMetricVariant::JaccardExpanded(vm) => vm.get_item_results(limit),
            VoteMetricVariant::JaccardExpandedWithoutDown(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportUp(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportExpanded(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportExpandedWithoutDown(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportWeightedUp(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportWeightedExpanded(vm) => vm.get_item_results(limit),
            VoteMetricVariant::SupportWeightedExpandedWithoutDown(vm) => vm.get_item_results(limit),
            VoteMetricVariant::LiftUp(vm) => vm.get_item_results(limit),
            VoteMetricVariant::LiftExpanded(vm) => vm.get_item_results(limit),
            VoteMetricVariant::LiftExpandedWithoutDown(vm) => vm.get_item_results(limit),
            VoteMetricVariant::Cosine(vm) => vm.get_item_results(limit),
        }
    }

    fn get_results(self, limit: usize) -> Vec<EventConflictItem> {
        match self {
            VoteMetricVariant::JaccardUp(vm) => vm.get_results(limit),
            VoteMetricVariant::JaccardExpanded(vm) => vm.get_results(limit),
            VoteMetricVariant::JaccardExpandedWithoutDown(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportUp(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportExpanded(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportExpandedWithoutDown(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportWeightedUp(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportWeightedExpanded(vm) => vm.get_results(limit),
            VoteMetricVariant::SupportWeightedExpandedWithoutDown(vm) => vm.get_results(limit),
            VoteMetricVariant::LiftUp(vm) => vm.get_results(limit),
            VoteMetricVariant::LiftExpanded(vm) => vm.get_results(limit),
            VoteMetricVariant::LiftExpandedWithoutDown(vm) => vm.get_results(limit),
            VoteMetricVariant::Cosine(vm) => vm.get_results(limit),
        }
    }
}

pub struct SourceToUpdate {
    pub update_url: String,
    pub event_slug: String,
    pub apikey: String,
    pub source_id: i32,
    pub event_id: i32,
    pub filter: SourceFilter,
    pub type_filter: Vec<i32>,
}

#[derive(Serialize)]
pub struct EventRatingItem {
    up: i64,
    down: i64,
    views: i64,
    expanded: i64,
}

// struct ConflictState {
//     fullcode_a: String,
//     fullcode_b: String,
//     index_a: usize,
//     index_b: usize,
//     count_joint: u64,
//     count_total: u64,
// }

// struct ViewConflictState {
//     fullcode_a: String,
//     fullcode_b: String,
//     index_a: usize,
//     index_b: usize,
//     up_joint: u64,
//     view_joint: u64,
// }

// struct CosineConflictState {
//     fullcode_a: String,
//     fullcode_b: String,
//     index_a: usize,
//     index_b: usize,
//     dotprod: i64,
//     norm_a: i64,
//     norm_b: i64,
// }

// struct WeightedConflictState {
//     fullcode_a: String,
//     fullcode_b: String,
//     index_a: usize,
//     index_b: usize,
//     votes: f64,
// }

#[derive(FromQueryResult)]
pub struct SubmissionConflictQueryItem {
    pub fullcode_a: String,
    pub fullcode_b: String,
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
    pub a: String,
    pub b: String,
    pub correlation: f64,
}

#[derive(PartialEq)]
pub enum ConflcitMode {
    JaccardUp,
    JaccardExpanded,
    JaccardExpandedWithoutDown,
    SupportUp,
    SupportExpanded,
    SupportExpandedWithoutDown,
    SupportWeightedUp,
    SupportWeightedExpanded,
    SupportWeightedExpandedWithoutDown,
    LiftUp,
    LiftExpanded,
    LiftExpandedWithoutDown,
    Cosine,
}

impl TryFrom<&str> for ConflcitMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "jaccard-up" => Ok(Self::JaccardUp),
            "jaccard-expanded" => Ok(Self::JaccardExpanded),
            "jaccard-expanded-without-down" => Ok(Self::JaccardExpandedWithoutDown),
            "support-up" => Ok(Self::SupportUp),
            "support-expanded" => Ok(Self::SupportExpanded),
            "support-expanded-without-down" => Ok(Self::SupportExpandedWithoutDown),
            "support-weighted-up" => Ok(Self::SupportWeightedUp),
            "support-weighted-expanded" => Ok(Self::SupportWeightedExpanded),
            "support-weighted-expanded-without-down" => Ok(Self::SupportWeightedExpandedWithoutDown),
            "lift-up" => Ok(Self::LiftUp),
            "lift-expanded" => Ok(Self::LiftExpanded),
            "lift-expanded-without-down" => Ok(Self::LiftExpandedWithoutDown),
            "cosine" => Ok(Self::Cosine),
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
pub struct SubmissionSimilarityItem {
    pub id: String,
    pub metric: f64,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct SubmissionSimilarityResponse {
    res: Vec<SubmissionSimilarityItem>,
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
    room: Option<String>,
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
    filter: Option<SourceFilter>,
    #[serde(rename="submissionTypes")]
    submission_types: Option<HashMap<i32, String>>,
    #[serde(rename="typeFilter")]
    type_filter: Vec<i32>,

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

    pub async fn get_source(db: &DbConn, event: i32, source: String) -> Result<SourceToUpdate, DbErr> {
        let source = source::Entity::find()
            .filter(source::Column::Event.eq(event))
            .filter(source::Column::Slug.eq(source))
            .one(db)
            .await?;
        if let Some(s) = source {
            if let Some(url) = s.url {
                if let Some(apikey) = s.apikey {
                    if let Some(event_slug) = s.event_slug {
                        if let Some(filter) = get_filter(s.filter)? {
                            return Ok(SourceToUpdate { 
                                update_url: url, 
                                event_slug: event_slug, 
                                apikey: apikey, 
                                source_id: s.id, 
                                event_id: event, 
                                filter: filter, 
                                type_filter: s.type_filter 
                            })
                        }
                    }
                }
            }
            
        }
        return Err(DbErr::RecordNotFound("Can't find the source".to_string()))
    }

    pub async fn get_sources(db: &DbConn, event: i32) -> Result<SoureceResponse, DbErr> {
        let sources = source::Entity::find()
            .filter(source::Column::Event.eq(event))
            .all(db)
            .await?;
        let res = sources
            .into_iter()
            .map(|m| {
                Ok::<(std::string::String, SourceResponseItem), DbErr>((m.slug.clone(), source_response_from_model(m)?))
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
                            end: end.and_utc().timestamp() as u32,
                            room: su.room,
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
                    type_filter: m.type_filter,
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


    pub async fn get_submission_similarities(db: &DbConn, event: i32, mode: ConflcitMode, submission_id: String, limit: u32) -> Result<SubmissionSimilarityResponse, DbErr> {
        let all_ids = get_all_ids_for_event(db, event, &submission_id).await?;
        // let vm  = match mode {
        //     ConflcitMode::JaccardUp => VoteMetricVariant::JaccardUp(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::JaccardExpanded => VoteMetricVariant::JaccardExpanded(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::JaccardExpandedWithoutDown => VoteMetricVariant::JaccardExpandedWithoutDown(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportUp => VoteMetricVariant::SupportUp(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportExpanded => VoteMetricVariant::SupportExpanded(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportExpandedWithoutDown => VoteMetricVariant::SupportExpandedWithoutDown(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedUp => VoteMetricVariant::SupportWeightedUp(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedExpanded => VoteMetricVariant::SupportWeightedExpanded(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedExpandedWithoutDown => VoteMetricVariant::SupportWeightedExpandedWithoutDown(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftUp => VoteMetricVariant::LiftUp(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftExpanded => VoteMetricVariant::LiftExpanded(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftExpandedWithoutDown => VoteMetricVariant::LiftExpandedWithoutDown(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::Cosine => VoteMetricVariant::Cosine(vote_metric::Similarity::from_single_submission(submission_id, all_ids)),
        // };
        // let mut vm: Box<dyn vote_metric::Similarity>  = match mode {
        //     ConflcitMode::JaccardUp => Box::new(vote_metric::SimilarityStruct::<JaccardVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::JaccardExpanded => Box::new(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::JaccardExpandedWithoutDown => Box::new(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportUp => Box::new(vote_metric::SimilarityStruct::<SupportVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportExpanded => Box::new(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportExpandedWithoutDown => Box::new(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedUp => Box::new(vote_metric::SimilarityStruct::<WeightedSupportMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedExpanded => Box::new(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::SupportWeightedExpandedWithoutDown => Box::new(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftUp => Box::new(vote_metric::SimilarityStruct::<LiftVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftExpanded => Box::new(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::LiftExpandedWithoutDown => Box::new(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
        //     ConflcitMode::Cosine => Box::new(vote_metric::SimilarityStruct::<CosineVoteMetric>::from_single_submission(submission_id, all_ids)),
        // };
        let mut vm: VoteMetricVariant  = match mode {
            // Construct enum variants instead of Box::new()
            ConflcitMode::JaccardUp => VoteMetricVariant::JaccardUp(vote_metric::SimilarityStruct::<JaccardVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::JaccardExpanded => VoteMetricVariant::JaccardExpanded(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::JaccardExpandedWithoutDown => VoteMetricVariant::JaccardExpandedWithoutDown(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportUp => VoteMetricVariant::SupportUp(vote_metric::SimilarityStruct::<SupportVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportExpanded => VoteMetricVariant::SupportExpanded(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportExpandedWithoutDown => VoteMetricVariant::SupportExpandedWithoutDown(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportWeightedUp => VoteMetricVariant::SupportWeightedUp(vote_metric::SimilarityStruct::<WeightedSupportMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportWeightedExpanded => VoteMetricVariant::SupportWeightedExpanded(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::SupportWeightedExpandedWithoutDown => VoteMetricVariant::SupportWeightedExpandedWithoutDown(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::LiftUp => VoteMetricVariant::LiftUp(vote_metric::SimilarityStruct::<LiftVoteMetric<UpVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::LiftExpanded => VoteMetricVariant::LiftExpanded(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::LiftExpandedWithoutDown => VoteMetricVariant::LiftExpandedWithoutDown(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedWithoutDownVoteSelector>>::from_single_submission(submission_id, all_ids)),
            ConflcitMode::Cosine => VoteMetricVariant::Cosine(vote_metric::SimilarityStruct::<CosineVoteMetric>::from_single_submission(submission_id, all_ids)),
        };
        let columns = vm.get_columns();
        let votes: Vec<Vec<Vec<String>>> = get_vote_columns(
                    db, 
                    event, 
                    columns,
                ).await?;
        vm.process_votes(votes);
        let res = vm.get_item_results(limit as usize);
        return Ok(SubmissionSimilarityResponse { res });
                
        // if total == 0 {
        //     return Ok(SubmissionSimilarityResponse{res: vec![]});
        // }
            
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
                
        let mut vm: VoteMetricVariant  = match mode {
            // Construct enum variants instead of Box::new()
            ConflcitMode::JaccardUp => VoteMetricVariant::JaccardUp(vote_metric::SimilarityStruct::<JaccardVoteMetric<UpVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::JaccardExpanded => VoteMetricVariant::JaccardExpanded(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::JaccardExpandedWithoutDown => VoteMetricVariant::JaccardExpandedWithoutDown(vote_metric::SimilarityStruct::<JaccardVoteMetric<ExpandedWithoutDownVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportUp => VoteMetricVariant::SupportUp(vote_metric::SimilarityStruct::<SupportVoteMetric<UpVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportExpanded => VoteMetricVariant::SupportExpanded(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportExpandedWithoutDown => VoteMetricVariant::SupportExpandedWithoutDown(vote_metric::SimilarityStruct::<SupportVoteMetric<ExpandedWithoutDownVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportWeightedUp => VoteMetricVariant::SupportWeightedUp(vote_metric::SimilarityStruct::<WeightedSupportMetric<UpVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportWeightedExpanded => VoteMetricVariant::SupportWeightedExpanded(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::SupportWeightedExpandedWithoutDown => VoteMetricVariant::SupportWeightedExpandedWithoutDown(vote_metric::SimilarityStruct::<WeightedSupportMetric<ExpandedWithoutDownVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::LiftUp => VoteMetricVariant::LiftUp(vote_metric::SimilarityStruct::<LiftVoteMetric<UpVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::LiftExpanded => VoteMetricVariant::LiftExpanded(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::LiftExpandedWithoutDown => VoteMetricVariant::LiftExpandedWithoutDown(vote_metric::SimilarityStruct::<LiftVoteMetric<ExpandedWithoutDownVoteSelector>>::from_query_results(submission_conflicts)),
            ConflcitMode::Cosine => VoteMetricVariant::Cosine(vote_metric::SimilarityStruct::<CosineVoteMetric>::from_query_results(submission_conflicts)),
        };
        let columns = vm.get_columns();
        let votes: Vec<Vec<Vec<String>>> = get_vote_columns(
                    db, 
                    event, 
                    columns,
                ).await?;
        vm.process_votes(votes);
        let res = vm.get_results(limit as usize);
        return Ok(EventConflictResponse { res: res })

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

// fn generate_simple_conflict_response(limit: u32, state: Vec<ConflictState>) -> EventConflictResponse {
//     let results = state.into_iter().map(|s| EventConflictItem {
//         a: s.fullcode_a,
//         b: s.fullcode_b,
//         correlation: if s.count_total > 0 { (s.count_joint as f64)/(s.count_total as f64)} else {0.0},
//     })
//     .sorted_by(|a,b| b.correlation.total_cmp(&a.correlation))
//     .take(limit as usize)
//     .collect();
//     let res = EventConflictResponse{res: results};
//     return res;
// }

// fn generate_weighted_conflict_response(limit: u32, state: Vec<WeightedConflictState>) -> EventConflictResponse {
//     let results = state.into_iter().map(|s| EventConflictItem {
//         a: s.fullcode_a,
//         b: s.fullcode_b,
//         correlation: s.votes,
//     })
//     .sorted_by(|a,b| b.correlation.total_cmp(&a.correlation))
//     .take(limit as usize)
//     .collect();
//     let res = EventConflictResponse{res: results};
//     return res;
// }

// fn create_simple_conflict_state_vector(submission_conflicts: Vec<SubmissionConflictQueryItem>, all_conflict_fullcodes: &HashMap<String, usize>) -> Vec<ConflictState> {
//     let mut state = submission_conflicts.into_iter().map(|i| ConflictState {
//         index_a: all_conflict_fullcodes.get(&i.fullcode_a).unwrap().clone(),
//         index_b: all_conflict_fullcodes.get(&i.fullcode_b).unwrap().clone(),
//         fullcode_a: i.fullcode_a,
//         fullcode_b: i.fullcode_b,
//         count_joint: 0,
//         count_total: 0,
//     }).collect::<Vec<ConflictState>>();
//     state
// }

// fn create_view_conflict_state_vector(submission_conflicts: Vec<SubmissionConflictQueryItem>, all_conflict_fullcodes: &HashMap<String, usize>) -> Vec<ViewConflictState> {
//     let mut state = submission_conflicts.into_iter().map(|i| ViewConflictState {
//         index_a: all_conflict_fullcodes.get(&i.fullcode_a).unwrap().clone(),
//         index_b: all_conflict_fullcodes.get(&i.fullcode_b).unwrap().clone(),
//         fullcode_a: i.fullcode_a,
//         fullcode_b: i.fullcode_b,
//         up_joint: 0,
//         view_joint: 0,
//     }).collect::<Vec<ViewConflictState>>();
//     state
// }

// fn create_weighted_conflict_state_vector(submission_conflicts: Vec<SubmissionConflictQueryItem>, all_conflict_fullcodes: &HashMap<String, usize>) -> Vec<WeightedConflictState> {
//     let mut state = submission_conflicts.into_iter().map(|i| WeightedConflictState {
//         index_a: all_conflict_fullcodes.get(&i.fullcode_a).unwrap().clone(),
//         index_b: all_conflict_fullcodes.get(&i.fullcode_b).unwrap().clone(),
//         fullcode_a: i.fullcode_a,
//         fullcode_b: i.fullcode_b,
//         votes: 0.0,
//     }).collect::<Vec<WeightedConflictState>>();
//     state
// }

// fn create_cosine_conflict_state_vector(submission_conflicts: Vec<SubmissionConflictQueryItem>, all_conflict_fullcodes: &HashMap<String, usize>) -> Vec<CosineConflictState> {
//     let mut state = submission_conflicts.into_iter().map(|i| CosineConflictState {
//         index_a: all_conflict_fullcodes.get(&i.fullcode_a).unwrap().clone(),
//         index_b: all_conflict_fullcodes.get(&i.fullcode_b).unwrap().clone(),
//         fullcode_a: i.fullcode_a,
//         fullcode_b: i.fullcode_b,
//         dotprod: 0,
//         norm_a: 0,
//         norm_b: 0,
//     }).collect::<Vec<CosineConflictState>>();
//     state
// }

// fn apply_map_to_bitset(all_conflict_fullcodes: &HashMap<String, usize>, exists: &mut BitVec, column: &Vec<String>, value: bool) {
//     for code in column.iter() {
//         match all_conflict_fullcodes.get(code) {
//             Some(idx) => exists.set(*idx,value),
//             None => ()
//         }
//     }
// }

// fn into_similarity_response<T: Iterator<Item=(String, f64)>>(results: T, limit: usize) -> SubmissionSimilarityResponse {
//     let resvec = results.sorted_by(|(_k1, a), (_k2,b)| b.partial_cmp(a).unwrap())
//         .take(limit as usize)
//         .map(|(k,v)| SubmissionSimilarityItem{ id: k, metric: v})
//         .collect::<Vec<_>>();
//     let res = SubmissionSimilarityResponse{res: resvec};
//     return res;
// }

// fn get_cosine_weight(submission_id: &String, expmap: &HashSet<String>, downmap: &HashSet<String>, upmap: &HashSet<String>) -> i32 {
//     let target_weigth = if downmap.contains(submission_id) { -4 } else if upmap.contains(submission_id) { 4 } else if expmap.contains(submission_id) {1} else {0};
//     target_weigth
// }

// fn create_empty_hash_map<'a, T: IntoIterator<Item=&'a String>, U:Default>(all_ids: T) -> HashMap<String, U> {
//     let iter = all_ids.into_iter();
//     let mut normmap = HashMap::with_capacity(iter.size_hint().0);
//     iter.for_each(|id| {normmap.insert(id.clone(), U::default());});
//     normmap
// }

async fn get_all_ids_for_event(db: &DatabaseConnection, event: i32, submission_id: &String) -> Result<Vec<String>, DbErr> {
    let all_submissions = source::Entity::find()
        .find_with_related(submission::Entity)
        .filter(source::Column::Event.eq(event))
        .all(db)
        .await?;
    let all_ids = all_submissions
        .iter()
        .map(
            |(so, su)| su
                .into_iter()
                .map(
                    |s| format!("{}/{}", so.slug, s.code)
                )
            )
        .flatten()
        .filter(|x| *submission_id != *x)
        .collect::<Vec<String>>();
    Ok(all_ids)
}

async fn get_vote_columns(db: &DatabaseConnection, event: i32, colums: Vec<vote::Column>) -> Result<Vec<Vec<Vec<String>>>, DbErr> {
    let num_columns = colums.len();
    
    let query = vote::Entity::find()
        .filter(vote::Column::Event.eq(event))
        .select_only()
        .columns(colums);
    let backend = db.get_database_backend();
    let statement = query.build(backend);
    let query_results: Vec<QueryResult> = db.query_all(statement).await?;

    let mut final_results: Vec<Vec<Vec<String>>> = Vec::new();

    for row in query_results {
        let mut row_vec: Vec<Vec<String>> = Vec::with_capacity(num_columns);
        for i in 0..num_columns {
            let value: Vec<String> = row.try_get_by_index(i)
                .map_err(|e| DbErr::TryIntoErr { from: "QueryResult", into: "String", source: Box::new(e) })?;
            row_vec.push(value);
        }
        final_results.push(row_vec);
    }


    Ok(final_results)
}

fn source_response_from_model(m: source::Model) -> Result<SourceResponseItem, DbErr> {
    Ok(SourceResponseItem {
        autoupdate: m.autoupdate,
        url: m.url,
        event_slug: m.event_slug,
        interval: m.interval,
        filter: get_filter(m.filter)?,
        submission_types: m.submission_types.map(|i| serde_json::from_value::<SubmissionTypesField>(i).unwrap().items),
        type_filter: m.type_filter,
    })
}

fn get_filter(m: Option<i32>) -> Result<Option<SourceFilter>, DbErr> {
    m
        .map(SourceFilter::try_from)
        .transpose()
        .map_err(|e| DbErr::Custom(format!("Can't convert filter: {}", e)))
}

// fn get_cosine_weight_cond(up: bool, down: bool, expanded: bool) -> i32 {
//     let weight = if up { 4 } else if down {-4} else if expanded {1} else {0};
//     return weight;
// }