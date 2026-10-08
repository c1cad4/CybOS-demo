//! Web page fetching and HTML-to-text conversion.

use super::web_html;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const WEB_BUDGET: Duration = Duration::from_secs(8);

fn bounded_fetch<F>(work: F) -> String
where
    F: FnOnce() -> String + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let _ = tx.send(work());
    });

    match rx.recv_timeout(WEB_BUDGET) {
        Ok(result) => result,
        Err(_) => "Web fetch timed out after 8 seconds; no unverified content was returned.".into(),
    }
}

fn web_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(WEB_BUDGET))
        .build()
        .into()
}

pub(crate) fn web_fetch(url: &str) -> String {
    let url = url.trim().to_string();

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return format!("Invalid web URL: {}", url);
    }

    bounded_fetch(move || {
        let response = match web_agent()
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.7",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web fetch failed for {}: {}", url, error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web source could not be read as text: {}", error);
            }
        };

        let text = web_html::html_to_text(&body);

        if text.trim().is_empty() {
            return format!("Web source returned no readable text: {}", url);
        }

        let max_chars = 7000usize;

        let excerpt = if text.chars().count() > max_chars {
            text.chars().take(max_chars).collect::<String>()
        } else {
            text
        };

        format!("WEB SOURCE
URL: {}

SOURCE TEXT:
{}", url, excerpt)
    })
}
