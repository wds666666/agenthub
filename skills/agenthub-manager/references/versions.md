# Review, discard and replace library content

Inspect the installed `agenthub git --help` first. These commands are library-only recovery, separate from host transaction rollback.

## Inspect unsaved changes

`agenthub git changes` returns JSON grouped by capability kind/ID, action (`create`, `update`, `delete`) and changed files. It includes staged, unstaged and new content. `git status` exposes Git status, and `git diff` shows staged plus unstaged content against HEAD Use `git diff --include-untracked` for new file bodies or `git diff --capability skill:<id> --include-untracked` for one capability. Binary files report size and SHA-256; recognizable credential-bearing new JSON bodies are omitted. Do not print credential-bearing bodies in chat. Desktop Versions shows the same capability grouping with expandable file paths and a read-only Refresh action.

## Discard pending edits

Run `agenthub git recovery-plan discard`, review its exact changes and source commit, then use the returned ID:

```text
agenthub git recovery-apply <plan-id> --confirmation DISCARD
```

This restores the last local saved content and clears pending Canonical index entries. Unsaved new resources are removed from the active library, edits/deletions are undone, and the previous content/index remains in a private backup. It requires a saved local version. It does not remove local commits, alter device settings/credentials or write tools.

## Use remote content

Run `agenthub git recovery-plan remote`. It authenticates, downloads and validates the connected branch without merging, uploading or touching tools. Review all affected capabilities, especially local-only resources scheduled for removal, and the exact remote commit:

```text
agenthub git recovery-apply <plan-id> --confirmation REMOTE
```

The operation replaces Canonical content only. Local Git history remains; it does not reset the branch or force-push. The replacement may remain an unsaved change: validate/review it before an authorized version save. Saving later may merge histories or report a conflict; remote replacement is not a way to bypass normal upload validation.

Both operations reject stale local/index/candidate snapshots; remote replacement also rejects a changed remote branch. Use a new preview after drift. Do not manufacture confirmation without authorization for the listed discard/replacement. Existing authorization does not need repeated questions. Authentication, malformed layouts and missing branch failures leave library content intact.

## Recovery result

Report `backup_path`, `source_commit` and `pending_changes` from JSON. Backups contain previous Canonical directories, the schema file and Git index; they are private and never uploaded. They are not complete machine backups and do not contain credentials/device settings/history. If rollback fails, retain the backup and report the failure. AgentHub recovers interrupted activation before reading inventory on next startup. No automatic host delivery occurs for recovery actions; later delivery requires a reviewed Plan or already-enabled device scope.

## Save selected capabilities

Check `agenthub version --help` and `agenthub git commit --help` for selective-save support first. Older installers with only message/name/email cannot safely execute these commands: stop and request an updated build; do not fall back to a legacy all-content commit. Saving is local-only and does not write tools by default. To save and publish exactly one pending Skill while leaving staged or unstaged MCP unchanged:

```text
agenthub version save --message "Update management Skill" --only skill:agenthub-manager --push --no-host-sync
```

`git commit` is a compatibility alias with the same flags. Repeat `--only` for multiple complete capabilities, including deletions. `--exclude mcp` excludes that pending category when saving all others. A first selective save also includes the mandatory library schema. Unknown IDs, duplicate IDs and empty selections fail. All previous commits remain in the published ancestry; selection does not rewrite earlier history.

For explicit review:

```text
agenthub version plan --message "Update management Skill" --only skill:agenthub-manager --push --host-sync none --json
agenthub version inspect <plan-id>
agenthub version apply <plan-id> --confirm
```

The Plan contains selected and excluded pending capabilities/files, HEAD, remote branch/HEAD (queried only when push is requested), selected snapshot tree and host Plans. It binds content/index, saved profiles and remote settings. Changed inputs or host state require a fresh preview; do not retry Apply against a stale Plan. Preview never changes the real index or commits to the user's branch.

`--selection <file>` accepts this version-save selection, separate from a host SyncSelection:

```json
{"only":[{"kind":"skill","id":"agenthub-manager"}],"exclude":[]}
```

Save it outside portable content. Action flags are always explicit CLI arguments. `--local-only` excludes push; `--host-sync none` (default) disables host writes. `--host-sync enabled` or a comma-separated enabled target subset projects only this save's capabilities from its commit, intersected with saved profiles in preserve mode. It never delivers pending excluded content or deletes unrelated host resources. Unknown/disabled/unreviewed profiles fail. A host edit or missing verified overwrite baseline blocks delivery; review reverse import or a separate host Plan first.

## Publish saved history

`agenthub remote push` fetches and merges in an isolated checkout, validates content/history, refuses overlapping pending capabilities, then publishes and receives safe non-overlapping committed changes. The user's disjoint staged/unstaged files remain unchanged; no manual stash or index manipulation is needed. Publication never writes tools. Remote conflicts remain in the isolated candidate and the local saved version stays intact. `agenthub remote receive` is download-only and requires a clean tree.

Read save JSON: `commit_hash`, `committed_capabilities`, `excluded_pending_changes`, `remote_branch`, `remote_head`, `remote_merged`, local/remote success and errors. Host outcomes include their actual Plan and transaction ID. A local save with failed upload is not permission to make the same commit again; retry `remote push` after resolving the reported issue.
