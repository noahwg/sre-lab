use std::{sync::Arc, time::Instant};

use axum::{
    extract::{MatchedPath, Path, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;

use crate::{
    codes,
    config::Config,
    error::AppError,
    store::{Link, LinkStore},
};

const MAX_CODE_ATTEMPTS: usize = 5;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn LinkStore>,
    pub config: Arc<Config>,
    pub metrics: PrometheusHandle,
}

pub fn router(state: AppState) -> Router {
    // Management API: requires the API key.
    let api = Router::new()
        .route("/api/links", post(create_link))
        .route("/api/links/{code}", get(link_details).delete(delete_link))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_api_key));

    // Public: the redirect itself and operational endpoints.
    let public = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_endpoint))
        .route("/{code}", get(redirect));

    Router::new()
        .merge(api)
        .merge(public)
        // route_layer so MatchedPath is available and metric labels stay low-cardinality.
        .route_layer(middleware::from_fn(track_metrics))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// ---------- middleware ----------

async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_time_eq(provided.as_bytes(), state.config.api_key.as_bytes()) {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(req).await)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn track_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());

    let res = next.run(req).await;

    let status = res.status().as_u16().to_string();
    counter!("http_requests_total", "method" => method.clone(), "route" => route.clone(), "status" => status)
        .increment(1);
    histogram!("http_request_duration_seconds", "method" => method, "route" => route)
        .record(start.elapsed().as_secs_f64());
    res
}

// ---------- operational ----------

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(s): State<AppState>) -> Result<&'static str, AppError> {
    if s.store.ping().await {
        Ok("ready")
    } else {
        Err(AppError::Unavailable)
    }
}

async fn metrics_endpoint(State(s): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        s.metrics.render(),
    )
}

// ---------- the redirect ----------

async fn redirect(State(s): State<AppState>, Path(code): Path<String>) -> Result<Response, AppError> {
    let Some(link) = s.store.get(&code).await else {
        counter!("redirects_total", "outcome" => "not_found").increment(1);
        return Err(AppError::NotFound);
    };
    if link.expires_at.is_some_and(|e| e <= Utc::now()) {
        counter!("redirects_total", "outcome" => "expired").increment(1);
        return Err(AppError::Gone);
    }
    let location = HeaderValue::from_str(&link.url).map_err(|_| AppError::Internal)?;
    s.store.record_hit(&code).await;
    counter!("redirects_total", "outcome" => "hit").increment(1);

    // 302, not 301: browsers must keep asking so clicks are counted and links can change.
    Ok((
        StatusCode::FOUND,
        [
            (header::LOCATION, location),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
    )
        .into_response())
}

// ---------- management API ----------

#[derive(Debug, Deserialize)]
struct CreateRequest {
    url: String,
    custom_code: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CreateResponse {
    code: String,
    short_url: String,
    url: String,
    expires_at: Option<DateTime<Utc>>,
}

async fn create_link(
    State(s): State<AppState>,
    Json(req): Json<CreateRequest>,
) -> Result<(StatusCode, Json<CreateResponse>), AppError> {
    let target = codes::validate_target(&req.url, s.config.base_url.host_str())
        .map_err(AppError::BadRequest)?;
    let expires_at = req.expires_at;
    if expires_at.is_some_and(|e| e <= Utc::now()) {
        return Err(AppError::BadRequest("expires_at must be in the future".into()));
    }

    let new_link = |code: String| Link {
        code,
        url: target.to_string(),
        created_at: Utc::now(),
        expires_at,
        clicks: 0,
    };

    let code = match req.custom_code {
        Some(code) => {
            codes::validate_custom(&code).map_err(AppError::BadRequest)?;
            s.store
                .insert(new_link(code.clone()))
                .await
                .map_err(|_| AppError::Conflict("custom_code is already in use".into()))?;
            code
        }
        None => {
            let mut chosen = None;
            for _ in 0..MAX_CODE_ATTEMPTS {
                let code = codes::generate();
                if s.store.insert(new_link(code.clone())).await.is_ok() {
                    chosen = Some(code);
                    break;
                }
            }
            chosen.ok_or(AppError::Internal)?
        }
    };

    counter!("links_created_total").increment(1);
    let short_url = s.config.base_url.join(&code).map_err(|_| AppError::Internal)?;
    Ok((
        StatusCode::CREATED,
        Json(CreateResponse {
            code,
            short_url: short_url.to_string(),
            url: target.to_string(),
            expires_at,
        }),
    ))
}

#[derive(Debug, Serialize)]
struct LinkDetails {
    code: String,
    url: String,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    clicks: u64,
}

async fn link_details(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<LinkDetails>, AppError> {
    let l = s.store.get(&code).await.ok_or(AppError::NotFound)?;
    Ok(Json(LinkDetails {
        code: l.code,
        url: l.url,
        created_at: l.created_at,
        expires_at: l.expires_at,
        clicks: l.clicks,
    }))
}

async fn delete_link(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<StatusCode, AppError> {
    if s.store.delete(&code).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
