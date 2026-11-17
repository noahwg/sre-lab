//! Requests produce OpenTelemetry spans named by route template; operational endpoints don't.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider, SpanData};
use serde_json::json;
use shortener::{config::Config, router, store::MemoryStore, telemetry, AppState};
use tower::ServiceExt;
use tracing_subscriber::prelude::*;

const KEY: &str = "test-api-key-0123456789";

fn attr(span: &SpanData, key: &str) -> Option<String> {
    span.attributes
        .iter()
        .find(|kv| kv.key.as_str() == key)
        .map(|kv| kv.value.to_string())
}

#[tokio::test]
async fn requests_become_spans_and_ops_endpoints_do_not() {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    // Scoped to this test's thread (current_thread runtime), so tests don't interfere.
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(provider.tracer("test"))),
    );

    let recorder = telemetry::build_metrics();
    let app = router(AppState {
        store: Arc::new(MemoryStore::new()),
        config: Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            base_url: "https://s.example.org".parse().unwrap(),
            api_key: KEY.into(),
        }),
        metrics: recorder.handle(),
    });

    let create = Request::builder()
        .method(Method::POST)
        .uri("/api/links")
        .header("x-api-key", KEY)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"url": "https://example.com", "custom_code": "abc123"}).to_string()))
        .unwrap();
    assert_eq!(app.clone().oneshot(create).await.unwrap().status(), StatusCode::CREATED);

    let hit = Request::builder().uri("/abc123").body(Body::empty()).unwrap();
    assert_eq!(app.clone().oneshot(hit).await.unwrap().status(), StatusCode::FOUND);

    let missing = Request::builder().uri("/nope999").body(Body::empty()).unwrap();
    assert_eq!(app.clone().oneshot(missing).await.unwrap().status(), StatusCode::NOT_FOUND);

    for path in ["/healthz", "/readyz", "/metrics"] {
        let r = Request::builder().uri(path).body(Body::empty()).unwrap();
        app.clone().oneshot(r).await.unwrap();
    }

    provider.force_flush().unwrap();
    let spans = exporter.get_finished_spans().unwrap();
    let names: Vec<&str> = spans.iter().map(|s| s.name.as_ref()).collect();

    // Named by route template, never by the raw path (which would blow up cardinality).
    assert!(names.contains(&"POST /api/links"), "{names:?}");
    assert_eq!(names.iter().filter(|n| **n == "GET /{code}").count(), 2, "{names:?}");
    assert!(!names.iter().any(|n| n.contains("abc123") || n.contains("nope999")), "{names:?}");
    // Probes and scrapes are not traced.
    assert!(!names.iter().any(|n| n.contains("healthz") || n.contains("readyz") || n.contains("metrics")), "{names:?}");

    let created = spans.iter().find(|s| s.name == "POST /api/links").unwrap();
    assert_eq!(attr(created, "http.response.status_code").as_deref(), Some("201"));
    assert_eq!(attr(created, "http.route").as_deref(), Some("/api/links"));
    assert_eq!(attr(created, "http.request.method").as_deref(), Some("POST"));
}
