# cybOS — product and engineering roadmap

This is an engineering decision record, not a claim that every listed capability already exists.

## Product principle

cybOS should be a local-first, cross-platform cybernetic workspace that helps people understand, coordinate and improve the real world. Its cyberpunk interface is the visual language; reliability, privacy, user agency and practical usefulness are the product.

Priorities:
- make real-world projects such as CicadaFarm observable and manageable;
- enable private collaboration across unreliable networks;
- keep knowledge portable, sourced and under the user's control;
- display market/network data without confusing estimates with verified state;
- run local AI agents under explicit permissions and resource budgets;
- connect sensors and hardware without pretending unavailable devices are online.

## Non-duplication rule

Before adding a dependency, determine whether an existing module owns the responsibility, whether it can be extended behind a stable interface, whether the new dependency adds unique value, and whether its license, maintenance, security and platform support are acceptable.

Current ownership boundaries:
- egui owns the native desktop UI and charts; do not embed a second terminal/UI framework for decoration.
- CybDEX owns general Solana pair discovery and market history. Assets should consume a shared market-data service for project tokens instead of duplicating HTTP/cache logic.
- CybLex owns P2P/torrent operations through the existing librqbit engine; do not add a second BitTorrent engine.
- CybChat owns peer identity and private messaging; do not create a parallel identity store.
- Cybergraph/local database owns durable relationships and knowledge; modules should use shared repositories.
- RobotCYB owns agent interaction; agents use capability-scoped tools, explicit budgets and observable status.
- CybBrowser owns browser-cell execution. Keep it separate from native workers unless a documented bridge is necessary.

## Cross-platform strategy

Treat Linux, macOS (Apple Silicon and Intel), and Windows as supported only when each has a tested build and documented installation path.

1. Keep platform-neutral business logic in Rust modules.
2. Put OS-specific behavior behind adapters: identity/key storage, BLE, serial devices, notifications, autostart and packaging.
3. Report unavailable hardware as UNAVAILABLE with a reason; never fake connected state.
4. Add CI for Linux, macOS ARM64 + Intel, and Windows before declaring cross-platform beta.
5. Test clean installs, upgrades, migrations, shutdown/restart and offline startup—not only compilation.
6. Publish checksums, release notes and reproducible build instructions. Add signing/notarization where needed.
7. Keep model weights and large optional assets separate from the core installer; preserve a graceful no-model mode.

## Cyberpunk interface and interaction

The design should be distinctive without reducing legibility:
- use a consistent neon/graphite design system with semantic colors, spacing, typography and icon meanings;
- provide command/navigation search and keyboard shortcuts;
- distinguish live, stale, offline, permission-denied and error states;
- animate only when motion conveys information; respect reduced-motion preferences;
- support keyboard navigation, scalable text, sufficient contrast and screen readers where the toolkit permits;
- provide low-power/reduced-effects mode for laptops and older hardware;
- make important actions explicit and reversible.

## Safety, privacy and user control

- Default to local storage and local AI; disclose external data flows.
- Require opt-in for discovery, sensors, network relays and hardware control.
- Never let an agent sign a transaction, spend funds, deploy a contract or actuate equipment without a separate user-approved permission path.
- Before any future signing feature, show transaction, destination, network, fees and token/program identity.
- Treat price, FDV, liquidity and third-party API data as untrusted, time-sensitive observations—not guarantees or investment advice.
- Protect identity keys with OS key stores where available; document weaker fallbacks.
- Add threat models, dependency/license review, secret scanning, parser/protocol fuzz tests and migration regression tests.
- Minimize telemetry; no hidden tracking or silent data export.

## Roadmap by value and risk

### P0 — trust and release quality
- Resolve and test open pull requests against current main; an open PR is not shipped.
- Keep CI green on every supported target; add Windows CI/package before claiming Windows support.
- Add tests for clean install, persistence/migrations, worker shutdown, offline startup and malformed provider responses.
- Audit cryptographic protocols and key migration; document limitations and run reproducible multi-device LAN tests.
- Correct stale or contradictory version/release documentation.

### P1 — remove duplicated market-data paths
- Extract one internal market-data service used by CybDEX and Assets.
- Keep provider parsing behind adapters: DexScreener and GeckoTerminal; add Bitquery only for a distinct requirement such as event/transaction history.
- Add bounded caching, request coalescing, rate-limit handling, timestamps and stale-data indicators.
- Validate Solana addresses and numeric market/candle values before rendering.
- Improve native candlesticks with volume bars, hover values, zoom, loading/error/empty states and data freshness. Avoid a second chart framework unless egui proves insufficient.

### P2 — real-world utility
- Build a consent-based CicadaFarm dashboard for animal/hive/sensor observations, maintenance and harvest records. Distinguish measured, estimated and manually entered values.
- Add open-format export/import and local backups.
- Add a knowledge/decision log with source, timestamp and provenance.
- Add an offline task board and resilient sync before adding more social features.
- Add a capability registry for sensors/agents with permissions, health, budgets and audit logs.

### P3 — optional network and finance adapters
- Evaluate Bitquery for historical Solana transactions only if existing public APIs do not meet a defined need.
- Evaluate Jupiter or another route-quote adapter as read-only simulation first. A quote adapter is not a wallet or signer.
- Audit trading SDKs, Pump.fun clients and on-chain programs for provenance, license, dependencies, program IDs, upgrade authority, transaction construction and security assumptions.
- Keep signing, trading and deployment separate, opt-in capabilities with independent tests and explicit confirmation.
- Do not add Superseedr beside librqbit, Alacritty inside egui, or multiple overlapping Ratatui chart libraries.

### P4 — long-term public benefit
- Make farm/sensor data useful to the owner first; sharing aggregated observations must be voluntary and privacy-preserving.
- Use open schemas and exports to avoid lock-in.
- Design for intermittent internet, modest hardware and low bandwidth.
- Support collaboration with contributor guidance, architecture decisions and a security-reporting process.
- Prioritize food resilience, ecological observation, accessibility, education and community coordination over engagement metrics or speculative token mechanics.

## Definition of done

A feature is not complete merely because it compiles or has a screen. It needs:
- a clear owner module and documented boundary;
- positive, negative and malformed-input tests;
- bounded resource use and graceful failure;
- honest status reporting and diagnostics;
- accessibility and reduced-motion consideration;
- tested cross-platform behavior or a documented limitation;
- third-party license/provenance review;
- updated documentation and a tested upgrade path when data changes.

## Focused next steps

1. Merge only after CI and review of current security/runtime PRs.
2. Finish market-data validation, then unify the Assets/CybDEX data path.
3. Standardize loading, stale, offline, error and permission states.
4. Establish Windows build/package CI before claiming full desktop multiplatform support.
5. Run real multi-device chat, clean-install and upgrade tests before calling the product release-ready.
