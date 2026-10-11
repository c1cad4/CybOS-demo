# cybOS 0.7.2 — CicadaFarm · RobotCYB · CybChat

This repository contains both the browser prototype and the native macOS-first Rust/egui application. The browser layer is the visual prototype; the native layer is the real desktop runtime.

The native implementation lives under `src/`, with the macOS launcher and BLE helper under `START_cybOS.command` and `macos/`.

For the native desktop installation path, see [NATIVE_INSTALL.md](NATIVE_INSTALL.md).

## What is in this repository

### Browser prototype

- Cyberpunk neon-green cybOS interface
- RADAR / contact visualization
- CYBCORE energy visualization and throttle interaction
- SIGNAL GRID with moving data packets
- CYBCHAT visual conversation layer
- CYBERGRAPH / knowledge graph visualization
- Hologram panel
- Interactive terminal
- WARP / SPHERE / ORACLE / SENATE controls
- Runtime status inspector

### Native desktop

- Rust/egui macOS-first application
- Local SQLite persistence
- RobotCYB local Qwen runtime
- CYB RADAR LAN + macOS BLE proximity discovery
- Noise XX secure CYBChat with delivery ACK
- Explicit peer approval with SHA-256 fingerprints and persistent Noise identity pins
- Cybergraph secure-peer relationships
- Native CybLex P2P archive powered by embedded librqbit 9.0.1

See [NATIVE_INSTALL.md](NATIVE_INSTALL.md) for build and packaging instructions.

**Latest hardened release:** [cybOS v0.7.2 for macOS](https://github.com/c1cad4/cybOS/releases/tag/v0.7.2)  

SHA-256 checksum is published as `cybOS-0.7.2-macOS.zip.sha256` alongside the release ZIP.

## CybDEX market terminal

The native desktop build now includes a read-only Solana market terminal.

Architecture:
- DexScreener: pair/token discovery and current pair metrics.
- GeckoTerminal: pool OHLCV candles and on-chain market history.
- Solana RPC: future chain-truth validation for mints, supply and account state.
- Future Jupiter adapter: route quotes before any wallet-signing layer.
- Future own AMM/DEX program: separate on-chain project, not mixed into the read-only terminal.

The current UI provides pair search, pool selection, live price/liquidity/volume, 5m/15m/1h/4h/1d candlesticks, volume and automatic refresh. No swap transaction is created or signed in this phase.

## CybLex P2P archive

The native desktop build now embeds **librqbit 9.0.1** as its BitTorrent engine. CybLex keeps the long-lived torrent session on a dedicated Tokio worker and exposes a native egui control plane.

Current native capabilities:
- download from magnet links or HTTP(S) .torrent URLs;
- choose a local output directory, with `~/Downloads/CybLex` as the default;
- create and seed a torrent from a local file or folder;
- save the generated `.torrent` sidecar and expose a shareable magnet URI;
- pause, resume and forget torrents;
- show progress, upload bytes, state, info hash and output path.

CybLex is for content the user owns or is authorized to distribute, including public-domain, open-license and creator-authorized archives. It is not a catalog of unauthorized material.

## Browser runtime architecture

runtime.js introduces the first explicit cell contract for the browser prototype.

Each logical cell declares:
- inputs — what the cell is allowed to consume
- outputs — what it produces
- status — BOOT, RUNNING, ONLINE, DEGRADED, ERROR, OFFLINE
- heartbeat — freshness of the cell's liveness signal
- budget — cooperative execution budget in milliseconds
- run / overrun / error counters

Current cells:

| Cell | Inputs | Outputs | Budget |
|---|---|---|---:|
| SYSTEM | clock, keyboard, pointer | global state, events | 4 ms |
| RADAR | system state, scan command | contacts, scan history | 4 ms |
| CYBCORE | throttle, warp, power | energy, resonance, status | 5 ms |
| SIGNAL_GRID | core state, throttle | packets, activity | 5 ms |
| CYBCHAT | messages, local events | conversation state | 5 ms |
| CYBERGRAPH | entities, relations, events | nodes, connections, signals | 6 ms |
| HOLOGRAM | system state, target | projection | 4 ms |
| TERMINAL | commands | events, state changes | 8 ms |

### Bounded execution

A browser JavaScript function cannot be safely hard-killed in the middle of synchronous execution. Therefore the demo uses a cooperative bound:

1. execution is measured;
2. overruns are recorded;
3. the cell becomes DEGRADED when it exceeds its budget;
4. errors transition the cell to ERROR;
5. heartbeat freshness detects stalled cells.

runtime-bridge.js currently provides a frame-level heartbeat for the legacy p5 renderer. The next architectural step is to move each real cell into an isolated task/worker boundary so its budget becomes enforceable rather than observational.

## Runtime inspector

Click the RUNTIME indicator in the lower-right corner or press R.

The inspector exposes:
- cell state
- heartbeat freshness
- execution duration vs budget
- run count
- overrun count
- error count
- declared inputs and outputs

This is deliberately visible: cybOS should never claim that a subsystem is connected or alive when the runtime has not observed it.

## Browser prototype limitations

The browser layer is still a prototype, not the final cybOS native runtime.

- The p5 renderer remains a legacy monolithic frame.
- The runtime currently observes that frame through runtime-bridge.js.
- P2P, Bluetooth, Nostr, cameras and external network services are not represented as connected unless a real implementation is added.
- The browser prototype does not provide the native Noise XX private-messaging transport; secure CYBChat is part of the native desktop layer.
- Persistent graph/database state is not part of this browser repository.
- The terminal is a local simulation and does not execute operating-system commands.

## Direction

The intended architecture is:

CicadaFarm = physical world
RobotCYB = mind
CybChat = communication
Cybergraph = memory/relations
cybOS = connection + runtime

The long-term target is a local-first cybernetic operating environment where components communicate through explicit inputs/outputs and bounded execution rather than directly calling arbitrary subsystems.

## Run

Open index.html in a modern browser.

Because the demo loads p5.js from jsDelivr, the first load requires network access. The application itself does not claim external connectivity merely because the renderer loaded.

## Repository structure

CybOS-demo/
├── index.html
├── sketch.js
├── runtime.js
├── runtime-bridge.js
├── style.css
└── README.md

## License

See the repository license configuration.

## User readiness

### Technical beta — READY

The native macOS application is usable now for a technically capable user who can build/package it locally. The core desktop runtime, local persistence, RobotCYB/Qwen path, CYB RADAR, unified LAN+BLE proximity, encrypted CYBChat, TOFU identity, Cybergraph and Assets services are implemented and covered by Linux/macOS CI.

### Public download — READY

| Area | Status |
|---|---|
| Native macOS app | READY |
| Persistence / graph / chat | READY |
| RobotCYB + local Qwen | READY WITH LOCAL QWEN SETUP |
| LAN + BLE Radar | READY / OPT-IN |
| Noise XX secure chat | READY |
| Worker budgets / lifecycle | READY |
| macOS packaging | READY |
| GitHub release publication | v0.7.1 published; v0.7.2 hardening release pending final publication |
| Developer ID signing / notarization | NOT CONFIGURED |
| Hardware / camera integrations | FUTURE LAYER |
| P2P / Nostr fallback | FUTURE LAYER |

Readiness is not expressed as a percentage. Signing/notarization, physical multi-device behavior and recovery remain acceptance work; successful compilation alone does not establish product readiness.

## Runtime stack status

The current native stack is organized as bounded local-first cells:

- **CYB RADAR:** opt-in LAN discovery + macOS BLE RSSI discovery; no GPS and no fabricated meter distances.
- **Unified proximity:** LAN and BLE observations are correlated into one peer projection.
- **CYBChat:** direct TCP transport protected by Noise XX with encrypted frames and delivery ACKs.
- **Identity:** each peer key requires explicit approval after fingerprint comparison; legacy TOFU pins require approval once after upgrade. Changed or malformed keys are rejected.
- **Cybergraph:** secure peers and secure-chat relationships are persisted as graph nodes/links.
- **Bounded execution:** discovery and secure transport use explicit time windows, I/O deadlines and connection limits.

A peer is not considered geographically located merely because BLE RSSI is available.


## Экосистема cybOS

[Роль проекта, команды и границы](docs/ECOSYSTEM.md) · [Карта всех компонентов](https://github.com/c1cad4/cybOS).

### Pinned component setup

Run `python3 scripts/bootstrap_components.py` before Cargo commands after a fresh clone.
The ten sibling libraries are pinned in `components.lock.json`; existing checkouts are preserved.

## Secure CybChat approval

[First contact, upgrade and durable delivery](docs/SECURE_CHAT.md). Unknown receivers receive no message body until their key is approved; incoming messages are acknowledged only after SQLite commit.

### Workspace and device chat

Navigation starts with the knowledge graph and includes chat, network, assistant, knowledge and Cyb Dispatcher. Farm counters are demonstration data, not live sensor readings. CybChat supports saved IPv4 addresses, explicit key approval, connection checks without sending message content, and separate assistant/device conversations. See [secure chat](docs/SECURE_CHAT.md).

### Private device network preparation

Network now prepares separate WireGuard device identities, imports public server invitations, validates private-subnet routing and exports private client profiles for the official WireGuard app. This stage does not activate a VPN. Read-only CLI diagnostics check matching public identities and handshake ages; App Store tunnels are not monitored. See [setup and validation](docs/WIREGUARD_DEVICE_NETWORK.md).

### Current desktop workspace

Navigation includes CybChat, CybBrowser, CybLex, knowledge, the local assistant, devices, tokens, CybDEX and Cyb Dispatcher. Planned global synchronization, agent teams and encrypted USB profiles are not implemented.

For a development macOS build, run `bash scripts/build_browser_macos.sh` after building Rust to package the WebKit companion next to the executable. Release packaging includes it automatically. CybBrowser opens in its own application window.

### Обновлённый интерфейс CybOS

Приложение начинает работу на весь экран; кнопка «Окно / полный экран» в шапке возвращает оконный режим. Знак CybOS виден во всех разделах и включён в macOS-пакеты.

Граф знаний находится первым в меню. Отправленные сообщения и личные заметки атомарно сохраняются в SQLite как пользовательские высказывания, создают узлы графа и используются в контексте локального ИИ. Повторы не создают новые узлы. Это накопление памяти, а не обучение весов модели; данные не синхронизируются с глобальным мозгом.

CybLex показывает каталог фильмов TMDB и личную torrent-библиотеку. В разделе «Подключить каталог TMDB» можно ввести **API Read Access Token**: он сохраняется только в памяти до закрытия приложения. Для автоматического подключения при запуске поддерживается переменная окружения `CYBOS_TMDB_TOKEN`. Каталог обновляется при открытой вкладке раз в шесть часов; ручное обновление доступно сразу. TMDB поставляет описания и обложки, а не сами фильмы. Загрузки запускаются пользователем по magnet/torrent-ссылке; собственные файлы можно перетащить в CybLex для подготовки раздачи. Без токена личная библиотека продолжает работать.

CybDEX показывает реальные пары Solana и свечи, позволяет копировать точные адреса токенов и перейти к обмену в Jupiter через системный браузер. Кошелёк, котировка и подтверждение сделки выполняются в Jupiter; встроенного подписания транзакций в CybOS пока нет. Раздел «Токены» отделяет адреса проекта от демонстрационных балансов.

Открытый Jellyfin рассмотрен как возможный будущий адаптер для личного медиасервера; его серверный код не включён в приложение. Текущий каталог реализован напрямую через TMDB API.

Технический анализ CybDEX: SMA (20), EMA (50) с настраиваемыми периодами; полосы Боллинджера (20, 2σ, стандартное отклонение генеральной совокупности); RSI (14, сглаживание Уайлдера); MACD (12/26/9, SMA-инициализация EMA); объём OHLCV. Индикаторы вычисляются по всей загруженной истории перед масштабированием. Значения до окончания периода прогрева не рисуются. Доступны курсор OHLC/цена/время UTC, масштаб по числу свечей, перемещение по истории, горизонтальные уровни, линии тренда и сетка Фибоначчи. Настройки и разметка сохраняются в SQLite отдельно для каждого адреса пула; незавершённую линию отменяет Esc, последнюю — «Отменить линию». Быстрые кнопки: SOL, USDC, ROBOTCYB и CICADAFARM; поиск выполняется по mint-адресам. Это инструменты анализа, без автоматической торговли или сигналов покупки.

### CybLex streaming prototype and conversations

CybLex can now pass rqbit's seekable, piece-aware file stream to a token-protected loopback HTTP Range server. MP4/MOV use the bundled macOS AVKit helper (`bash scripts/build_player_macos.sh`); other video containers use an installed VLC. Choosing a catalog source never starts a download automatically. TMDB metadata and posters are cached locally; favorites, movie/TV shelves, genres, and an opt-in local Prowlarr magnet search adapter are included. Provider keys are session-only. Real provider integration, subtitle support, in-window codec-capable playback, and playback against remote peers still require validation/work. See [the detailed architecture](docs/CYBLEX_STREAMING_ARCHITECTURE_RU.md).

CybChat now has separate conversations, friends with local names, per-conversation drafts, and small groups with explicit invitations. Group messages fan out over existing authenticated Noise connections to members whose keys have been approved; per-peer delivery results are shown. There is no offline queue or membership-changing protocol yet. All members need compatible clients and direct reachable addresses.

The home menu entry was removed; startup opens the knowledge graph. Application measurements now appear in **Cyb Диспетчер**, including process CPU/RSS on supported systems and real module counters. These measurements exclude separate browser, model, and player processes.

## Platform foundations: current development stage

CybVPN is now a separate page with private-subnet and full-internet WireGuard profile preparation. Export is not activation; there is no integrated kill switch or verified internet tunnel yet. The navigation and common controls have local translations for ten widely used languages plus Russian; internal copy and native Swift helpers are not fully localized. CybMind uses SQLite FTS5 to retrieve older memories. The assistant records durable run lifecycle metadata without replaying tools automatically.

CybRadar has an opt-in dump1090 JSON adapter, distinct from CybChat peers. Cyb Dispatcher has an original read-only process snapshot; sandboxed runs may prohibit process enumeration. The browser helper adds HTTPS-only, session reset and default media-capture denial; it does not claim Tor/Mullvad fingerprint protection. Optional `tools/read_only_markets.py` exports CCXT public spot OHLCV; it neither trades nor lists CYBH.

See [upstream integration review](docs/UPSTREAM_INTEGRATION_RU.md) for every requested source, license observations, implementation boundaries and the development sequence. [CybHero](https://github.com/c1cad4/cybhero) is an unlaunched protocol development project; no mining reward or post-quantum transaction privacy is claimed.

### Localization and Stratux follow-up

The local interface catalog now contains 128 keys in ten widely used languages plus Russian. Browser actions, VPN guidance, WireGuard profile fields, and technical chart controls for SMA/EMA, Bollinger bands, RSI, MACD and drawing tools are localized. Backend errors and native Swift helpers remain partially localized. The aircraft radar imports a recorded Stratux `/traffic` JSON frame or an array of frames using the upstream TrafficInfo schema. It rejects stale/invalid positions and labels imported snapshots; live Stratux WebSocket and AIS reception are not implemented. No radio hardware or VPN server is assumed.
