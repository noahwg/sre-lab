use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Link {
    pub code: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub clicks: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    /// The code is already taken.
    Conflict,
}

/// Storage boundary. `MemoryStore` backs it for now; a Postgres implementation
/// replaces it without touching the handlers.
#[async_trait]
pub trait LinkStore: Send + Sync {
    async fn insert(&self, link: Link) -> Result<(), StoreError>;
    async fn get(&self, code: &str) -> Option<Link>;
    /// Count one redirect. Fire-and-forget from the caller's point of view.
    async fn record_hit(&self, code: &str);
    async fn delete(&self, code: &str) -> bool;
    /// Readiness probe: can the store serve requests?
    async fn ping(&self) -> bool;
}

#[derive(Default)]
pub struct MemoryStore {
    links: RwLock<HashMap<String, Link>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl LinkStore for MemoryStore {
    async fn insert(&self, link: Link) -> Result<(), StoreError> {
        let mut links = self.links.write().expect("store lock poisoned");
        if links.contains_key(&link.code) {
            return Err(StoreError::Conflict);
        }
        links.insert(link.code.clone(), link);
        Ok(())
    }

    async fn get(&self, code: &str) -> Option<Link> {
        self.links.read().expect("store lock poisoned").get(code).cloned()
    }

    async fn record_hit(&self, code: &str) {
        if let Some(l) = self.links.write().expect("store lock poisoned").get_mut(code) {
            l.clicks += 1;
        }
    }

    async fn delete(&self, code: &str) -> bool {
        self.links.write().expect("store lock poisoned").remove(code).is_some()
    }

    async fn ping(&self) -> bool {
        true
    }
}
