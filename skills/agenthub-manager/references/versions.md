# Review, discard and replace library content

Inspect the installed `agenthub git --help` first. These commands are library-only recovery, separate from host transaction rollback.

## Inspect unsaved changes

`agenthub git changes` returns JSON grouped by capability kind/ID, action (`create`, `update`, `delete`) and changed files. It includes staged, unstaged and new content. `git status` exposes Git status, and `git diff` shows staged plus unstaged content against HEAD (new file paths are listed). Do not print credential-bearing bodies in chat. Desktop Versions shows the same capability grouping with expandable file paths and a read-only Refresh action.

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
