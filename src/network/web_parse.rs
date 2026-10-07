//! URL and source parsing helpers for the web layer.

use super::web_html;

pub(crate) fn percent_encode(input: &str) -> String {
            let mut out = String::new();

            for byte in input.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(byte as char)
                    }
                    b' ' => out.push('+'),
                    _ => {
                        out.push('%');
                        out.push_str(&format!("{:02X}", byte));
                    }
                }
            }

            out
        }

pub(crate) fn percent_decode(input: &str) -> String {
            let bytes = input.as_bytes();
            let mut out = Vec::with_capacity(bytes.len());
            let mut i = 0;

            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    let h1 = bytes[i + 1] as char;
                    let h2 = bytes[i + 2] as char;

                    let hex = format!("{}{}", h1, h2);

                    if let Ok(value) = u8::from_str_radix(&hex, 16) {
                        out.push(value);
                        i += 3;
                        continue;
                    }
                }

                if bytes[i] == b'+' {
                    out.push(b' ');
                } else {
                    out.push(bytes[i]);
                }

                i += 1;
            }

            String::from_utf8_lossy(&out).to_string()
        }

pub(crate) fn resolve_url(href: &str) -> String {
            let href = web_html::html_unescape(href.trim());

            if let Some(pos) = href.find("uddg=") {
                let value = &href[pos + 5..];
                let value = value.split('&').next().unwrap_or(value);

                return percent_decode(value);
            }

            if href.starts_with("//") {
                return format!("https:{}", href);
            }

            href
        }

pub(crate) fn source_class(url: &str) -> &'static str {
            if url.contains("government.ru")
                || url.contains("pravo.gov.ru")
                || url.contains("publication.pravo.gov.ru")
            {
                "OFFICIAL"
            } else if url.contains("consultant.ru") || url.contains("garant.ru") {
                "LEGAL DATABASE"
            } else {
                "OTHER"
            }
        }
