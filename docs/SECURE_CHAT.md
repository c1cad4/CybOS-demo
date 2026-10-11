# Secure CybChat: identity, approval and durable delivery

## First contact

Both peers must approve the other's exact Noise public key. LAN discovery is an
address hint, not proof of identity.

1. Open CybChat on each device. Compare the displayed local SHA-256 fingerprint
   with the pending peer fingerprint through a separate trusted channel.
2. Select the LAN peer and press SEND ENCRYPTED. If its key is unknown, the
   handshake collects the receiver's public key, but no message body is sent.
3. Approve the verified receiver key and retry. The receiver rejects the first
   message from an unapproved sender and offers that sender's fingerprint.
4. The receiver approves that key after comparison. Retry the retained draft.
   Delivery succeeds when the receiver has committed the message to SQLite.

Approval never automatically retries or sends a message. Rejecting removes the
pending request without persisting trust. Changed keys are rejected; a pending
request cannot silently substitute another key for the same node id. Up to 16
peer approval requests can be held in memory.

The full local fingerprint is cached at startup and shown as selectable text.
Only the public-key fingerprint is displayed; private key material is not.

## Existing installations

Existing Noise private keys and peer pins are retained. Old automatically
accepted TOFU pins require explicit approval once after upgrade. Matching an
old pin is not treated as proof that the user verified it. A malformed local
key or pin is an error, not a reason to rotate or replace an identity.

New node and Noise identities use INSERT OR IGNORE plus a checked read-back;
concurrent startups converge on the persisted identity. Read/write failure
prevents the secure listener from starting. Bind/setup failure is reported in
the CybChat status and disables encrypted sending for that application instance.
These errors remain visible instead of being overwritten with READY.

## Delivery contract

On receive, the pinned public key must match an explicit durable approval.
Replay identity, message history and pruning of the 1024-entry replay window
are committed in one SQLite transaction. Write or commit failure rolls back
both history and the replay claim and produces REJECT, not ACK. A subsequent
retry can succeed after the storage problem is resolved. A successful duplicate
inside the retained replay window is rejected.

On send, the receiver's Noise public key is checked during the handshake,
before any message body is transmitted. An unknown receiver offers an approval
request; a changed receiver fails closed. A message is added to outgoing UI
history only after a delivery ACK. Failed, timed-out and unapproved attempts
retain the input draft.

The existing 8-session limit is retained. The listener event queue is bounded
to 8 events. Every partial read/write reapplies the remaining absolute
12-second session budget, so a trickling peer cannot refresh the timeout for
every byte.

## Validation and remaining work

Automated checks include occupied ports, malformed identities, storage-write
failures, durable approvals/history after reopening SQLite, changed-key
rejection, unknown receivers, replay, rollback on history/pruning errors and
slow frame reads. An actual Noise loopback test verifies REJECT on SQLite
failure, successful retry and rejection of a repeated message id.

Before distribution, test this first-contact flow on two real Mac devices,
including restart, Wi-Fi changes, occupied port and disk-write failures.
Unit/loopback tests do not establish real LAN/BLE interoperability. Noise keys
remain local SQLite data; OS keychain integration, authenticated key rotation,
cross-device trust synchronization and external cryptographic review are
separate work. There is no automatic trust reset or key replacement flow.
