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
    PlanetaryPulse,
    OfflineAtlas,
    Browser,
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
            Page::PlanetaryPulse => "PLANETARY PULSE",
            Page::OfflineAtlas => "OFFLINE ATLAS",
            Page::Browser => "CYBBROWSER",
        }
    }

    /// Plain-language guidance shown in the shared page header.
    /// Each entry explains purpose, a safe first action, and an important limitation.
    pub(crate) fn guidance(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Page::Dashboard => ("Your overview of cybOS modules, node state, and recent activity.", "Select a labelled module in the map or use the left navigation rail.", "Status cards reflect the app's current local state; demo values may not be live sensor readings."),
            Page::Robot => ("Ask RobotCYB for help and define work with a clear, checkable outcome.", "Enter a request, then use the work queue to create a task proposal with acceptance criteria.", "Task budgets are accounting limits only. Workflow checks do not authorize payments or physical actions."),
            Page::Farm => ("Keep CicadaFarm observations, animals, plants, and farm activity in one workspace.", "Choose a farm section and record an observation or review the available farm status.", "Only connected and explicitly reported readings should be treated as live measurements."),
            Page::Chat => ("Communicate with RobotCYB locally or with discovered peers when a secure channel is available.", "Start with the local conversation; for peer chat, discover a peer and verify its identity status.", "A first-seen peer is not independently verified. Check the fingerprint with the other person."),
            Page::Graph => ("Explore the nodes and relationships stored in the local knowledge graph.", "Select a node to inspect it; zoom or pan to explore larger graphs.", "A graph relationship is stored data, not proof that the relationship is true."),
            Page::Brain => ("Review local memory, graph knowledge, and the availability of the local AI model.", "Check Qwen status first; if it is offline, follow the first-run setup guidance on this page.", "cybOS can open without Qwen, but AI requests may not complete until the local model is running."),
            Page::Network => ("Inspect local network discovery and peer connectivity.", "Run a scan, select a discovered peer, and check its current status before sending.", "Discovery does not itself establish trust; secure peer identity checks still matter."),
            Page::Radar => ("See nearby nodes discovered through supported LAN or Bluetooth mechanisms.", "Enable visibility only when you want this node to be discoverable, then refresh discovery.", "Visibility can reveal that your node is nearby. Turn it off when you do not want to advertise."),
            Page::Assets => ("Review configured token and wallet information.", "Inspect the displayed address and balances before copying or sharing anything.", "Market or balance data may be delayed. Never enter a seed phrase or private key into cybOS."),
            Page::Cameras => ("Review configured farm camera sources and their connection state.", "Select a camera zone and check its status before expecting a live feed.", "A configured source is not necessarily connected; do not assume video is live unless the UI says so."),
            Page::CybLex => ("Manage authorized file downloads, sharing, and torrent jobs.", "Choose a magnet link or .torrent URL, verify the source, and choose the destination folder.", "Share only content you own or are authorized to distribute. Check the destination before starting."),
            Page::CybDex => ("Explore Solana token pairs, liquidity, volume, and price history in read-only mode.", "Search a token symbol or mint, choose a pair, then select a chart timeframe.", "Market data is informational and may be delayed. This page does not sign or execute swaps."),
            Page::PlanetaryPulse => ("Monitor biodiversity, air quality, and humanity indicators using attributed public sources.", "Review the data date and source notes on each card before interpreting a value.", "The wildlife index is an average change across monitored vertebrate populations, not a count of all animals lost. Air data is not live until a provider is connected."),
            Page::OfflineAtlas => ("Inspect local MBTiles map archives for use without internet access.", "Enter the path to an MBTiles file and inspect its metadata and tile coverage.", "This first stage validates local archives only; it does not yet render maps, provide live GPS, or calculate routes."),
            Page::Browser => ("Open bounded text views of web and supported decentralized-protocol addresses.", "Enter a full URL such as https://example.com and press Open.", "Remote JavaScript is not executed here. Treat page content and links as untrusted."),
            Page::System => ("Inspect local runtime health, database status, and system components.", "Review any ERROR or OFFLINE status and open the related module for details.", "A READY status means the local component reports ready; it does not guarantee external services are reachable."),
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
            Page::PlanetaryPulse => Icon::Environment,
            Page::OfflineAtlas => Icon::Search,
            Page::Browser => Icon::Network,
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
            Page::Assets => "assets tokens cicadafarm robotcyb mint solana токены токен",
            Page::Cameras => "cameras camera rtsp farm live",
            Page::CybLex => "cyblex archive torrent bittorrent magnet seed share download p2p files",
            Page::CybDex => "cybdex dex market tokens pairs pools ohlcv candlestick price liquidity volume swap solana",
            Page::PlanetaryPulse => "planetary pulse planet wildlife biodiversity living planet index nature air pollution airnow openaq births deaths population climate environment earth пульс планеты природа дикая природа воздух загрязнение рождаемость смертность население",
            Page::OfflineAtlas => "offline atlas offline maps mbtiles satellite imagery gps navigation waypoints offline-ready map tiles офлайн карты спутник спутниковые снимки навигация",
            Page::Browser => "browser cybbrowser web web2 http https ipfs ipns arweave ar decentralized browser",
            Page::System => "system energy battery node database",
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
