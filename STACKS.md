# cybOS external stacks

cybOS uses separate technology stacks with explicit boundaries. An external
project does not become a hidden dependency of the whole application.

Rule:

`UI → cell → stack adapter/worker → external system`

## Active now

### CYBLEX_RQBIT

`CybLex` embeds `librqbit 9.0.1` inside a dedicated worker. It is not mixed
into Network, Brain or the UI thread.

Responsibilities:
- BitTorrent download
- seeding
- magnet links
- .torrent files
- DHT and trackers
- persistent rqbit session state
- progress and uploaded-byte reporting
- torrent sidecar generation when seeding local content

The user-facing CybLex scope is limited to content the user is entitled to
distribute: owned, public-domain, open-license and creator-authorized content.

## Active but isolated

### CYBDEX_SOLANA_DATA

The current market stack is a read-only data plane using bounded HTTP and
Solana RPC. It does not embed a full validator.

## Next adapters

### FARM_MODBUS → tokio-modbus

Use for industrial telemetry and control at the farm boundary:

`CICADAFARM → Modbus adapter → device`

Expose typed observations, health, timestamps and bounded command results;
keep register maps out of UI and Brain.

### MEMORY_QDRANT → Qdrant

Qdrant is a vector search engine and Apache-2.0 licensed. It is a strong
candidate for local semantic memory, but remains an optional external service.

`MEMORY → embedding worker → Qdrant adapter → vector collection`

SQLite remains the canonical local source of truth.

### KNOWLEDGE_PARADEDB → ParadeDB

Use ParadeDB as an external PostgreSQL search layer when the knowledge corpus
needs BM25/full-text/vector hybrid search at larger scale. It should not
replace the native SQLite state store.

### ROBOT_KLIPPER → Klipper

Use a dedicated adapter for telemetry and bounded machine integration. The
first layer should be read-only status; any later machine action must pass
the RobotCYB action bus and its execution budget.

### BIO_RUST_BIO → Rust-Bio

Use only inside a specialized biological-data worker for bioinformatics
datasets. Do not inject the library into generic farm state.

### AI_RAY → Ray

Ray belongs outside the native Rust process as an external distributed
Python/ML compute service. cybOS should talk to it through a bounded adapter.

## Deliberately excluded

### async-std

Do not add it. The project is discontinued and recommends moving to other async
ecosystems. cybOS already has a Tokio boundary through CybLex.

### full solana-labs/solana node

Do not embed the historical validator repository. Use the appropriate client
and RPC layer for the existing Assets/CybDex plane instead.

## Stack contract

Every future integration must declare:
- owner cell
- transport
- input contract
- output contract
- status
- heartbeat
- execution budget or explicit long-lived service lifecycle
- shutdown behavior
- persistence boundary
- failure state