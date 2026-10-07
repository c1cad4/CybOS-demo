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
}

#[test]
fn navigation_does_not_match_partial_unrelated_words() {
    assert!(!Page::Farm.matches_query("farmer"));
    assert!(!Page::Robot.matches_query("roboticsx"));
}

#[test]
fn configuration_is_release_ready() {
    assert_eq!(config::APP_VERSION, "0.7.0");
    assert!(config::CICADAFARM_MINT.len() > 20);
    assert!(config::ROBOTCYB_MINT.len() > 20);
    assert_eq!(config::DEFAULT_CICADA_WALLET.len(), 44);
}
