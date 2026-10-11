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
            Page::Dashboard => "Главная",
            Page::Graph => "Граф знаний",
            Page::Network => "Устройства",
            Page::Radar => "Поиск устройств",
            Page::Brain => "Мозг",
            Page::Farm => "CICADAFARM",
            Page::Robot => "Агенты",
            Page::System => "Диагностика",
            Page::Chat => "CybChat",
            Page::Assets => "ASSETS",
            Page::Cameras => "FARM CAMERAS",
            Page::CybLex => "CybLex",
            Page::CybDex => "CYBDEX · MARKET",
            Page::Browser => "Браузер",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Dashboard => "Ваше пространство для общения, знаний и работы с интернетом.",
            Self::Chat => "Переписывайтесь с подключёнными устройствами через CybChat.",
            Self::Brain | Self::Graph => {
                "Сохраняйте знания локально и исследуйте связи между ними."
            }
            Self::Browser => {
                "Читайте веб-страницы внутри CybOS. Полная анонимность не гарантируется."
            }
            Self::CybLex => "Музыка, фильмы и файлы: загрузка и раздача через BitTorrent.",
            Self::Robot => "Поручите задачу локальному ИИ-помощнику.",
            Self::Network | Self::Radar => {
                "Найдите устройства и подготовьте защищённое соединение."
            }
            Self::System => "Состояние приложения и технические сведения.",
            _ => "Дополнительный модуль CybOS.",
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
            Page::Browser => Icon::Network,
        }
    }

    pub(crate) fn matches_query(self, q: &str) -> bool {
        let keys = match self {
            Page::Dashboard => "dashboard home главная core sigma live node activity",
            Page::Robot => "robot robotcyb agent agents qwen ai робот агенты помощник",
            Page::Farm => "farm cicadafarm hive chicken goat honey eggs environment ферма улей пасека",
            Page::Chat => "chat cybchat message peer чат",
            Page::Graph => "graph cybergraph nodes links граф",
            Page::Brain => "brain memory knowledge qwen мозг память знания",
            Page::Network => "network devices lan p2p ble nostr сеть устройства",
            Page::Radar => "radar proximity nearby peers bluetooth ble nearby узлы друзья",
            Page::Assets => "assets tokens cicadafarm robotcyb mint solana токены токен",
            Page::Cameras => "cameras camera rtsp farm live",
            Page::CybLex => "cyblex сайблекс музыка фильмы файлы библиотека torrent archive bittorrent magnet seed share download p2p files",
            Page::CybDex => "cybdex dex market tokens pairs pools ohlcv candlestick price liquidity volume swap solana",
            Page::Browser => "browser браузер cybbrowser web web2 http https ipfs ipns arweave ar decentralized browser",
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
