# CybCore — optional server for cybOS

CybCore is a **separate, opt-in** backend for collaboration, remote workers, Nostr relays and durable shared data. It is not required to boot cybOS, send local LAN messages, or use the farm dashboard.

## First deployable milestone

This directory introduces an infrastructure-only Compose stack: PostgreSQL 16, Redis 7 and MinIO object storage. **It does not yet provide a CybCore API, Nostr relay, authentication, E2E chat bridge, or worker execution.** None of these services should be exposed to the public internet as configured.

Prerequisites: Docker Engine with Compose plugin. From the repository root:

```sh
cd cybcore
cp .env.example .env
# Edit .env: replace ALL placeholder credentials with unique strong secrets.
docker compose up -d
docker compose ps
docker compose down
```

By default services are reachable **only inside the Docker network**; no database or object-store ports are published on the host. An application server can join the `cybcore` network in a later PR.

State persists in named volumes. `docker compose down -v` **deletes** local data; do not use it for routine shutdown. Backups, TLS, account management, audit logs, secret rotation and migrations are required before production.

## Integration boundaries

- **cybOS local node:** owns local SQLite state, Noise identity and offline workflows; remote server outages must not block startup.
- **CybCore server:** owns multi-user authorization, shared event persistence, search and background job coordination.
- **Transport:** Nostr events may carry signed public metadata; never upload private Noise keys or plaintext E2E messages.
- **Workers:** bounded jobs, explicit cancellation, heartbeats, resource budgets, scoped capabilities and durable audit.
- **Identity:** do not equate a Nostr signing key with a Noise static key without an explicit verified binding.

### Milestones
1. Compose healthchecks and private networking (this PR).
2. Rust CybCore API service with authenticated health/status, database migrations and integration tests.
3. Signed event ingestion, schema/version limits, tenant-scoped authorization and append-only audit.
4. Agent worker queues, bounded execution, retry/ACK semantics and metrics.
5. Optional Nostr relay and encrypted CybChat delivery; test two-node offline/online recovery.
6. Reproducible macOS .app release and end-to-end smoke tests.
