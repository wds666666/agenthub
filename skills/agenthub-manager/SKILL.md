---
name: agenthub-manager
description: Manage the user-global AgentHub library, migrate capability backups, synchronize version history across devices, and resolve library Git conflicts. Use for Skills, MCP, Rules and supported local Plugins; excludes project-local agent configuration.
---

# AgentHub Manager

AgentHub maintains a portable content library at `~/.agenthub` (`AGENTHUB_HOME` can override it). It is independent of the current repository. Canonical files own content; SQLite owns device state; local backups own recovery.

## Locate the installed command

The complete installer includes the UI and CLI; users do not need a second CLI download. This Skill is an optional separate download, copied manually only on request. Read [installation and command discovery](references/installation.md) if `agenthub` is unavailable, on Windows with an old process PATH, or with a portable ZIP. This Skill supports Windows and Ubuntu. `scripts/agenthub.ps1` is only the Windows command-discovery helper; Ubuntu deb already provides `/usr/bin/agenthub`. Copy the complete Skill directory, not only its scripts. The wrapper locates the executable and forwards arguments and exit codes. Confirm `--version` and `--help`; examples below use `agenthub` to mean that resolved executable. The UI does not need to be running.

## Inspect before changing

1. Run `agenthub doctor --json`, `agenthub inventory --json`, `agenthub auto-sync status` and `agenthub git remote-status`. Use the returned root for every operation, never the current workspace.
2. Match the workflow below to the user's request. Reuse authorization already given for that operation; do not ask repeatedly. If target, destructive scope or intent is unclear, show the concrete preview before asking.
3. Scan only fixed user-global locations. Never use the current project, its parents, or its `AGENTS.md` / `CLAUDE.md` as import inputs.

## Choose the actual interface

| Workflow | Available interface |
| --- | --- |
| Inventory, validation, selected Plan/Apply, saved automatic runs | CLI |
| Local versions, capability changes, reviewed discard/remote content recovery, remote connection/sync, host rollback | CLI |
| First initialization | Desktop selection, or CLI `init --empty` / explicitly authorized `init --import-all` |
| Later selected import | Desktop or CLI `discover` → `import-plan` → `import-apply` |
| Bulk library deletion, host cleanup, reset | Desktop |
| Enable/edit automatic scope and preserve/replace mode | Desktop or CLI reviewed Plan + `auto-sync enable`; CLI `auto-sync disable` |
| Skill/content editing and Git conflict resolution | Canonical files + validation; reviewed native Git for conflicts |

Do not claim the CLI exposes every Desktop operation. Do not edit SQLite to simulate missing commands. `target enable` is device bookkeeping, not automatic-sync authorization. Read [backup migration](references/migration.md) before replacing an existing backup workflow or onboarding another device.

## Import and edit

- Desktop **Import Skill** lets the user pick one local Skill folder containing `SKILL.md`, review the complete portable file list and exclusions, and confirm import. Scripts/references/assets travel with the Skill; nested skills remain parent content. Exact duplicates are disabled. Import writes only the library, without saving a version, pushing or changing tools. This explicit user selection does not expand automatic discovery's global allowlist.

- Desktop initialization and **Scan and import** support source filters for Shared Agents, Cursor, Codex and Claude Code, then Skills/MCP/Plugins/Rules. Switching filters preserves selection; exact content duplicates import once. Zero-result sources stay visible. Shared Agents is Skills-only.
- CLI `discover --target all` scans the same allowlist. Review discovery IDs, run `import-plan --id <id>` (repeat as needed), then `import-apply <plan-id> --confirm`. This is separate from version saving and tool projection.
- Import copies selected validated content into AgentHub. It does not grant permission to change hosts. Import does not save, upload or project pending content; these require separate explicit actions.
- System/hidden skills and CLI-managed plugin caches are not imported. Codex cache plugins are currently display-only; Claude local plugin manifests are recognized, but marketplace cache import and general cross-tool plugin conversion are not supported. Protected means unsafe to delete directly, not impossible to read.
- Edit Canonical or use the Desktop Rule editor. Read [file formats](references/formats.md) for actual fields; keep Skill resources and whole Plugin payloads with their manifests. Run `agenthub validate --json` after edits. Validation checks structure and supported schemas, not instruction quality, credential safety or executable availability. Never execute plugin payloads to inspect them.
- Library multi-selection supports visible search results and categories. Bulk deletion confirms exact resources, archives them locally, without running host profiles. It remains an uncommitted Git change. Report the recovery path. Host cleanup is a separate action and preserves the library; protected stores cannot be selected.

## Synchronize hosts

Use Desktop **Sync to tools** to choose target/scope and review named changes. Plan is read-only; Apply backs up, writes, verifies and rolls back on failure. Preserve mode retains every unselected host capability. Replace mode may delete extras only in managed categories after review. Preserve unrelated tool settings and protected stores.

For CLI, prefer `agenthub plan <agents|cursor|codex|claude> --selection <scope.json> --json`; read [selection and recovery](references/selection.md) for the JSON shape and replacement consequences. Without `--selection`, Plan covers all compatible content. Review the exact ID, resources, deletions and warnings, then apply the authorized preview with `agenthub sync <target> --plan-id <id> --confirm`. Never bypass a stale-plan failure.

An enabled automatic profile defines an authorized target/scope; it does not make every pending mutation a delivery event. `agenthub auto-sync run` explicitly projects saved HEAD content only and blocks affected host edits since the last verified projection. Pending library content is excluded. Use `version save --host-sync enabled` only when the user requests delivery; selective saves intersect the committed capabilities with enabled scopes in preserve mode. External edits are not watched. Read [selection and recovery](references/selection.md) for domain semantics and rollback.

## Save and share versions

On a new device, choose Desktop **Restore an existing AgentHub library** instead of scanning, or run `agenthub bootstrap <URL> --branch <branch>`. Omitting `--branch` prefers `agenthub`, otherwise the remote default branch. HTTPS token authentication adds `--username <name>` and reads the token from user-controlled stdin; never request a token in chat or arguments. Restoration downloads and validates content/history without pushing or modifying tools. It requires an empty uninitialized library and a dedicated AgentHub repository; a plain Skills repository must first be migrated on its original device. Existing local data is never overwritten. After restoration, inspect inventory and remote-status, then review target/scope separately. Reinstall keeps the library and credentials; it is not a reset.

Run `agenthub validate --json`, review `agenthub git status` and `agenthub git diff --include-untracked` locally without exposing credential contents, then use a user-approved `agenthub version save --message "..."`. For a single Skill, use `--only skill:<id>`; add `--push --no-host-sync` only when publication is requested. Read [version selection and review](references/versions.md) for Plan/Apply and JSON selection files. Read its JSON result: local success is separate from remote success; a remote error is not permission to create repeated local commits. Read [portable storage and remote synchronization](references/storage.md) before connecting/authenticating or explaining what travels between devices. For an actual conflict, read [conflict resolution](references/conflicts.md): ordinary sync aborts its merge, so establish a reviewed merge before editing conflict stages. Do not choose one device's content silently.

`version save` and its `git commit` compatibility alias are local-only with no host writes by default. `remote push` publishes saved history without projecting to tools, and preserves disjoint staged/unstaged capability changes. If incoming versions overlap a pending capability, it stops without overwriting it. `git receive` downloads versions only and still requires a clean working tree. Recovery never writes hosts. A selective save never uploads pending excluded MCP entries and never projects them. Remote publication includes already-existing committed ancestry; `--only` controls the new commit, not earlier commits.

For unsaved changes, staged edits, discard or choosing cloud content over local content, read [version review and recovery](references/versions.md). Use the preview/apply commands, not an unchecked reset, clean or force-push.

## Recovery and reset

Use `agenthub history --json` and `agenthub rollback <transaction-id>` for host recovery. Explain the exact target and captured state: rollback does not restore library content, and the next sync may recreate drift.

Desktop Reset AgentHub requires typing `AGENTHUB`; it moves the complete old root into a private sibling recovery directory and restarts initialization. It preserves host resources. A reset archive includes keys and local versions; report its path and do not delete it without user instruction.

Never print secrets or include them in chat, logs, diffs, or examples. Recognizable JSON credentials block remote upload; this does not guarantee arbitrary Markdown or plugin assets are secret-free.

HTTPS sign-in is available in the desktop for GitHub and self-hosted Git/Gitea; its encrypted credentials are shared with CLI. Do not ask the user to provide access tokens in chat or command arguments. `git login <URL> --username <name> --branch <branch>` reads a token only from stdin for direct user-controlled setup. Login verifies reading, not write permission. Inspect `remote-status` verification state, not merely the presence of a URL.

The desktop automatically checks read-only Skill differences for existing library IDs across Shared Agents, Cursor, Codex and Claude Code at a two-minute maximum frequency while visible. Red reminders open explicit reverse-import review; no CLI watcher or automatic reverse import is exposed. Imported changed copies retain originals and receive unique IDs. Do not treat opening a reminder as saving or synchronizing a version.

New synchronization scopes default to `preserve`, not destructive replacement. Read [selection](references/selection.md) before projecting any resources. New Rules stay in the library until explicitly selected. Do not fabricate automatic installation of this Skill or tool rules. For download-only library updates, use `agenthub git receive`; it validates incoming content without pushing or touching tools.
