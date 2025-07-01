use askama::Template;
use axum::{
    Extension, Json, Router,
    extract::{self, Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, get_service, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::FixedOffset;
use migration::{
    DbErr, Migrator, MigratorTrait,
    sea_orm::{Database, DatabaseConnection},
};
use std::{collections::HashMap, env, fmt::Display, path::PathBuf, sync::Arc, time::Duration};
use reqwest::{header::{HeaderValue as rhv, InvalidHeaderValue}, ClientBuilder};

use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, IssuerUrl, JsonWebKeySet, Nonce,
    OAuth2TokenResponse, RedirectUrl, TokenResponse,
    core::{CoreClient, CoreProviderMetadata},
    reqwest,
};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use service::{
    ConflcitMode, CreateEventRequest, DeleteVoteRequest, ExportRequest, Mutation, RoleInfo,
    SourceToUpdate, SubmissionStartEnd, SubmissionSyncItem, SyncSourceRequest, TokenCreator,
    UpdateEventRequest, UpdateVoteRequest, Votes, sea_orm::DbConn,
};
use tokio::fs;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use url::{ParseError, Url};

use service::{Mutation as MutationCore, Query as QueryCore};

const CSP_CLIENT_VALUE: &str = "default-src 'self'; script-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; ";
const CSP_ADMIN_VALUE: &str = "default-src 'self'; script-src 'self'; img-src 'self'; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; ";

struct OpenIdConfig {
    issuer_url: String,
    client_id: String,
    callback_url: String,
    client_secret: String,
}

struct ServerConfig {
    external_url: Url,
    default_redirect: Option<String>,
    new_users_can_create: bool,
    open_id_config: Option<OpenIdConfig>,
}

#[derive(Debug)]
enum UpdateError {
    Parse(ParseError),
    Request(reqwest::Error),
    Db(DbErr),
    HeaderValue(InvalidHeaderValue),
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Parse(parse_error) => f.write_fmt(format_args!("UpdateError ParsingError: {:#?}", parse_error)),
            UpdateError::Request(error) => f.write_fmt(format_args!("UpdateError RequestError: {:#?}", error)),
            UpdateError::Db(db_err) => f.write_fmt(format_args!("UpdateError DatabaseError: {:#?}", db_err)),
            UpdateError::HeaderValue(invalid_header_value) => f.write_fmt(format_args!("UpdateError InvalidHeaderValue: {:#?}", invalid_header_value)),
        }
    }
}

impl From<ParseError> for UpdateError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<DbErr> for UpdateError {
    fn from(value: DbErr) -> Self {
        Self::Db(value)
    }
}

impl From<reqwest::Error> for UpdateError {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(value)
    }
}

impl From<InvalidHeaderValue> for UpdateError {
    fn from(value: InvalidHeaderValue) ->  Self {
        Self::HeaderValue(value)
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ManualUpdateRequest {
    submissions: Option<HashMap<String, SubmissionSyncItem>>,
    schedule: Option<HashMap<String, SubmissionStartEnd>>,
}

fn clientbuilder_with_auth(token: &String) -> Result<ClientBuilder,InvalidHeaderValue> {
    let mut headers = header::HeaderMap::new();

    let mut auth_value = rhv::from_str(format!("token {}", token).as_str())?;
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    Ok(reqwest::Client::builder()
    .default_headers(headers))
}

async fn update_source(db: &DbConn, source: SourceToUpdate) -> Result<(), UpdateError> {
    MutationCore::update_source_attemp(db, source.source_id).await?;
    let state = match source.filter {
        service::SourceFilter::Confirmed => "state=confirmed",
        service::SourceFilter::Accepted => "state=accepted&state=confirmed",
    };
    let mut url = Url::parse(source.update_url.as_str())?.join(
        format!(
            "/api/events/{}/submissions/?{}&expand=slots",
            &source.event_slug, state
        )
        .as_str(),
    )?;
    let on_https = url.scheme() == "https";
    let mut all_submissions = vec![];
    loop {
        debug!("Performing request for {}", url);
        let mut res = clientbuilder_with_auth(&source.apikey)?
            .build()?
            .get(url)
            .send()
            .await?
            .json::<PretalxSubmissionsResponse>()
            .await?;

        all_submissions.append(&mut res.results);
        if let Some(next_url) = res.next {
            url = Url::parse(next_url.as_str())?;
            if on_https {
                info!("Updating url {} to https", &url);
                url.set_scheme("https").unwrap();
            }
            
            debug!("Next url is: {}", url);
        } else {
            break;
        };
    }
    let schedule_url = Url::parse(source.update_url.as_str())?.join(
        format!(
            "/api/events/{}/schedules/wip/?expand=slots",
            &source.event_slug
        )
        .as_str(),
    )?;
    debug!("Schedule url is {}", &schedule_url);
    let res = clientbuilder_with_auth(&source.apikey)?
        .build()?
        .get(schedule_url)
        .send()
        .await?
        .json::<PretalxScheduleResponse>()
        .await?;
    let schedule_map = res
        .slots
        .into_iter()
        .filter_map(|slot| match (slot.submission, slot.start, slot.end) {
            (Some(code), Some(start_timestamp), Some(end_timestamp)) => Some((
                code,
                SubmissionStartEnd {
                    start: start_timestamp.to_utc(),
                    end: end_timestamp.to_utc(),
                },
            )),
            _ => None,
        })
        .collect::<HashMap<String, SubmissionStartEnd>>();

    let req = all_submissions
        .into_iter()
        .map(|submission| {
            (
                submission.code.clone(),
                SubmissionSyncItem {
                    title: submission.title,
                    r#abstract: submission.r#abstract.unwrap_or("".to_string()),
                    schedule_time: schedule_map.get(&submission.code).map(|t| t.clone()),
                    code: submission.code,
                },
            )
        })
        .collect::<HashMap<String, _>>();
    MutationCore::sync_source(
        db,
        SyncSourceRequest {
            source_id: source.source_id,
            submissions: req,
        },
    )
    .await?;
    Ok(())
}

async fn perform_updates(db: &DbConn) -> Result<(), UpdateError> {
    let sources_to_update = QueryCore::get_sources_for_updates(db).await?;
    for source in sources_to_update {
        let db = db.clone();
        let res = update_source(&db, source).await;
        if let Err(e) = res {
            warn!("Failed to update source: {:?}", e);
        }
    }
    Ok(())
}

#[tokio::main]
async fn start(user_to_create: Option<(String, String, String)>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        // .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");

    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");

    let new_users_can_create = env::var("NEW_USERS_CAN_CREATE")
        .unwrap_or("FALSE".into())
        .parse()
        .unwrap_or(false);

    if let Some((name, email, password)) = user_to_create {
        // Just create a user and then return
        MutationCore::create_user(&conn, name, email, password, new_users_can_create).await?;
        return Ok(());
    }

    Migrator::up(&conn, None).await.unwrap();
    let server_url = format!("{host}:{port}");
    let external_url = Url::parse(
        env::var("EXT_URL")
            .expect("EXTURL is not set in .env file")
            .as_str(),
    )?;

    let seed_event = env::var("SEED_EVENT")
        .unwrap_or("FALSE".into())
        .parse()
        .unwrap_or(false);
    let default_redirect = env::var("DEFAULT_REDIRECT").ok();

    let openid_enabled: bool = env::var("OPENID_ENABLED")
        .unwrap_or("FALSE".into())
        .parse()
        .unwrap_or(false);
    let server_config = ServerConfig {
        open_id_config: match openid_enabled {
            true => Some(OpenIdConfig {
                issuer_url: env::var("OPENID_ISSUERURL").unwrap(),
                client_id: env::var("OPENID_CLIENTID").unwrap(),
                // callback_url: env::var("OPENID_CALLBACKURL").unwrap(),
                callback_url: external_url.join("admin/callback").unwrap().to_string(),
                client_secret: env::var("OPENID_CLIENTSECRET").unwrap(),
            }),
            false => None,
        },
        external_url: external_url,
        new_users_can_create: new_users_can_create,
        default_redirect: default_redirect,
    };

    let arc_config = Arc::new(server_config);

    if seed_event {
        Mutation::seed_event(&conn)
            .await
            .unwrap_or_else(|err| println!("Seed event could not be created! {}", err));
    }

    refresh_metadata(&conn, &arc_config)
        .await
        .unwrap_or_else(|e| {
            error!(
                "Failed to refresh the provider metadata on startup: {:?}",
                e
            )
        });

    let state = AppState { conn };
    let state_for_refresh = state.clone();
    let config_for_refresh = arc_config.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1800)).await;
        refresh_metadata(&state_for_refresh.conn, &config_for_refresh)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to refresh the provider metadata on startup: {:?}",
                    e
                )
            });
    });

    let state_for_update = state.clone();

    tokio::spawn(async move {
        loop {
            let c = state_for_update.conn.clone();
            tokio::spawn(async move { perform_updates(&c).await })
                .await
                .unwrap_or_else(|e| {
                    warn!("Failed to join perform_updates join handle: {:?}", e);
                    Ok(())
                })
                .unwrap_or_else(|e| {
                    warn!("Failed to update soruces: {:?}", e);
                });
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

    let mut frondends_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    frondends_dir.pop();
    frondends_dir.push("frontends");

    let static_router = Router::new()
        .nest_service(
            "/frontend",
            get_service(ServeDir::new(
                frondends_dir.join("client-frontend/dist/static/frontend/"),
            ))
            .handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {error}"),
                )
            }),
        )
        .nest_service(
            "/admin",
            get_service(ServeDir::new(
                frondends_dir.join("admin-frontend/dist/static/admin/"),
            ))
            .handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {error}"),
                )
            }),
        );

    let admin_api = Router::new()
        .route("/exchange", post(post_exchange_token))
        .route("/login", post(login_user))
        .route("/events", get(get_events))
        .route("/events", post(post_create_event))
        .route("/events/{slug}", post(post_update_event))
        .route("/overview/{slug}", get(get_overview))
        .route("/submissions/{slug}", get(get_submissions))
        .route("/sources/{slug}", get(get_sources))
        .route("/users/{slug}", get(get_users))
        .route("/usersearch/{slug}", get(get_search_users))
        .route("/users/{slug}/{user}", post(post_edit_user))
        .route("/sources/{slug}/{source}", post(post_update_source))
        .route("/sources/{slug}/{source}", delete(delete_source))
        .route("/urls/{slug}", get(get_source_update_urls))
        .route("/ratings/{slug}", get(get_event_ratings))
        .route("/conflicts/{slug}/{mode}/{limit}", get(get_event_conflicts))
        .route(
            "/updatesource/{event_slug}/{source_slug}/{token}",
            post(post_manual_update_source),
        )
        .layer(Extension(arc_config.clone()));

    let csp_admin_header_value = HeaderValue::from_static(CSP_ADMIN_VALUE);
    let csp_admin_layer = SetResponseHeaderLayer::if_not_present(
        axum::http::header::CONTENT_SECURITY_POLICY,
        csp_admin_header_value.clone(),
    );

    let admin_app = Router::new()
        .route("/", get(deliver_admin_app).layer(csp_admin_layer))
        .route("/callback", get(deliver_callback_page));

    let client_api = Router::new()
        .route("/{slug}/getsubmissions", get(get_submissions_no_version))
        .route(
            "/{slug}/getsubmissions/{version}",
            get(get_submissions_with_version),
        )
        .route("/{slug}/sendvotes", post(post_update_votes))
        .route("/{slug}/delete", post(delete_votes))
        .route("/{slug}/delete-all", post(delete_votes_all))
        .route("/{slug}/export", post(export_votes));

    let csp_client_header_value = HeaderValue::from_static(CSP_CLIENT_VALUE);
    let csp_client_layer = SetResponseHeaderLayer::if_not_present(
        axum::http::header::CONTENT_SECURITY_POLICY,
        csp_client_header_value.clone(),
    );

    let app = Router::new()
        .route("/{slug}", get(deliver_client_app).layer(csp_client_layer))
        .route("/", get(deliver_redirect))
        .nest("/api/client", client_api)
        .nest("/api/admin", admin_api)
        .nest("/admin", admin_app)
        .nest("/static", static_router)
        .layer(Extension(arc_config))
        .with_state(state);
    debug!("Final app: {:?}", app);
    let listener = tokio::net::TcpListener::bind(&server_url).await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
}

#[derive(Template)]
#[template(path = "../../frontends/client-frontend/dist/index.html")]
struct ClientApp<'a> {
    title: &'a str,
    apiprefix: &'a str,
    storage: &'a str,
}

#[derive(Template)]
#[template(path = "../../frontends/admin-frontend/dist/index.html")]
struct AdminApp<'a> {
    apiprefix: &'a str,
    providerurl: &'a str,
    clientid: &'a str,
    callback: &'a str,
}

// --- 2. Define a custom error type for Askama rendering failures ---
#[derive(Debug)]
enum AppError {
    Render(askama::Error),
    Db(DbErr),
    Parameter,
    JsonRejection(JsonRejection),
    Oauth(String),
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        AppError::JsonRejection(rejection)
    }
}

// Implement From trait to convert askama::Error into AppError
impl From<askama::Error> for AppError {
    fn from(inner: askama::Error) -> Self {
        AppError::Render(inner)
    }
}

impl From<DbErr> for AppError {
    fn from(inner: DbErr) -> Self {
        AppError::Db(inner)
    }
}

impl From<url::ParseError> for AppError {
    fn from(_inner: url::ParseError) -> Self {
        AppError::Parameter
    }
}


// Implement IntoResponse for AppError so Axum can convert it to an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let resp = Html(format!(
                    r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <title>Internal Server Error</title>
                        <style>
                            body {{ font-family: sans-serif; background-color: #f8f8f8; color: #333; }}
                            .container {{ margin: 50px auto; padding: 30px; border-radius: 8px; background-color: #fff; box-shadow: 0 2px 4px rgba(0,0,0,0.1); max-width: 600px; text-align: center; }}
                            h1 {{ color: #dc3545; }}
                        </style>
                    </head>
                    <body>
                        <div class="container">
                            <h1>500 Internal Server Error</h1>
                            <p>Something went wrong on our end. Please try again later.</p>
                        </div>
                    </body>
                    </html>
                    "#,
                )).into_response();
        let body = match self {
            AppError::Render(err) => {
                warn!("Template rendering error: {}", err);
                resp
            }
            AppError::Db(db_err) => {
                warn!("DB error: {}", db_err);
                resp
            }
            AppError::Parameter => {
                warn!("Wrong parameters");
                resp
            }
            AppError::Oauth(s) => {
                warn!("Oauth problem: {}", s);
                resp
            }
            AppError::JsonRejection(json_rejection) => {
                warn!("JSON parsing error: {:?}", json_rejection);
                resp
            }
        };

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")] 
pub enum Rating {
    Up,
    Neutral,
    Down,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SingleVote {
    expanded: bool,
    vote: Rating,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SubmitVotesPayload {
    #[serde(rename = "clientId")]
    clientid: String,
    #[serde(rename = "sequenceNumber")]
    sequence_number: u32,
    votes: HashMap<String, SingleVote>,
}

impl From<SubmitVotesPayload> for Votes {
    fn from(value: SubmitVotesPayload) -> Self {
        value.votes.iter().fold(
            Votes {
                shown: vec![],
                expanded: vec![],
                up: vec![],
                down: vec![],
            },
            |mut vecs, (code, value)| {
                vecs.shown.push(code.into());
                if value.expanded {
                    vecs.expanded.push(code.into());
                }
                match value.vote {
                    Rating::Up => vecs.up.push(code.into()),
                    Rating::Neutral => {}
                    Rating::Down => vecs.down.push(code.into()),
                };
                vecs
            },
        )
    }
}

#[derive(Deserialize)]
struct DeleteVotesPayload {
    #[serde(rename = "clientId")]
    client_id: String,
}

async fn export_votes(
    state: State<AppState>,
    Path(_slug): Path<String>,
    extract::Json(payload): extract::Json<ExportRequest>,
) -> Result<impl IntoResponse, AppError> {
    let res = QueryCore::export_for_events(&state.conn, payload).await?;
    Ok(Json(res))
}

async fn delete_votes_all(
    state: State<AppState>,
    Path(_slug): Path<String>,
    extract::Json(payload): extract::Json<DeleteVoteRequest>,
) -> Result<impl IntoResponse, AppError> {
    MutationCore::delete_votes(&state.conn, payload).await?;
    Ok(())
}

async fn delete_votes(
    state: State<AppState>,
    Path(slug): Path<String>,
    extract::Json(payload): extract::Json<DeleteVotesPayload>,
) -> Result<impl IntoResponse, AppError> {
    let m = HashMap::from([(slug, payload.client_id)]);
    let req = DeleteVoteRequest { client_ids: m };
    MutationCore::delete_votes(&state.conn, req).await?;
    Ok(())
}

async fn post_update_votes(
    state: State<AppState>,
    Path(slug): Path<String>,
    extract::Json(payload): extract::Json<SubmitVotesPayload>,
) -> Result<impl IntoResponse, AppError> {
    let seq = payload.sequence_number;
    let client = payload.clientid.clone();
    let votes = Votes::from(payload);
    let request = UpdateVoteRequest {
        sequence: seq,
        votes: votes,
        event: slug,
        client: client,
    };
    MutationCore::update_votes(&state.conn, request).await?;
    let response: Value = serde_json::from_str(r#"{"status": "success"}"#).unwrap();
    Ok(Json(response))
}

async fn get_submissions_no_version(
    state: State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let res = QueryCore::get_conditional_content(&state.conn, slug, None).await?;
    Ok(Json(res))
}

async fn get_submissions_with_version(
    state: State<AppState>,
    Path((slug, version)): Path<(String, i32)>,
) -> Result<impl IntoResponse, AppError> {
    let res = QueryCore::get_conditional_content(&state.conn, slug, Some(version)).await?;
    Ok(Json(res))
}

async fn deliver_callback_page() -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let file_path = "frontends/admin-frontend/dist/oidc-callback.html"; 

    // Read the file asynchronously
    match fs::read(file_path).await {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));

            Ok((headers, bytes))
        }
        Err(e) => {
            warn!("Error reading HTML file '{}': {}", file_path, e);

            match e.kind() {
                std::io::ErrorKind::NotFound => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

#[derive(Deserialize)]
struct ExchangeCode {
    code: String,
}

#[derive(Serialize)]
struct TokenResponseJson {
    token: String,
}

#[derive(Serialize, Deserialize)]
struct SavedMetadata {
    keyset: Value,
    metadata: Value,
}

async fn save_provider(
    db: &DbConn,
    name: impl Into<String>,
    metadata: CoreProviderMetadata,
) -> Result<(), AppError> {
    let authurl = metadata.authorization_endpoint().to_string();
    let keyset = serde_json::to_value(metadata.jwks())
        .map_err(|e| AppError::Oauth(format!("Could not serialize jwks: {}", e)))?;
    let serialized = serde_json::to_value(metadata)
        .map_err(|e| AppError::Oauth(format!("Could not serialize provider: {}", e)))?;
    let to_save = SavedMetadata {
        keyset: keyset,
        metadata: serialized,
    };
    MutationCore::update_oicd_provider(
        db,
        name,
        authurl,
        serde_json::to_value(to_save)
            .map_err(|e| AppError::Oauth(format!("Could not serialize saved metadata: {}", e)))?,
    )
    .await?;
    Ok(())
}

async fn get_metadata(
    db: &DbConn,
    name: impl Into<String>,
) -> Result<Option<CoreProviderMetadata>, AppError> {
    let res = QueryCore::get_provider_metadata(db, name).await?;
    match res {
        Some(v) => {
            let saved = serde_json::from_value::<SavedMetadata>(v).map_err(|e| {
                AppError::Oauth(format!("Failed to deserialize saved value: {}", e))
            })?;
            let keyset =
                serde_json::from_value::<JsonWebKeySet<openidconnect::core::CoreJsonWebKey>>(
                    saved.keyset,
                )
                .map_err(|e| AppError::Oauth(format!("Could not deserialize keyset: {}", e)))?;
            let metadata = serde_json::from_value::<CoreProviderMetadata>(saved.metadata)
                .map_err(|e| AppError::Oauth(format!("Could not deserialize metadata: {}", e)))?;
            Ok(Some(metadata.set_jwks(keyset)))
        }
        None => Ok(None),
    }
}

async fn refresh_metadata(db: &DbConn, config: &Arc<ServerConfig>) -> Result<(), AppError> {
    let openidconfig = config
        .open_id_config
        .as_ref()
        .ok_or(AppError::Oauth("Could not build config".to_string()))?;
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    // Use OpenID Connect Discovery to fetch the provider metadata.
    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(openidconfig.issuer_url.to_string())
            .map_err(|e| AppError::Oauth(format!("Could not parse issuer: {}", e)))?,
        &http_client,
    )
    .await
    .map_err(|e| AppError::Oauth(format!("Failed to get metadata: {}", e)))?;
    let auth_url = provider_metadata.authorization_endpoint().to_string();
    debug!("auth_url: {}", auth_url);
    save_provider(db, "default", provider_metadata).await
}

#[derive(Serialize)]
struct OverviewResponse {
    total_voters: i32,
    total_submissions: i32,
    total_sources: i32,
}

#[derive(Serialize)]
struct SuccessResponse {
    status: String,
}

fn get_success_response() -> Json<SuccessResponse> {
    return Json(SuccessResponse {
        status: "success".to_string(),
    });
}

async fn post_manual_update_source(
    state: State<AppState>,
    Path((event_slug, source_slug, token)): Path<(String, String, String)>,
    Json(req): Json<ManualUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let source_id =
        QueryCore::verify_update_token(&state.conn, event_slug, source_slug, token).await?;
    if req.schedule.is_some() && req.submissions.is_some() {
        return Err(AppError::Parameter);
    }
    if let Some(submissions) = req.submissions {
        MutationCore::sync_source(
            &state.conn,
            SyncSourceRequest {
                source_id: source_id,
                submissions: submissions,
            },
        )
        .await?;
    } else if let Some(schedule) = req.schedule {
        MutationCore::sync_schedule(&state.conn, source_id, schedule).await?;
    } else {
        return Err(AppError::Parameter);
    }

    Ok(get_success_response())
}

async fn post_update_event(
    state: State<AppState>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                MutationCore::update_event(&state.conn, event_with_perm.event, req).await?;
                return Ok(get_success_response());
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn get_event_conflicts(
    state: State<AppState>,
    Path((slug, mode, limit)): Path<(String, String, u32)>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let confict_mode = ConflcitMode::try_from(mode.as_str()).map_err(|_e| AppError::Parameter)?;
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, &slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            let conflicts = QueryCore::get_event_conflicts(
                &state.conn,
                event_with_perm.event,
                confict_mode,
                limit,
            )
            .await?;
            return Ok(Json(conflicts));
        }
    }
    return Err(AppError::Parameter);
}

async fn get_event_ratings(
    state: State<AppState>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, &slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            // Perms are fine
            let ratings = QueryCore::get_event_ratings(&state.conn, event_with_perm.event).await?;
            return Ok(Json(ratings));
        }
    }
    return Err(AppError::Parameter);
}

async fn get_source_update_urls(
    state: State<AppState>,
    Extension(sc): Extension<Arc<ServerConfig>>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, &slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if (event_with_perm.perm == RoleInfo::Admin)
                || (event_with_perm.perm == RoleInfo::Update)
            {
                let prefix = sc
                    .external_url
                    .join("api/admin/updatesource/")?
                    .join(format!("{}/", slug).as_str())?;
                debug!("Prefix is {}", &prefix);
                let res =
                    QueryCore::get_update_urls(&state.conn, event_with_perm.event, prefix).await?;
                return Ok(Json(res));
            }
        }
    }
    return Err(AppError::Parameter);
}

#[axum::debug_handler]
async fn post_update_source(
    state: State<AppState>,
    Path((slug, source)): Path<(String, String)>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Json(update): Json<service::UpdateSourceRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                MutationCore::update_source(&state.conn, event_with_perm.event, source, update)
                    .await?;
                return Ok(get_success_response());
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn get_users(
    state: State<AppState>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                let resp = QueryCore::get_conference_users(&state.conn, event_with_perm.event, uid)
                    .await?;
                return Ok(Json(resp));
            }
        }
    }
    return Err(AppError::Parameter);
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    n: u64,
}

#[derive(Deserialize)]
struct NewPermissions {
    permissions: Option<RoleInfo>,
}

async fn delete_source(
    state: State<AppState>,
    Path((slug, source)): Path<(String, String)>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                MutationCore::delete_source(&state.conn, event_with_perm.event, source).await?;
                return Ok(get_success_response());
            }
        }
    }
    Err(AppError::Parameter)
}

#[axum::debug_handler]
async fn post_edit_user(
    state: State<AppState>,
    Path((slug, user_to_edit)): Path<(String, i32)>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Json(newperm): Json<NewPermissions>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                MutationCore::edit_user(
                    &state.conn,
                    event_with_perm.event,
                    user_to_edit,
                    newperm.permissions,
                )
                .await?;
                return Ok(get_success_response());
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn get_search_users(
    state: State<AppState>,
    Path(slug): Path<String>,
    params: Query<SearchParams>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            if event_with_perm.perm == RoleInfo::Admin {
                let resp = QueryCore::search_users(
                    &state.conn,
                    event_with_perm.event,
                    params.q.clone(),
                    params.n,
                )
                .await?;
                return Ok(Json(resp));
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn get_sources(
    state: State<AppState>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            let sources = QueryCore::get_sources(&state.conn, event_with_perm.event).await?;
            return Ok(Json(sources));
        }
    }
    return Err(AppError::Parameter);
}

async fn get_overview(
    state: State<AppState>,
    Path(slug): Path<String>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            let maybe_overview =
                QueryCore::get_event_overview(&state.conn, event_with_perm.event).await?;
            if let Some(overview) = maybe_overview {
                return Ok(Json(OverviewResponse {
                    total_voters: overview.votes,
                    total_submissions: overview.submissions,
                    total_sources: overview.sources,
                }));
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn get_events(
    state: State<AppState>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let er = QueryCore::get_events(&state.conn, uid).await?;
        match er {
            Some(r) => Ok(Json(r)),
            None => Err(AppError::Parameter),
        }
    } else {
        Err(AppError::Parameter)
    }
}

async fn post_create_event(
    state: State<AppState>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Json(req): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let create = QueryCore::user_can_create(&state.conn, uid).await?;
        if create {
            MutationCore::create_event(&state.conn, req, uid).await?;
            return Ok(get_success_response());
        }
    }
    return Err(AppError::Parameter);
}

async fn login_user(
    state: State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let res = MutationCore::login(&state.conn, req.username, req.password).await?;
    let resp = TokenResponseJson { token: res };
    Ok(Json(resp))
}

async fn get_submissions(
    state: State<AppState>,
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = QueryCore::get_user_from_token(&state.conn, token.token()).await?;
    if let Some(uid) = user {
        let maybe_event_with_perm =
            QueryCore::get_event_perm_for_user(&state.conn, uid, slug).await?;
        if let Some(event_with_perm) = maybe_event_with_perm {
            let res = QueryCore::get_submissions(&state.conn, event_with_perm.event).await?;
            if let Some(r) = res {
                return Ok(Json(r));
            }
        }
    }
    return Err(AppError::Parameter);
}

async fn post_exchange_token(
    Extension(sc): Extension<Arc<ServerConfig>>,
    state: State<AppState>,
    Json(code): Json<ExchangeCode>,
) -> Result<impl IntoResponse, AppError> {
    let openidconfig = sc
        .open_id_config
        .as_ref()
        .ok_or(AppError::Oauth("Could not build config".to_string()))?;
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");
    // Use OpenID Connect Discovery to fetch the provider metadata.
    let provider_metadata = get_metadata(&state.conn, "default")
        .await?
        .ok_or_else(|| AppError::Oauth("Could not find metadata in database".to_string()))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(openidconfig.client_id.clone()),
        Some(ClientSecret::new(openidconfig.client_secret.clone())),
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(
        RedirectUrl::new(openidconfig.callback_url.clone())
            .map_err(|e| AppError::Oauth(format!("Building client failed: {}", e)))?,
    );

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(code.code))
        .map_err(|e| AppError::Oauth(format!("Configuration error for oauth code: {}", e)))?
        // Set the PKCE code verifier.
        .request_async(&http_client)
        .await
        .map_err(|e| AppError::Oauth(format!("Could not exchange token: {}", e)))?;
    let id_token = token_response
        .id_token()
        .ok_or_else(|| AppError::Oauth(format!("Could not get id token")))?;

    let id_token_verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&id_token_verifier, |_: Option<&Nonce>| Ok(()))
        .map_err(|e| AppError::Oauth(format!("Could not get claims: {}", e)))?;

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token
                .signing_alg()
                .map_err(|e| AppError::Oauth(format!("Could not verify signature: {}", e)))?,
            id_token
                .signing_key(&id_token_verifier)
                .map_err(|e| AppError::Oauth(format!("Could not verify id token: {}", e)))?,
        )
        .map_err(|e| AppError::Oauth(format!("Signing error: {}", e)))?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(AppError::Oauth(format!("Hashes do not match")));
        }
    }
    let subject = claims.subject().as_str();
    let email = claims
        .email()
        .map(|email| email.as_str())
        .ok_or_else(|| AppError::Oauth("No mail supplied".to_string()))?;
    let email_verified = claims
        .email_verified()
        .ok_or_else(|| AppError::Oauth("Email verified not in claims".to_string()))?;
    if !email_verified {
        return Err(AppError::Oauth("Email not verified".to_string()));
    }
    let name = claims
        .name()
        .map(|lname| lname.get(None).map(|name| name.as_str()))
        .flatten()
        .ok_or_else(|| AppError::Oauth("No name provided".to_string()))?;

    let user = MutationCore::get_or_create_oid_user(
        &state.conn,
        "default".into(),
        name.to_string(),
        email.to_string(),
        subject.to_string(),
        sc.new_users_can_create,
    )
    .await?
    .ok_or_else(|| AppError::Oauth("Problem with DB sync".to_string()))?;

    let response_string = MutationCore::gentoken();

    MutationCore::insert_token(
        &state.conn,
        user,
        response_string.clone(),
        TokenCreator::OpenID,
    )
    .await?;

    let resp = TokenResponseJson {
        token: response_string,
    };
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct PretalxSubmission {
    code: String,
    title: String,
    r#abstract: Option<String>,
}

#[derive(Deserialize)]
struct PretalxSubmissionsResponse {
    next: Option<String>,
    results: Vec<PretalxSubmission>,
}

#[derive(Deserialize)]
struct PretalxScheduleSlot {
    start: Option<chrono::DateTime<FixedOffset>>,
    end: Option<chrono::DateTime<FixedOffset>>,
    submission: Option<String>,
}

#[derive(Deserialize)]
struct PretalxScheduleResponse {
    slots: Vec<PretalxScheduleSlot>,
}

#[axum::debug_handler]
async fn deliver_admin_app(
    state: State<AppState>,
    Extension(sc): Extension<Arc<ServerConfig>>,
) -> Result<impl IntoResponse, AppError> {
    let authurl = QueryCore::get_provider_authurl(&state.conn, "default")
        .await?
        .ok_or_else(|| AppError::Parameter)?;
    Ok(Html(match &sc.open_id_config {
        Some(openidconfig) => AdminApp {
            apiprefix: "/api/admin",
            providerurl: &authurl,
            clientid: &openidconfig.client_id,
            callback: &openidconfig.callback_url,
        }
        .render()?,
        None => AdminApp {
            apiprefix: "/api/admin",
            providerurl: "",
            clientid: "",
            callback: "",
        }
        .render()?,
    }))
}

async fn deliver_redirect(
    Extension(sc): Extension<Arc<ServerConfig>>,
) -> Result<Response, AppError> {
    match &sc.default_redirect {
            Some(url) => {
                debug!("Default recirect: {}", &url);
                Ok(Redirect::temporary(url).into_response())
            },
            None => Err(AppError::Parameter),
    }
}

async fn deliver_client_app(
    state: State<AppState>,
    Extension(sc): Extension<Arc<ServerConfig>>,
    Path(slug): Path<String>,
) -> Result<Response, AppError> {
    let title = QueryCore::get_active_event_title(&state.conn, &slug).await?;
    match title {
        Some(t) => {
            let prefix = format!("/api/client/{}", &slug);
            let client_app = ClientApp {
                title: &t,
                apiprefix: &prefix,
                storage: &slug,
            };
            Ok(Html(client_app.render()?).into_response())
        }
        None => match &sc.default_redirect {
            Some(url) => Ok(Redirect::temporary(url).into_response()),
            None => Err(AppError::Parameter),
        },
    }
}

pub fn main(user_to_create: Option<(String, String, String)>) {
    let result = start(user_to_create);

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
