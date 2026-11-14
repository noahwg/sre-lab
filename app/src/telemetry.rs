use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusRecorder};
use tracing_subscriber::{fmt, EnvFilter};

/// Structured logs to stdout. `LOG_FORMAT=json` for machines, anything else for humans.
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var("LOG_FORMAT").is_ok_and(|v| v == "json") {
        fmt().json().with_env_filter(filter).init();
    } else {
        fmt().with_env_filter(filter).init();
    }
}

/// Build (but don't install) the Prometheus recorder. `main` installs it globally;
/// tests keep their own so they don't fight over the global slot.
pub fn build_metrics() -> PrometheusRecorder {
    PrometheusBuilder::new()
        .set_buckets(&[0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5])
        .expect("valid histogram buckets")
        .build_recorder()
}
