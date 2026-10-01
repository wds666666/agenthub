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

`~/.agents/skills` is a separate Skills-only shared target named `agents`. It is not Canonical: users can project a selected subset to it, explicitly project an empty Skills domain to clear it, or leave it untouched. Because multiple agents may load this directory alongside their own user directory, its Plan and transaction stay independent from Cursor, Codex and Claude Code.

Post-initialization discovery is an explicit reverse-import workflow. It scans only the user-global allowlist, groups exact content duplicates across sources, and imports only user-selected new content into Canonical. It never runs in the background, scans projects, or silently resolves same-ID/different-content conflicts.

Host inventory is a separate read-only projection of the same user-global allowlist. It never implies ownership: every entry is classified as a Canonical match, host-only content, or a protected host constraint. The desktop may request an explicit cleanup transaction for selected writable entries. Cleanup re-scans stable IDs, backs up every affected path under `backups/host-cleanup-<id>/`, mutates only the selected resource, and automatically restores the backup on failure. Collection files such as MCP JSON/TOML are edited at the selected server key while preserving unrelated top-level configuration.

Plugin storage managed by an official CLI or marketplace is not treated as a deletable directory. In v0.1 Claude Code's `~/.claude/plugins` and Codex marketplace/cache state are inventory constraints; future adapters must use the vendor's supported install/uninstall interface and receive their own transaction contract before becoming writable. Cursor local plugins with a recognized manifest under `~/.cursor/plugins/local` remain ordinary writable user resources.

Deleting SQLite permits inventory reconstruction from Canonical files. Transaction history and UI preferences may be lost; Canonical content and Git history remain intact.

## Library reset and remote versions
Desktop operations are serialized by a process-wide guard. Reset closes SQLite, moves the complete active root to a private sibling recovery directory, creates a fresh uninitialized root, and returns to import selection. Hosts are untouched; all automatic profiles and remote settings are reset. Recovery copies contain secrets and must stay private.
Remote settings live in repository-local Git configuration, not versioned content. Version saves commit locally first; remote failures are returned separately from local success. Explicit remote sync requires a clean working tree and reconciles by fetch and merge before push. Remote updates do not silently project to hosts.
