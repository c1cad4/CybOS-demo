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
    Radar,
    Assets,
    Cameras,
    CybLex,
    CybDex,
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
            Page::Radar => "CYB RADAR",
            Page::Brain => "CYBOS BRAIN",
            Page::Farm => "CICADAFARM",
            Page::Robot => "ROBOTCYB",
            Page::System => "SYSTEM CORE",
            Page::Chat => "CYBCHAT",
            Page::Assets => "ASSETS",
            Page::Cameras => "FARM CAMERAS",
            Page::CybLex => "CYBLEX",
            Page::CybDex => "CYBDEX · MARKET",
        }
    }

    pub(crate) fn icon(self) -> Icon {
        match self {
            Page::Dashboard => Icon::Dashboard,
            Page::Graph => Icon::Graph,
            Page::Network => Icon::Network,
            Page::Radar => Icon::Network,
            Page::Brain => Icon::Brain,
            Page::Farm => Icon::Farm,
            Page::Robot => Icon::Robot,
            Page::System => Icon::System,
            Page::Chat => Icon::Chat,
            Page::Assets => Icon::Assets,
            Page::Cameras => Icon::Camera,
            Page::CybLex => Icon::Network,
            Page::CybDex => Icon::Assets,
        }
    }

    pub(crate) fn matches_query(self, q: &str) -> bool {
        let keys = match self {
            Page::Dashboard => "dashboard home core sigma live node activity",
            Page::Robot => "robot robotcyb agent qwen ai робот",
            Page::Farm => "farm cicadafarm hive chicken goat honey eggs environment ферма улей пасека",
            Page::Chat => "chat cybchat message peer чат",
            Page::Graph => "graph cybergraph nodes links граф",
            Page::Brain => "brain memory knowledge qwen мозг память знания",
            Page::Network => "network lan p2p ble nostr сеть",
            Page::Radar => "radar proximity nearby peers bluetooth ble nearby узлы друзья",
            Page::Assets => "assets tokens cicadafarm robotcyb mint solana pumpfun program токены токен pump",
            Page::Cameras => "cameras camera rtsp farm live",
            Page::CybLex => "cyblex archive torrent bittorrent magnet seed share download p2p files",
            Page::CybDex => "cybdex dex market tokens pairs pools ohlcv candlestick price liquidity volume swap solana",
            Page::System => "system energy battery node database cyblaunch launcher bridge",
        };

        if q.is_empty() {
            return false;
        }

        if self.title().to_lowercase().contains(q) {
            return true;
        }

        keys.split_whitespace().any(|key| key == q)
    }
}
