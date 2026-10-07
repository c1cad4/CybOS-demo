# cybOS 0.7.0

Core release for the native local-first cybOS runtime.

## Communication layer additions

- LAN discovery is broadcast-only; CybChat delivery is directed to a selected peer;
- chat envelopes carry `message_id`, `from`, `to`, `message` and `version`;
- receivers return a bounded delivery acknowledgement;
- end-to-end encryption is not implemented yet, so LAN payloads remain plaintext.

- centralized application configuration and release versioning;
- persistent CybChat history in SQLite;
- background token-market refreshes;
- background Solana balance refreshes;
- real local LAN peer discovery;
- explicit LAN CybChat broadcast;
- thin eframe shell with native UI composition;
- planner JSON tests and navigation/configuration tests;
- Linux and macOS CI coverage.
