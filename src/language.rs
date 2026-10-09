//! Small, dependency-free interface localization for cybOS.
//! English remains the fallback for untranslated module-specific content.

use crate::navigation::Page;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Language {
    English,
    Chinese,
    Russian,
    Hindi,
}

impl Language {
    pub(crate) const ALL: [Language; 4] = [
        Language::English,
        Language::Chinese,
        Language::Russian,
        Language::Hindi,
    ];

    pub(crate) fn from_code(code: &str) -> Self {
        match code {
            "zh" => Self::Chinese,
            "ru" => Self::Russian,
            "hi" => Self::Hindi,
            _ => Self::English,
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "zh",
            Self::Russian => "ru",
            Self::Hindi => "hi",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "中文",
            Self::Russian => "Русский",
            Self::Hindi => "हिन्दी",
        }
    }
}

pub(crate) fn tr(language: Language, key: &str) -> &'static str {
    match (language, key) {
        (_, "language") => match language {
            Language::English => "LANGUAGE",
            Language::Chinese => "语言",
            Language::Russian => "ЯЗЫК",
            Language::Hindi => "भाषा",
        },
        (_, "quick_guide") => match language {
            Language::English => "QUICK GUIDE",
            Language::Chinese => "快速指南",
            Language::Russian => "КРАТКОЕ РУКОВОДСТВО",
            Language::Hindi => "त्वरित मार्गदर्शिका",
        },
        (_, "how_to_use") => match language {
            Language::English => "HOW TO USE",
            Language::Chinese => "使用方法",
            Language::Russian => "КАК ИСПОЛЬЗОВАТЬ",
            Language::Hindi => "कैसे उपयोग करें",
        },
        (_, "start_here") => match language {
            Language::English => "START HERE",
            Language::Chinese => "从这里开始",
            Language::Russian => "НАЧНИТЕ ЗДЕСЬ",
            Language::Hindi => "यहाँ से शुरू करें",
        },
        (_, "good_to_know") => match language {
            Language::English => "GOOD TO KNOW",
            Language::Chinese => "须知",
            Language::Russian => "ВАЖНО ЗНАТЬ",
            Language::Hindi => "जानना ज़रूरी है",
        },
        (_, "search_hint") => match language {
            Language::English => "⌘K  search pages, tokens, cameras…",
            Language::Chinese => "⌘K  搜索页面、代币、摄像头…",
            Language::Russian => "⌘K  поиск разделов, токенов, камер…",
            Language::Hindi => "⌘K  पेज, टोकन, कैमरे खोजें…",
        },
        (_, "go") => match language {
            Language::English => "GO",
            Language::Chinese => "前往",
            Language::Russian => "ОТКРЫТЬ",
            Language::Hindi => "खोलें",
        },
        (_, "event") => match language {
            Language::English => "+ EVENT",
            Language::Chinese => "+ 事件",
            Language::Russian => "+ СОБЫТИЕ",
            Language::Hindi => "+ घटना",
        },
        (_, "tip") => match language {
            Language::English => "TIP: use ⌘K to find a page; hover over icons and buttons for hints.",
            Language::Chinese => "提示：使用 ⌘K 查找页面；将鼠标悬停在图标和按钮上查看说明。",
            Language::Russian => "Совет: нажмите ⌘K для поиска раздела; наведите курсор на значки и кнопки для подсказок.",
            Language::Hindi => "सुझाव: पेज खोजने के लिए ⌘K दबाएँ; संकेतों के लिए आइकन और बटन पर कर्सर रखें।",
        },
        (_, "live_node") => match language {
            Language::English => "LIVE NODE",
            Language::Chinese => "实时节点",
            Language::Russian => "УЗЕЛ",
            Language::Hindi => "लाइव नोड",
        },
        (_, "qwen") => "QWEN",
        (_, "memory") => match language {
            Language::English => "MEMORY",
            Language::Chinese => "记忆",
            Language::Russian => "ПАМЯТЬ",
            Language::Hindi => "मेमोरी",
        },
        (_, "energy") => match language {
            Language::English => "ENERGY",
            Language::Chinese => "能源",
            Language::Russian => "ЭНЕРГИЯ",
            Language::Hindi => "ऊर्जा",
        },
        (_, "environment") => match language {
            Language::English => "ENVIRONMENT",
            Language::Chinese => "环境",
            Language::Russian => "СРЕДА",
            Language::Hindi => "पर्यावरण",
        },
        (_, "activity") => match language {
            Language::English => "ACTIVITY",
            Language::Chinese => "活动",
            Language::Russian => "АКТИВНОСТЬ",
            Language::Hindi => "गतिविधि",
        },
        (_, "tokens") => match language {
            Language::English => "TOKENS",
            Language::Chinese => "代币",
            Language::Russian => "ТОКЕНЫ",
            Language::Hindi => "टोकन",
        },
        (_, "dashboard") => match language {
            Language::English => "CENTRAL CYBOS DASHBOARD",
            Language::Chinese => "cybOS 控制面板",
            Language::Russian => "ГЛАВНАЯ ПАНЕЛЬ CYBOS",
            Language::Hindi => "cybOS मुख्य डैशबोर्ड",
        },
        (_, "graph") => match language {
            Language::English => "CYBOS GRAPH",
            Language::Chinese => "cybOS 知识图谱",
            Language::Russian => "ГРАФ CYBOS",
            Language::Hindi => "cybOS ग्राफ",
        },
        (_, "network") => match language {
            Language::English => "NETWORK MATRIX",
            Language::Chinese => "网络矩阵",
            Language::Russian => "СЕТЕВАЯ МАТРИЦА",
            Language::Hindi => "नेटवर्क मैट्रिक्स",
        },
        (_, "radar") => match language {
            Language::English => "CYB RADAR",
            Language::Chinese => "CYB 雷达",
            Language::Russian => "РАДАР CYB",
            Language::Hindi => "CYB रडार",
        },
        (_, "brain") => match language {
            Language::English => "CYBOS BRAIN",
            Language::Chinese => "cybOS 智能核心",
            Language::Russian => "МОЗГ CYBOS",
            Language::Hindi => "cybOS मस्तिष्क",
        },
        (_, "farm") => match language {
            Language::English => "CICADAFARM",
            Language::Chinese => "CICADAFARM 农场",
            Language::Russian => "ФЕРМА CICADAFARM",
            Language::Hindi => "CICADAFARM फ़ार्म",
        },
        (_, "robot") => match language {
            Language::English => "ROBOTCYB",
            Language::Chinese => "ROBOTCYB 机器人",
            Language::Russian => "ROBOTCYB",
            Language::Hindi => "ROBOTCYB रोबोट",
        },
        (_, "offline_atlas") => match language {
            Language::English => "OFFLINE ATLAS",
            Language::Chinese => "离线地图集",
            Language::Russian => "ОФЛАЙН-АТЛАС",
            Language::Hindi => "ऑफ़लाइन एटलस",
        },
        (_, "planetary_pulse") => match language {
            Language::English => "PLANETARY PULSE",
            Language::Chinese => "地球脉搏",
            Language::Russian => "ПУЛЬС ПЛАНЕТЫ",
            Language::Hindi => "ग्रह की धड़कन",
        },
        (_, "pulse_subtitle") => match language {
            Language::English => "PUBLIC DATA · SOURCE-ATTRIBUTED",
            Language::Chinese => "公开数据 · 标注来源",
            Language::Russian => "ОТКРЫТЫЕ ДАННЫЕ · ИСТОЧНИКИ УКАЗАНЫ",
            Language::Hindi => "सार्वजनिक डेटा · स्रोत दिए गए",
        },
        (_, "pulse_intro") => match language {
            Language::English => "A view of the living world and humanity — not a fabricated real-time counter.",
            Language::Chinese => "观察生命世界与人类，而不是虚构的实时计数器。",
            Language::Russian => "Панель о живом мире и человечестве — не выдуманный счётчик в реальном времени.",
            Language::Hindi => "जीव-जगत और मानवता का अवलोकन — कोई मनगढ़ंत लाइव काउंटर नहीं।",
        },
        (_, "pulse_wildlife") => match language {
            Language::English => "◉ WILDLIFE",
            Language::Chinese => "◉ 野生动物",
            Language::Russian => "◉ ДИКАЯ ПРИРОДА",
            Language::Hindi => "◉ वन्यजीव",
        },
        (_, "pulse_avg_change") => match language {
            Language::English => "average change in monitored wildlife population abundance",
            Language::Chinese => "受监测野生动物种群数量的平均变化",
            Language::Russian => "среднее изменение численности наблюдаемых популяций диких животных",
            Language::Hindi => "निगरानी की गई वन्यजीव आबादी की संख्या में औसत बदलाव",
        },
        (_, "pulse_index_baseline") => match language {
            Language::English => "2022 index ≈ 27% of the 1970 baseline",
            Language::Chinese => "2022 年指数约为 1970 年基准的 27%",
            Language::Russian => "Индекс 2022 года ≈ 27% от уровня 1970 года",
            Language::Hindi => "2022 सूचकांक ≈ 1970 के आधार स्तर का 27%",
        },
        (_, "pulse_wildlife_period") => match language {
            Language::English => "1970–2022 · Living Planet Index 2026",
            Language::Chinese => "1970–2022 · 2026 年地球生命指数",
            Language::Russian => "1970–2022 · Living Planet Index 2026",
            Language::Hindi => "1970–2022 · लिविंग प्लैनेट इंडेक्स 2026",
        },
        (_, "pulse_wildlife_coverage") => match language {
            Language::English => "35,803 populations · 5,790 vertebrate species",
            Language::Chinese => "35,803 个种群 · 5,790 种脊椎动物",
            Language::Russian => "35 803 популяции · 5 790 видов позвоночных",
            Language::Hindi => "35,803 आबादियाँ · 5,790 कशेरुकी प्रजातियाँ",
        },
        (_, "pulse_wildlife_caveat") => match language {
            Language::English => "Important: this is not a claim that 73% of all animals or species disappeared.",
            Language::Chinese => "注意：这不表示所有动物或物种中有 73% 已经消失。",
            Language::Russian => "Важно: это не означает, что исчезли 73% всех животных или видов.",
            Language::Hindi => "ध्यान दें: इसका अर्थ यह नहीं कि सभी जानवरों या प्रजातियों में से 73% गायब हो गए।",
        },
        (_, "pulse_open_lpi") => match language {
            Language::English => "OPEN LIVING PLANET DATA ↗",
            Language::Chinese => "查看地球生命指数 ↗",
            Language::Russian => "ОТКРЫТЬ ДАННЫЕ LPI ↗",
            Language::Hindi => "लिविंग प्लैनेट डेटा खोलें ↗",
        },
        (_, "pulse_air") => match language {
            Language::English => "◌ AIR & POLLUTION",
            Language::Chinese => "◌ 空气与污染",
            Language::Russian => "◌ ВОЗДУХ И ЗАГРЯЗНЕНИЕ",
            Language::Hindi => "◌ हवा और प्रदूषण",
        },
        (_, "pulse_no_live") => match language {
            Language::English => "NO LIVE FEED",
            Language::Chinese => "无实时数据流",
            Language::Russian => "НЕТ ПРЯМОГО ПОТОКА",
            Language::Hindi => "लाइव फ़ीड नहीं",
        },
        (_, "pulse_air_unavailable") => match language {
            Language::English => "No air-quality provider or monitoring location is connected in this build.",
            Language::Chinese => "此版本尚未连接空气质量服务或监测地点。",
            Language::Russian => "В этой сборке не подключены поставщик данных о воздухе и точка мониторинга.",
            Language::Hindi => "इस बिल्ड में वायु गुणवत्ता प्रदाता या निगरानी स्थान जुड़ा नहीं है।",
        },
        (_, "pulse_openaq_note") => match language {
            Language::English => "OpenAQ aggregates public sensor measurements, but global coverage is incomplete.",
            Language::Chinese => "OpenAQ 汇总公开传感器测量值，但全球覆盖并不完整。",
            Language::Russian => "OpenAQ собирает открытые измерения датчиков, но глобальный охват неполон.",
            Language::Hindi => "OpenAQ सार्वजनिक सेंसर माप एकत्र करता है, लेकिन वैश्विक कवरेज अधूरा है।",
        },
        (_, "pulse_open_openaq") => match language {
            Language::English => "OPEN OPENAQ ↗",
            Language::Chinese => "打开 OpenAQ ↗",
            Language::Russian => "ОТКРЫТЬ OPENAQ ↗",
            Language::Hindi => "OPENAQ खोलें ↗",
        },
        (_, "pulse_open_airnow") => match language {
            Language::English => "OPEN AIRNOW WIDGETS ↗",
            Language::Chinese => "打开 AirNow 小组件 ↗",
            Language::Russian => "ОТКРЫТЬ ВИДЖЕТЫ AIRNOW ↗",
            Language::Hindi => "AIRNOW विजेट खोलें ↗",
        },
        (_, "pulse_humanity") => match language {
            Language::English => "◎ HUMANITY",
            Language::Chinese => "◎ 人类",
            Language::Russian => "◎ ЧЕЛОВЕЧЕСТВО",
            Language::Hindi => "◎ मानवता",
        },
        (_, "pulse_population") => match language {
            Language::English => "estimated world population · 2025",
            Language::Chinese => "全球人口估算 · 2025 年",
            Language::Russian => "оценка населения мира · 2025",
            Language::Hindi => "विश्व जनसंख्या का अनुमान · 2025",
        },
        (_, "pulse_births") => match language {
            Language::English => "BORN IN 2025",
            Language::Chinese => "2025 年出生",
            Language::Russian => "РОДИЛОСЬ В 2025",
            Language::Hindi => "2025 में जन्म",
        },
        (_, "pulse_deaths") => match language {
            Language::English => "DIED IN 2025",
            Language::Chinese => "2025 年死亡",
            Language::Russian => "УМЕРЛО В 2025",
            Language::Hindi => "2025 में मृत्यु",
        },
        (_, "pulse_demographic_period") => match language {
            Language::English => "UN World Population Prospects 2024 · annual estimates/projections, not live counts",
            Language::Chinese => "联合国《世界人口展望 2024》· 年度估算/预测，非实时计数",
            Language::Russian => "ООН World Population Prospects 2024 · годовые оценки/прогнозы, не live-счётчик",
            Language::Hindi => "UN World Population Prospects 2024 · वार्षिक अनुमान/पूर्वानुमान, लाइव गिनती नहीं",
        },
        (_, "pulse_births_deaths") => match language {
            Language::English => "BIRTHS & DEATHS",
            Language::Chinese => "出生与死亡",
            Language::Russian => "РОЖДЕНИЯ И СМЕРТИ",
            Language::Hindi => "जन्म और मृत्यु",
        },
        (_, "pulse_demographic_note") => match language {
            Language::English => "Annual demographic series are published estimates, not a live count of every birth or death.",
            Language::Chinese => "年度人口数据是已发布的估算值，并非每次出生或死亡的实时计数。",
            Language::Russian => "Годовые демографические ряды — опубликованные оценки, а не подсчёт каждого рождения или смерти в реальном времени.",
            Language::Hindi => "वार्षिक जनसांख्यिकीय श्रृंखलाएँ प्रकाशित अनुमान हैं, हर जन्म या मृत्यु की लाइव गिनती नहीं।",
        },
        (_, "pulse_open_demographics") => match language {
            Language::English => "OPEN OWID DEMOGRAPHICS ↗",
            Language::Chinese => "打开 OWID 人口数据 ↗",
            Language::Russian => "ОТКРЫТЬ ДЕМОГРАФИЮ OWID ↗",
            Language::Hindi => "OWID जनसांख्यिकी खोलें ↗",
        },
        (_, "pulse_integrity") => match language {
            Language::English => "DATA INTEGRITY",
            Language::Chinese => "数据完整性",
            Language::Russian => "ЦЕЛОСТНОСТЬ ДАННЫХ",
            Language::Hindi => "डेटा अखंडता",
        },
        (_, "pulse_integrity_note") => match language {
            Language::English => "Each metric must show its publisher, reference period, last update and coverage. Estimates and projections must be visibly labelled. Missing data stays missing — cybOS will not invent measurements.",
            Language::Chinese => "每项指标都应显示发布方、参考时期、更新时间和覆盖范围。估算与预测必须明确标注。缺失数据保持缺失，cybOS 不会编造测量值。",
            Language::Russian => "У каждой метрики должны быть указаны источник, период, дата обновления и охват. Оценки и прогнозы маркируются отдельно. Нет данных — значит нет данных: cybOS не выдумывает измерения.",
            Language::Hindi => "हर संकेतक में प्रकाशक, संदर्भ अवधि, अंतिम अपडेट और कवरेज दिखना चाहिए। अनुमान और पूर्वानुमान स्पष्ट रूप से चिह्नित हों। डेटा न हो तो उसे गढ़ा नहीं जाएगा।",
        },
        (_, "pulse_next_integration") => match language {
            Language::English => "Next integration: population, annual births and deaths, with source year and uncertainty shown on each metric.",
            Language::Chinese => "下一步：接入人口、年度出生和死亡数据，并为每项指标显示来源年份和不确定性。",
            Language::Russian => "Следующий этап: подключить население, годовые рождения и смерти, показывая год источника и неопределённость каждой метрики.",
            Language::Hindi => "अगला चरण: जनसंख्या, वार्षिक जन्म और मृत्यु डेटा जोड़ना तथा हर संकेतक का स्रोत वर्ष और अनिश्चितता दिखाना।",
        },
        (_, "pulse_current_state") => match language {
            Language::English => "Current state: source-linked prototype · external data is not yet ingested or refreshed automatically.",
            Language::Chinese => "当前状态：已链接来源的原型 · 尚未自动导入或刷新外部数据。",
            Language::Russian => "Текущее состояние: прототип со ссылками на источники · внешние данные ещё не загружаются и не обновляются автоматически.",
            Language::Hindi => "वर्तमान स्थिति: स्रोत-लिंक वाला प्रोटोटाइप · बाहरी डेटा अभी स्वतः आयात या रीफ़्रेश नहीं होता।",
        },
        (_, "system") => match language {
            Language::English => "SYSTEM CORE",
            Language::Chinese => "系统核心",
            Language::Russian => "ЯДРО СИСТЕМЫ",
            Language::Hindi => "सिस्टम कोर",
        },
        (_, "chat") => match language {
            Language::English => "CYBCHAT",
            Language::Chinese => "CYBCHAT 聊天",
            Language::Russian => "CYBCHAT",
            Language::Hindi => "CYBCHAT चैट",
        },
        (_, "assets") => match language {
            Language::English => "ASSETS",
            Language::Chinese => "资产",
            Language::Russian => "АКТИВЫ",
            Language::Hindi => "एसेट",
        },
        (_, "cameras") => match language {
            Language::English => "FARM CAMERAS",
            Language::Chinese => "农场摄像头",
            Language::Russian => "КАМЕРЫ ФЕРМЫ",
            Language::Hindi => "फ़ार्म कैमरे",
        },
        (_, "cyblex") => match language {
            Language::English => "CYBLEX",
            Language::Chinese => "CYBLEX 文件",
            Language::Russian => "CYBLEX",
            Language::Hindi => "CYBLEX फ़ाइलें",
        },
        (_, "cybdex") => match language {
            Language::English => "CYBDEX · MARKET",
            Language::Chinese => "CYBDEX · 市场",
            Language::Russian => "CYBDEX · РЫНОК",
            Language::Hindi => "CYBDEX · बाज़ार",
        },
        (_, "browser") => match language {
            Language::English => "CYBBROWSER",
            Language::Chinese => "CYBBROWSER 浏览器",
            Language::Russian => "CYBBROWSER",
            Language::Hindi => "CYBBROWSER ब्राउज़र",
        },
        (_, "chat_subtitle") => match language {
            Language::English => "MATHEMATICAL CHANNEL · LOCAL-FIRST · DIRECT PEERS · LAN · P2P · NOSTR",
            Language::Chinese => "数学频道 · 本地优先 · 直连节点 · 局域网 · P2P · NOSTR",
            Language::Russian => "МАТЕМАТИЧЕСКИЙ КАНАЛ · ЛОКАЛЬНО · ПРЯМЫЕ УЗЛЫ · LAN · P2P · NOSTR",
            Language::Hindi => "गणितीय चैनल · लोकल-फर्स्ट · सीधे पीयर · LAN · P2P · NOSTR",
        },
        (_, "peers") => match language {
            Language::English => "∴ PEERS",
            Language::Chinese => "∴ 节点",
            Language::Russian => "∴ УЗЛЫ",
            Language::Hindi => "∴ पीयर",
        },
        (_, "no_peers") => match language {
            Language::English => "No direct LAN peers discovered.",
            Language::Chinese => "未发现可直连的局域网节点。",
            Language::Russian => "Прямые узлы LAN не обнаружены.",
            Language::Hindi => "कोई सीधा LAN पीयर नहीं मिला।",
        },
        (_, "select") => match language {
            Language::English => "SELECT",
            Language::Chinese => "选择",
            Language::Russian => "ВЫБРАТЬ",
            Language::Hindi => "चुनें",
        },
        (_, "target") => match language {
            Language::English => "TARGET",
            Language::Chinese => "目标",
            Language::Russian => "ЦЕЛЬ",
            Language::Hindi => "लक्ष्य",
        },
        (_, "network_subtitle") => match language {
            Language::English => "ONE NODE IDENTITY · DISCOVERY · DIRECT DELIVERY · NO FAKE CONNECTIONS",
            Language::Chinese => "统一节点身份 · 发现 · 直接传输 · 不伪造连接",
            Language::Russian => "ЕДИНАЯ ИДЕНТИЧНОСТЬ УЗЛА · ПОИСК · ПРЯМАЯ ДОСТАВКА · БЕЗ ИМИТАЦИИ СОЕДИНЕНИЙ",
            Language::Hindi => "एक नोड पहचान · खोज · सीधा संदेश · नकली कनेक्शन नहीं",
        },
        (_, "radar_visibility") => match language {
            Language::English => "RADAR VISIBILITY",
            Language::Chinese => "雷达可见性",
            Language::Russian => "ВИДИМОСТЬ РАДАРА",
            Language::Hindi => "रडार दृश्यता",
        },
        (_, "visible") => match language {
            Language::English => "VISIBLE",
            Language::Chinese => "可见",
            Language::Russian => "ВИДИМ",
            Language::Hindi => "दृश्य",
        },
        (_, "hidden") => match language {
            Language::English => "HIDDEN",
            Language::Chinese => "隐藏",
            Language::Russian => "СКРЫТ",
            Language::Hindi => "छिपा हुआ",
        },
        (_, "brain_subtitle") => match language {
            Language::English => "MEMORY · GRAPH · LOCAL AI · NO FAKE CLOUD BRAIN",
            Language::Chinese => "记忆 · 图谱 · 本地 AI · 不伪造云端智能",
            Language::Russian => "ПАМЯТЬ · ГРАФ · ЛОКАЛЬНЫЙ ИИ · БЕЗ ИМИТАЦИИ ОБЛАЧНОГО МОЗГА",
            Language::Hindi => "मेमोरी · ग्राफ़ · लोकल AI · नकली क्लाउड ब्रेन नहीं",
        },
        (_, "qwen_setup") => match language {
            Language::English => "QWEN FIRST-RUN SETUP",
            Language::Chinese => "QWEN 首次设置",
            Language::Russian => "ПЕРВОНАЧАЛЬНАЯ НАСТРОЙКА QWEN",
            Language::Hindi => "QWEN पहली बार सेटअप",
        },
        (_, "local_memory") => match language {
            Language::English => "WRITE TO LOCAL MEMORY",
            Language::Chinese => "写入本地记忆",
            Language::Russian => "ЗАПИСАТЬ В ЛОКАЛЬНУЮ ПАМЯТЬ",
            Language::Hindi => "लोकल मेमोरी में लिखें",
        },
        (_, "store") => match language {
            Language::English => "STORE",
            Language::Chinese => "保存",
            Language::Russian => "СОХРАНИТЬ",
            Language::Hindi => "सहेजें",
        },
        (_, "public_identifiers") => match language {
            Language::English => "◇ PUBLIC IDENTIFIERS",
            Language::Chinese => "◇ 公共标识",
            Language::Russian => "◇ ПУБЛИЧНЫЕ ИДЕНТИФИКАТОРЫ",
            Language::Hindi => "◇ सार्वजनिक पहचानकर्ता",
        },
        (_, "physical_world") => match language {
            Language::English => "PHYSICAL WORLD",
            Language::Chinese => "现实世界",
            Language::Russian => "ФИЗИЧЕСКИЙ МИР",
            Language::Hindi => "भौतिक दुनिया",
        },
        (_, "digital_world") => match language {
            Language::English => "DIGITAL WORLD",
            Language::Chinese => "数字世界",
            Language::Russian => "ЦИФРОВОЙ МИР",
            Language::Hindi => "डिजिटल दुनिया",
        },
        (_, "copy") => match language {
            Language::English => "COPY",
            Language::Chinese => "复制",
            Language::Russian => "КОПИРОВАТЬ",
            Language::Hindi => "कॉपी करें",
        },
        (_, "system_subtitle") => match language {
            Language::English => "NATIVE DESKTOP RUNTIME · NO BROWSER SHELL",
            Language::Chinese => "原生桌面运行时 · 无浏览器外壳",
            Language::Russian => "НАТИВНАЯ СРЕДА РАБОЧЕГО СТОЛА · БЕЗ БРАУЗЕРНОЙ ОБОЛОЧКИ",
            Language::Hindi => "नेटिव डेस्कटॉप रनटाइम · ब्राउज़र शेल नहीं",
        },
        (_, "check_database") => match language {
            Language::English => "CHECK DATABASE",
            Language::Chinese => "检查数据库",
            Language::Russian => "ПРОВЕРИТЬ БАЗУ",
            Language::Hindi => "डेटाबेस जाँचें",
        },
        (_, "export_state") => match language {
            Language::English => "EXPORT STATE",
            Language::Chinese => "导出状态",
            Language::Russian => "ЭКСПОРТ СОСТОЯНИЯ",
            Language::Hindi => "स्थिति निर्यात करें",
        },
        (_, "copy_node_id") => match language {
            Language::English => "COPY NODE ID",
            Language::Chinese => "复制节点 ID",
            Language::Russian => "КОПИРОВАТЬ ID УЗЛА",
            Language::Hindi => "नोड ID कॉपी करें",
        },
        (_, "open_data_folder") => match language {
            Language::English => "OPEN DATA FOLDER",
            Language::Chinese => "打开数据文件夹",
            Language::Russian => "ОТКРЫТЬ ПАПКУ ДАННЫХ",
            Language::Hindi => "डेटा फ़ोल्डर खोलें",
        },
        (_, "write_system_event") => match language {
            Language::English => "WRITE SYSTEM EVENT",
            Language::Chinese => "写入系统事件",
            Language::Russian => "ЗАПИСАТЬ СОБЫТИЕ СИСТЕМЫ",
            Language::Hindi => "सिस्टम इवेंट लिखें",
        },
        _ => "",
    }
}

pub(crate) fn page_title(language: Language, page: Page) -> &'static str {
    let key = match page {
        Page::Dashboard => "dashboard",
        Page::Graph => "graph",
        Page::Network => "network",
        Page::Radar => "radar",
        Page::Brain => "brain",
        Page::Farm => "farm",
        Page::Robot => "robot",
        Page::System => "system",
        Page::Chat => "chat",
        Page::Assets => "assets",
        Page::Cameras => "cameras",
        Page::CybLex => "cyblex",
        Page::CybDex => "cybdex",
        Page::PlanetaryPulse => "planetary_pulse",
        Page::OfflineAtlas => "offline_atlas",
        Page::Browser => "browser",
    };
    tr(language, key)
}

pub(crate) fn page_guidance(language: Language, page: Page) -> (&'static str, &'static str, &'static str) {
    if language == Language::English {
        return page.guidance();
    }
    match (language, page) {
        (Language::Chinese, Page::Dashboard) => ("查看 cybOS 模块、本机节点状态和近期活动。", "选择地图中的模块，或使用左侧导航栏。", "状态卡片反映应用的本地状态；演示数据不一定来自实时传感器。"),
        (Language::Chinese, Page::Robot) => ("向 RobotCYB 提问，并定义可验证结果的任务。", "输入请求，然后创建包含验收标准的任务提案。", "任务预算只是记账限制；工作流检查不授权付款或物理操作。"),
        (Language::Chinese, Page::Farm) => ("集中记录 CicadaFarm 的动物、植物、观察和农场活动。", "选择农场区域，记录观察或查看农场状态。", "只有明确连接并报告的数据才应视为实时测量。"),
        (Language::Chinese, Page::Chat) => ("在本地与 RobotCYB 通信，或在安全通道可用时联系已发现的节点。", "先尝试本地对话；使用节点聊天前请核实对方身份。", "首次发现的节点并未自动验证，请与对方核对指纹。"),
        (Language::Chinese, Page::Graph) => ("探索本地知识图谱中的节点和关系。", "选择节点查看详情，并缩放或平移图谱。", "图谱中的关系只是已存储的数据，并不证明其真实性。"),
        (Language::Chinese, Page::Brain) => ("查看本地记忆、图谱知识和本地 AI 模型状态。", "先检查 Qwen 状态；若离线，请按照本页的首次设置说明操作。", "没有 Qwen 也可启动 cybOS，但 AI 请求可能无法完成。"),
        (Language::Chinese, Page::Network) => ("查看本地网络发现和节点连接状态。", "扫描并选择节点，发送前检查其状态。", "发现节点并不代表已建立信任。"),
        (Language::Chinese, Page::Radar) => ("查看通过支持的局域网或蓝牙机制发现的附近节点。", "仅在需要被发现时开启可见性，然后刷新发现列表。", "可见性可能暴露节点附近存在；不需要时请关闭。"),
        (Language::Chinese, Page::Assets) => ("查看已配置的代币和钱包信息。", "复制或分享前请检查地址和余额。", "市场或余额数据可能延迟；切勿向 cybOS 输入助记词或私钥。"),
        (Language::Chinese, Page::PlanetaryPulse) => ("使用有来源标注的公开数据观察生物多样性、空气质量和人类指标。", "查看每张卡片的数据日期和来源说明。", "野生动物指数表示受监测脊椎动物种群的平均变化，不代表所有动物的损失数量。未连接数据源时，空气质量不是实时数据。"),
        (Language::Chinese, Page::Cameras) => ("查看已配置农场摄像头的来源和连接状态。", "选择摄像头区域并检查状态。", "已配置的来源不一定已连接。"),
        (Language::Chinese, Page::CybLex) => ("管理已授权的文件下载、共享和种子任务。", "选择磁力链接或种子网址，核实来源并选择保存目录。", "仅共享你拥有或获准分发的内容。"),
        (Language::Chinese, Page::CybDex) => ("以只读方式查看 Solana 交易对、流动性、成交量和价格历史。", "搜索代币符号或 mint，选择交易对和图表周期。", "行情可能延迟；本页不会签名或执行兑换。"),
        (Language::Chinese, Page::Browser) => ("查看网页和支持的去中心化协议地址中的文本。", "输入完整网址，例如 https://example.com，然后点击打开。", "此处不会执行远程 JavaScript；请将页面内容和链接视为不可信。"),
        (Language::Chinese, Page::System) => ("检查本地运行状态、数据库和系统组件。", "查看 ERROR 或 OFFLINE 状态，并打开相关模块。", "READY 仅表示本地组件报告就绪，不保证外部服务可访问。"),
        (Language::Russian, Page::Dashboard) => ("Обзор модулей cybOS, состояния локального узла и недавних событий.", "Выберите модуль на карте или воспользуйтесь левой панелью навигации.", "Карточки показывают локальное состояние; демонстрационные значения могут не быть показаниями датчиков."),
        (Language::Russian, Page::Robot) => ("Общайтесь с RobotCYB и задавайте задачи с проверяемым результатом.", "Введите запрос и создайте предложение задачи с критериями приёмки.", "Бюджет задачи — только учётное ограничение; проверки не разрешают платежи или физические действия."),
        (Language::Russian, Page::Farm) => ("Записывайте наблюдения, животных, растения и события CicadaFarm.", "Выберите раздел фермы и внесите наблюдение или проверьте состояние.", "Считайте данными датчиков только явно подключённые и переданные измерения."),
        (Language::Russian, Page::Chat) => ("Общайтесь локально с RobotCYB или с обнаруженными узлами при наличии защищённого канала.", "Начните с локального чата; перед общением с узлом проверьте его личность.", "Новый узел не считается проверенным: сравните отпечаток с собеседником."),
        (Language::Russian, Page::Graph) => ("Изучайте узлы и связи локального графа знаний.", "Выберите узел, чтобы увидеть сведения; масштабируйте и перемещайте граф.", "Связь в графе — сохранённые данные, а не доказательство факта."),
        (Language::Russian, Page::Brain) => ("Просматривайте локальную память, знания графа и доступность модели ИИ.", "Сначала проверьте статус Qwen; если он отключён, следуйте инструкции настройки на этой странице.", "cybOS работает и без Qwen, но запросы к ИИ могут не выполниться."),
        (Language::Russian, Page::Network) => ("Проверяйте обнаружение устройств и соединения локальной сети.", "Запустите сканирование, выберите узел и проверьте статус перед отправкой.", "Обнаружение устройства само по себе не означает доверия."),
        (Language::Russian, Page::Radar) => ("Просматривайте ближайшие узлы, обнаруженные через LAN или Bluetooth.", "Включайте видимость только когда хотите, чтобы узел находили, затем обновите поиск.", "Видимость может раскрывать присутствие узла поблизости; отключайте её при необходимости."),
        (Language::Russian, Page::Assets) => ("Просматривайте настроенные токены и сведения о кошельке.", "Проверьте адрес и балансы перед копированием или передачей.", "Данные рынка и баланса могут запаздывать. Не вводите seed-фразу или приватный ключ."),
        (Language::Russian, Page::PlanetaryPulse) => ("Отслеживайте биоразнообразие, качество воздуха и показатели человечества по открытым источникам.", "Проверяйте дату и источник данных в каждой карточке.", "Индекс дикой природы показывает среднее изменение наблюдаемых популяций позвоночных, а не число исчезнувших животных. Без подключённого источника данные о воздухе не являются текущими."),
        (Language::Russian, Page::Cameras) => ("Проверяйте источники и подключение камер фермы.", "Выберите зону камеры и проверьте её состояние.", "Настроенный источник не обязательно передаёт видео."),
        (Language::Russian, Page::CybLex) => ("Управляйте разрешёнными загрузками, раздачами и торрент-задачами.", "Укажите magnet-ссылку или URL торрента, проверьте источник и папку сохранения.", "Распространяйте только контент, которым вы владеете или имеете право делиться."),
        (Language::Russian, Page::CybDex) => ("Просматривайте пары Solana, ликвидность, объём и историю цен в режиме чтения.", "Найдите токен по символу или mint, выберите пару и период графика.", "Рыночные данные могут запаздывать; обмены здесь не подписываются и не выполняются."),
        (Language::Russian, Page::Browser) => ("Просматривайте текст веб-страниц и поддерживаемых децентрализованных адресов.", "Введите полный URL, например https://example.com, и нажмите «Открыть».", "Удалённый JavaScript не выполняется; считайте содержимое и ссылки недоверенными."),
        (Language::Russian, Page::System) => ("Проверяйте состояние локального процесса, базы данных и компонентов системы.", "Обратите внимание на ERROR или OFFLINE и откройте соответствующий модуль.", "READY означает готовность локального компонента, но не гарантирует доступность внешних сервисов."),
        (Language::Hindi, Page::Dashboard) => ("cybOS मॉड्यूल, स्थानीय नोड स्थिति और हाल की गतिविधि का अवलोकन।", "मानचित्र में मॉड्यूल चुनें या बाएँ नेविगेशन का उपयोग करें।", "स्थिति कार्ड स्थानीय स्थिति दिखाते हैं; डेमो मान लाइव सेंसर रीडिंग नहीं भी हो सकते।"),
        (Language::Hindi, Page::Robot) => ("RobotCYB से बात करें और जाँचने योग्य परिणाम वाले कार्य बनाएँ।", "अनुरोध दर्ज करें और स्वीकृति मानदंडों वाला कार्य प्रस्ताव बनाएँ।", "कार्य बजट केवल लेखांकन सीमा है; वर्कफ़्लो जाँच भुगतान या भौतिक कार्रवाई की अनुमति नहीं देती।"),
        (Language::Hindi, Page::Farm) => ("CicadaFarm के जानवरों, पौधों, अवलोकनों और गतिविधियों का रिकॉर्ड रखें।", "फ़ार्म अनुभाग चुनें और अवलोकन दर्ज करें या स्थिति देखें।", "केवल स्पष्ट रूप से जुड़े और रिपोर्ट किए गए माप को लाइव डेटा मानें।"),
        (Language::Hindi, Page::Chat) => ("स्थानीय रूप से RobotCYB से बात करें या सुरक्षित चैनल उपलब्ध होने पर नोड से संपर्क करें।", "पहले स्थानीय चैट आज़माएँ; पीयर चैट से पहले पहचान सत्यापित करें।", "पहली बार मिला नोड सत्यापित नहीं है; दूसरे व्यक्ति के साथ फ़िंगरप्रिंट जाँचें।"),
        (Language::Hindi, Page::Graph) => ("स्थानीय ज्ञान ग्राफ़ के नोड और संबंध देखें।", "विवरण देखने के लिए नोड चुनें; ग्राफ़ को ज़ूम या पैन करें।", "ग्राफ़ का संबंध संग्रहीत डेटा है, सत्य का प्रमाण नहीं।"),
        (Language::Hindi, Page::Brain) => ("स्थानीय मेमोरी, ग्राफ़ ज्ञान और स्थानीय AI मॉडल की उपलब्धता देखें।", "पहले Qwen स्थिति जाँचें; ऑफ़लाइन होने पर इस पेज के सेटअप निर्देश देखें।", "Qwen के बिना भी cybOS खुलता है, लेकिन AI अनुरोध पूरे नहीं हो सकते।"),
        (Language::Hindi, Page::Network) => ("स्थानीय नेटवर्क खोज और पीयर कनेक्टिविटी देखें।", "स्कैन करें, पीयर चुनें और संदेश भेजने से पहले उसकी स्थिति जाँचें।", "डिस्कवरी अपने आप भरोसा स्थापित नहीं करती।"),
        (Language::Hindi, Page::Radar) => ("LAN या Bluetooth से खोजे गए नज़दीकी नोड देखें।", "केवल खोजे जाने की इच्छा होने पर दृश्यता चालू करें, फिर खोज रीफ़्रेश करें।", "दृश्यता से पास में आपके नोड की मौजूदगी पता चल सकती है।"),
        (Language::Hindi, Page::Assets) => ("कॉन्फ़िगर किए गए टोकन और वॉलेट जानकारी देखें।", "कॉपी या साझा करने से पहले पता और बैलेंस जाँचें।", "बाज़ार डेटा देर से आ सकता है। cybOS में seed phrase या निजी कुंजी न डालें।"),
        (Language::Hindi, Page::PlanetaryPulse) => ("स्रोत-आधारित सार्वजनिक डेटा से जैव विविधता, वायु गुणवत्ता और मानवता के संकेतक देखें।", "हर कार्ड पर डेटा की तारीख और स्रोत जाँचें।", "वन्यजीव सूचकांक निगरानी की गई कशेरुकी आबादियों में औसत बदलाव बताता है, सभी जानवरों की हानि नहीं। स्रोत जुड़ने तक वायु डेटा लाइव नहीं है।"),
        (Language::Hindi, Page::Cameras) => ("फ़ार्म कैमरा स्रोत और कनेक्शन स्थिति देखें।", "कैमरा क्षेत्र चुनें और स्थिति जाँचें।", "कॉन्फ़िगर किया गया स्रोत ज़रूरी नहीं कि लाइव हो।"),
        (Language::Hindi, Page::CybLex) => ("अनुमत फ़ाइल डाउनलोड, शेयरिंग और टोरेंट कार्य प्रबंधित करें।", "मैग्नेट लिंक या टोरेंट URL चुनें, स्रोत जाँचें और फ़ोल्डर चुनें।", "केवल वही सामग्री साझा करें जिसे साझा करने का अधिकार है।"),
        (Language::Hindi, Page::CybDex) => ("Solana जोड़े, लिक्विडिटी, वॉल्यूम और कीमत इतिहास को केवल पढ़ने के मोड में देखें।", "टोकन प्रतीक या mint खोजें, जोड़ा और चार्ट अवधि चुनें।", "बाज़ार डेटा देर से आ सकता है; यहाँ स्वैप साइन या निष्पादित नहीं होते।"),
        (Language::Hindi, Page::Browser) => ("वेब और समर्थित विकेंद्रीकृत पते की टेक्स्ट सामग्री देखें।", "पूरा URL डालें, जैसे https://example.com, और Open दबाएँ।", "दूरस्थ JavaScript नहीं चलता; पेज सामग्री और लिंक को अविश्वसनीय मानें।"),
        (Language::Hindi, Page::System) => ("स्थानीय रनटाइम, डेटाबेस और सिस्टम घटकों की स्थिति देखें।", "ERROR या OFFLINE स्थिति देखें और संबंधित मॉड्यूल खोलें।", "READY स्थानीय घटक की रिपोर्ट है; बाहरी सेवाओं की उपलब्धता की गारंटी नहीं।"),
        _ => page.guidance(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_round_trip() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), language);
        }
        assert_eq!(Language::from_code("unknown"), Language::English);
    }

    #[test]
    fn core_controls_have_translations_for_all_languages() {
        let keys = [
            "language",
            "quick_guide",
            "chat_subtitle",
            "network_subtitle",
            "brain_subtitle",
            "public_identifiers",
            "system_subtitle",
            "pulse_subtitle",
            "pulse_wildlife",
            "pulse_air",
            "pulse_humanity",
            "pulse_population",
            "pulse_births",
            "pulse_deaths",
            "pulse_demographic_period",
            "pulse_next_integration",
            "pulse_integrity",
            "offline_atlas",
        ];
        for language in Language::ALL {
            for key in keys {
                assert!(!tr(language, key).is_empty(), "missing {key} for {}", language.code());
            }
        }
    }

    #[test]
    fn offline_atlas_title_is_localized_for_all_languages() {
        for language in Language::ALL {
            assert_eq!(page_title(language, Page::OfflineAtlas), tr(language, "offline_atlas"));
        }
    }

    #[test]
    fn planetary_pulse_is_localized_for_all_languages() {
        for language in Language::ALL {
            assert_eq!(page_title(language, Page::PlanetaryPulse), tr(language, "planetary_pulse"));
            assert!(!page_guidance(language, Page::PlanetaryPulse).0.is_empty());
            for key in ["pulse_subtitle", "pulse_wildlife", "pulse_air", "pulse_humanity", "pulse_integrity", "pulse_next_integration", "pulse_index_baseline"] {
                assert!(!tr(language, key).is_empty(), "missing {key} for {}", language.code());
            }
        }
    }
}
