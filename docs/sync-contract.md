# Synchronization contract

## State machine

`Draft → Planned → Confirmed → BackedUp → Applying → Verifying → Applied`.

Any Apply or verification failure enters `RollingBack`, then `RolledBack` or `RollbackFailed`. A Plan never writes host files. Non-interactive Apply requires the exact Plan ID plus an explicit confirmation flag.

The desktop offers a reviewed one-off path and an automatic-sync path. Enabling automatic sync first reconciles the selected target and scope through the same deterministic Plan, backup, Apply and verification pipeline; only after success is the profile stored in SQLite. Every later Canonical mutation performed through AgentHub invokes all enabled profiles. This is event-driven application behavior, not a background watcher, host scan, merge, or import mechanism.

If an automatic run has no steps, it returns `changed=false` without persisting the transient Plan, creating a backup, or recording a transaction. If any host path changes, the transaction is always persisted with `mode=auto_sync` so rollback remains possible. Targets run independently: one failure is reported without preventing the other enabled targets or reverting the already-saved Canonical mutation. History may visually group adjacent successful automatic-sync records, but storage never coalesces or discards them.

## Deterministic Plan

A Plan records Canonical digest, Git HEAD, dirty state, target, selected Skills/Plugins/MCP IDs, the Rules toggle, expected projection digest, warnings and ordered file steps. Steps classify create, update, replace, delete, skip and constraint. Secrets are redacted before persistence, logs or JSON output. Apply re-generates the same selected projection and blocks if the selection or any other Plan input has drifted.

Plan presentation must expose a capability-level change list before raw file details. For Skills, Plugins and individually projected Rules this names the exact Canonical or host capability ID and whether it will be created, replaced or deleted, with the affected file count. Combined host documents such as an MCP collection may be labeled as a writable-domain replacement when the adapter cannot safely attribute a file diff to one server. Raw paths remain a secondary expandable diagnostic view.

## Explicit sync scope

Target Sync begins with a scope selector. Skills, Plugins and MCP servers are selected by Canonical ID; Rules are selected with a target-wide toggle because some adapters render them as a combined file or plugin. A selected domain is replaced from its selected Canonical subset, so host extras in that domain appear as deletes. A domain with no selected resources is omitted from the projection and is not touched. Rules disabled omits the Rules projection entirely. The selection snapshot is persisted in the Plan and is part of the Apply contract.

Each list domain also carries an explicit managed flag. This separates "do not touch" from "manage with an empty desired set". The latter produces deletion steps and is how a user intentionally clears `~/.agents/skills` or another writable host domain. Legacy selections remain managed whenever they contain selected IDs.

The automatic-sync profile uses the same visible selector. The UI shows the saved target scope and requires the user to review named Skills, Plugins, MCP servers and the Rules toggle when enabling or changing it; automatic sync is never represented as an opaque all-capabilities switch.

CLI `plan <target> --selection <json-file> --json` accepts the same selection, rejects unknown fields and missing/duplicate resource IDs, and persists it in the Plan. Omitting `--selection` retains full compatible scope. CLI initialization cannot overwrite an initialized library; whole-scan first import uses atomic staging and skips nonimportable discoveries.

## Target domains

| Target | Skills | MCP | Plugins | Rules |
| --- | --- | --- | --- | --- |
| Shared Agents | replace or explicitly clear `~/.agents/skills` | unsupported | unsupported | unsupported |
| Cursor | replace `~/.cursor/skills` | replace `mcpServers`, preserve other JSON | replace writable local plugins | generated global plugin |
| Codex | replace `~/.codex/skills` | replace `mcp_servers`, preserve other TOML | preserve CLI/marketplace-managed state and report v0.1 constraint | replace global `AGENTS.md` |
| Claude | replace writable `~/.claude/skills` except reserved dirs | replace user-scope servers | preserve CLI-managed plugin store and report v0.1 constraint | replace `~/.claude/rules` |

Before writing, the executor resolves all paths and records type, mode, hash and symlink target. It then creates a complete transaction backup, writes the projection, reads the target again and compares it with Expected State. Verification failure automatically restores the backup. Cleanup is manual and warns that rollback ability will be lost.

A verified Apply atomically records the applied transaction and marks that target enabled with its last-sync timestamp. Dashboard target state must never diverge from a successful Apply. Existing databases migrate historical applied transactions only when no explicit target row exists, preserving a later explicit disable.

## Manual rollback

Manual rollback is a new audited transaction, not a mutation of the historical Apply record. Given an Apply transaction, AgentHub restores the writable target domains from that transaction's pre-Apply backup. Before restoration it creates a fresh backup of the target's current writable domains so the rollback itself can be reversed. The restored paths are re-read and compared with the selected backup; failure restores the pre-rollback backup automatically.

Rollback never changes Canonical files or Git history. The restored host can therefore intentionally drift from Canonical, and the next Plan must show that drift before another Apply. The desktop confirmation names the target and full transaction ID and labels the action as restoring the state before that Apply.

`scan` is an import discovery operation only before initialization. Afterwards it is a read-only drift audit; it never merges or imports host changes.

The desktop exposes a separate **Scan and import** action after initialization. Discovery is read-only; exact `(kind, digest)` duplicates already in Canonical are disabled, duplicates across hosts are grouped, and only explicit confirmation imports selected candidates. Import is a Canonical mutation; it triggers saved automatic-sync profiles only when the user has enabled the separate “sync after reverse import” policy.

## Enforcement policy

The default policy is scoped: only explicitly managed domains are replaced. Optional strict-authoritative mode makes every adapter-confirmed writable domain for the selected target managed, including empty domains. It never expands into models, themes, accounts, permissions, cloud-managed capabilities, organization policy, reserved vendor directories, or undocumented plugin storage. Every strict write still uses Plan, backup, Apply and verification.

Post-initialization reverse import is a separate user action. Exact `(kind, digest)` duplicates are skipped and all selected items are staged and validated before Canonical activation. By default it does not write back to any host. Users may explicitly enable “sync after reverse import”; when enabled, only already-enabled automatic-sync profiles run, and unchanged targets create no transaction.

## Host inventory and cleanup

The Host Resources page is diagnostic and operational, not another source of truth. It scans only the documented user-global allowlist and labels every item as `canonical_match`, `host_only`, or `constraint`. A Canonical match means the host item corresponds to current Canonical content; it is not an ownership marker. Deleting such an item creates drift and an enabled automatic profile may recreate it after the next AgentHub mutation.

Cleanup is always a separate explicit transaction. The user selects exact resources, reviews their target, kind, name and path, and confirms deletion. Before mutation AgentHub re-scans the target and rejects stale or non-deletable IDs, then backs up all unique affected paths. Standalone Skills, writable Rules and recognized Cursor local plugins may be deleted directly. MCP cleanup removes only the selected server entry and preserves every unrelated key in the host configuration. Claude Code plugin storage and Codex marketplace/cache/plugin state are protected constraints in v0.1 and never expose a raw delete action.

The desktop may offer a one-click pre-selection of every `host_only` deletable resource for the current target (“not imported into AgentHub”). It is only a selection shortcut: the same review list, explicit acknowledgement and cleanup transaction apply, and Canonical matches or constraints are never included.

Cleanup never changes Canonical. Failure restores the pre-cleanup backup automatically. A successful cleanup reports the backup location and leaves any resulting drift visible; it does not silently trigger synchronization.

## Remote version synchronization
A user explicitly connects a dedicated AgentHub repository and branch. Each version save automatically synchronizes when connected; a manual sync retries failed uploads and receives changes on other devices without polling. Fetch and merge preserve both histories; conflicts abort the merge and preserve the local commit. Remote trees must contain only Canonical domains, agenthub.toml and an optional .gitignore, with no symlinks or submodules. Validate merged Canonical before committing. Push never forces; a concurrent remote update triggers one bounded fetch/merge retry. Dirty trees block manual sync. Connection tests authenticate read access before storing configuration; write access is verified at first push.
Host Resources also offers a quick-clean action for every adapter-confirmed deletable resource on the current target, including Canonical matches. It uses the same exact review, acknowledgement, rescan, backup and rollback flow. Protected vendor stores remain visible and excluded.
