//! cybOS local runtime initialization.
//!
//! Owns persistent runtime bootstrap such as the local node identity
//! and the initial system event.

use crate::store::Store;
use uuid::Uuid;

pub(crate) fn open_store() -> Store {
    Store::open()
}

pub(crate) fn load_or_create_node_id(store: &Store) -> String {
    store.get("node_id").unwrap_or_else(|| {
        let value = format!("cyb-{}", &Uuid::new_v4().to_string()[..8]);
        store.set("node_id", &value);
        value
    })
}

pub(crate) fn load_events(store: &Store) -> Vec<crate::models::Event> {
    let mut events = store.events();

    if events.is_empty() {
        store.add_event("SYSTEM", "cybOS local runtime initialized");
        events = store.events();
    }

    events
}
