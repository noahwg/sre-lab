use rand::{distributions::Alphanumeric, Rng};
use url::Url;

pub const CODE_LEN: usize = 7;
pub const MAX_URL_LEN: usize = 2048;
const RESERVED: &[&str] = &["api", "healthz", "readyz", "metrics"];

/// Random base62 code. 62^7 is ~3.5 trillion combinations; callers retry on collision.
pub fn generate() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(CODE_LEN)
        .map(char::from)
        .collect()
}

pub fn validate_custom(code: &str) -> Result<(), String> {
    if !(3..=32).contains(&code.len()) {
        return Err("custom_code must be 3-32 characters".into());
    }
    if !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("custom_code may only contain letters, digits, '-' and '_'".into());
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(code)) {
        return Err("custom_code is reserved".into());
    }
    Ok(())
}

/// Accept only absolute http(s) URLs that don't embed credentials or point back at us.
pub fn validate_target(raw: &str, own_host: Option<&str>) -> Result<Url, String> {
    if raw.len() > MAX_URL_LEN {
        return Err(format!("url must be at most {MAX_URL_LEN} characters"));
    }
    let url = Url::parse(raw).map_err(|_| "url is not valid".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("url must start with http:// or https://".into());
    }
    let host = url.host_str().ok_or_else(|| "url must include a host".to_string())?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err("url must not contain credentials".into());
    }
    if own_host.is_some_and(|h| h.eq_ignore_ascii_case(host)) {
        return Err("url must not point at this service".into());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_base62_and_fixed_length() {
        let c = generate();
        assert_eq!(c.len(), CODE_LEN);
        assert!(c.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn custom_code_rules() {
        assert!(validate_custom("my-link_1").is_ok());
        assert!(validate_custom("ab").is_err());
        assert!(validate_custom("has space").is_err());
        assert!(validate_custom("API").is_err());
        assert!(validate_custom("metrics").is_err());
    }

    #[test]
    fn target_rules() {
        assert!(validate_target("https://example.com/a?b=1", Some("s.example.org")).is_ok());
        assert!(validate_target("ftp://example.com", None).is_err());
        assert!(validate_target("javascript:alert(1)", None).is_err());
        assert!(validate_target("not a url", None).is_err());
        assert!(validate_target("https://user:pw@example.com", None).is_err());
        assert!(validate_target("https://S.example.org/x", Some("s.example.org")).is_err());
    }
}
