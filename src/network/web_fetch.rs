//! Web page fetching and HTML-to-text conversion.

use super::web_html;

pub(crate) fn web_fetch(url: &str) -> String {




        let url = url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return format!("Invalid web URL: {}", url);
        }

        let response = match ureq::get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
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

        format!("WEB SOURCE\nURL: {}\n\nSOURCE TEXT:\n{}", url, excerpt)
    }
