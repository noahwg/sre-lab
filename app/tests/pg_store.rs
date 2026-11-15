//! Runs against a real Postgres with the migrations applied. Skipped unless
//! `DATABASE_URL` is set (CI sets it; locally: see docs/app.md).

use std::time::Duration;

use chrono::Utc;
use shortener::{
    codes,
    pg_store::PgStore,
    store::{Link, LinkStore, StoreError},
};

fn store() -> Option<PgStore> {
    let url = std::env::var("DATABASE_URL").ok()?;
    Some(PgStore::connect_lazy(&url).expect("valid DATABASE_URL"))
}

fn link(code: &str) -> Link {
    Link {
        code: code.into(),
        url: "https://example.com/a".into(),
        created_at: Utc::now(),
        expires_at: None,
        clicks: 0,
    }
}

#[tokio::test]
async fn insert_get_delete_round_trip() {
    let Some(s) = store() else { return };
    let code = codes::generate();

    assert!(s.get(&code).await.unwrap().is_none());
    s.insert(link(&code)).await.unwrap();
    let got = s.get(&code).await.unwrap().expect("link exists");
    assert_eq!(got.url, "https://example.com/a");
    assert_eq!(got.clicks, 0);

    assert!(s.delete(&code).await.unwrap());
    assert!(!s.delete(&code).await.unwrap());
    assert!(s.get(&code).await.unwrap().is_none());
}

#[tokio::test]
async fn duplicate_code_is_a_conflict() {
    let Some(s) = store() else { return };
    let code = codes::generate();
    s.insert(link(&code)).await.unwrap();
    assert_eq!(s.insert(link(&code)).await, Err(StoreError::Conflict));
    s.delete(&code).await.unwrap();
}

#[tokio::test]
async fn hits_are_counted_in_the_background() {
    let Some(s) = store() else { return };
    let code = codes::generate();
    s.insert(link(&code)).await.unwrap();

    s.record_hit(&code).await;
    s.record_hit(&code).await;

    let mut clicks = 0;
    for _ in 0..40 {
        clicks = s.get(&code).await.unwrap().unwrap().clicks;
        if clicks == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(clicks, 2);
    s.delete(&code).await.unwrap();
}

#[tokio::test]
async fn ping_reflects_the_schema() {
    let Some(s) = store() else { return };
    assert!(s.ping().await);
}
