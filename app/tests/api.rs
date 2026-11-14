use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use shortener::{
    config::Config,
    router,
    store::{Link, LinkStore, MemoryStore},
    telemetry, AppState,
};
use tower::ServiceExt;

const KEY: &str = "test-api-key-0123456789";

fn app() -> (Router, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::new());
    let recorder = telemetry::build_metrics();
    let state = AppState {
        store: store.clone(),
        config: Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url: "https://s.example.org".parse().unwrap(),
            api_key: KEY.into(),
        }),
        metrics: recorder.handle(),
    };
    (router(state), store)
}

fn req(method: Method, uri: &str, key: bool, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if key {
        b = b.header("x-api-key", KEY);
    }
    match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn json_body(res: axum::response::Response) -> Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn create_redirect_details_delete_roundtrip() {
    let (app, _) = app();

    let res = app
        .clone()
        .oneshot(req(Method::POST, "/api/links", true, Some(json!({"url": "https://example.com/page?a=1"}))))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = json_body(res).await;
    let code = created["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 7);
    assert_eq!(created["short_url"], format!("https://s.example.org/{code}"));

    // Redirect is public, 302, and not cacheable.
    let res = app.clone().oneshot(req(Method::GET, &format!("/{code}"), false, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(res.headers()[header::LOCATION], "https://example.com/page?a=1");
    assert_eq!(res.headers()[header::CACHE_CONTROL], "no-store");

    // The hit was counted.
    let res = app.clone().oneshot(req(Method::GET, &format!("/api/links/{code}"), true, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(json_body(res).await["clicks"], 1);

    let res = app.clone().oneshot(req(Method::DELETE, &format!("/api/links/{code}"), true, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let res = app.oneshot(req(Method::GET, &format!("/{code}"), false, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn management_api_requires_key() {
    let (app, _) = app();
    for (m, uri) in [(Method::POST, "/api/links"), (Method::GET, "/api/links/abc"), (Method::DELETE, "/api/links/abc")] {
        let res = app
            .clone()
            .oneshot(req(m, uri, false, Some(json!({"url": "https://example.com"}))))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn rejects_bad_input() {
    let (app, _) = app();
    for body in [
        json!({"url": "ftp://example.com"}),
        json!({"url": "https://s.example.org/loop"}),
        json!({"url": "https://example.com", "custom_code": "api"}),
        json!({"url": "https://example.com", "custom_code": "x"}),
        json!({"url": "https://example.com", "expires_at": (Utc::now() - Duration::hours(1))}),
    ] {
        let res = app.clone().oneshot(req(Method::POST, "/api/links", true, Some(body.clone()))).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{body}");
    }
}

#[tokio::test]
async fn custom_code_conflicts() {
    let (app, _) = app();
    let body = json!({"url": "https://example.com", "custom_code": "my-link"});
    let first = app.clone().oneshot(req(Method::POST, "/api/links", true, Some(body.clone()))).await.unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);
    let second = app.oneshot(req(Method::POST, "/api/links", true, Some(body))).await.unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn expired_link_is_gone() {
    let (app, store) = app();
    store
        .insert(Link {
            code: "old".into(),
            url: "https://example.com/".into(),
            created_at: Utc::now() - Duration::days(2),
            expires_at: Some(Utc::now() - Duration::days(1)),
            clicks: 0,
        })
        .await
        .unwrap();
    let res = app.oneshot(req(Method::GET, "/old", false, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::GONE);
}

#[tokio::test]
async fn operational_endpoints() {
    let (app, _) = app();
    for uri in ["/healthz", "/readyz", "/metrics"] {
        let res = app.clone().oneshot(req(Method::GET, uri, false, None)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
    }
}
