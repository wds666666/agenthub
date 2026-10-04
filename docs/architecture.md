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

Installers supply the Desktop, same-version CLI and management skill as one product. Windows NSIS registers only a user PATH entry and tracks whether it added that entry; uninstall preserves preexisting user entries. MSI uses a user-scoped Environment component. The skill's PowerShell wrapper resolves installed/portable executables even when its parent process retains an old PATH. Installer registration runs hidden and does not open AgentHub storage. Ubuntu deb installs the CLI in `/usr/bin` and the skill in `/usr/share/agenthub/skills`. Portable Windows distribution is a complete ZIP, not an isolated GUI executable.

CLI `validate` checks portable file structure and supported schemas without executing payloads. Version saves and reconciled remote merges validate before committing. CLI Plans accept an explicit JSON `SyncSelection`; absent selection retains full compatible scope. Selected IDs and unknown fields are checked before planning. Desktop-only operations remain explicitly documented by the skill; agents must not fabricate CLI endpoints or mutate SQLite as an API.

Desktop IPC commands that touch Git, SQLite, the filesystem, scanners, adapters, planning, backups or verification execute as asynchronous Tauri commands so blocking work never runs on the WebView event thread. Every external process launched by the desktop uses a shared platform wrapper; on Windows it applies `CREATE_NO_WINDOW`. Git also disables terminal prompting and pagers so an unexpected credential helper or pager cannot stall the desktop. Hiding the desktop executable itself is not considered sufficient because each console child process has an independent creation policy.

Automatic sync is coordinated inside `agenthub-core`. AgentHub mutation commands save Canonical content first and then run every enabled target profile independently through Plan → backup → Apply → verify. No background filesystem watcher is used, and host-side changes are never imported by this path.

`~/.agents/skills` is a separate Skills-only shared target named `agents`. It is not Canonical: users can project a selected subset to it, explicitly project an empty Skills domain to clear it, or leave it untouched. Because multiple agents may load this directory alongside their own user directory, its Plan and transaction stay independent from Cursor, Codex and Claude Code.

Post-initialization discovery is an explicit reverse-import workflow. It scans only the user-global allowlist, groups exact content duplicates across sources, and imports only user-selected new content into Canonical. Import never runs in the background, scans projects, or silently resolves same-ID/different-content conflicts. A separate read-only Skills check runs on initialized desktop entry, navigation and visible-window refresh, at most every two minutes. It compares existing IDs across the four allowlisted tool roots; it does not scan MCP, Rules or Plugins. A process-local cache fingerprints portable file metadata and reuses content digests until metadata changes, with a thirty-minute content recheck. Cache state and host paths are never versioned.

Skill discovery is limited to immediate child directories of each allowlisted Skills root. Hidden directories and deeper folders are not separate resources. Skill digests and imports share a portable-content traversal: nested instructions and supporting files stay with their parent; local virtual environments, dependency installations, Git metadata and generated caches are excluded. Scan preflight rejects nonportable symlinks/special files and invalid instruction files before selection, with a reason per resource. Import repeats the same checks and remains atomic.

Host inventory is a separate read-only projection of the same user-global allowlist. It never implies ownership: every entry is classified as a Canonical match, host-only content, or a protected host constraint. The desktop may request an explicit cleanup transaction for selected writable entries. Cleanup re-scans stable IDs, backs up every affected path under `backups/host-cleanup-<id>/`, mutates only the selected resource, and automatically restores the backup on failure. Collection files such as MCP JSON/TOML are edited at the selected server key while preserving unrelated top-level configuration.

Plugin storage managed by an official CLI or marketplace is not treated as a deletable directory. In v0.1 Claude Code's `~/.claude/plugins` and Codex marketplace/cache state are inventory constraints; future adapters must use the vendor's supported install/uninstall interface and receive their own transaction contract before becoming writable. Cursor local plugins with a recognized manifest under `~/.cursor/plugins/local` remain ordinary writable user resources.

Deleting SQLite permits inventory reconstruction from Canonical files. Transaction history and UI preferences may be lost; Canonical content and Git history remain intact.

## Library reset and remote versions
Desktop operations are serialized by a process-wide guard. Reset closes SQLite, moves the complete active root to a private sibling recovery directory, creates a fresh uninitialized root, and returns to import selection. Hosts are untouched; all automatic profiles and remote settings are reset. Recovery copies contain secrets and must stay private.
Remote settings live in repository-local Git configuration, not versioned content. Version saves commit locally first; remote failures are returned separately from local success. Explicit remote sync requires a clean working tree and reconciles by fetch and merge before push. Remote updates do not silently project to hosts.

## Local repository authentication

First run offers local import or restoration of an existing dedicated AgentHub repository. Restoration stages a fresh checkout and machine-local state in a private sibling directory, validates portable content and reachable history, then switches roots with a recovery journal and private original-root backup. Open SQLite handles are closed before activation. No initial commit, merge, push, host projection or automatic profile is created. An existing initialized library, content, Git history or unknown root file blocks restoration; reinstall opens existing data normally. Missing SQLite can be reconstructed from a validated existing library, including uncommitted content, without deleting files. A private journal is recorded before authentication/download, and an OS file lock prevents another application instance from recovering a live restoration.

Installation upgrades use package-manager ownership: overwrite/update registered program files and remove obsolete owned resources, never recursively clean the install directory or user library. Windows NSIS keeps an explicit installed-resource manifest to retire stale management-skill files on subsequent upgrades. Unknown files and symlinks/reparse points are preserved.

GitHub and self-hosted Git/Gitea HTTPS repositories accept a username and access token in the desktop. Desktop and CLI share the core authentication module and local encrypted credentials. Both executables can dispatch the built-in Git credential helper without starting a window or requiring a separate installation. Read verification precedes connection persistence; loading the page never contacts the remote. SSH and repositories without an AgentHub credential continue using existing system authentication.
