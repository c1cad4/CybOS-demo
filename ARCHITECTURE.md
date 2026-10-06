# cybOS architecture

cybOS is a single native desktop product combining:

- RobotCYB — agent / mind
- CicadaFarm — physical-world model / body
- CybChat — communication UI and transport adapters
- Brain — memory, context, knowledge and AI adapters
- Cybergraph — knowledge topology
- Network — BLE/LAN/P2P/Nostr adapters
- Identity — persistent local node identity
- Assets — public token identifiers only

The MVP is local-first. The UI and storage do not require a web browser. SQLite persists events and the local node identity. Cybergraph is represented by a native graph model now; an upstream cybergraph adapter can be integrated without changing the UI layer.
