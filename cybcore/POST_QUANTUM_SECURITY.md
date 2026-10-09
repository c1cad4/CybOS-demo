# CybShield PQ — phased security architecture

This is a **design proposal**, not a claim that cybOS or CybCore is quantum-resistant today. No system can guarantee immunity to all attacks.

## Standards and primitives

- Prefer standardized NIST FIPS 203 ML-KEM-768 for key establishment, FIPS 204 ML-DSA for signatures, and FIPS 205 SLH-DSA as a signature diversity option.
- Use a reviewed hybrid X25519 + ML-KEM-768 handshake with explicit domain separation and transcript binding. Never invent a custom cryptographic combiner.
- Prefer TLS 1.3 with a maintained rustls/aws-lc-rs provider supporting X25519MLKEM768. Verify the *negotiated* group in interoperability tests; TLS hybrid key exchange does not automatically give PQ server authentication.
- Keep AEAD encryption (AES-256-GCM or ChaCha20-Poly1305) with strict nonce management, key rotation, and forward secrecy. Encrypt data before it enters storage or P2P distribution.

## Trust boundaries

1. **cybOS identity:** Current Ed25519 signing key is stored in SQLite; move private material to OS-backed key protection and support key revocation, backup/recovery, and signed rotation before production. Do not reuse Noise X25519 keys as ML-DSA keys.
2. **CybChat:** Design and independently review an authenticated hybrid handshake, session ratchet, replay prevention, device verification, downgrade resistance, and offline prekeys. Preserve existing E2E guarantees during migration.
3. **CybCore:** Default-deny ACL, short-lived scoped credentials, rate limits, per-tenant isolation, audit trails, and TLS with verified identity. Never use the internal status bearer token as a public node identity.
4. **CybLex:** Content-addressed encrypted blobs, authenticated manifests, signed publishers, untrusted peer isolation, and no automatic execution of downloaded files.
5. **RobotCYB:** Explicit human approval for physical actions, signed capability-limited commands, watchdogs, resource quotas, and an independent emergency stop. Treat model outputs as untrusted requests, never as authorization.
6. **Releases:** Signed artifacts, dependency pinning, SBOM, provenance verification, reproducible builds where feasible, and rollback protection.

## Migration gates

- Inventory cryptography and sensitive data lifetimes; prioritize harvest-now-decrypt-later exposure.
- Prototype ML-KEM hybrid transport behind an opt-in feature flag and test failure, downgrade, and interoperability paths.
- Adopt independently reviewed, maintained implementations; Open Quantum Safe liboqs is useful for research but its maintainers explicitly do not recommend it as sole production protection.
- Run adversarial integration tests, fuzzing, dependency audits, and independent security review before default enablement.
- Do not claim post-quantum E2E until *both* key agreement and authentication are covered and tested.

References: https://www.nist.gov/pqc ; https://rustls.dev/docs/rustls/manual/_04_features/index.html ; https://github.com/open-quantum-safe/liboqs
