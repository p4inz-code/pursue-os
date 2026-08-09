# PURSUE OS — Third-Party Notices

> **Source:** PURSUE OS master handoff (authoritative) for policy; contents reflect the actual implementation. Restored 2026-08-09.
> **Category:** B — implementation documentation (maintained as components are added).

This file records third-party components used by PURSUE. PURSUE's own code is Apache-2.0; each component below carries its own license.

## Foundation (Rust crates)

Direct dependencies, verified from `cargo metadata` (resolved 2026-08-09). The full transitive set (21 crates total) is permissively licensed (MIT OR Apache-2.0, MIT, or Unlicense/MIT); the authoritative version-pinned record is `Cargo.lock`.

| Component | Version | License | Purpose |
|---|---|---|---|
| sha2 | 0.10.9 | MIT OR Apache-2.0 | SHA-256 content addressing / integrity hashing |
| serde | 1.0.229 | MIT OR Apache-2.0 | Serialization of records, manifests, audit events |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | JSON manifest / event serialization |

Full transitive dependency list and versions are captured in `Cargo.lock` (the authoritative record for reproducibility).

## System / OS layer

None yet — the OS package foundation is a later phase. Policy: any Linux packages distributed with PURSUE will be recorded here with their own licenses, separate from Apache-2.0.
