# CYBCORE Chain — protocol charter v0.1

Status: experimental design. No blockchain or real token has been deployed.

## Mission

Build an independently verifiable network for cooperation among humans, AI services, robots and projects protecting living systems. No protocol can guarantee perfect security or preservation of life.

## Core architecture

- Consensus: reviewed Byzantine-fault-tolerant engine, initially a four-validator permissioned local devnet; do not invent unreviewed consensus.
- Execution: deterministic state transitions, versioned transactions, gas limits, checked integer arithmetic, replay protection and deterministic encoding. AI inference never runs inside consensus.
- Identity: separate wallet, validator, CybCore node, chat and robot keys. Plan crypto agility and post-quantum authentication; do not claim quantum resistance before implementation and independent audit.
- Economy: valueless test credits first. Genesis-defined supply, auditable issuance, conservation invariants, independent proof-of-service validation, anti-Sybil controls and disputes.
- Robots: blockchain records approvals but never directly actuates physical hardware. Local safety interlocks, explicit human authorization and independent emergency stop remain mandatory.
- Privacy: only minimal commitments and receipts on chain; no private conversations, raw sensor streams, location traces or private keys.
- Storage: encrypted content-addressed data off-chain in CybLex with explicit consent.
- Governance: transparent upgrades, delayed activation, independent audits and incident response.

## First engineering milestone

1. Separate Rust cybchain crate and versioned genesis schema.
2. Deterministic transactions: transfer test credits, register service, record service receipt, revoke capability.
3. Property tests: supply conservation, signature failure, nonce replay, overdraft, overflow, duplicate receipts, deterministic execution.
4. Four local validators using reviewed BFT software, restart and partition tests.
5. Read-only cybOS explorer and opt-in test wallet. No real money or unattended AI spending.

## Useful contribution

Reward verified delivered service, not uptime, idle CPU cycles or installation count. Bind each receipt to request, provider, measured resource, nonce and audit evidence. Arbitrary AI outputs are not cheaply verifiable; use explicit task-specific validation and fraud review. Phones are optional light clients with user-controlled battery and bandwidth limits.

## Public launch gates

Threat model, fuzzing, dependency and supply-chain audit, independent consensus and cryptographic review, economic attack simulations, key recovery, legal review and published incident plan.

## Open decisions

Consensus framework, public validator entry, token issuance, fees, governance, post-quantum wallet signatures, light-client proofs and useful-work verification must be documented as separate architecture decisions before deployment.
