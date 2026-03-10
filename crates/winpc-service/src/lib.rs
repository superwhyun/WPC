mod html;
mod ipc;
mod platform;
mod state;

use std::{future::Future, path::PathBuf, time::Duration};

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use tokio::net::TcpListener;
use tracing::{error, info};
use uuid::Uuid;
use winpc_core::{
    config::default_config_path, AppConfig, AuthPinRequest, AuthPinResponse, DeviceStatus, Error,
    LockActionResponse, LockCommandRequest,
};

use crate::{ipc::run_ipc_server, state::SharedState};

const LISTEN_ADDR: &str = "127.0.0.1:46391";
const SESSION_TTL_MINUTES: i64 = 10;

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "winpc_service=info".into()),
        )
        .without_time()
        .try_init();
}

pub fn run() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if try_handle_cli()? {
        return Ok(());
    }
    platform::run()
}

pub async fn run_console() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path =
        std::env::var_os("WINPC_CONFIG_PATH").map_or_else(default_config_path, Into::into);
    run_with_shutdown_signal(config_path, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

pub async fn run_with_shutdown_signal<S>(
    config_path: PathBuf,
    shutdown: S,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: Future<Output = ()> + Send + 'static,
{
    let state = SharedState::load(config_path).await?;
    let supervisor = tokio::spawn(supervisor_loop(state.clone()));
    let ipc = tokio::spawn(run_ipc_server(state.clone()));
    let http = tokio::spawn(run_http_server(state, shutdown));

    tokio::select! {
        result = http => {
            result??;
        }
        result = ipc => {
            result??;
        }
    }

    supervisor.abort();
    Ok(())
}

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/api/device/status", get(get_status))
        .route("/api/auth/pin", post(auth_pin))
        .route("/api/device/unlock", post(unlock))
        .route("/api/device/extend", post(extend))
        .route("/api/device/lock", post(lock))
        .with_state(state)
}

async fn run_http_server<S>(
    state: SharedState,
    shutdown: S,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: Future<Output = ()> + Send + 'static,
{
    let app = build_router(state);
    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    let local_addr = listener.local_addr()?;
    info!("HTTP control plane listening on {local_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn supervisor_loop(state: SharedState) {
    loop {
        if let Err(error) = platform::supervisor_tick(&state).await {
            error!("supervisor tick failed: {error}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn index() -> Html<&'static str> {
    Html(html::INDEX_HTML)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn get_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> std::result::Result<Json<DeviceStatus>, HttpError> {
    require_tailnet(&state, &headers).await?;
    Ok(Json(state.device_status().await))
}

async fn auth_pin(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<AuthPinRequest>,
) -> std::result::Result<Json<AuthPinResponse>, HttpError> {
    require_tailnet(&state, &headers).await?;
    state.verify_pin(&payload.pin).await?;
    let expires_at_utc = Utc::now() + chrono::Duration::minutes(SESSION_TTL_MINUTES);
    let token = Uuid::new_v4().to_string();
    state.record_session(token.clone(), expires_at_utc).await;
    Ok(Json(AuthPinResponse {
        token,
        expires_at_utc,
    }))
}

async fn unlock(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<LockCommandRequest>,
) -> std::result::Result<Json<LockActionResponse>, HttpError> {
    authorize_control(&state, &headers).await?;
    let status = state.unlock(payload.duration_minutes).await?;
    Ok(Json(LockActionResponse { status }))
}

async fn extend(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<LockCommandRequest>,
) -> std::result::Result<Json<LockActionResponse>, HttpError> {
    authorize_control(&state, &headers).await?;
    let status = state.extend(payload.duration_minutes).await?;
    Ok(Json(LockActionResponse { status }))
}

async fn lock(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> std::result::Result<Json<LockActionResponse>, HttpError> {
    authorize_control(&state, &headers).await?;
    let status = state.lock().await?;
    Ok(Json(LockActionResponse { status }))
}

async fn authorize_control(
    state: &SharedState,
    headers: &HeaderMap,
) -> std::result::Result<(), HttpError> {
    require_tailnet(state, headers).await?;
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(HttpError::from(Error::InvalidSessionToken))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or(HttpError::from(Error::InvalidSessionToken))?;
    state.require_session(token).await?;
    Ok(())
}

async fn require_tailnet(
    state: &SharedState,
    headers: &HeaderMap,
) -> std::result::Result<String, HttpError> {
    state
        .authorize_tailnet(headers)
        .await
        .map_err(HttpError::from)
}

#[derive(Debug)]
struct HttpError {
    status: StatusCode,
    message: String,
}

impl From<Error> for HttpError {
    fn from(value: Error) -> Self {
        let status = match value {
            Error::MissingTailnetIdentity | Error::UnauthorizedTailnetIdentity => {
                StatusCode::UNAUTHORIZED
            }
            Error::InvalidPin | Error::InvalidSessionToken => StatusCode::UNAUTHORIZED,
            Error::InvalidDuration => StatusCode::BAD_REQUEST,
            Error::ConfigIncomplete(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: value.to_string(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

fn try_handle_cli() -> std::result::Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) != Some("--init-config") {
        return Ok(false);
    }

    let _ = args.next();
    let mut config_path =
        std::env::var_os("WINPC_CONFIG_PATH").map_or_else(default_config_path, Into::into);
    let mut protected_user_sid = None;
    let mut allowed_logins = Vec::new();
    let mut pin = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let value = args.next().ok_or("missing value for --config")?;
                config_path = PathBuf::from(value);
            }
            "--protected-user-sid" => {
                protected_user_sid = Some(
                    args.next()
                        .ok_or("missing value for --protected-user-sid")?,
                );
            }
            "--allowed-login" => {
                allowed_logins.push(args.next().ok_or("missing value for --allowed-login")?);
            }
            "--pin" => {
                pin = Some(args.next().ok_or("missing value for --pin")?);
            }
            other => {
                return Err(format!("unsupported init-config argument: {other}").into());
            }
        }
    }

    let mut config = AppConfig::load(&config_path)?;
    if let Some(sid) = protected_user_sid {
        config.protected_user_sid = Some(sid);
    }
    if !allowed_logins.is_empty() {
        config.allowed_tailnet_logins = allowed_logins;
    }
    if let Some(pin) = pin {
        config.set_pin(&pin)?;
    }
    config.save(&config_path)?;
    println!("Wrote config to {}", config_path.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use winpc_core::AppConfig;

    use super::*;

    #[tokio::test]
    async fn status_requires_tailnet_header() {
        let tempdir = tempfile::tempdir().unwrap();
        let state = SharedState::load(tempdir.path().join("config.json"))
            .await
            .unwrap();
        state
            .replace_config(AppConfig {
                allowed_tailnet_logins: vec!["parent@example.com".into()],
                ..AppConfig::default()
            })
            .await
            .unwrap();

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/device/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unlock_requires_session_token() {
        let tempdir = tempfile::tempdir().unwrap();
        let state = SharedState::load(tempdir.path().join("config.json"))
            .await
            .unwrap();

        let mut config = AppConfig {
            allowed_tailnet_logins: vec!["parent@example.com".into()],
            ..AppConfig::default()
        };
        config.set_pin("1234").unwrap();
        state.replace_config(config).await.unwrap();

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/device/unlock")
                    .header("content-type", "application/json")
                    .header("tailscale-user-login", "parent@example.com")
                    .body(Body::from(r#"{"durationMinutes":30}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
