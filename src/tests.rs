use crate::{config, Page};

#[test]
fn navigation_supports_russian_page_names() {
    assert!(Page::Farm.matches_query("ферма"));
    assert!(Page::Robot.matches_query("робот"));
    assert!(Page::Chat.matches_query("чат"));
    assert!(Page::Graph.matches_query("граф"));
    assert!(Page::Brain.matches_query("мозг"));
    assert!(Page::Network.matches_query("сеть"));
    assert!(Page::Assets.matches_query("токен"));
    assert!(Page::Browser.matches_query("browser"));
    assert!(Page::Vpn.matches_query("vpn"));
    assert!(Page::Vpn.matches_query("впн"));
    assert!(Page::Radar.matches_query("adsb"));
}

#[test]
fn navigation_does_not_match_partial_unrelated_words() {
    assert!(!Page::Farm.matches_query("farmer"));
    assert!(!Page::Robot.matches_query("roboticsx"));
}

#[test]
fn configuration_is_release_ready() {
    assert_eq!(config::APP_VERSION, "0.7.2");
    assert!(config::CICADAFARM_MINT.len() > 20);
    assert!(config::ROBOTCYB_MINT.len() > 20);
    assert_eq!(config::DEFAULT_CICADA_WALLET.len(), 44);
}

#[test]
fn package_metadata_version_matches_app_version() {
    assert_eq!(env!("CARGO_PKG_VERSION"), config::APP_VERSION);
}

#[test]
fn user_statements_grow_graph_once_and_persist_as_unverified_memory() {
    let store = crate::store::Store::test_store(":memory:");
    store.test_execute("CREATE TABLE memories(id TEXT PRIMARY KEY,time TEXT,text TEXT,source TEXT,importance REAL); CREATE TABLE graph_nodes(id TEXT PRIMARY KEY,label TEXT,kind TEXT,x REAL,y REAL); CREATE TABLE graph_links(from_id TEXT,to_id TEXT,relation TEXT,PRIMARY KEY(from_id,to_id,relation));");
    assert!(!store.capture_user_message("  ").unwrap());
    assert!(store.capture_user_message("Мне нравится космос").unwrap());
    assert!(!store.capture_user_message("Мне нравится космос").unwrap());
    assert_eq!(store.graph_nodes().len(), 2);
    assert_eq!(store.graph_links().len(), 1);
    assert_eq!(store.memories()[0].source, "USER_MESSAGE");
    assert!(!store.search_memories("космос").is_empty());
    assert!(store.capture_user_message("Хочу изучать звёзды").unwrap());
    assert_eq!(store.graph_nodes().len(), 3);
}

#[test]
fn graph_write_failure_rolls_back_new_memory() {
    let store = crate::store::Store::test_store(":memory:");
    store.test_execute("CREATE TABLE memories(id TEXT PRIMARY KEY,time TEXT,text TEXT,source TEXT,importance REAL);");
    assert!(store.capture_user_message("Не должно сохраниться частично").is_err());
    assert!(store.memories().is_empty());
}

#[test]
fn indexed_memory_search_finds_old_unicode_notes_and_tracks_changes() {
    let store = crate::store::Store::test_store(":memory:");
    store.test_execute("CREATE TABLE memories(id TEXT PRIMARY KEY,time TEXT,text TEXT,source TEXT,importance REAL);
        INSERT INTO memories VALUES('old','2020','Квантовая защита проект','USER_MESSAGE',0.5);");
    for i in 0..250 {store.test_execute(&format!("INSERT INTO memories VALUES('id-{i}','2026','обычная заметка','USER_MESSAGE',0.5);"));}
    assert!(store.memories().iter().all(|m|m.id!="old"));
    assert_eq!(store.search_memories("квантов")[0].id,"old");
    store.test_execute("UPDATE memories SET text='Новый термин' WHERE id='old';");
    assert!(store.search_memories("квантов").is_empty());
    assert_eq!(store.search_memories("термин")[0].id,"old");
    store.test_execute("DELETE FROM memories WHERE id='old';");
    assert!(store.search_memories("термин").is_empty());
    assert!(store.search_memories("\" OR NOT *").len()<=8);
}
