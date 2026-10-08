//! Web search transport and result extraction.

use super::web_html;
use super::web_parse;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const WEB_SEARCH_BUDGET: Duration = Duration::from_secs(8);

fn bounded_search<F>(work: F, budget: Duration) -> String
where
    F: FnOnce() -> String + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(work());
    });
    match rx.recv_timeout(budget) {
        Ok(result) => result,
        Err(_) => format!("Web search timed out after {} ms.", budget.as_millis()),
    }
}

fn web_agent(budget: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(budget))
        .build()
        .into()
}

pub(crate) fn web_search(query: &str) -> String {
    bounded_web_search(query, WEB_SEARCH_BUDGET)
}

pub(crate) fn web_search_with_deadline(query: &str, deadline: std::time::Instant) -> String {
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return "Web search deadline expired before request start.".into();
    }
    bounded_web_search(query, remaining)
}

fn bounded_web_search(query: &str, budget: Duration) -> String {
    let query = query.trim().to_string();

    if query.is_empty() {
        return "Web search query is empty.".into();
    }

    bounded_search(move || {
        let query = query.trim();
        let encoded = web_parse::percent_encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

        let response = match web_agent(budget)
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => return format!("Web search failed: {}", error),
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => return format!("Web search response could not be read: {}", error),
        };

        let mut results = Vec::new();
        let mut remaining = body.as_str();

        while let Some(start) = remaining.find("result__a") {
            remaining = &remaining[start..];

            let href_start = match remaining.find("href=\"") {
                Some(pos) => pos + 6,
                None => break,
            };

            let href_end = match remaining[href_start..].find('"') {
                Some(pos) => href_start + pos,
                None => break,
            };

            let raw_href = &remaining[href_start..href_end];
            let href = web_parse::resolve_url(raw_href);

            let title_start = match remaining[href_end..].find('>') {
                Some(pos) => href_end + pos + 1,
                None => break,
            };

            let title_end = match remaining[title_start..].find("</a>") {
                Some(pos) => title_start + pos,
                None => break,
            };

            let title = web_html::html_unescape(&remaining[title_start..title_end])
                .trim()
                .to_string();

            if !href.is_empty() && !title.is_empty() {
                results.push(format!(
                    "{}. {}
SOURCE: {}
URL: {}",
                    results.len() + 1,
                    title,
                    web_parse::source_class(&href),
                    href
                ));
            }

            remaining = &remaining[title_end..];

            if results.len() >= 5 {
                break;
            }
        }

        if results.is_empty() {
            return format!("Web search returned no parsed results for: {}", query);
        }

        format!(
            "WEB SEARCH RESULTS FOR: {}

{}

IMPORTANT:
Search results are discovery data only. Read a relevant source with web_fetch before making factual claims.",
            query,
            results.join("

")
        )
    }, budget)
}
