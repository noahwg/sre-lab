use std::{env, net::SocketAddr};

use url::Url;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    /// Public base URL used to build `short_url` and to reject links pointing back at ourselves.
    pub base_url: Url,
    /// Required on the management API (`x-api-key` header).
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".into())
            .parse()
            .map_err(|e| format!("BIND_ADDR: {e}"))?;
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into())
            .parse()
            .map_err(|e| format!("BASE_URL: {e}"))?;
        let api_key = env::var("API_KEY").map_err(|_| "API_KEY is required".to_string())?;
        if api_key.len() < 16 {
            return Err("API_KEY must be at least 16 characters".into());
        }
        Ok(Self { bind_addr, base_url, api_key })
    }
}
