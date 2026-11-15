//! `shortener migrate`: applies the SQL files in `app/migrations` (embedded in the binary
//! at compile time) to the database in `DATABASE_URL`, then exits.
//!
//! Run it as the owner role, never the app's runtime role. sqlx takes an advisory lock
//! and records a checksum per file in `_sqlx_migrations`, so it is safe to run on every
//! deploy, safe to run concurrently, and fails if an applied file was edited.

use std::time::Duration;

use sqlx::{migrate::Migrate, postgres::PgPoolOptions};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn run(database_url: &str) -> Result<(), String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .map_err(|e| format!("connecting to the database: {e}"))?;

    let mut conn = pool.acquire().await.map_err(|e| e.to_string())?;
    conn.ensure_migrations_table().await.map_err(|e| e.to_string())?;
    let before = conn.list_applied_migrations().await.map_err(|e| e.to_string())?.len();
    drop(conn);

    MIGRATOR.run(&pool).await.map_err(|e| format!("running migrations: {e}"))?;

    let total = MIGRATOR.iter().count();
    // Plain line on stdout: Ansible greps it to decide whether anything changed.
    println!("migrate: applied {} new, {} total", total.saturating_sub(before), total);
    Ok(())
}
