# cybOS × Cyberia design adoption

This document records which ideas from the Cyberia CybOS design are useful for
the current native macOS cybOS and which ideas remain future protocol or kernel
research.

## Adopt now

### 1. Identity is a first-class concept

cybOS keeps a persistent node identity in local storage and uses it in every
network operation. The current identity is a stable local node ID, not yet a
cryptographic public-key identity.

The next identity step is a real key-backed identity layer. Until that exists,
the UI must not describe the node ID as cryptographically authenticated.

### 2. Address the message, not just the socket

Cyberia communication separates the logical destination from the transport
endpoint. cybOS now does the same at the LAN protocol level:

    sender node
        |
        v
    message_id + from + to + payload
        |
        v
    selected peer address

LAN discovery may use broadcast; message delivery does not.

### 3. Delivery is a protocol state

A directed CybChat message receives an explicit ACK within a bounded timeout.
This is not a zero-knowledge delivery proof and does not prove route integrity;
it simply gives the application a concrete delivered / timed-out / failed state.

This pattern should later extend to P2P and Cyblex transfers.

### 4. Bounded liveness

The Cyberia design treats non-blocking progress as a system property. cybOS is
already moving network refreshes into background workers and the LAN delivery
path has a bounded wait.

The next runtime step is to give Brain/RobotCYB tasks the same contract:

    REQUEST → THINKING → TOOL → RESPONSE → DONE
                    ________ TIMEOUT ________/

The UI should remain responsive throughout.

### 5. Cells instead of one giant subsystem

The current module layout is already compatible with the "cell" idea: Brain,
Network, Assets, Graph, Store and UI have explicit boundaries.

We should evolve these boundaries into small runtime services with explicit
inputs, outputs, status and heartbeat rather than letting modules call each
other arbitrarily.

### 6. Graph-first state

The Cyberia design treats the graph as infrastructure rather than decoration.
cybOS already persists graph nodes and links separately from page rendering.

The long-term direction is:

    event → domain entity → graph relation → agent context

rather than hard-coded UI counters.

## Deliberately defer

### Advanced private-network protocol research

The Cyberia design also discusses non-interactive key agreement, relay/onion routing,
delivery proofs and deeper protocol-native addressing. Those are separate research layers.

For current cybOS the concrete secure path is:

    stable local node
        ↓
    Noise XX static keys
        ↓
    authenticated encrypted transport
        ↓
    TOFU key persistence
        ↓
    changed-key rejection
        ↓
    bounded delivery ACK + replay cache
        ↓
    optional future relay / onion routing

Secure CYBChat is therefore already part of the current desktop beta; the more advanced
Cyberia-specific protocol is deliberately deferred.

### Onion routing and delivery proofs

Onion routing, relay incentives and recursive proof-of-delivery are future
network protocol work. They should sit above a reliable encrypted transport,
not inside the current UDP discovery implementation.

### "No filesystem" / new kernel

The Cyberia CybOS text describes a true kernel architecture with cells, HAL,
MMIO, a single address space and graph-native persistence. The current target
is a macOS application, so replacing the host OS abstractions would be the
wrong engineering move now.

We can, however, keep the conceptual interfaces compatible with that future:

    cybOS app
      ├── cells / services
      ├── bounded runtime
      ├── graph state
      ├── secure communication
      └── hardware adapters

## Target architecture

    ┌──────────────────────────────────────────────┐
    │                  cybOS UI                    │
    ├──────────────────────────────────────────────┤
    │ Brain · RobotCYB · CybChat · Farm · Graph   │
    ├──────────────────────────────────────────────┤
    │             Bounded Cell Runtime             │
    │       tasks · events · heartbeats            │
    ├──────────────────────────────────────────────┤
    │       Communication / Radio Abstraction      │
    │   LAN · future QUIC/P2P · future Cyblex      │
    ├──────────────────────────────────────────────┤
    │   Identity · Keys · Encrypted Messaging      │
    ├──────────────────────────────────────────────┤
    │          SQLite + Cybergraph state           │
    └──────────────────────────────────────────────┘

The goal is not to copy another OS. The goal is to take its strongest
architectural principles and make them real, testable parts of cybOS.
