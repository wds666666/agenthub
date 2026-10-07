# Migrate an existing backup workflow

## Preserve the source first

Inspect the user's current backup scope and repository before replacing it. AgentHub backs up user-global capability content, not conversations, tool accounts, project settings, executable installations, or all application state. Keep the old backup and scheduled jobs until a restore/sync has been verified; do not delete or disable them as an implied side effect.

Prefer a dedicated empty remote repository for the portable AgentHub library. Existing backup repos with tool caches, SQLite, credentials or unrelated projects are not valid destinations: AgentHub checks reachable history, not just the latest snapshot. Never rewrite old history or force-push to make a migration pass.

If the local library already exists, inspect `doctor`, inventory, saved automatic profiles, working-tree status and remote settings. Do not reset or reinitialize it to simplify onboarding. Copy the old content and any pending edits to a private recovery location before changing an existing workflow.

## Import, verify and save

1. Use Desktop source/category selection to copy the intended global capabilities. Skills are discovered only in immediate child directories of each tool's global Skills root; a parent's nested instructions and support files travel with it, rather than becoming separate imports. Preserve the originals. For a CLI-only first setup, use `init --empty` to start an empty library; `init --import-all` requires authorization for its whole scan and skips nonimportable items. An initialized library uses Desktop **Scan and import** for later imports.
2. Run `validate --json` and `inventory --json`; compare IDs, counts and support files with the selected source. Skill imports exclude local virtual environments (`.venv`/`venv`), installed dependencies (`node_modules`), Git metadata and generated caches; scripts, dependency manifests/lockfiles, resources and nested instructions remain. Recreate dependencies on each destination device instead of backing up its runtime installation. Exact duplicates may reduce the count. Unsupported caches must be reported rather than claimed as backed up.
3. Review the working tree and save an initial version with the user's message and repository-local author identity. Then connect the dedicated URL/branch with `git connect`; this verifies read access, not write access. Run `git sync`, inspect the result, and verify the destination has the expected commit and content.
4. Generate an explicit selected host Plan only if deployment is requested. Review deletion steps before applying; Preserve updates selected abilities and keeps extras; Replace reconciles only explicitly managed categories after deletion review. Existing source resources must not be cleaned just because they were imported.
5. Enable automatic profiles through the Desktop only after reviewing their scope. Each device has its own scope; `git receive` and first-run restoration do not write tools; ordinary `git sync` with received content runs enabled profiles.

Do not promise automatic backup of every filesystem edit. Desktop/CLI version saves push when connected. An external Agent editing Canonical runs validation, makes an authorized version save, and explicitly delivers through reviewed Plans or previously enabled profiles. Offline or failed remote uploads leave the local version intact; retry synchronization without adding repeated commits.

## Other devices and recovery

On a new empty device, prefer `bootstrap <URL>` to restore exact content/history without an initial local commit or host writes. If combining existing local library content, save it first and connect the same dedicated repository and branch. Reconcile both histories; use [conflict resolution](conflicts.md) if the same resources differ. Validate received content and review that device's host Plan before deployment.

Git recovers committed capability files. Host transaction backups recover host writes. Library deletion archives recover uncommitted removed files. A private complete copy of the AgentHub root additionally preserves SQLite, keys, profiles and pending content; it must not be uploaded as the portable Git repository. Close the UI and stop CLI writes before taking/restoring that complete local copy. Test recovery with a temporary root, and use temporary **host homes** for any delivery test; `AGENTHUB_HOME` alone does not isolate host paths.

Report what migrated, skipped resources, root/repository/branch, local and remote commit state, and whether hosts were deployed. A missing credential, conflict or unsupported resource is an incomplete part of the migration, not a successful backup.

## HTTPS authentication

In desktop Versions, use repository sign-in for GitHub or self-hosted Git/Gitea. The username and access token are encrypted locally and shared with CLI. Never ask for a token in chat, put it in command arguments or embed it in a repository URL. Login verifies reading; explicit remote push publishes saved history. Failed uploads preserve local versions. Re-enter expired credentials in the desktop, or delete the saved entry to use system Git/SSH. Credentials do not synchronize across devices.
