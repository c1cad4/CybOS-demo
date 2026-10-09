//! Built-in CybBrowser protocol router and bounded document engine.
//!
//! The browser cell deliberately does not execute remote JavaScript. It resolves
//! Web2, IPFS/IPNS and Arweave addresses, fetches bounded documents and returns
//! parsed text/links to the native UI. A future Servo adapter can replace the
//! renderer without changing the protocol router.

use std::{
    io::Read,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use super::web_html;

const JOB_BUDGET: Duration = Duration::from_secs(12);
const HTTP_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_LINKS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BrowserRoute {
    WebHttp,
    IpfsLocal,
    IpfsGateway,
    IpnsLocal,
    IpnsGateway,
    ArweaveGateway,
}

impl BrowserRoute {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::WebHttp => "WEB2 · HTTP(S)",
            Self::IpfsLocal => "IPFS · LOCAL NODE",
            Self::IpfsGateway => "IPFS · GATEWAY FALLBACK",
            Self::IpnsLocal => "IPNS · LOCAL NODE",
            Self::IpnsGateway => "IPNS · GATEWAY FALLBACK",
            Self::ArweaveGateway => "ARWEAVE · GATEWAY",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BrowserLink {
    pub(crate) label: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug)]
pub(crate) struct BrowserDocument {
    pub(crate) requested_url: String,
    pub(crate) resolved_url: String,
    pub(crate) route: BrowserRoute,
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) links: Vec<BrowserLink>,
    pub(crate) bytes: usize,
}

pub(crate) enum BrowserEvent {
    Document(BrowserDocument),
    Status(String),
    Error(String),
}

enum BrowserCommand {
    Navigate(String),
    Shutdown,
}

pub(crate) struct BrowserRuntime {
    tx: Sender<BrowserCommand>,
    rx: Receiver<BrowserEvent>,
}

impl Default for BrowserRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserRuntime {
    /// Creates a disconnected runtime for short-lived worker snapshots.
    pub(crate) fn empty() -> Self {
        let (tx, command_rx) = mpsc::channel();
        drop(command_rx);
        let (event_tx, rx) = mpsc::channel();
        drop(event_tx);
        Self { tx, rx }
    }

    pub(crate) fn new() -> Self {
        let (tx, command_rx) = mpsc::channel();
        let (event_tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("cybbrowser-engine".into())
            .spawn(move || run_worker(command_rx, event_tx))
            .expect("failed to spawn CybBrowser worker");

        Self { tx, rx }
    }

    pub(crate) fn navigate(&self, url: String) -> Result<(), String> {
        validate_input(&url)?;
        self.tx
            .send(BrowserCommand::Navigate(url.trim().to_string()))
            .map_err(|_| "CybBrowser worker is not running".into())
    }

    pub(crate) fn poll(&self) -> Vec<BrowserEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for BrowserRuntime {
    fn drop(&mut self) {
        let _ = self.tx.send(BrowserCommand::Shutdown);
    }
}

fn run_worker(command_rx: Receiver<BrowserCommand>, event_tx: Sender<BrowserEvent>) {
    let _ = event_tx.send(BrowserEvent::Status(
        "CYBBROWSER · READY · NO SCRIPT EXECUTION".into(),
    ));

    loop {
        match command_rx.recv() {
            Ok(BrowserCommand::Navigate(input)) => {
                let _ = event_tx.send(BrowserEvent::Status(format!(
                    "CYBBROWSER · RESOLVING · {}",
                    input
                )));

                let started = std::time::Instant::now();
                match resolve(&input) {
                    Ok(resolved) => {
                        let _ = event_tx.send(BrowserEvent::Status(format!(
                            "CYBBROWSER · {} · FETCHING",
                            resolved.route.label()
                        )));

                        match fetch_document(&resolved, started) {
                            Ok(document) => {
                                let _ = event_tx.send(BrowserEvent::Document(document));
                            }
                            Err(error) => {
                                let _ = event_tx.send(BrowserEvent::Error(error));
                            }
                        }
                    }
                    Err(error) => {
                        let _ = event_tx.send(BrowserEvent::Error(error));
                    }
                }
            }
            Ok(BrowserCommand::Shutdown) | Err(mpsc::RecvError) => break,
        }
    }
}

#[derive(Clone, Debug)]
struct ResolvedBrowserUrl {
    requested_url: String,
    resolved_url: String,
    route: BrowserRoute,
    fallback_url: Option<String>,
}

fn resolve(input: &str) -> Result<ResolvedBrowserUrl, String> {
    let input = input.trim();

    if input.starts_with("http://") || input.starts_with("https://") {
        return Ok(ResolvedBrowserUrl {
            requested_url: input.into(),
            resolved_url: input.into(),
            route: BrowserRoute::WebHttp,
            fallback_url: None,
        });
    }

    if let Some(path) = input.strip_prefix("ipfs://") {
        return resolve_ipfs(input, path, false);
    }

    if let Some(path) = input.strip_prefix("ipns://") {
        return resolve_ipns(input, path);
    }

    if let Some(path) = input.strip_prefix("ar://") {
        return resolve_arweave(input, path);
    }

    if let Some(path) = input.strip_prefix("arweave://") {
        return resolve_arweave(input, path);
    }

    if let Some(path) = input.strip_prefix("cyb://ipfs/") {
        return resolve_ipfs(input, path, true);
    }

    if let Some(path) = input.strip_prefix("cyb://ipns/") {
        return resolve_ipns(input, &format!("cyb://ipns/{path}"));
    }

    if let Some(path) = input.strip_prefix("cyb://ar/") {
        return resolve_arweave(input, path);
    }

    Err("Unsupported browser protocol. Use http://, https://, ipfs://, ipns://, ar:// or cyb://ipfs/...".into())
}

fn resolve_ipfs(original: &str, path: &str, cyb: bool) -> Result<ResolvedBrowserUrl, String> {
    let path = path.trim_start_matches('/');
    let cid = path.split('/').next().unwrap_or_default();

    if cid.len() < 10 || cid.chars().any(|c| c.is_whitespace()) {
        return Err("Invalid IPFS CID/path".into());
    }

    let suffix = if path.is_empty() { cid } else { path };
    let local_base = std::env::var("CYBOS_IPFS_GATEWAY")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".into())
        .trim_end_matches('/')
        .to_string();
    let local_url = format!("{local_base}/ipfs/{suffix}");
    let gateway_url = format!("https://ipfs.io/ipfs/{suffix}");

    let requested_url = if cyb {
        original.to_string()
    } else {
        format!("ipfs://{path}")
    };

    Ok(ResolvedBrowserUrl {
        requested_url,
        resolved_url: local_url,
        route: BrowserRoute::IpfsLocal,
        fallback_url: Some(gateway_url),
    })
}

fn resolve_ipns(original: &str, path: &str) -> Result<ResolvedBrowserUrl, String> {
    let normalized = path
        .strip_prefix("cyb://ipns/")
        .or_else(|| path.strip_prefix("ipns://"))
        .unwrap_or(path)
        .trim_start_matches('/');
    let name = normalized.split('/').next().unwrap_or_default();

    if name.len() < 5 || name.chars().any(|c| c.is_whitespace()) {
        return Err("Invalid IPNS name/path".into());
    }

    let local_base = std::env::var("CYBOS_IPFS_GATEWAY")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".into())
        .trim_end_matches('/')
        .to_string();

    let local_url = format!("{local_base}/ipns/{normalized}");
    let gateway_url = format!("https://ipfs.io/ipns/{normalized}");

    Ok(ResolvedBrowserUrl {
        requested_url: original.to_string(),
        resolved_url: local_url,
        route: BrowserRoute::IpnsLocal,
        fallback_url: Some(gateway_url),
    })
}

fn resolve_arweave(original: &str, path: &str) -> Result<ResolvedBrowserUrl, String> {
    let normalized = path.trim_start_matches('/');
    let txid = normalized.split('/').next().unwrap_or_default();

    if txid.len() < 20 || txid.chars().any(|c| c.is_whitespace()) {
        return Err("Invalid Arweave transaction/path".into());
    }

    Ok(ResolvedBrowserUrl {
        requested_url: original.to_string(),
        resolved_url: format!("https://arweave.net/{normalized}"),
        route: BrowserRoute::ArweaveGateway,
        fallback_url: None,
    })
}

fn fetch_document(
    resolved: &ResolvedBrowserUrl,
    started: std::time::Instant,
) -> Result<BrowserDocument, String> {
    let budget = JOB_BUDGET
        .checked_sub(started.elapsed())
        .ok_or_else(|| "CybBrowser worker budget expired".to_string())?;

    match fetch_text(&resolved.resolved_url, budget) {
        Ok(body) => parse_document(resolved, &body),
        Err(primary_error) => {
            let Some(fallback) = &resolved.fallback_url else {
                return Err(primary_error);
            };

            let remaining = JOB_BUDGET
                .checked_sub(started.elapsed())
                .ok_or_else(|| "CybBrowser worker budget expired".to_string())?;

            let body = fetch_text(fallback, remaining).map_err(|fallback_error| {
                format!(
                    "{}; fallback also failed: {}",
                    primary_error, fallback_error
                )
            })?;

            let mut document = parse_document(resolved, &body)?;
            document.resolved_url = fallback.clone();
            document.route = match &resolved.route {
                BrowserRoute::IpfsLocal => BrowserRoute::IpfsGateway,
                BrowserRoute::IpnsLocal => BrowserRoute::IpnsGateway,
                other => other.clone(),
            };
            Ok(document)
        }
    }
}

fn fetch_text(url: &str, budget: Duration) -> Result<String, String> {
    if url.len() > 4096 {
        return Err("Resolved browser URL exceeds 4096 bytes".into());
    }

    let timeout = std::cmp::min(budget, HTTP_TIMEOUT);
    if timeout.is_zero() {
        return Err("Browser request budget expired".into());
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();

    let response = agent
        .get(url)
        .header("User-Agent", "cybOS/CybBrowser-0.7.2")
        .call()
        .map_err(|e| format!("Browser fetch failed: {e}"))?;

    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take((MAX_BODY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Browser body read failed: {e}"))?;

    if bytes.len() > MAX_BODY_BYTES {
        return Err(format!(
            "Browser document exceeds {} MiB limit",
            MAX_BODY_BYTES / 1024 / 1024
        ));
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_document(
    resolved: &ResolvedBrowserUrl,
    body: &str,
) -> Result<BrowserDocument, String> {
    let title = extract_title(body).unwrap_or_else(|| resolved.requested_url.clone());
    let text = web_html::html_to_text(body);

    if text.trim().is_empty() {
        return Err("Browser document contains no readable text".into());
    }

    Ok(BrowserDocument {
        requested_url: resolved.requested_url.clone(),
        resolved_url: resolved.resolved_url.clone(),
        route: resolved.route.clone(),
        title,
        text,
        links: extract_links(body),
        bytes: body.len(),
    })
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')? + start + 1;
    let close = lower[open_end..].find("</title>")? + open_end;

    let title = web_html::html_to_text(&html[open_end..close]);
    if title.is_empty() {
        None
    } else {
        Some(title.chars().take(240).collect())
    }
}

fn extract_links(html: &str) -> Vec<BrowserLink> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut links = Vec::new();

    while cursor < html.len() && links.len() < MAX_LINKS {
        let Some(rel_start) = lower[cursor..].find("href=") else {
            break;
        };
        let start = cursor + rel_start + "href=".len();
        let quote = html.as_bytes().get(start).copied();

        let (value_start, value_end) = match quote {
            Some(b'"') | Some(b'\'') => {
                let end = match html[start + 1..].find(quote.unwrap() as char) {
                    Some(rel) => start + 1 + rel,
                    None => break,
                };
                (start + 1, end)
            }
            _ => {
                let tail = &html[start..];
                let end = tail
                    .find(|c: char| c.is_ascii_whitespace() || c == '>')
                    .unwrap_or(tail.len());
                (start, start + end)
            }
        };

        let raw_url = html[value_start..value_end].trim();
        let resolved_url = resolve_link(raw_url);

        if !resolved_url.is_empty()
            && (resolved_url.starts_with("http://")
                || resolved_url.starts_with("https://")
                || resolved_url.starts_with("ipfs://")
                || resolved_url.starts_with("ipns://")
                || resolved_url.starts_with("ar://")
                || resolved_url.starts_with("cyb://"))
        {
            let label = web_html::html_to_text(
                &html[value_end..]
                    .split_once("</a>")
                    .map(|(text, _)| text)
                    .unwrap_or("link"),
            )
            .chars()
            .take(120)
            .collect::<String>();

            links.push(BrowserLink {
                label: if label.is_empty() { resolved_url.clone() } else { label },
                url: resolved_url,
            });
        }

        cursor = value_end.saturating_add(1);
    }

    links.sort_by(|a, b| a.url.cmp(&b.url));
    links.dedup_by(|a, b| a.url == b.url);
    links
}

fn resolve_link(value: &str) -> String {
    let value = web_html::html_unescape(value.trim());

    if value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("ipfs://")
        || value.starts_with("ipns://")
        || value.starts_with("ar://")
        || value.starts_with("cyb://")
    {
        return value;
    }

    value.to_string()
}

fn validate_input(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        return Err("Browser URL is empty".into());
    }

    if input.trim().len() > 4096 {
        return Err("Browser URL exceeds 4096 bytes".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{resolve, BrowserRoute};

    #[test]
    fn resolves_http() {
        let resolved = resolve("https://example.org").unwrap();
        assert_eq!(resolved.route, BrowserRoute::WebHttp);
        assert_eq!(resolved.resolved_url, "https://example.org");
    }

    #[test]
    fn resolves_cyb_ipfs() {
        let resolved = resolve("cyb://ipfs/bafybeigdyrzt5example");
        assert_eq!(resolved.route, BrowserRoute::IpfsLocal);
        assert!(resolved.resolved_url.contains("/ipfs/"));
        assert!(resolved.fallback_url.is_some());
    }

    #[test]
    fn resolves_arweave() {
        let resolved = resolve("ar://AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(resolved.route, BrowserRoute::ArweaveGateway);
        assert!(resolved.resolved_url.contains("arweave.net"));
    }

    #[test]
    fn rejects_unknown_scheme() {
        assert!(resolve("ftp://example.org").is_err());
    }
}
