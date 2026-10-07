//! Web search transport and result extraction.

use super::web_html;
use super::web_parse;

pub(crate) fn web_search(query: &str) -> String {






        let query = query.trim();

        if query.is_empty() {
            return "Web search query is empty.".into();
        }

        let encoded = web_parse::percent_encode(query);

        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

        let response = match ureq::get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X) cybOS/0.6",
            )
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                return format!("Web search failed: {}", error);
            }
        };

        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(error) => {
                return format!("Web search response could not be read: {}", error);
            }
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
                    "{}. {}\nSOURCE: {}\nURL: {}",
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
            "WEB SEARCH RESULTS FOR: {}\n\n{}\n\nIMPORTANT:\nSearch results are discovery data only. Read a relevant source with web_fetch before making factual claims.",
            query,
            results.join("\n\n")
        )
    }
