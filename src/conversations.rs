//! Small, explicit groups over authenticated direct connections. No relay or offline queue.
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const PREFIX: &str = "CYBCHAT-GROUP/1:";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Group {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub members: Vec<String>,
}
impl Group {
    pub fn validate(&self) -> Result<(), String> {
        uuid::Uuid::parse_str(&self.id).map_err(|_| "Некорректный ID группы")?;
        if self.title.trim().is_empty()
            || self.title.chars().count() > 64
            || self.title.chars().any(char::is_control)
        {
            return Err("Название группы: от 1 до 64 символов".into());
        }
        if !(2..=16).contains(&self.members.len()) || !self.members.contains(&self.creator) {
            return Err("В группе должно быть от 2 до 16 участников, включая создателя".into());
        }
        let mut unique = std::collections::HashSet::new();
        for member in &self.members {
            if member.trim().is_empty()
                || member.len() > 128
                || member.chars().any(char::is_control)
                || !unique.insert(member)
            {
                return Err("Некорректный или повторяющийся участник".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Book {
    pub names: BTreeMap<String, String>,
    pub groups: Vec<Group>,
}
impl Book {
    pub fn load(store: &Store) -> Result<Self, String> {
        let book: Self = match store.get_checked("cybchat_conversations_v1")? {
            Some(raw) => {
                serde_json::from_str(&raw).map_err(|_| "Не удалось прочитать список чатов")?
            }
            None => Self::default(),
        };
        for group in &book.groups {
            group.validate()?;
        }
        Ok(book)
    }
    pub fn save(&self, store: &Store) -> Result<(), String> {
        if self.groups.len() > 100 {
            return Err("Лимит: 100 групп".into());
        }
        let mut ids = std::collections::HashSet::new();
        for group in &self.groups {
            group.validate()?;
            if !ids.insert(&group.id) {
                return Err("Повторяющийся ID группы".into());
            }
        }
        store.set_checked(
            "cybchat_conversations_v1",
            &serde_json::to_string(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn name<'a>(&'a self, node: &'a str) -> &'a str {
        self.names
            .get(node)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(node)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct GroupMessage {
    pub id: String,
    pub group: Group,
    pub text: String,
}
impl GroupMessage {
    pub fn encode(&self) -> Result<String, String> {
        self.group.validate()?;
        uuid::Uuid::parse_str(&self.id).map_err(|_| "Некорректный ID сообщения")?;
        if self.text.trim().is_empty() || self.text.len() > 6000 {
            return Err("Сообщение пустое или длиннее 6000 байт".into());
        }
        let encoded = format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| e.to_string())?
        );
        if encoded.len() > 7000 {
            return Err("Сообщение слишком большое".into());
        }
        Ok(encoded)
    }
    pub fn decode(raw: &str) -> Option<Self> {
        if raw.len() > 7000 {
            return None;
        }
        let value: Self = serde_json::from_str(raw.strip_prefix(PREFIX)?).ok()?;
        value.encode().ok()?;
        Some(value)
    }
    /// Transport supplies the authenticated sender. Never trust a sender in the payload.
    pub fn accepted(&self, group: &Group, local: &str, sender: &str) -> bool {
        &self.group == group
            && group.members.iter().any(|m| m == local)
            && group.members.iter().any(|m| m == sender)
    }
    pub fn invitation(&self, local: &str, sender: &str) -> bool {
        sender == self.group.creator && self.group.members.iter().any(|m| m == local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group() -> Group {
        Group {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Друзья".into(),
            creator: "alice".into(),
            members: vec!["alice".into(), "bob".into()],
        }
    }
    #[test]
    fn messages_require_membership_and_unchanged_group_definition() {
        let g = group();
        let m = GroupMessage {
            id: uuid::Uuid::new_v4().to_string(),
            group: g.clone(),
            text: "Привет".into(),
        };
        let m = GroupMessage::decode(&m.encode().unwrap()).unwrap();
        assert!(m.accepted(&g, "bob", "alice"));
        assert!(!m.accepted(&g, "outsider", "alice"));
        assert!(!m.accepted(&g, "bob", "outsider"));
        assert!(m.invitation("bob", "alice"));
        assert!(!m.invitation("bob", "bob"));
        let mut changed = g.clone();
        changed.title = "Подмена".into();
        assert!(!m.accepted(&changed, "bob", "alice"));
    }
    #[test]
    fn book_persists_groups_and_detects_failed_writes() {
        let store = Store::test_store(":memory:");
        let mut b = Book::default();
        b.groups.push(group());
        b.names.insert("bob".into(), "Борис".into());
        b.save(&store).unwrap();
        assert_eq!(Book::load(&store).unwrap().name("bob"), "Борис");
        store.test_execute("CREATE TRIGGER deny_chat BEFORE UPDATE ON kv BEGIN SELECT RAISE(ABORT, 'denied'); END;");
        b.groups.clear();
        assert!(b.save(&store).is_err());
        assert_eq!(Book::load(&store).unwrap().groups.len(), 1);
    }
    #[test]
    fn malformed_or_duplicate_groups_are_rejected() {
        let mut g = group();
        g.members.push("bob".into());
        assert!(g.validate().is_err());
        assert!(GroupMessage::decode("CYBCHAT-GROUP/1:{}").is_none());
    }
}
