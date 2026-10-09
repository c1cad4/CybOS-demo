# CybCore — optional server for cybOS

CybCore is a **separate, opt-in** backend for collaboration, remote workers, Nostr relays and durable shared data. It is not required to boot cybOS, send local LAN messages, or use the farm dashboard.

## First deployable milestone

This directory introduces a Compose stack: PostgreSQL 16, Redis 7, MinIO and a minimal Rust Axum API with database migrations, public /healthz liveness and bearer-token-protected /readyz database readiness endpoints. **It does not yet provide authenticated application APIs, a Nostr relay, E2E chat bridge, or worker execution.** None of these services should be exposed to the public internet as configured.

Prerequisites: Docker Engine with Compose plugin. From the repository root:

```sh
cd cybcore
cp .env.example .env
# Edit .env: replace ALL placeholder credentials and CYBCORE_STATUS_TOKEN with unique strong secrets.
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
1. Compose private networking and minimal Rust health/readiness API (this PR).
2. Authenticated application APIs, database integration tests and production healthchecks.
3. Signed event ingestion, schema/version limits, tenant-scoped authorization and append-only audit.
4. Agent worker queues, bounded execution, retry/ACK semantics and metrics.
5. Optional Nostr relay and encrypted CybChat delivery; test two-node offline/online recovery.
6. Reproducible macOS .app release and end-to-end smoke tests.

## Internal API diagnostics

The API listens on port 8080 **inside** the internal Docker network; no host port is exposed. `/healthz` reports process liveness without database access. `/readyz` checks PostgreSQL and requires `Authorization: Bearer <CYBCORE_STATUS_TOKEN>`. Neither endpoint is a user-facing application API. The token is an internal diagnostic credential, **not** a replacement for user/node authentication or TLS. Never commit `.env` or print credentials in logs.

## Signed event ingestion (experimental)

`POST /v1/events` accepts a JSON Ed25519 signed event with fields `version`, `event_id`, `author_key` (32-byte hex), `kind`, `created_at_ms`, `content` and `signature` (64-byte hex). The signature covers the JSON serialization of the positional tuple `("cybcore-event-v1", version, event_id, author_key, kind, created_at_ms, content)`. Requests also require the **internal** bearer status token until scoped node authorization is implemented. The event timestamp must be within five minutes of server time; content is limited to 16 KiB. Repeated `(author_key, event_id)` returns HTTP 409. Do not use this endpoint for private chat plaintext or production multi-tenant workloads.

**Security limitation:** the signing public key is self-asserted and is not yet registered to a tenant, device or capability. Signature validity alone does not grant authorization. The API is isolated on the internal Docker network; the shared bearer token is a temporary administrative gate, not per-node authentication. Add allowlists/ACL and rate limiting before exposing any ingestion endpoint externally.

## Trusted nodes and event permissions

Migration `0003_node_registry.sql` introduces a default-deny allowlist. Signed events are rejected with HTTP 403 unless the Ed25519 public key is enabled in `trusted_nodes` **and** explicitly allowed to publish the event's exact `kind` in `node_event_permissions`. No keys or permissions are seeded by default. The internal bearer token is still required, but it is **not** sufficient for publication. Revoking a node is done by setting `trusted_nodes.enabled = FALSE`. Provisioning currently requires a trusted database administrator; do not expose SQL provisioning or accept arbitrary self-registration. Key enrollment, tenant scopes, audit and integration tests remain necessary before production use.
