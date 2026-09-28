# Architecture

```text
Canonical files --hash--> Planner --steps--> Transaction executor --> host
      |                    |                       |                 |
      +-- Git history      +-- SQLite index       +-- backups       +-- verify scan
```

## Three independent responsibilities

- **Git** records user-selected history of Canonical content. Edits and imports remain uncommitted until the user supplies a commit message. Apply records HEAD, dirty state and a Canonical digest, but never auto-commits.
- **SQLite** records business state: schema/init, scan and import decisions, Canonical index, targets, compatibility, drift, Plans, transactions, verification, encrypted secrets, UI state and backup indexes. It does not store Skill, Rule or Plugin bodies.
- **backups/** contains the complete pre-Apply writable capability state per target. It is the only Apply rollback source and is retained until explicit user cleanup.

The shared `agenthub-core` Rust crate owns paths, schemas, scanning, adapters, planning, Git, secrets and transactions. Both `agenthub` CLI and Tauri commands call this crate, which guarantees identical Plans.

Deleting SQLite permits inventory reconstruction from Canonical files. Transaction history and UI preferences may be lost; Canonical content and Git history remain intact.

