use std::{process::ExitCode, sync::Arc, time::Duration};

use shortener::{config::Config, router, store::MemoryStore, telemetry, AppState};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> ExitCode {
    // The runtime image is distroless (no shell, no curl), so the container
    // healthcheck calls back into this binary.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return healthcheck().await;
    }

    telemetry::init_logging();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("configuration error: {e}");
            return ExitCode::from(2);
        }
    };

    let recorder = telemetry::build_metrics();
    let metrics = recorder.handle();
    if metrics::set_global_recorder(recorder).is_err() {
        error!("metrics recorder was already installed");
        return ExitCode::FAILURE;
    }

    let bind_addr = config.bind_addr;
    let state = AppState {
        // TODO: swap for the Postgres-backed store once the database exists.
        store: Arc::new(MemoryStore::new()),
        config: Arc::new(config),
        metrics,
    };

    let listener = match TcpListener::bind(bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("failed to bind {bind_addr}: {e}");
            return ExitCode::FAILURE;
        }
    };
    info!(%bind_addr, "listening");

    if let Err(e) = axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!("server error: {e}");
        return ExitCode::FAILURE;
    }
    info!("shut down cleanly");
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}

/// Minimal `GET /healthz` against ourselves. Exit 0 if it answers 200.
async fn healthcheck() -> ExitCode {
    let port = std::env::var("BIND_ADDR")
        .ok()
        .and_then(|a| a.parse::<std::net::SocketAddr>().ok())
        .map(|a| a.port())
        .unwrap_or(8080);

    let probe = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.ok()?;
        stream
            .write_all(b"GET /healthz HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .await
            .ok()?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.ok()?;
        let head = String::from_utf8_lossy(&buf);
        head.lines().next().filter(|l| l.contains(" 200 ")).map(|_| ())
    };

    match timeout(Duration::from_secs(2), probe).await {
        Ok(Some(())) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
