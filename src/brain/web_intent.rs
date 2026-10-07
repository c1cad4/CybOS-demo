//! Web intent detection for the cybOS agent.

pub(crate) fn is_web_request(q: &str) -> bool {
        let lower = q.to_lowercase();

        lower.contains("найди в интернете")
            || lower.contains("найти в интернете")
            || lower.contains("поищи в интернете")
            || lower.contains("проверь в интернете")
            || lower.contains("в интернете")
            || lower.contains("источник")
            || lower.contains("ссылк")
            || lower.contains("актуальн")
            || lower.contains("действует ли")
            || lower.contains("утратил силу")
            || lower.contains("отменен")
            || lower.contains("отменено")
            || lower.contains("постановлен")
            || lower.contains("закон")
            || lower.contains("указ")
            || lower.contains("приказ")
            || lower.contains("норматив")
            || lower.contains("правительств")
    }

pub(crate) fn is_web_followup(q: &str) -> bool {
        let lower = q.to_lowercase();

        let pronoun = lower.contains("он")
            || lower.contains("него")
            || lower.contains("нему")
            || lower.contains("этот документ")
            || lower.contains("это постановление")
            || lower.contains("этого документа")
            || lower.contains("в нем")
            || lower.contains("в нём");

        let continuation = lower.contains("какие изменения")
            || lower.contains("какие изменения внесли")
            || lower.contains("что изменилось")
            || lower.contains("действует ли")
            || lower.contains("действует сейчас")
            || lower.contains("актуален ли")
            || lower.contains("актуально ли")
            || lower.contains("отменен ли")
            || lower.contains("отменили ли")
            || lower.contains("последние изменения")
            || lower.contains("текущий статус")
            || lower.contains("сейчас");

        pronoun || continuation
    }

pub(crate) fn web_followup_query(
    q: &str,
    previous: Option<String>,
) -> Option<String> {
        if !is_web_followup(q) {
            return None;
        }

        let previous = previous?;

        let lower = q.to_lowercase();

        if lower.contains("какие изменения")
            || lower.contains("что изменилось")
            || lower.contains("последние изменения")
        {
            return Some(format!(
                "{} изменения редакция актуальные изменения",
                previous
            ));
        }

        if lower.contains("действует")
            || lower.contains("актуален")
            || lower.contains("сейчас")
            || lower.contains("текущий статус")
            || lower.contains("отменен")
        {
            return Some(format!(
                "{} действует ли сейчас актуальный статус на 2026 год",
                previous
            ));
        }

        Some(format!("{} {}", previous, q))
    }
