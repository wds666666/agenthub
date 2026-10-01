---
name: agenthub-manager
description: Inspect, import, edit and synchronize user-global AgentHub Skills, MCP servers, Plugins and Rules. Use when managing the AgentHub library, reviewing host changes, cleaning resources, resetting the library, or connecting its version history across devices. Excludes project-local agent configuration.
---

# AgentHub Manager

AgentHub maintains a portable content library at `~/.agenthub` (`AGENTHUB_HOME` can override it). It is independent of the current repository. Canonical files own content; SQLite owns device state; local backups own recovery. Use the installed CLI's `--help` for exact arguments.

## Inspect before changing

1. Run `agenthub doctor --json` and `agenthub inventory --json`.
2. Match the workflow below to the user's request. Reuse authorization already given for that operation; do not ask repeatedly. If target, destructive scope or intent is unclear, show the concrete preview before asking.
3. Scan only fixed user-global locations. Never use the current project, its parents, or its `AGENTS.md` / `CLAUDE.md` as import inputs.

## Import and edit

- Desktop initialization and **Scan and import** support source filters for Shared Agents, Cursor, Codex and Claude Code, then Skills/MCP/Plugins/Rules. Switching filters preserves selection; exact content duplicates import once. Zero-result sources stay visible. Shared Agents is Skills-only.
- Import copies selected validated content into AgentHub. It does not grant permission to change hosts. Later import runs saved profiles only if the user enabled that policy.
- System/hidden skills and CLI-managed plugin caches are not imported. Codex cache plugins are currently display-only; Claude local plugin manifests are recognized, but marketplace cache import and general cross-tool plugin conversion are not supported. Protected means unsafe to delete directly, not impossible to read.
- Edit Canonical or use the Desktop Rule editor. Validate before saving. Inventory details are read-only previews; never execute plugin payloads to inspect them.
- Library multi-selection supports visible search results and categories. Bulk deletion confirms exact resources, archives them locally, then runs saved automatic-sync profiles once. It remains an uncommitted Git change. Report the recovery path. Host cleanup is a separate action and preserves the library; protected stores cannot be selected.

## Synchronize hosts

Use Desktop **Sync to tools** to choose target/scope and review named changes. Plan is read-only; Apply backs up, writes, verifies and rolls back on failure. Host extras in managed domains may be deleted. Preserve unrelated tool settings and protected stores.

For CLI, generate `agenthub plan <cursor|codex|claude> --json`, review its exact ID and scope, then apply the authorized preview with `agenthub sync <target> --plan-id <id> --confirm`. Never bypass a stale-plan failure.

An enabled automatic profile authorizes subsequent mutations within its saved target/scope. External Canonical edits need `agenthub auto-sync run`; it runs only saved profiles and does not import host changes. Read [selection and recovery](references/selection.md) for domain semantics and rollback.

## Save and share versions

Use `agenthub git status`, `agenthub git diff`, and a user-approved `agenthub git commit --message "..."`. Do not commit unrelated content. Read [portable storage and remote synchronization](references/storage.md) before connecting a remote, handling authentication/conflicts, or explaining what travels between devices.

## Recovery and reset

Use `agenthub history --json` and `agenthub rollback <transaction-id>` for host recovery. Explain the exact target and captured state: rollback does not restore library content, and the next sync may recreate drift.

Desktop Reset AgentHub requires typing `AGENTHUB`; it moves the complete old root into a private sibling recovery directory and restarts initialization. It preserves host resources. A reset archive includes keys and local versions; report its path and do not delete it without user instruction.

Never print secrets or include them in chat, logs, diffs, or examples. Recognizable JSON credentials block remote upload; this does not guarantee arbitrary Markdown or plugin assets are secret-free.
