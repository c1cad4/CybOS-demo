use crate::CybOs;

impl CybOs {
    pub(crate) fn normalize_learning_id(value: &str) -> Option<String> {
        let mut id = String::new();

        for ch in value.trim().to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                id.push(ch);
            } else if ch.is_whitespace() {
                id.push('-');
            }
        }

        while id.contains("--") {
            id = id.replace("--", "-");
        }

        let id = id.trim_matches('-').to_string();

        if id.is_empty() || id.len() > 80 {
            None
        } else {
            Some(id)
        }
    }

}
