//! HTML decoding and text extraction for fetched web pages.

pub(crate) fn html_unescape(input: &str) -> String {
            input
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&#x27;", "'")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&nbsp;", " ")
        }

pub(crate) fn html_to_text(html: &str) -> String {
            let body = if let Some(start) = html.find("<body") {
                if let Some(open_end) = html[start..].find('>') {
                    let body_start = start + open_end + 1;

                    if let Some(close) = html[body_start..].find("</body>") {
                        &html[body_start..body_start + close]
                    } else {
                        &html[body_start..]
                    }
                } else {
                    html
                }
            } else {
                html
            };

            let mut cleaned = body.to_string();

            loop {
                let Some(start) = cleaned.find("<script") else {
                    break;
                };

                let Some(end_rel) = cleaned[start..].find("</script>") else {
                    cleaned.replace_range(start.., "");
                    break;
                };

                let end = start + end_rel + "</script>".len();
                cleaned.replace_range(start..end, " ");
            }

            loop {
                let Some(start) = cleaned.find("<style") else {
                    break;
                };

                let Some(end_rel) = cleaned[start..].find("</style>") else {
                    cleaned.replace_range(start.., "");
                    break;
                };

                let end = start + end_rel + "</style>".len();
                cleaned.replace_range(start..end, " ");
            }

            let mut result = String::new();
            let mut in_tag = false;

            for ch in cleaned.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => {
                        in_tag = false;
                        result.push(' ');
                    }
                    _ if in_tag => {}
                    '\n' | '\r' | '\t' => result.push(' '),
                    _ => result.push(ch),
                }
            }

            let result = html_unescape(&result);

            result.split_whitespace().collect::<Vec<_>>().join(" ")
        }
