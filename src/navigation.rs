//! cybOS application navigation model.
//!
//! Defines application pages, navigation icons, titles and search matching.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Dashboard,
    Robot,
    Farm,
    Chat,
    Graph,
    Brain,
    Network,
    Assets,
    Cameras,
    System,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Dashboard,
    Graph,
    Network,
    Brain,
    Farm,
    Robot,
    Camera,
    System,
    Assets,
    Chat,
    Energy,
    Environment,
    Activity,
    Node,
    Search,
    Plus,
}

impl Page {
    pub(crate) fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "CENTRAL CYBOS DASHBOARD",
            Page::Graph => "CYBOS GRAPH",
            Page::Network => "NETWORK MATRIX",
            Page::Brain => "CYBOS BRAIN",
            Page::Farm => "CICADAFARM",
            Page::Robot => "ROBOTCYB",
            Page::System => "SYSTEM CORE",
            Page::Chat => "CYBCHAT",
            Page::Assets => "ASSETS",
            Page::Cameras => "FARM CAMERAS",
        }
    }

    pub(crate) fn icon(self) -> Icon {
        match self {
            Page::Dashboard => Icon::Dashboard,
            Page::Graph => Icon::Graph,
            Page::Network => Icon::Network,
            Page::Brain => Icon::Brain,
            Page::Farm => Icon::Farm,
            Page::Robot => Icon::Robot,
            Page::System => Icon::System,
            Page::Chat => Icon::Chat,
            Page::Assets => Icon::Assets,
            Page::Cameras => Icon::Camera,
        }
    }

    pub(crate) fn matches_query(self, q: &str) -> bool {
        let keys = match self {
            Page::Dashboard => "dashboard home core sigma live node activity",
            Page::Robot => "robot robotcyb agent qwen ai",
            Page::Farm => "farm cicadafarm hive chicken goat honey eggs environment",
            Page::Chat => "chat cybchat message peer",
            Page::Graph => "graph cybergraph nodes links",
            Page::Brain => "brain memory knowledge qwen",
            Page::Network => "network lan p2p ble nostr",
            Page::Assets => "assets tokens cicadafarm robotcyb mint solana",
            Page::Cameras => "cameras camera rtsp farm live",
            Page::System => "system energy battery node database",
        };

        keys.contains(q) || self.title().to_lowercase().contains(q)
    }
}
