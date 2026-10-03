# Resolve library Git conflicts with an Agent

Ordinary `agenthub git sync` fetches, attempts a merge, and **aborts the merge on failure**. It preserves the local commit and reports conflicting paths. There is usually no pending merge to continue. Native Git is required for the reviewed resolution workflow below; AgentHub has no automatic semantic conflict solver.

## Establish the exact merge

1. Confirm the library root from `doctor`, URL/branch from `git remote-status`, a saved local version and a clean working tree. Inspect saved automatic profiles, but do not run them during resolution. If pending edits exist, preserve and save them under the user's authorization first. Do not use a blind stash or destructive reset.
2. Run normal `agenthub git sync` once. If it fails on authentication, network, repository layout or schema validation instead of a content conflict, fix that cause rather than opening an arbitrary merge.
3. After a conflict abort, record immutable `HEAD` and `FETCH_HEAD` commit IDs with native Git. The latter is the remote version whose tree AgentHub just checked. Confirm there is no existing `MERGE_HEAD` and no subsequent fetch or local edit. Inspect the conflicting resources locally without exposing secrets.
4. With authorization to resolve these resources, establish a native merge of the recorded remote SHA using `--no-ff --no-commit --allow-unrelated-histories`. For every native Git invocation use the library root and disable hooks/signing. For example, with properly separated arguments:

```text
git -c core.hooksPath= -c commit.gpgSign=false -C <library-root> merge --no-ff --no-commit --allow-unrelated-histories <recorded-remote-sha>
git -C <library-root> diff --name-only --diff-filter=U
```

Exit status 1 with a pending merge and unmerged paths is expected. Other errors need investigation. If a previous attempt already left a pending merge, inspect its parents and edits before continuing; never abort it silently. Do not fetch or run normal AgentHub sync while resolving it.

## Merge meaning, then validate

Read base/local/remote versions, for example index stages `:1:<path>`, `:2:<path>`, `:3:<path>`. A newly added resource may have no base. Resolve the user's intended content rather than applying `ours`/`theirs` globally. Preserve complete Skill support files and Plugin payloads. For MCP, keep the intended transport/configuration and device portability; do not spread machine credentials. Keep Rule/Plugin/MCP IDs consistent with their directories.

If both edits are compatible, combine them. If instructions or delete/edit decisions contradict, use an established user preference or ask about those specific alternatives. A time limit does not authorize choosing one device. Retaining two resources is possible only when the user wants both and identities/references are updated consistently.

Stage only the resolved portable paths (include explicit deletions). Confirm no unmerged entries remain; review both working-tree and staged changes, run native `git diff --cached --check`, then `agenthub validate --json`. Validation checks file structure/schemas, not whether the merged instructions make sense. Do not add SQLite, backups, keys, unrelated project files or tool installation state. If the requested result cannot be established, preserve the resolution work in a private recovery copy and report it rather than discarding it.

## Save, share and deliver

Create the merge commit with native Git, disabled hooks/signing and the user's approved message. Native Git is used here because a merge may need saving even when its final content equals one parent. Verify both parents and a clean tree. Then use `agenthub git sync` so remote layout/history/credential checks and concurrent-update handling still apply; do not substitute a raw or forced push. If another device has pushed meanwhile, a new conflict may require another review.

Report local and remote success separately. A push failure does not erase the merge commit. After a successful sync, generate a new selected host Plan or run only already-enabled automatic profiles if deployment is authorized. Resolving a library conflict alone never implies permission to overwrite host tools.
