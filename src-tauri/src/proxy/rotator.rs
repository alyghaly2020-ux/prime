//! # Proxy Rotator
//!
//! A thread-safe round-robin proxy rotation module with failure tracking,
//! automatic deactivation, and env-var-based configuration.
//!
//! Supports building [`reqwest::Client`] instances pre-configured with a
//! proxy URL for HTTP/HTTPS proxying.

use parking_lot::RwLock;
use std::time::Duration;

// =============================================================================
// ProxyEntry
// =============================================================================

/// A single proxy entry tracked by the rotator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyEntry {
    /// Proxy URL (e.g. `http://gate.smartproxy.com:7000`)
    pub url: String,
    /// Optional username for authenticated proxies.
    pub username: Option<String>,
    /// Optional password for authenticated proxies.
    pub password: Option<String>,
    /// Optional geographic region tag.
    pub region: Option<String>,
    /// Timestamp of the last time this proxy was used.
    pub last_used: Option<String>,
    /// Consecutive failure count.
    pub fail_count: u32,
    /// Total successful requests through this proxy.
    pub success_count: u32,
    /// Whether the proxy is still considered active.
    pub is_active: bool,
}

// =============================================================================
// ProxyRotator
// =============================================================================

/// A thread-safe round-robin proxy rotator.
///
/// Proxies are stored in a [`Vec`] behind a [`RwLock`]. The rotator advances
/// through them in round-robin fashion, skipping inactive entries. Proxies
/// are automatically deactivated after `max_fails` consecutive failures.
#[derive(Debug)]
pub struct ProxyRotator {
    proxies: RwLock<Vec<ProxyEntry>>,
    current_index: RwLock<usize>,
    rotation_interval: RwLock<Duration>,
    max_fails: RwLock<u32>,
}

impl ProxyRotator {
    /// Create a new `ProxyRotator` with default settings.
    ///
    /// Defaults:
    /// - `rotation_interval`: 60 seconds
    /// - `max_fails`: 3
    pub fn new() -> Self {
        Self {
            proxies: RwLock::new(Vec::new()),
            current_index: RwLock::new(0),
            rotation_interval: RwLock::new(Duration::from_secs(60)),
            max_fails: RwLock::new(3),
        }
    }

    /// Add a new proxy entry.
    ///
    /// If `region` is provided it can be used for geographic filtering.
    /// Duplicate URLs are allowed (callers should check via `list_proxies`
    /// if uniqueness is desired).
    pub fn add_proxy(
        &self,
        url: impl Into<String>,
        username: Option<String>,
        password: Option<String>,
        region: Option<String>,
    ) {
        let entry = ProxyEntry {
            url: url.into(),
            username,
            password,
            region,
            last_used: None,
            fail_count: 0,
            success_count: 0,
            is_active: true,
        };
        self.proxies.write().push(entry);
    }

    /// Remove a proxy by its URL. Returns `true` if a proxy was removed.
    pub fn remove_proxy(&self, url: &str) -> bool {
        let mut proxies = self.proxies.write();
        let len_before = proxies.len();
        proxies.retain(|p| p.url != url);
        proxies.len() != len_before
    }

    /// Get the next active proxy URL in round-robin order.
    ///
    /// Returns the proxy at the current index, then advances the internal
    /// pointer so the next call returns the following active proxy.
    /// Skips proxies whose `is_active` is `false`. If no active proxies
    /// remain, returns `None`.
    pub fn next(&self) -> Option<String> {
        // Find the next active index while holding only the read lock.
        let next_idx = {
            let proxies = self.proxies.read();
            if proxies.is_empty() {
                return None;
            }

            let start = *self.current_index.read();
            let len = proxies.len();

            let mut found = None;
            for offset in 0..len {
                let candidate = (start + offset) % len;
                if proxies[candidate].is_active {
                    found = Some(candidate);
                    break;
                }
            }
            found
        };

        let idx = next_idx?;

        // Advance current_index to the next slot for the following call.
        let len = self.proxies.read().len();
        *self.current_index.write() = (idx + 1) % len;

        let mut proxies = self.proxies.write();
        proxies[idx].last_used = Some("now".to_string());
        Some(proxies[idx].url.clone())
    }

    /// Get the current proxy URL without advancing the index.
    pub fn current(&self) -> Option<String> {
        let proxies = self.proxies.read();
        let idx = *self.current_index.read();
        if proxies.is_empty() || idx >= proxies.len() {
            return None;
        }
        Some(proxies[idx].url.clone())
    }

    /// Mark a proxy as successfully used.
    ///
    /// Increments `success_count` and resets `fail_count` to 0.
    /// Reactivates the proxy if it was previously deactivated.
    pub fn mark_success(&self, url: &str) {
        let mut proxies = self.proxies.write();
        if let Some(entry) = proxies.iter_mut().find(|p| p.url == url) {
            entry.success_count = entry.success_count.saturating_add(1);
            entry.fail_count = 0;
            entry.is_active = true;
            entry.last_used = Some("now".to_string());
        }
    }

    /// Mark a proxy as failed.
    ///
    /// Increments `fail_count`. If `fail_count` reaches `max_fails`,
    /// the proxy is deactivated (`is_active = false`).
    pub fn mark_failed(&self, url: &str) {
        let mut proxies = self.proxies.write();
        let max = *self.max_fails.read();
        if let Some(entry) = proxies.iter_mut().find(|p| p.url == url) {
            entry.fail_count = entry.fail_count.saturating_add(1);
            if entry.fail_count >= max {
                entry.is_active = false;
            }
        }
    }

    /// Return the number of currently active proxies.
    pub fn active_count(&self) -> usize {
        self.proxies
            .read()
            .iter()
            .filter(|p| p.is_active)
            .count()
    }

    /// Change the rotation interval (in seconds).
    pub fn set_rotation_interval(&self, secs: u64) {
        *self.rotation_interval.write() = Duration::from_secs(secs);
    }

    /// Return a cloned copy of all proxy entries (for inspection).
    pub fn list_proxies(&self) -> Vec<ProxyEntry> {
        self.proxies.read().clone()
    }

    /// Load proxies from the `PROXY_LIST` environment variable.
    ///
    /// The variable should contain comma-separated proxy URLs:
    ///
    /// ```text
    /// PROXY_LIST=http://user1:pass1@host1:port1,http://user2:pass2@host2:port2
    /// ```
    ///
    /// Entries with the format `protocol://user:pass@host:port` are parsed
    /// and added with `username` and `password` extracted from the URL.
    /// Plain URLs are added as-is.
    pub fn load_from_env(&self) {
        let Ok(val) = std::env::var("PROXY_LIST") else {
            return;
        };

        for raw in val.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Attempt to parse auth credentials from the proxy URL.
            let (url, username, password) = Self::parse_proxy_url(raw);
            self.add_proxy(url, username, password, None);
        }
    }

    /// Build a [`reqwest::Client`] configured with the given proxy URL.
    ///
    /// The proxy URL is parsed and used as an HTTP/HTTPS proxy. A timeout
    /// of `timeout_secs` seconds is applied to all requests.
    ///
    /// # Errors
    ///
    /// Returns an error if the proxy URL is malformed or if reqwest fails
    /// to build the client.
    pub fn build_client(
        url: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<reqwest::Client> {
        let proxy = reqwest::Proxy::all(url)?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;
        Ok(client)
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Parse a proxy URL of the form `protocol://user:pass@host:port` and
    /// return a triple of (clean_url, username, password).
    ///
    /// If the URL does not contain credentials, the original URL is returned
    /// with `None` for both username and password.
    fn parse_proxy_url(raw: &str) -> (String, Option<String>, Option<String>) {
        // Quick check: does the URL contain a '@'?
        let at_pos = match raw.find('@') {
            Some(p) => p,
            None => return (raw.to_string(), None, None),
        };

        // Find the protocol separator to ensure '@' is not part of the scheme.
        let scheme_end = raw.find("://").map(|p| p + 3).unwrap_or(0);
        if at_pos < scheme_end {
            return (raw.to_string(), None, None);
        }

        let userinfo = &raw[scheme_end..at_pos];
        let host_part = &raw[at_pos + 1..];
        let clean_url = format!("{}://{}", &raw[..scheme_end.saturating_sub(3)], host_part);

        let (user, pass) = match userinfo.split_once(':') {
            Some((u, p)) => (Some(u.to_string()), Some(p.to_string())),
            None => (Some(userinfo.to_string()), None),
        };

        (clean_url, user, pass)
    }
}

impl Default for ProxyRotator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a rotator with two test proxies.
    fn test_rotator() -> ProxyRotator {
        let r = ProxyRotator::new();
        r.add_proxy("http://proxy1:8080", None, None, None);
        r.add_proxy("http://proxy2:8080", None, None, None);
        r
    }

    // -----------------------------------------------------------------
    // add / remove / next rotation
    // -----------------------------------------------------------------

    #[test]
    fn test_add_proxy() {
        let r = ProxyRotator::new();
        assert_eq!(r.list_proxies().len(), 0);
        r.add_proxy("http://p1:8080", None, None, None);
        assert_eq!(r.list_proxies().len(), 1);
        assert_eq!(r.list_proxies()[0].url, "http://p1:8080");
    }

    #[test]
    fn test_remove_proxy_exists() {
        let r = test_rotator();
        assert!(r.remove_proxy("http://proxy1:8080"));
        assert_eq!(r.list_proxies().len(), 1);
    }

    #[test]
    fn test_remove_proxy_not_found() {
        let r = test_rotator();
        assert!(!r.remove_proxy("http://nonexistent:9999"));
        assert_eq!(r.list_proxies().len(), 2);
    }

    #[test]
    fn test_next_round_robin() {
        let r = test_rotator();
        let first = r.next().expect("should have first proxy");
        let second = r.next().expect("should have second proxy");
        let third = r.next().expect("should wrap around");
        assert_eq!(first, "http://proxy1:8080");
        assert_eq!(second, "http://proxy2:8080");
        assert_eq!(third, "http://proxy1:8080");
    }

    #[test]
    fn test_next_empty_rotator() {
        let r = ProxyRotator::new();
        assert!(r.next().is_none());
    }

    #[test]
    fn test_next_skips_inactive() {
        let r = test_rotator();
        // Deactivate proxy1 — should always return proxy2.
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080"); // 3rd fail → deactivated

        let first = r.next().expect("should get proxy2");
        let second = r.next().expect("should get proxy2 again");
        assert_eq!(first, "http://proxy2:8080");
        assert_eq!(second, "http://proxy2:8080");
    }

    #[test]
    fn test_next_all_inactive_returns_none() {
        let r = test_rotator();
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy2:8080");
        r.mark_failed("http://proxy2:8080");
        r.mark_failed("http://proxy2:8080");
        assert!(r.next().is_none());
    }

    #[test]
    fn test_current_after_next() {
        let r = test_rotator();
        // Before any next() call, current() returns the proxy at index 0.
        assert_eq!(r.current().unwrap(), "http://proxy1:8080");
        // next() returns proxy1, then advances current_index to 1.
        assert_eq!(r.next().unwrap(), "http://proxy1:8080");
        // current() now points at index 1 → proxy2.
        assert_eq!(r.current().unwrap(), "http://proxy2:8080");
        // next() returns proxy2, advances current_index to 0.
        assert_eq!(r.next().unwrap(), "http://proxy2:8080");
        // current() now points at index 0 → proxy1.
        assert_eq!(r.current().unwrap(), "http://proxy1:8080");
    }

    // -----------------------------------------------------------------
    // mark_success / mark_failed / deactivation
    // -----------------------------------------------------------------

    #[test]
    fn test_mark_success_resets_fails() {
        let r = test_rotator();
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        assert_eq!(r.list_proxies()[0].fail_count, 2);
        r.mark_success("http://proxy1:8080");
        let entry = &r.list_proxies()[0];
        assert_eq!(entry.fail_count, 0);
        assert_eq!(entry.success_count, 1);
        assert!(entry.is_active);
    }

    #[test]
    fn test_mark_failed_deactivates_at_max() {
        let r = test_rotator();
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        assert!(r.list_proxies()[0].is_active);
        r.mark_failed("http://proxy1:8080"); // 3rd fail → deactivated
        assert!(!r.list_proxies()[0].is_active);
        assert_eq!(r.list_proxies()[0].fail_count, 3);
    }

    #[test]
    fn test_mark_failed_below_max_stays_active() {
        let r = test_rotator();
        r.mark_failed("http://proxy1:8080");
        assert!(r.list_proxies()[0].is_active);
    }

    #[test]
    fn test_mark_success_reactivates() {
        let r = test_rotator();
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080"); // deactivated
        assert!(!r.list_proxies()[0].is_active);
        r.mark_success("http://proxy1:8080"); // reactivates
        assert!(r.list_proxies()[0].is_active);
    }

    #[test]
    fn test_mark_nonexistent_proxy_is_noop() {
        let r = test_rotator();
        r.mark_success("http://ghost:9999");
        r.mark_failed("http://ghost:9999");
        assert_eq!(r.list_proxies().len(), 2); // no new entry added
    }

    // -----------------------------------------------------------------
    // active_count
    // -----------------------------------------------------------------

    #[test]
    fn test_active_count() {
        let r = test_rotator();
        assert_eq!(r.active_count(), 2);
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        r.mark_failed("http://proxy1:8080");
        assert_eq!(r.active_count(), 1);
        r.mark_failed("http://proxy2:8080");
        r.mark_failed("http://proxy2:8080");
        r.mark_failed("http://proxy2:8080");
        assert_eq!(r.active_count(), 0);
    }

    // -----------------------------------------------------------------
    // set_rotation_interval
    // -----------------------------------------------------------------

    #[test]
    fn test_set_rotation_interval() {
        let r = ProxyRotator::new();
        assert_eq!(*r.rotation_interval.read(), Duration::from_secs(60));
        r.set_rotation_interval(120);
        assert_eq!(*r.rotation_interval.read(), Duration::from_secs(120));
    }

    // -----------------------------------------------------------------
    // load_from_env
    // -----------------------------------------------------------------

    #[test]
    fn test_load_from_env_empty() {
        // Ensure the env var is not set, then verify no proxies loaded.
        std::env::remove_var("PROXY_LIST");
        let r = ProxyRotator::new();
        r.load_from_env();
        assert_eq!(r.list_proxies().len(), 0);
    }

    #[test]
    fn test_load_from_env_with_values() {
        let var = "http://user1:pass1@host1:80,http://host2:8080";
        std::env::set_var("PROXY_LIST", var);
        let r = ProxyRotator::new();
        r.load_from_env();
        std::env::remove_var("PROXY_LIST");

        let proxies = r.list_proxies();
        assert_eq!(proxies.len(), 2);

        // First proxy has auth parsed out.
        assert_eq!(proxies[0].url, "http://host1:80");
        assert_eq!(proxies[0].username.as_deref(), Some("user1"));
        assert_eq!(proxies[0].password.as_deref(), Some("pass1"));

        // Second proxy is plain.
        assert_eq!(proxies[1].url, "http://host2:8080");
        assert!(proxies[1].username.is_none());
        assert!(proxies[1].password.is_none());
    }

    #[test]
    fn test_load_from_env_ignores_empty_segments() {
        std::env::set_var("PROXY_LIST", "http://a:1,,http://b:2,");
        let r = ProxyRotator::new();
        r.load_from_env();
        std::env::remove_var("PROXY_LIST");
        assert_eq!(r.list_proxies().len(), 2);
    }

    // -----------------------------------------------------------------
    // build_client
    // -----------------------------------------------------------------

    #[test]
    fn test_build_client_ok() {
        let client = ProxyRotator::build_client("http://localhost:8888", 30);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_client_bad_url() {
        let client = ProxyRotator::build_client("\0invalid", 30);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_client_zero_timeout() {
        // A timeout of 0 is technically valid (instant timeout).
        let client = ProxyRotator::build_client("http://localhost:8888", 0);
        assert!(client.is_ok());
    }

    // -----------------------------------------------------------------
    // ProxyEntry defaults / list_proxies
    // -----------------------------------------------------------------

    #[test]
    fn test_list_proxies_returns_all_fields() {
        let r = ProxyRotator::new();
        r.add_proxy(
            "http://p1:3128",
            Some("alice".into()),
            Some("secret".into()),
            Some("us-east".into()),
        );
        let entries = r.list_proxies();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.url, "http://p1:3128");
        assert_eq!(e.username.as_deref(), Some("alice"));
        assert_eq!(e.password.as_deref(), Some("secret"));
        assert_eq!(e.region.as_deref(), Some("us-east"));
        assert!(e.last_used.is_none());
        assert_eq!(e.fail_count, 0);
        assert_eq!(e.success_count, 0);
        assert!(e.is_active);
    }

    #[test]
    fn test_list_proxies_does_not_block_mutation() {
        let r = ProxyRotator::new();
        r.add_proxy("http://p1:8080", None, None, None);
        let _snapshot = r.list_proxies();
        // Mutations should still work while snapshot is alive.
        r.add_proxy("http://p2:8080", None, None, None);
        assert_eq!(r.list_proxies().len(), 2);
    }

    // -----------------------------------------------------------------
    // parse_proxy_url helper
    // -----------------------------------------------------------------

    #[test]
    fn test_parse_proxy_url_with_auth() {
        let (url, user, pass) = ProxyRotator::parse_proxy_url("http://u:p@host:80");
        assert_eq!(url, "http://host:80");
        assert_eq!(user.as_deref(), Some("u"));
        assert_eq!(pass.as_deref(), Some("p"));
    }

    #[test]
    fn test_parse_proxy_url_without_auth() {
        let (url, user, pass) = ProxyRotator::parse_proxy_url("http://host:80");
        assert_eq!(url, "http://host:80");
        assert!(user.is_none());
        assert!(pass.is_none());
    }

    #[test]
    fn test_parse_proxy_url_with_at_in_scheme() {
        // URL with user:password in authority: http://host:80@foo
        let (url, user, pass) = ProxyRotator::parse_proxy_url("http://host:80@foo");
        assert_eq!(url, "http://foo");
        assert_eq!(user.as_deref(), Some("host"));
        assert_eq!(pass.as_deref(), Some("80"));
    }

    // -----------------------------------------------------------------
    // Default trait
    // -----------------------------------------------------------------

    #[test]
    fn test_default() {
        let r = ProxyRotator::default();
        assert_eq!(r.list_proxies().len(), 0);
        assert_eq!(*r.max_fails.read(), 3);
    }
}
