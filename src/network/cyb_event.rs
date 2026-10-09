//! Minimal, local-first CybOS event envelope inspired by Buzz's signed-event architecture.
//!
//! This is deliberately *not* a Nostr event: signature verification and relay
//! interoperability require a separate reviewed implementation. Consumers must
//! never treat this envelope as authenticated merely because it has an author.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const EVENT_VERSION: u16 = 1;
pub const MAX_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_PENDING_EVENTS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CybEvent {
    pub version: u16,
    pub id: String,
    pub author: String,
    pub cell: String,
    pub kind: String,
    pub created_at_ms: i64,
    pub payload: String,
}

impl CybEvent {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != EVENT_VERSION { return Err("unsupported event version"); }
        for value in [&self.id, &self.author, &self.cell, &self.kind] {
            if value.trim().is_empty() || value.len() > 128 {
                return Err("event identifier must contain 1..=128 bytes");
            }
        }
        if self.payload.len() > MAX_EVENT_BYTES {
            return Err("event payload exceeds size limit");
        }
        Ok(())
    }
}

/// Bounded queue for local workflow/event delivery; no implied persistence.
#[derive(Debug, Default)]
pub struct EventInbox {
    queue: VecDeque<CybEvent>,
}

impl EventInbox {
    pub fn enqueue(&mut self, event: CybEvent) -> Result<(), &'static str> {
        event.validate()?;
        if self.queue.len() >= MAX_PENDING_EVENTS {
            return Err("event inbox full");
        }
        self.queue.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<CybEvent> { self.queue.pop_front() }
    pub fn len(&self) -> usize { self.queue.len() }
    pub fn is_empty(&self) -> bool { self.queue.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(id: &str) -> CybEvent {
        CybEvent {
            version: EVENT_VERSION,
            id: id.into(),
            author: "node-a".into(),
            cell: "CYBCHAT".into(),
            kind: "message.received".into(),
            created_at_ms: 1,
            payload: "hello".into(),
        }
    }
    #[test]
    fn rejects_invalid_event_metadata_and_oversized_payload() {
        let mut e = event("1");
        e.author.clear();
        assert!(e.validate().is_err());
        e = event("1");
        e.payload = "x".repeat(MAX_EVENT_BYTES + 1);
        assert!(e.validate().is_err());
        e = event("1");
        e.version += 1;
        assert!(e.validate().is_err());
    }
    #[test]
    fn inbox_enforces_backpressure_and_preserves_fifo() {
        let mut inbox = EventInbox::default();
        for i in 0..MAX_PENDING_EVENTS {
            inbox.enqueue(event(&i.to_string())).unwrap();
        }
        assert!(inbox.enqueue(event("overflow")).is_err());
        assert_eq!(inbox.len(), MAX_PENDING_EVENTS);
        for i in 0..MAX_PENDING_EVENTS {
            assert_eq!(inbox.pop().unwrap().id, i.to_string());
        }
        assert!(inbox.is_empty());
    }
}
