# CYBCORE implementation roadmap

## Milestone 0: foundations
- [x] Architecture documents for chain, learning, reputation and agents
- [x] Shared draft JSON schemas for task envelopes and contribution attestations
- [ ] Review threat model and repository boundaries
- [ ] Establish separate repositories with CI and ownership

## Milestone 1: demonstrable learning loop
- [ ] Rust typed task envelope with signature verification and capability checks
- [ ] Two sandboxed agents and bounded runner with cancellation
- [ ] Provenance-aware memory and evidence references
- [ ] Independent evaluation and reproducible task tests
- [ ] Local-only cybOS activity and learning UI

## Milestone 2: infrastructure
- [ ] CybCore API integration and authenticated service discovery
- [ ] CybLex encrypted artifact exchange with consent
- [ ] CybShield independent policy engine and audit logs
- [ ] CybTrust opt-in attestations, corrections and appeals

## Milestone 3: experimental chain
- [ ] Deterministic ledger and valueless test credits
- [ ] Four-validator local devnet, partition/restart testing
- [ ] Read-only explorer and optional test wallet
- [ ] Independent audit before public network

## Milestone 4: physical systems
- [ ] Simulated robot actions with safety interlocks
- [ ] Explicit human approval, local stop and rollback
- [ ] Field pilot only after hardware safety review

All checkboxes represent proposed milestones, not claims of completed implementation except the marked design work.
