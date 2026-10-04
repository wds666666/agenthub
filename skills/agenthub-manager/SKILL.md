---
name: agenthub-manager
description: Manage the user-global AgentHub library, migrate capability backups, synchronize version history across devices, and resolve library Git conflicts. Use for Skills, MCP, Rules and supported local Plugins; excludes project-local agent configuration.
---

# AgentHub Manager

AgentHub maintains a portable content library at `~/.agenthub` (`AGENTHUB_HOME` can override it). It is independent of the current repository. Canonical files own content; SQLite owns device state; local backups own recovery.

## Locate the installed command

The complete installer includes the UI, CLI and this skill; users do not need a second CLI download. Read [installation and command discovery](references/installation.md) if `agenthub` is unavailable, on Windows with an old process PATH, or with a portable ZIP. The bundled `scripts/agenthub.ps1` locates the executable and forwards arguments and exit codes. Confirm `--version` and `--help`; examples below use `agenthub` to mean that resolved executable. The UI does not need to be running.

## Inspect before changing

1. Run `agenthub doctor --json`, `agenthub inventory --json`, `agenthub auto-sync status` and `agenthub git remote-status`. Use the returned root for every operation, never the current workspace.
2. Match the workflow below to the user's request. Reuse authorization already given for that operation; do not ask repeatedly. If target, destructive scope or intent is unclear, show the concrete preview before asking.
3. Scan only fixed user-global locations. Never use the current project, its parents, or its `AGENTS.md` / `CLAUDE.md` as import inputs.

## Choose the actual interface

| Workflow | Available interface |
| --- | --- |
| Inventory, validation, selected Plan/Apply, saved automatic runs | CLI |
| Local versions, remote connection/sync, content restore, host rollback | CLI |
| First initialization | Desktop selection, or CLI `init --empty` / explicitly authorized `init --import-all` |
| Later selected import, bulk library deletion, host cleanup, reset | Desktop |
| Enable/edit automatic scope and import/strict policies | Desktop |
| Skill/content editing and Git conflict resolution | Canonical files + validation; reviewed native Git for conflicts |

Do not claim the CLI exposes every Desktop operation. Do not edit SQLite to simulate missing commands. `target enable` is device bookkeeping, not automatic-sync authorization. Read [backup migration](references/migration.md) before replacing an existing backup workflow or onboarding another device.

## Import and edit

- Desktop initialization and **Scan and import** support source filters for Shared Agents, Cursor, Codex and Claude Code, then Skills/MCP/Plugins/Rules. Switching filters preserves selection; exact content duplicates import once. Zero-result sources stay visible. Shared Agents is Skills-only.
- Import copies selected validated content into AgentHub. It does not grant permission to change hosts. Later import runs saved profiles only if the user enabled that policy.
- System/hidden skills and CLI-managed plugin caches are not imported. Codex cache plugins are currently display-only; Claude local plugin manifests are recognized, but marketplace cache import and general cross-tool plugin conversion are not supported. Protected means unsafe to delete directly, not impossible to read.
- Edit Canonical or use the Desktop Rule editor. Read [file formats](references/formats.md) for actual fields; keep Skill resources and whole Plugin payloads with their manifests. Run `agenthub validate --json` after edits. Validation checks structure and supported schemas, not instruction quality, credential safety or executable availability. Never execute plugin payloads to inspect them.
- Library multi-selection supports visible search results and categories. Bulk deletion confirms exact resources, archives them locally, then runs saved automatic-sync profiles once. It remains an uncommitted Git change. Report the recovery path. Host cleanup is a separate action and preserves the library; protected stores cannot be selected.

## Synchronize hosts

Use Desktop **Sync to tools** to choose target/scope and review named changes. Plan is read-only; Apply backs up, writes, verifies and rolls back on failure. Host extras in managed domains may be deleted. Preserve unrelated tool settings and protected stores.

For CLI, prefer `agenthub plan <agents|cursor|codex|claude> --selection <scope.json> --json`; read [selection and recovery](references/selection.md) for the JSON shape and replacement consequences. Without `--selection`, Plan covers all compatible content. Review the exact ID, resources, deletions and warnings, then apply the authorized preview with `agenthub sync <target> --plan-id <id> --confirm`. Never bypass a stale-plan failure.

An enabled automatic profile authorizes subsequent mutations within its saved target/scope. External Canonical edits need `agenthub auto-sync run`; it runs only saved profiles and does not import host changes. Read [selection and recovery](references/selection.md) for domain semantics and rollback.

## Save and share versions

Run `agenthub validate --json`, review `agenthub git status` and `agenthub git diff` locally without exposing credential contents, then use a user-approved `agenthub git commit --message "..."`. Read its JSON result: local success is separate from remote success; a remote error is not permission to create repeated local commits. Read [portable storage and remote synchronization](references/storage.md) before connecting/authenticating or explaining what travels between devices. For an actual conflict, read [conflict resolution](references/conflicts.md): ordinary sync aborts its merge, so establish a reviewed merge before editing conflict stages. Do not choose one device's content silently.

Remote reception and native Git commits do not update hosts. Review a new host Plan or run only the user's previously enabled automatic profiles when that delivery is authorized. Direct filesystem edits are also not automatically uploaded; a version save triggers connected remote synchronization.

## Recovery and reset

Use `agenthub history --json` and `agenthub rollback <transaction-id>` for host recovery. Explain the exact target and captured state: rollback does not restore library content, and the next sync may recreate drift.

Desktop Reset AgentHub requires typing `AGENTHUB`; it moves the complete old root into a private sibling recovery directory and restarts initialization. It preserves host resources. A reset archive includes keys and local versions; report its path and do not delete it without user instruction.

Never print secrets or include them in chat, logs, diffs, or examples. Recognizable JSON credentials block remote upload; this does not guarantee arbitrary Markdown or plugin assets are secret-free.

HTTPS sign-in is available in the desktop for GitHub and self-hosted Git/Gitea; its encrypted credentials are shared with CLI. Do not ask the user to provide access tokens in chat or command arguments. `git login <URL> --username <name> --branch <branch>` reads a token only from stdin for direct user-controlled setup. Login verifies reading, not write permission. Inspect `remote-status` verification state, not merely the presence of a URL.
