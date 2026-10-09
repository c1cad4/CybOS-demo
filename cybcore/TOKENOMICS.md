# CYBCORE — utility economy and CybWallet specification

**Status: design only. No token contract, mint address, market, real balance or custody is deployed.**

## Purpose

CYBCORE is the proposed utility unit for metering optional CybCore compute, CybLex encrypted storage and delivery, verified node services, and bounded robot workloads. cybOS must remain usable offline and without any token. Human operators can pay for optional network resources; devices may request budgets but cannot autonomously authorize transfers.

## Test economy first

Build an off-chain, non-transferable, valueless sandbox ledger before considering any public blockchain. Use integer units (never floating point), unique transaction IDs, idempotent posting, double-entry accounting, nonnegative balances, audit history and explicit debit/credit invariants. Never call sandbox credits real CYBCORE or display a market price. Simulate metering and fees, then evaluate spam resistance and fairness.

## Wallet security

- Noncustodial, opt-in wallet. Show address and read-only status first; do not auto-create an on-chain account or sign transactions.
- Never store a seed phrase or spend key in SQLite, logs, telemetry, browser storage or AI prompts. Prefer platform-protected signing with explicit human confirmation for each value transfer.
- Require transaction previews: network, destination, token identifier, exact amount, fees, expiry, allowance and risk. Default deny; no blind signing.
- Robots and AI agents have zero spending authority by default. Optional scoped budgets require human authorization, per-action limits, expiry, revocation, and independent policy enforcement.
- Separate Ed25519 CybCore node identity, CybChat Noise identity, and any blockchain wallet keys. Never derive or reuse keys across purposes without reviewed domain separation.
- The eventual on-chain deployment requires a chain selection, audited token contract, supply policy, legal review, governance and incident response. No guaranteed returns, appreciation or security claims.

## Economic design decisions still open

Supply cap vs dynamic issuance, initial allocation, vesting, emissions, governance, transaction fees, resource pricing, bridge exposure, anti-sybil controls, and public launch jurisdiction. These are governance decisions, not safe defaults to silently deploy.

## Proposed modules

`cybcore/economy`: sandbox ledger and policy; `cybOS Assets`: CYBCORE design/status panel; `CybWallet`: isolated read-only wallet shell first; `RobotCYB`: request-only resource spending API. No automatic transfers.
