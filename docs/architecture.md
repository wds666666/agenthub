# Architecture

```text
Canonical files --hash--> Planner --steps--> Transaction executor --> host
      |                    |                       |                 |
      +-- Git history      +-- SQLite index       +-- backups       +-- verify scan
```

## Three independent responsibilities

- **Git** records user-selected history of Canonical content. Edits and imports remain uncommitted until the user supplies a commit message. Apply records HEAD, dirty state and a Canonical digest, but never auto-commits.
- **SQLite** records business state: schema/init, scan and import decisions, Canonical index, targets, persisted automatic-sync profiles and their selected scopes, compatibility, drift, Plans, transactions, verification, encrypted secrets, UI state and backup indexes. It does not store Skill, Rule or Plugin bodies.
- **backups/** contains the complete pre-Apply writable capability state per target. It is the only Apply rollback source and is retained until explicit user cleanup.

The shared `agenthub-core` Rust crate owns paths, schemas, scanning, adapters, planning, Git, secrets and transactions. Both `agenthub` CLI and Tauri commands call this crate, which guarantees identical Plans.

Desktop IPC commands that touch Git, SQLite, the filesystem, scanners, adapters, planning, backups or verification execute as asynchronous Tauri commands so blocking work never runs on the WebView event thread. Every external process launched by the desktop uses a shared platform wrapper; on Windows it applies `CREATE_NO_WINDOW`. Git also disables terminal prompting and pagers so an unexpected credential helper or pager cannot stall the desktop. Hiding the desktop executable itself is not considered sufficient because each console child process has an independent creation policy.

Automatic sync is coordinated inside `agenthub-core`. AgentHub mutation commands save Canonical content first and then run every enabled target profile independently through Plan → backup → Apply → verify. No background filesystem watcher is used, and host-side changes are never imported by this path.

Deleting SQLite permits inventory reconstruction from Canonical files. Transaction history and UI preferences may be lost; Canonical content and Git history remain intact.
