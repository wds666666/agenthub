# Synchronization contract

## State machine

`Draft → Planned → Confirmed → BackedUp → Applying → Verifying → Applied`.

Any Apply or verification failure enters `RollingBack`, then `RolledBack` or `RollbackFailed`. A Plan never writes host files. Non-interactive Apply requires the exact Plan ID plus an explicit confirmation flag.

## Deterministic Plan

A Plan records Canonical digest, Git HEAD, dirty state, target, expected projection digest, warnings and ordered file steps. Steps classify create, update, replace, delete, skip and constraint. Secrets are redacted before persistence, logs or JSON output.

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
