# Synchronization contract

## State machine

`Draft → Planned → Confirmed → BackedUp → Applying → Verifying → Applied`.

Any Apply or verification failure enters `RollingBack`, then `RolledBack` or `RollbackFailed`. A Plan never writes host files. Non-interactive Apply requires the exact Plan ID plus an explicit confirmation flag.

The desktop offers a reviewed one-off path and an automatic-sync path. Enabling automatic sync first reconciles the selected target and scope through the same deterministic Plan, backup, Apply and verification pipeline; only after success is the profile stored in SQLite. Every later Canonical mutation performed through AgentHub invokes all enabled profiles. This is event-driven application behavior, not a background watcher, host scan, merge, or import mechanism.

If an automatic run has no steps, it returns `changed=false` without persisting the transient Plan, creating a backup, or recording a transaction. If any host path changes, the transaction is always persisted with `mode=auto_sync` so rollback remains possible. Targets run independently: one failure is reported without preventing the other enabled targets or reverting the already-saved Canonical mutation. History may visually group adjacent successful automatic-sync records, but storage never coalesces or discards them.

## Deterministic Plan

A Plan records Canonical digest, Git HEAD, dirty state, target, selected Skills/Plugins/MCP IDs, the Rules toggle, expected projection digest, warnings and ordered file steps. Steps classify create, update, replace, delete, skip and constraint. Secrets are redacted before persistence, logs or JSON output. Apply re-generates the same selected projection and blocks if the selection or any other Plan input has drifted.

## Explicit sync scope

Target Sync begins with a scope selector. Skills, Plugins and MCP servers are selected by Canonical ID; Rules are selected with a target-wide toggle because some adapters render them as a combined file or plugin. A selected domain is replaced from its selected Canonical subset, so host extras in that domain appear as deletes. A domain with no selected resources is omitted from the projection and is not touched. Rules disabled omits the Rules projection entirely. The selection snapshot is persisted in the Plan and is part of the Apply contract.

## Target domains

| Target | Skills | MCP | Plugins | Rules |
| --- | --- | --- | --- | --- |
| Cursor | replace `~/.cursor/skills` | replace `mcpServers`, preserve other JSON | replace writable local plugins | generated global plugin |
| Codex | replace `~/.codex/skills` | replace `mcp_servers`, preserve other TOML | replace writable user plugins | replace global `AGENTS.md` |
| Claude | replace writable `~/.claude/skills` except reserved dirs | replace user-scope servers | replace writable user plugins | replace `~/.claude/rules` |

Before writing, the executor resolves all paths and records type, mode, hash and symlink target. It then creates a complete transaction backup, writes the projection, reads the target again and compares it with Expected State. Verification failure automatically restores the backup. Cleanup is manual and warns that rollback ability will be lost.

A verified Apply atomically records the applied transaction and marks that target enabled with its last-sync timestamp. Dashboard target state must never diverge from a successful Apply. Existing databases migrate historical applied transactions only when no explicit target row exists, preserving a later explicit disable.

## Manual rollback

Manual rollback is a new audited transaction, not a mutation of the historical Apply record. Given an Apply transaction, AgentHub restores the writable target domains from that transaction's pre-Apply backup. Before restoration it creates a fresh backup of the target's current writable domains so the rollback itself can be reversed. The restored paths are re-read and compared with the selected backup; failure restores the pre-rollback backup automatically.

Rollback never changes Canonical files or Git history. The restored host can therefore intentionally drift from Canonical, and the next Plan must show that drift before another Apply. The desktop confirmation names the target and full transaction ID and labels the action as restoring the state before that Apply.

`scan` is an import discovery operation only before initialization. Afterwards it is a read-only drift audit; it never merges or imports host changes.
