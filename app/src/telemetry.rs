use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusRecorder};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{
    trace::{Sampler, SdkTracerProvider},
    Resource,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Flushes buffered spans when dropped, so keep it alive for the life of `main`.
pub struct TelemetryGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            let _ = provider.shutdown();
        }
    }
}

/// Structured logs to stdout (`LOG_FORMAT=json` for machines, anything else for humans),
/// plus trace export over OTLP/HTTP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set.
///
/// Tracing never gets in the way of serving: spans are batched and exported on a background
/// thread, and are dropped if the collector is slow or down.
pub fn init_logging() -> TelemetryGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = if std::env::var("LOG_FORMAT").is_ok_and(|v| v == "json") {
        fmt::layer().json().boxed()
    } else {
        fmt::layer().boxed()
    };

    let provider = build_tracer_provider();
    let otel_layer = provider
        .as_ref()
        .map(|p| tracing_opentelemetry::layer().with_tracer(p.tracer("shortener")));

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .with(filter)
        .init();

    TelemetryGuard { provider }
}

/// `None` (tracing off) unless an OTLP endpoint is configured. Standard OTel variables apply:
/// `OTEL_EXPORTER_OTLP_ENDPOINT` (base URL; `/v1/traces` is appended),
/// `OTEL_TRACES_SAMPLER_ARG` (fraction of new traces to keep, default 1.0).
fn build_tracer_provider() -> Option<SdkTracerProvider> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())?;

    let exporter = match opentelemetry_otlp::SpanExporter::builder().with_http().build() {
        Ok(e) => e,
        Err(e) => {
            // The logger isn't installed yet.
            eprintln!("tracing disabled: could not build the OTLP exporter: {e}");
            return None;
        }
    };

    Some(
        SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                sample_ratio(),
            ))))
            .with_resource(Resource::builder().with_service_name("shortener").build())
            .build(),
    )
}

fn sample_ratio() -> f64 {
    std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|r| (0.0..=1.0).contains(r))
        .unwrap_or(1.0)
}

/// Build (but don't install) the Prometheus recorder. `main` installs it globally;
/// tests keep their own so they don't fight over the global slot.
pub fn build_metrics() -> PrometheusRecorder {
    PrometheusBuilder::new()
        .set_buckets(&[0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5])
        .expect("valid histogram buckets")
        .build_recorder()
}
