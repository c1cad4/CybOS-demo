//! Optional CybCore connectivity. Never blocks cybOS startup or local workflows.
//! This client intentionally does not send secrets, messages or signed events.
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreStatus {
    Disabled,
    Online,
    Offline,
}

pub struct CybCoreClient {
    endpoint: Option<String>,
}

impl CybCoreClient {
    pub fn new(endpoint: Option<String>) -> Self {
        Self { endpoint }
    }

    pub fn status(&self) -> CoreStatus {
        let Some(endpoint) = self.endpoint.as_deref() else {
            return CoreStatus::Disabled;
        };
        // Explicit opt-in; prohibit insecure public HTTP and credentials in URLs.
        let Ok(url) = ureq::http::Uri::try_from(endpoint) else {
            return CoreStatus::Offline;
        };
        let scheme = url.scheme_str().unwrap_or_default();
        let host = url.host().unwrap_or_default();
        let loopback = matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1");
        if !(scheme == "https" || (scheme == "http" && loopback))
            || url.authority().is_none()
            || endpoint.contains('@')
        {
            return CoreStatus::Offline;
        }
        let url = format!("{}/healthz", endpoint.trim_end_matches('/'));
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .build()
            .new_agent();
        match agent.get(&url).call() {
            Ok(response) if response.status() == 200 => CoreStatus::Online,
            _ => CoreStatus::Offline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_without_endpoint() {
        assert_eq!(CybCoreClient::new(None).status(), CoreStatus::Disabled);
    }
    #[test]
    fn refuses_public_plaintext_http() {
        assert_eq!(
            CybCoreClient::new(Some("http://example.com:8080".into())).status(),
            CoreStatus::Offline
        );
    }
}
