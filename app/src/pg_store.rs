//! Postgres-backed `LinkStore`. The schema comes from `migrations/`, applied by
//! `shortener migrate` before the app starts; the app's own role can read and write
//! rows but has no DDL rights.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use metrics::counter;
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    Row,
};
use tokio::{sync::Semaphore, time::timeout};
use tracing::{error, field::Empty, info_span, warn, Instrument, Span};

use crate::store::{Link, LinkStore, StoreError};

/// SQLSTATE for a unique constraint violation.
const UNIQUE_VIOLATION: &str = "23505";
/// Upper bound on click updates in flight. Beyond it, clicks are dropped (and counted)
/// rather than letting a slow database pile up tasks on the redirect path.
const MAX_PENDING_HITS: usize = 32;

#[derive(Clone)]
pub struct PgStore {
    pool: PgPool,
    pending_hits: Arc<Semaphore>,
}

impl PgStore {
    /// Builds the pool without connecting, so startup doesn't depend on the database.
    pub fn connect_lazy(url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect_lazy(url)
            .map_err(|e| e.to_string())?;
        Ok(Self::from_pool(pool))
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool, pending_hits: Arc::new(Semaphore::new(MAX_PENDING_HITS)) }
    }
}

fn unavailable(op: &str, e: sqlx::Error) -> StoreError {
    error!(op, error = %e, "database error");
    // Mark the surrounding db.* span as failed in the trace.
    Span::current().record("otel.status_code", "ERROR");
    counter!("store_errors_total", "op" => op.to_owned()).increment(1);
    StoreError::Unavailable
}

#[async_trait]
impl LinkStore for PgStore {
    #[tracing::instrument(name = "db.insert", skip_all, fields(db.system = "postgresql", db.operation.name = "INSERT", link.code = %link.code, otel.status_code = Empty))]
    async fn insert(&self, link: Link) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO links (code, url, created_at, expires_at, clicks) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&link.code)
        .bind(&link.url)
        .bind(link.created_at)
        .bind(link.expires_at)
        .bind(link.clicks as i64)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.code().as_deref() == Some(UNIQUE_VIOLATION) => {
                StoreError::Conflict
            }
            _ => unavailable("insert", e),
        })
    }

    #[tracing::instrument(name = "db.get", skip(self), fields(db.system = "postgresql", db.operation.name = "SELECT", link.code = %code, otel.status_code = Empty))]
    async fn get(&self, code: &str) -> Result<Option<Link>, StoreError> {
        let row = sqlx::query(
            "SELECT code, url, created_at, expires_at, clicks FROM links WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| unavailable("get", e))?;

        row.map(|r| {
            Ok(Link {
                code: r.try_get("code")?,
                url: r.try_get("url")?,
                created_at: r.try_get("created_at")?,
                expires_at: r.try_get("expires_at")?,
                clicks: r.try_get::<i64, _>("clicks")?.max(0) as u64,
            })
        })
        .transpose()
        .map_err(|e: sqlx::Error| unavailable("get", e))
    }

    /// Counting a click never delays the redirect: the update runs in the background.
    async fn record_hit(&self, code: &str) {
        let Ok(permit) = self.pending_hits.clone().try_acquire_owned() else {
            counter!("link_hits_dropped_total").increment(1);
            return;
        };
        let pool = self.pool.clone();
        let code = code.to_owned();
        // A child of the request span that may outlive it (the redirect doesn't wait for it).
        let span = info_span!(
            "db.record_hit",
            db.system = "postgresql",
            db.operation.name = "UPDATE",
            link.code = %code,
            otel.status_code = Empty,
        );
        tokio::spawn(
            async move {
                let _permit = permit;
                if let Err(e) = sqlx::query("UPDATE links SET clicks = clicks + 1 WHERE code = $1")
                    .bind(&code)
                    .execute(&pool)
                    .await
                {
                    counter!("link_hits_failed_total").increment(1);
                    Span::current().record("otel.status_code", "ERROR");
                    warn!(error = %e, "could not record click");
                }
            }
            .instrument(span),
        );
    }

    #[tracing::instrument(name = "db.delete", skip(self), fields(db.system = "postgresql", db.operation.name = "DELETE", link.code = %code, otel.status_code = Empty))]
    async fn delete(&self, code: &str) -> Result<bool, StoreError> {
        sqlx::query("DELETE FROM links WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .map_err(|e| unavailable("delete", e))
    }

    /// Ready only if the database answers and the app role can read the table, which
    /// also catches "migrations not applied" and "wrong grants".
    async fn ping(&self) -> bool {
        let q = sqlx::query("SELECT 1 FROM links LIMIT 1").fetch_optional(&self.pool);
        matches!(timeout(Duration::from_secs(2), q).await, Ok(Ok(_)))
    }
}
