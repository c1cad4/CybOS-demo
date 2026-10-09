# External agent framework review for cybOS

Reviewed repositories:
- elizaOS/eliza — https://github.com/elizaOS/eliza
- valory-xyz/open-autonomy — https://github.com/valory-xyz/open-autonomy
- crewAIInc/crewAI — https://github.com/crewAIInc/crewAI

This is an architecture review, not copied framework code. cybOS is a native Rust application, so importing an entire TypeScript or Python runtime would add a second runtime, duplicate persistence and permissions, and complicate packaging. Reuse the proven patterns behind narrow Rust interfaces instead.

## What is worth adopting

### 1. Eliza: capability plugins, not hard-coded integrations

Eliza's runtime separates actions, context providers, evaluators, services, events, and model handlers behind plugin contracts.

**cybOS adaptation**
- Define a small Rust capability/tool contract with a stable ID, human-readable description, input schema, required capabilities, execution deadline, and structured result.
- Keep providers read-only where possible: local memories, knowledge records, CicadaFarm sensor readings, Cybergraph, and network status.
- Keep actions explicit and separately permissioned: create a file, send a message, fetch a URL, or control a device.
- Add evaluators for task acceptance criteria, output validity, policy compliance, and cost limits.
- Load integrations as registered adapters; do not let a model invent arbitrary tool names or bypass the registry.

**Do not copy:** Eliza's whole runtime/app/plugin ecosystem into the Rust binary. If a remote Eliza adapter is ever useful, keep it optional and isolated behind a protocol.

### 2. CrewAI: deterministic workflow shell around agent autonomy

CrewAI distinguishes flexible role-based collaboration (Crews) from event-driven Flows with explicit state, branching, and human input.

**cybOS adaptation**
- Keep the workflow deterministic even when a model proposes the next step.
- Persist a checkpoint after each meaningful transition: task selected, plan proposed, tool invoked, result received, validation completed.
- Support pause, resume, cancel, retry, timeout, and explicit approval gates.
- Treat model output as an untrusted proposal; validate structured output and capability grants before running tools.
- Allow specialist agents later (researcher, builder, reviewer), but require a single auditable task record and acceptance test for the overall result.

**Do not copy:** CrewAI's Python execution model or require a crew of agents for every task. One local agent with a verifiable workflow is a better first milestone.

### 3. Open Autonomy: explicit state machines and distributed agreement

Open Autonomy uses agent services and finite-state-machine rounds, with synchronized state and consensus patterns for multi-agent operation.

**cybOS adaptation**
- Model important workflows as explicit states and legal transitions, rather than relying on prompt instructions.
- For future multi-node tasks, define which messages are proposals, votes, results, and finalized decisions; include task ID, protocol version, signer identity, and replay protection.
- Keep local single-node execution independent from network availability.
- Use consensus only where multiple independent nodes genuinely need to agree. Do not add a blockchain or consensus layer to ordinary local tasks.
- Keep signing and on-chain settlement as separate permissioned capabilities.

**Do not copy:** the full AEA/Tendermint/IPFS deployment stack into the desktop app. It is operationally heavy for the current local-first stage.

## Proposed cybOS execution contract

A task should move through this sequence:

1. **Proposed** — objective, acceptance criteria, budget ceiling, and allowed capabilities are recorded.
2. **Ready** — inputs and permissions are checked; the task can be scheduled.
3. **Planned** — the agent proposes a finite list of typed steps. The plan is stored before execution.
4. **Running** — each step is checked against the capability registry and resource limits.
5. **Checkpointed** — input references, tool name/version, result digest, timestamps, and status are persisted after each step.
6. **Validating** — deterministic checks and, where appropriate, a separate reviewer assess acceptance criteria.
7. **Submitted / Accepted / Rejected** — a human or explicitly configured policy decides acceptance. Rejected work can be retried with a recorded reason.
8. **Settled** — accounting may record a verified income/expense event. This state is not a wallet instruction and does not imply payment has occurred.

Every run should carry a cancellation token/deadline, a bounded compute/tool budget, and a correlation ID. Tool execution must fail closed when permissions are missing. Side effects must not be blindly replayed after a restart; adapters need idempotency keys or a recovery/approval step.

## Implementation order

1. **Next:** add typed tool/capability interfaces and a persisted workflow-run/checkpoint model to the existing Rust runtime.
2. Connect the task queue to a deterministic runner with one safe built-in tool and unit tests.
3. Add a reviewer/evaluator path and visible run timeline in RobotCYB.
4. Add read-only CicadaFarm/knowledge providers.
5. Add peer-to-peer task exchange only after identity, signed messages, replay protection, and E2E chat are solid.
6. Add money-moving integrations last, behind separate explicit grants, spending limits, and user confirmation.

## Acceptance criteria for this design

- A restart can resume from a persisted checkpoint without silently repeating a non-idempotent side effect.
- Every tool invocation is attributable to an agent identity and a task/run ID.
- A model cannot invoke a capability absent from the manifest or permission grant.
- Task result, validator outcome, and accounting record are distinguishable.
- The app remains useful offline; network-connected features are optional.
- Unit and integration tests cover illegal transitions, denied capabilities, timeout/cancellation, malformed tool results, and recovery.

## Source and license handling

The recommendations above are original cybOS design decisions inspired by public project documentation and interfaces; no source files have been copied. Before adapting any concrete implementation, check the exact source file's license and preserve required copyright/attribution notices. Keep framework-specific code behind a narrow adapter rather than mixing incompatible runtimes into the native application.
