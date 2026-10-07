# Synchronization contract

## State machine

`Draft → Planned → Confirmed → BackedUp → Applying → Verifying → Applied`.

Any Apply or verification failure enters `RollingBack`, then `RolledBack` or `RollbackFailed`. A Plan never writes host files. Non-interactive Apply requires the exact Plan ID plus an explicit confirmation flag.

The desktop offers a reviewed one-off path and an automatic-sync path. Enabling automatic sync first reconciles the selected target and scope through the same deterministic Plan, backup, Apply and verification pipeline; only after success is the profile stored in SQLite. Every later Canonical mutation performed through AgentHub invokes all enabled profiles. This is event-driven application behavior, not a background watcher, host scan, merge, or import mechanism.

If an automatic run has no steps, it returns `changed=false` without persisting the transient Plan, creating a backup, or recording a transaction. If any host path changes, the transaction is always persisted with `mode=auto_sync` so rollback remains possible. Targets run independently: one failure is reported without preventing the other enabled targets or reverting the already-saved Canonical mutation. History may visually group adjacent successful automatic-sync records, but storage never coalesces or discards them.

## Deterministic Plan

A Plan records Canonical digest, Git HEAD, dirty state, target, selected Skills/Plugins/MCP IDs, the individual Rules, expected projection digest, warnings and ordered file steps. Steps classify create, update, replace, delete, skip and constraint. Secrets are redacted before persistence, logs or JSON output. Apply re-generates the same selected projection and blocks if the selection or any other Plan input has drifted.

Plan presentation must expose a capability-level change list before raw file details. For Skills, Plugins and individually projected Rules this names the exact Canonical or host capability ID and whether it will be created, replaced or deleted, with the affected file count. Combined host documents such as an MCP collection may be labeled as a writable-domain replacement when the adapter cannot safely attribute a file diff to one server. Raw paths remain a secondary expandable diagnostic view.

## Explicit sync scope

Target Sync begins with a scope selector. Skills, Plugins and MCP servers are selected by Canonical ID; Rules are selected by individual Canonical ID even when an adapter renders them as a combined file or plugin. Preserve mode updates only selected capabilities and retains all host extras. Replace mode reconciles the selected category to the selected subset and previews host-extra deletions. An unmanaged category is not touched. An empty managed category retains content in Preserve and explicitly clears writable content in Replace; protected content remains. The selection snapshot is persisted in the Plan and is part of the Apply contract.

Each list domain also carries an explicit managed flag. This separates "do not touch" from "manage with an empty desired set". In Replace mode the latter produces deletion steps and is how a user intentionally clears `~/.agents/skills` or another writable host domain. Legacy selections remain managed whenever they contain selected IDs.

The automatic-sync profile uses the same visible selector. The UI shows the saved target scope and requires the user to review named Skills, Plugins, MCP servers and the individual Rules when enabling or changing it; automatic sync is never represented as an opaque all-capabilities switch.

CLI `plan <target> --selection <json-file> --json` accepts the same selection, rejects unknown fields and missing/duplicate resource IDs, and persists it in the Plan. Omitting `--selection` retains full compatible scope. CLI initialization cannot overwrite an initialized library; whole-scan first import uses atomic staging and skips nonimportable discoveries.

## Target domains

| Target | Skills | MCP | Plugins | Rules |
| --- | --- | --- | --- | --- |
| Shared Agents | merge selected or explicitly replace `~/.agents/skills` | unsupported | unsupported | unsupported |
| Cursor | merge selected or replace `~/.cursor/skills` | merge selected or replace `mcpServers`, preserve other JSON | merge selected or replace writable local plugins | selected files in generated Rules container |
| Codex | merge selected or replace `~/.codex/skills` | merge selected or replace `mcp_servers`, preserve other TOML | preserve CLI/marketplace-managed state and report v0.1 constraint | selected identifiable blocks in `AGENTS.md` |
| Claude | merge selected or replace writable Skills, except reserved dirs | merge selected or replace user-scope servers | preserve CLI-managed plugin store and report v0.1 constraint | merge selected or replace `~/.claude/rules` |

Before writing, the executor resolves all paths and records type, mode, hash and symlink target. It then creates a complete transaction backup, writes the projection, reads the target again and compares it with Expected State. Verification failure automatically restores the backup. Cleanup is manual and warns that rollback ability will be lost.

On Unix, nested host symlinks are retained in local recovery copies without following their targets, so an installed `.venv` does not prevent sync or cleanup. Rollback compares link metadata as well as regular files and empty directories. Backup roots remain ordinary paths; Canonical import and projection still reject retained symlinks. Platforms without this recovery implementation refuse symlink-containing backups before any host write.

A verified Apply atomically records the applied transaction and marks that target enabled with its last-sync timestamp. Dashboard target state must never diverge from a successful Apply. Existing databases migrate historical applied transactions only when no explicit target row exists, preserving a later explicit disable.

## Manual rollback

Manual rollback is a new audited transaction, not a mutation of the historical Apply record. Given an Apply transaction, AgentHub restores the writable target domains from that transaction's pre-Apply backup. Before restoration it creates a fresh backup of the target's current writable domains so the rollback itself can be reversed. The restored paths are re-read and compared with the selected backup; failure restores the pre-rollback backup automatically.

Rollback never changes Canonical files or Git history. The restored host can therefore intentionally drift from Canonical, and the next Plan must show that drift before another Apply. The desktop confirmation names the target and full transaction ID and labels the action as restoring the state before that Apply.

`scan` is an import discovery operation only before initialization. Afterwards it is a read-only drift audit; it never merges or imports host changes.

The desktop exposes a separate **Scan and import** action after initialization. Discovery is read-only; exact normalized portable-content duplicates already in Canonical are disabled, duplicates across hosts are grouped, and only explicit confirmation imports selected candidates. Import is a Canonical mutation; it triggers saved automatic-sync profiles only when the user has enabled the separate “sync after reverse import” policy.

## Enforcement policy

The default mode is Preserve: only explicitly selected capabilities are added/replaced, and every unselected resource remains. Replace reconciles only explicitly managed categories; it never expands selection into other categories or automatically includes new library items. Changing the default in Settings does not change saved profiles. Legacy profiles are paused for review. Protected vendor stores, organization policy and undocumented plugin storage remain excluded.

Post-initialization reverse import is a separate user action. Exact normalized portable-content duplicates are skipped and all selected items are staged and validated before Canonical activation. By default it does not write back to any host. Users may explicitly enable “sync after reverse import”; when enabled, only already-enabled automatic-sync profiles run, and unchanged targets create no transaction.

## Host inventory and cleanup

The Host Resources page is diagnostic and operational, not another source of truth. It scans only the documented user-global allowlist and labels every item as `canonical_match`, `host_only`, or `constraint`. A Canonical match means the host item corresponds to current Canonical content; it is not an ownership marker. Deleting such an item creates drift and an enabled automatic profile may recreate it after the next AgentHub mutation.

Cleanup is always a separate explicit transaction. The user selects exact resources, reviews their target, kind, name and path, and confirms deletion. Before mutation AgentHub re-scans the target and rejects stale or non-deletable IDs, then backs up all unique affected paths. Standalone Skills, writable Rules and recognized Cursor local plugins may be deleted directly. MCP cleanup removes only the selected server entry and preserves every unrelated key in the host configuration. Claude Code plugin storage and Codex marketplace/cache/plugin state are protected constraints in v0.1 and never expose a raw delete action.

The desktop may offer a one-click pre-selection of every `host_only` deletable resource for the current target (“not imported into AgentHub”). It is only a selection shortcut: the same review list, explicit acknowledgement and cleanup transaction apply, and Canonical matches or constraints are never included.

Cleanup never changes Canonical. Failure restores the pre-cleanup backup automatically. A successful cleanup reports the backup location and leaves any resulting drift visible; it does not silently trigger synchronization.

## Remote version synchronization
A user explicitly connects a dedicated AgentHub repository and branch. Each version save automatically synchronizes when connected; a manual sync retries failed uploads and receives changes on other devices without polling. Fetch and merge preserve both histories; conflicts abort the merge and preserve the local commit. Remote trees must contain only Canonical domains, agenthub.toml and an optional .gitignore, with no symlinks or submodules. Validate merged Canonical before committing. Push never forces; a concurrent remote update triggers one bounded fetch/merge retry. Dirty trees block manual sync. Connection tests authenticate read access before storing configuration; write access is verified at first push.
Host Resources also offers a quick-clean action for every adapter-confirmed deletable resource on the current target, including Canonical matches. It uses the same exact review, acknowledgement, rescan, backup and rollback flow. Protected vendor stores remain visible and excluded.

## Repository authentication and state

First-run restoration reads only. A supplied branch must exist; an omitted branch prefers `agenthub` when present, otherwise the remote HEAD branch. Empty repositories and non-AgentHub/unsupported layouts fail without changing the local library or remote. The restored local HEAD equals the remote branch commit, and subsequent version saves use the same remote/branch and normal fetch/merge/push rules. Connection after restoration is read-verified, not write-verified. Each device authorizes separately and starts with no enabled host or automatic-sync profiles.

When the local version is an ancestor of the fetched version, validate the incoming library and history in a private local checkout before fast-forwarding. This receives versions without creating a synthetic local merge commit or requiring a new device to configure an author first. Divergent versions still use the reviewed conflict-preserving merge path. Sync remains bidirectional and needs write permission; author identity is required when saving local changes.

GitHub and self-hosted Git/Gitea support HTTPS username/access-token sign-in. Login verifies reading without uploading; an actual push establishes upload permission. UI states distinguish configured but unverified, last read verification, last successful sync, failed authentication and incomplete sync. A stored URL alone is not successful authentication. Only an explicit sign-in-and-retry action retries the failed upload after login, and local commits remain saved. Forgetting credentials preserves repository configuration, library content and versions.

## Skill change reminders

The initialized desktop checks existing library Skill IDs in Shared Agents, Cursor, Codex and Claude Code automatically while visible, with a two-minute minimum interval. Only ordinary first-level Skill directories are candidates. Compare the complete portable content, including supporting files, using the existing runtime exclusions. Metadata fingerprints cache per-directory digests; unchanged content is rehashed after thirty minutes. Filesystem errors produce a partial-check warning, never a false clean result. The check never imports, writes hosts, commits or accesses the network.

A differing same-ID resource shows a red reminder in library navigation and on the corresponding Skill. Content already present anywhere in the library is excluded, so explicitly importing a changed copy clears its reminder after recheck. Opening a reminder does not mark it resolved. The review dialog shows source and path, leaves selection empty, and uses the existing atomic reverse-import command with stale-result validation. Import retains the original library item and adds the changed content under a unique ID; it is not an in-place update or merge. Existing optional sync-after-import policy remains applicable.

## Selected capability synchronization (current contract)

Preserve is the default: add/replace only explicitly selected capabilities; leave every unselected host resource intact, including retired selections and library deletions. Replace reconciles only explicitly managed categories to the selected subset. Neither mode expands scope. Rules have individual IDs; creating a rule saves only to Canonical and never distributes it or selects it. Existing selected rules may synchronize on subsequent edits. Old automatic profiles pause for mode/rule-scope review. Package installation supplies Desktop and CLI, with the manager Skill available separately for manual installation.

Discovery uses a shared comparison digest of portable capability content, separate from transaction file digests. MCP compares normalized target-representable configuration, Rules exclude generated wrappers, Plugins compare their complete payload, and Skills retain runtime exclusions. Generated rule containers are identified explicitly. Equal content is blocked from reverse import in the backend as well as the UI; same-name differences remain reviewable. Parsing/identity uncertainty blocks destructive rewriting rather than treating the configuration as empty.

Successful remote synchronization first validates updates in Canonical, then runs enabled device profiles only when library content changed. Saving a version also runs enabled device profiles. Host failures are returned separately from successful library saves or remote updates. `git receive` is a library-only maintenance operation and never writes tools or uploads.

## Version recovery actions

Versions shows unsaved changes grouped by capability (Skills, MCP, Rules, Plugins), with file details separately available. Refresh is read-only. Discard changes restores the last local saved content; Use remote content previews a validated dedicated-repository snapshot and replaces library content only. The remote action does not reset local history, force-push, upload or deliver to hosts: the replacement remains a working-tree change for explicit saving. Both actions require an immutable preview, exact confirmation, private recovery backup including the index, stale-input checks, verification and recovery on failure/interruption. Device state, credentials, Git history and host paths are excluded from replacement. A missing local saved version blocks discard; remote authentication/layout errors do not change local content.

Recovery previews use Git clean-filter comparison (including Windows CRLF and repository attributes) to select changed canonical paths. Raw byte digests remain unchanged for backup integrity and concurrent-edit detection. Ignored/untracked canonical files that replacement will remove are still listed.
