//! Capability-scoped, reviewed saves. Pending content never becomes a projection source.
use crate::{
    canonical, git,
    models::{CapabilityKey, CapabilityKind, Plan, SyncMode, SyncSelection, Target},
    paths::{self, AgentHubPaths},
    storage::Store,
    transaction,
    versions::{self, CapabilityChange},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveOptions {
    /// None means all pending Canonical changes. Some([]) is intentionally invalid.
    pub only: Option<Vec<CapabilityKey>>,
    #[serde(default)]
    pub exclude: Vec<CapabilityKind>,
    #[serde(default)]
    pub push: bool,
    /// "none", "enabled", or a comma-separated subset of enabled profiles.
    #[serde(default = "no_hosts")]
    pub host_sync: String,
}
fn no_hosts() -> String {
    "none".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReview {
    pub path: String,
    pub size: u64,
    pub digest: String,
    pub binary: bool,
    pub credential_warning: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePreview {
    pub id: Uuid,
    pub message: String,
    pub options: SaveOptions,
    pub changes: Vec<CapabilityChange>,
    pub file_review: Vec<FileReview>,
    pub excluded_pending_changes: Vec<CapabilityChange>,
    pub head: Option<String>,
    pub head_ref: String,
    pub remote_head: Option<String>,
    pub remote_url: Option<String>,
    pub remote_branch: Option<String>,
    pub candidate_tree: String,
    pub before_digest: String,
    pub index_digest: String,
    pub profiles_digest: String,
    pub host_plans: Vec<Plan>,
    pub name: String,
    pub email: String,
}
fn stage_path(paths: &AgentHubPaths, id: Uuid) -> PathBuf {
    paths.root.join(format!("runtime/save-{id}"))
}
fn plan_path(paths: &AgentHubPaths, id: Uuid) -> PathBuf {
    paths.root.join(format!("runtime/save-{id}.json"))
}
fn prefix(kind: CapabilityKind) -> &'static str {
    match kind {
        CapabilityKind::Skill => "skills",
        CapabilityKind::Mcp => "mcp",
        CapabilityKind::Rule => "rules",
        CapabilityKind::Plugin => "plugins",
    }
}
pub fn parse_key(value: &str) -> Result<CapabilityKey> {
    let (kind, id) = value
        .split_once(':')
        .context("use kind:id, for example skill:agenthub-manager")?;
    let kind = match kind {
        "skill" => CapabilityKind::Skill,
        "mcp" => CapabilityKind::Mcp,
        "plugin" => CapabilityKind::Plugin,
        "rule" => CapabilityKind::Rule,
        _ => anyhow::bail!("unknown capability kind"),
    };
    anyhow::ensure!(canonical::valid_id(id), "invalid capability id");
    Ok(CapabilityKey {
        kind,
        id: id.into(),
    })
}
fn resource_path(change: &CapabilityChange) -> String {
    change
        .kind
        .map(|k| format!("{}/{}", prefix(k), change.id))
        .unwrap_or_else(|| change.id.clone())
}
fn profiles_digest(store: &Store) -> Result<String> {
    Ok(canonical::sha256(&serde_json::to_vec(
        &store.auto_sync_profiles()?,
    )?))
}
/// Copy only ordinary files and directories, preserving executable bits. No payload is executed.
fn copy_content(source: &Path, destination: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(source)?;
    anyhow::ensure!(
        !meta.file_type().is_symlink(),
        "capability contains a symlink"
    );
    if meta.is_dir() {
        fs::create_dir_all(destination)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            copy_content(&item.path(), &destination.join(item.file_name()))?;
        }
    } else {
        anyhow::ensure!(meta.is_file(), "unsupported capability file");
        fs::create_dir_all(destination.parent().context("file parent")?)?;
        fs::copy(source, destination)?;
        fs::set_permissions(destination, meta.permissions())?;
    }
    Ok(())
}
fn candidate(paths: &AgentHubPaths, stage: &Path, head: Option<&str>) -> Result<AgentHubPaths> {
    if let Some(head) = head {
        versions::checkout(paths, stage, head, false)?;
    } else {
        fs::create_dir_all(stage)?;
        git::run(stage, &["init"])?;
    }
    let snapshot = AgentHubPaths::new(paths.user_home.clone(), stage.to_path_buf());
    snapshot.ensure_runtime()?;
    Ok(snapshot)
}
fn identity(root: &Path, name: Option<&str>, email: Option<&str>) -> Result<(String, String)> {
    let current = git::identity(root)?;
    let name = name
        .map(str::to_owned)
        .or(current.name)
        .context("Git identity is required")?;
    let email = email
        .map(str::to_owned)
        .or(current.email)
        .context("Git identity is required")?;
    anyhow::ensure!(
        !name.trim().is_empty()
            && !name.contains(['\r', '\n'])
            && email.contains('@')
            && !email.chars().any(char::is_whitespace),
        "valid Git identity is required"
    );
    Ok((name, email))
}
pub fn preview(
    paths: &AgentHubPaths,
    store: &Store,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
    mut options: SaveOptions,
) -> Result<SavePreview> {
    anyhow::ensure!(!message.trim().is_empty(), "commit message is required");
    if options.host_sync.is_empty() {
        options.host_sync = no_hosts();
    }
    if let Some(keys) = &options.only {
        anyhow::ensure!(!keys.is_empty(), "select at least one capability");
        let mut seen = BTreeSet::new();
        for key in keys {
            parse_key(&format!("{}:{}", key.kind.as_str(), key.id))?;
            anyhow::ensure!(seen.insert(key), "duplicate capability selection");
        }
    }
    let changes = versions::changes(paths)?;
    let _lock = versions::lock(paths)?;
    anyhow::ensure!(
        git::run(&paths.root, &["ls-files", "-u"])?.is_empty(),
        "resolve Git conflicts before saving"
    );
    anyhow::ensure!(
        !paths.root.join(".git/MERGE_HEAD").exists(),
        "resolve or abort the active Git merge before saving"
    );
    let head = git::snapshot(&paths.root)?.head;
    let before_digest = versions::digest(&paths.root)?;
    let index_digest = versions::index_digest(paths)?;
    let (name, email) = identity(&paths.root, name, email)?;
    let mut selected: Vec<_> = changes
        .iter()
        .filter(|c| {
            !c.kind.is_some_and(|k| options.exclude.contains(&k))
                && options.only.as_ref().is_none_or(|keys| {
                    c.kind.is_some_and(|kind| {
                        keys.contains(&CapabilityKey {
                            kind,
                            id: c.id.clone(),
                        })
                    })
                })
        })
        .cloned()
        .collect();
    anyhow::ensure!(
        !selected.is_empty(),
        "there are no selected Canonical changes to commit"
    );
    if let Some(keys) = &options.only {
        for key in keys {
            anyhow::ensure!(
                selected
                    .iter()
                    .any(|c| c.kind == Some(key.kind) && c.id == key.id),
                "selected capability has no pending changes: {}:{}",
                key.kind.as_str(),
                key.id
            );
        }
    }
    if head.is_none()
        && !selected
            .iter()
            .any(|c| c.kind.is_none() && c.id == "agenthub.toml")
    {
        if let Some(schema) = changes
            .iter()
            .find(|c| c.kind.is_none() && c.id == "agenthub.toml")
        {
            selected.push(schema.clone());
        }
    }
    let excluded = changes
        .into_iter()
        .filter(|c| !selected.iter().any(|s| s.kind == c.kind && s.id == c.id))
        .collect();
    let id = Uuid::new_v4();
    let stage = stage_path(paths, id);
    let result = (|| {
        let snapshot = candidate(paths, &stage, head.as_deref())?;
        // Saving must use the source repository's clean rules. Recovery snapshots
        // deliberately disable autocrlf; reusing that setting here would produce
        // a different tree from a Windows path-scoped commit.
        let autocrlf = git::run(&paths.root, &["config", "--get", "core.autocrlf"])
            .unwrap_or_else(|_| "false".into());
        git::run(&stage, &["config", "core.autocrlf", &autocrlf])?;
        for key in ["core.eol", "core.safecrlf", "core.filemode"] {
            if let Ok(value) = git::run(&paths.root, &["config", "--get", key]) {
                git::run(&stage, &["config", key, &value])?;
            }
        }

        let mut include: Vec<String> = selected.iter().map(resource_path).collect();
        // Every new dedicated library requires its schema, even for a selective first save.
        if head.is_none() && !include.iter().any(|p| p == "agenthub.toml") {
            include.push("agenthub.toml".into());
        }
        for path in &include {
            let dest = stage.join(path);
            if dest.is_dir() {
                fs::remove_dir_all(&dest)?;
            } else if dest.exists() {
                fs::remove_file(&dest)?;
            }
            let source = paths.root.join(path);
            match fs::symlink_metadata(&source) {
                Ok(_) => copy_content(&source, &dest)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e).context("cannot inspect selected capability"),
            }
        }
        canonical::validate(&snapshot).context("selected snapshot validation failed")?;
        git::run(&stage, &["config", "user.name", &name])?;
        git::run(&stage, &["config", "user.email", &email])?;
        let mut add = vec!["add", "-A", "--"];
        add.extend(include.iter().map(String::as_str));
        git::run(&stage, &add)?;
        let tree = git::run(&stage, &["write-tree"])?;
        git::run(&stage, &["commit", "-m", message])?;
        if options.push {
            git::check_upload_history(&stage)?;
        }
        let (remote_url, remote_branch) = git::remote_connection(&paths.root)?;
        let remote_head = if options.push {
            let url = remote_url
                .as_deref()
                .context("connect a repository before requesting push")?;
            git::run(
                &paths.root,
                &[
                    "ls-remote",
                    "--heads",
                    url,
                    &format!("refs/heads/{remote_branch}"),
                ],
            )?
            .split_whitespace()
            .next()
            .map(str::to_owned)
        } else {
            None
        };
        let host_plans = host_plans(&snapshot, store, &selected, &options.host_sync)?;
        let mut file_review = Vec::new();
        for path in selected.iter().flat_map(|c| &c.files) {
            let file = stage.join(path);
            if file.is_file() {
                let bytes = fs::read(&file)?;
                let text = std::str::from_utf8(&bytes);
                let warning = text
                    .as_ref()
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
                    .is_some_and(|v| git::has_plaintext_credentials(&v));
                file_review.push(FileReview {
                    path: path.clone(),
                    size: bytes.len() as u64,
                    digest: canonical::sha256(&bytes),
                    binary: text.is_err() || bytes.contains(&0),
                    credential_warning: warning,
                });
            }
        }
        let plan = SavePreview {
            id,
            message: message.into(),
            options,
            changes: selected,
            file_review,
            excluded_pending_changes: excluded,
            head,
            head_ref: fs::read_to_string(paths.root.join(".git/HEAD"))?,
            remote_head,
            remote_url,
            remote_branch: Some(remote_branch),
            candidate_tree: tree,
            before_digest,
            index_digest,
            profiles_digest: profiles_digest(store)?,
            host_plans,
            name,
            email,
        };
        anyhow::ensure!(
            versions::digest(&paths.root)? == plan.before_digest
                && versions::index_digest(paths)? == plan.index_digest
                && git::snapshot(&paths.root)?.head == plan.head,
            "library changed during preview; retry"
        );
        let bytes = serde_json::to_vec(&plan)?;
        store.set_meta(
            &format!("version_save_plan_{id}"),
            &canonical::sha256(&bytes),
        )?;
        fs::write(plan_path(paths, id), bytes)?;
        paths::set_private_file(&plan_path(paths, id))?;
        Ok(plan)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}
fn selected_scope(selection: &SyncSelection, changes: &[CapabilityChange]) -> SyncSelection {
    let has = |kind, id: &String| {
        changes
            .iter()
            .any(|c| c.kind == Some(kind) && c.id == *id && c.action != "delete")
    };
    SyncSelection {
        mode: SyncMode::Preserve,
        skills: selection
            .skills
            .iter()
            .filter(|id| has(CapabilityKind::Skill, id))
            .cloned()
            .collect(),
        mcp: selection
            .mcp
            .iter()
            .filter(|id| has(CapabilityKind::Mcp, id))
            .cloned()
            .collect(),
        plugins: selection
            .plugins
            .iter()
            .filter(|id| has(CapabilityKind::Plugin, id))
            .cloned()
            .collect(),
        rule_ids: selection
            .rule_ids
            .iter()
            .filter(|id| has(CapabilityKind::Rule, id))
            .cloned()
            .collect(),
        ..Default::default()
    }
}
fn host_plans(
    snapshot: &AgentHubPaths,
    store: &Store,
    changes: &[CapabilityChange],
    mode: &str,
) -> Result<Vec<Plan>> {
    if mode == "none" {
        return Ok(Vec::new());
    }
    let targets = if mode == "enabled" {
        None
    } else {
        Some(
            mode.split(',')
                .map(str::parse::<Target>)
                .collect::<Result<BTreeSet<_>>>()?,
        )
    };
    let profiles = store.auto_sync_profiles()?;
    if let Some(targets) = &targets {
        for target in targets {
            anyhow::ensure!(
                profiles
                    .iter()
                    .any(|p| p.target == *target && p.enabled && !p.needs_review),
                "requested target needs an enabled, reviewed profile"
            );
        }
    }
    profiles
        .into_iter()
        .filter(|p| {
            p.enabled && !p.needs_review && targets.as_ref().is_none_or(|t| t.contains(&p.target))
        })
        .map(|p| {
            crate::planner::create_with_selection(
                snapshot,
                p.target,
                Some(&selected_scope(&p.selection, changes)),
            )
        })
        .collect()
}
pub fn inspect(paths: &AgentHubPaths, id: Uuid) -> Result<SavePreview> {
    Ok(serde_json::from_slice(
        &fs::read(plan_path(paths, id)).context("version Plan is unavailable")?,
    )?)
}
pub fn apply(
    paths: &AgentHubPaths,
    store: &Store,
    id: Uuid,
    confirm: bool,
) -> Result<git::CommitResult> {
    anyhow::ensure!(confirm, "review the version Plan and pass --confirm");
    let _lock = versions::lock(paths)?;
    let plan_bytes = fs::read(plan_path(paths, id))?;
    anyhow::ensure!(
        store.meta(&format!("version_save_plan_{id}"))?.as_deref()
            == Some(&canonical::sha256(&plan_bytes)),
        "version Plan changed; preview again"
    );
    let plan = inspect(paths, id)?;
    let stage = stage_path(paths, id);
    anyhow::ensure!(
        plan.id == id
            && versions::digest(&paths.root)? == plan.before_digest
            && versions::index_digest(paths)? == plan.index_digest
            && git::snapshot(&paths.root)?.head == plan.head
            && fs::read_to_string(paths.root.join(".git/HEAD"))? == plan.head_ref
            && profiles_digest(store)? == plan.profiles_digest,
        "stale version Plan; preview again"
    );
    let (remote_url, remote_branch) = git::remote_connection(&paths.root)?;
    anyhow::ensure!(
        remote_url == plan.remote_url && Some(remote_branch.clone()) == plan.remote_branch,
        "remote settings changed; preview again"
    );
    if plan.options.push {
        let url = plan
            .remote_url
            .as_deref()
            .context("connected remote missing")?;
        let actual = git::run(
            &paths.root,
            &[
                "ls-remote",
                "--heads",
                url,
                &format!("refs/heads/{remote_branch}"),
            ],
        )?
        .split_whitespace()
        .next()
        .map(str::to_owned);
        anyhow::ensure!(
            actual == plan.remote_head,
            "remote HEAD changed; preview again"
        );
    }
    anyhow::ensure!(
        git::run(&stage, &["rev-parse", "HEAD^{tree}"])? == plan.candidate_tree
            && !git::snapshot(&stage)?.dirty,
        "candidate snapshot changed; preview again"
    );
    let mut snapshot = AgentHubPaths::new(paths.user_home.clone(), stage.clone());
    snapshot.backups = paths.backups.clone();
    // Bind the actual host to the preview before committing any local version.
    let fresh = host_plans(&snapshot, store, &plan.changes, &plan.options.host_sync)?;
    anyhow::ensure!(
        fresh.len() == plan.host_plans.len()
            && fresh
                .iter()
                .zip(&plan.host_plans)
                .all(|(a, b)| a.steps == b.steps),
        "host changed since preview; preview again"
    );
    for p in &fresh {
        transaction::check_auto_drift(&snapshot, store, p)?;
    }
    let mut include: Vec<String> = plan.changes.iter().map(resource_path).collect();
    if plan.head.is_none() && !include.iter().any(|p| p == "agenthub.toml") {
        include.push("agenthub.toml".into());
    }
    let mut args = Vec::new();
    let name_arg = format!("user.name={}", plan.name);
    let email_arg = format!("user.email={}", plan.email);
    args.extend([
        "-c",
        name_arg.as_str(),
        "-c",
        email_arg.as_str(),
        "commit",
        "--only",
        "-m",
        plan.message.as_str(),
        "--",
    ]);
    args.extend(include.iter().map(String::as_str));
    git::run(&paths.root, &["config", "user.name", &plan.name])?;
    git::run(&paths.root, &["config", "user.email", &plan.email])?;
    let index_file = paths.root.join(".git/index");
    let old_index = fs::read(&index_file).ok();
    let saved = (|| -> Result<()> {
        let mut add = vec!["add", "-A", "--"];
        add.extend(include.iter().map(String::as_str));
        git::run(&paths.root, &add)?;
        git::run(&paths.root, &args)?;
        let actual = git::run(&paths.root, &["rev-parse", "HEAD^{tree}"])?;
        if actual != plan.candidate_tree {
            let saved = git::snapshot(&paths.root)?
                .head
                .context("saved HEAD missing")?;
            if let Some(before) = &plan.head {
                git::run(&paths.root, &["update-ref", "HEAD", before, &saved])?;
            } else {
                git::run(&paths.root, &["update-ref", "-d", "HEAD", &saved])?;
            }
            anyhow::bail!(
                "selected content changed while saving; commit rolled back, preview again"
            );
        }
        Ok(())
    })();
    if let Err(error) = saved {
        if let Some(bytes) = old_index {
            fs::write(index_file, bytes)?;
        } else {
            let _ = fs::remove_file(index_file);
        }
        return Err(error);
    }
    let hash = git::snapshot(&paths.root)?
        .head
        .context("saved HEAD missing")?;
    let mut result = git::CommitResult {
        auto_sync: Vec::new(),
        auto_sync_error: None,
        local_saved: true,
        remote_synced: false,
        remote_error: None,
        commit_hash: hash.clone(),
        remote_branch: plan.remote_branch.clone(),
        remote_head: None,
        remote_merged: false,
        committed_capabilities: plan.changes.clone(),
        excluded_pending_changes: plan.excluded_pending_changes.clone(),
    };
    if plan.options.push {
        match push_committed_inner(paths) {
            Ok(r) => {
                result.remote_synced = true;
                result.remote_head = Some(r.head);
                result.remote_merged = r.merged;
            }
            Err(e) => result.remote_error = Some(crate::secrets::redact(&format!("{e:#}"))),
        }
    }
    // Re-create from the actual commit, never from the live pending library or merged remote files.
    let projection_stage = paths
        .root
        .join(format!("runtime/committed-{}", Uuid::new_v4()));
    if plan.options.host_sync != "none" {
        let projected = (|| -> Result<Vec<crate::models::AutoSyncOutcome>> {
            versions::checkout(paths, &projection_stage, &hash, false)?;
            let mut committed =
                AgentHubPaths::new(paths.user_home.clone(), projection_stage.clone());
            committed.backups = paths.backups.clone();
            let host = host_plans(&committed, store, &plan.changes, &plan.options.host_sync)?;
            host.into_iter()
                .map(|p| transaction::run_reviewed_auto_plan(&committed, store, p))
                .collect()
        })();
        match projected {
            Ok(outcomes) => result.auto_sync = outcomes,
            Err(e) => result.auto_sync_error = Some(crate::secrets::redact(&format!("{e:#}"))),
        }
    }
    let _ = fs::remove_dir_all(projection_stage);
    let _ = fs::remove_dir_all(stage);
    let _ = fs::remove_file(plan_path(paths, id));
    Ok(result)
}
#[derive(Debug, Serialize)]
pub struct PushResult {
    pub head: String,
    pub branch: String,
    pub merged: bool,
}
pub fn push_committed(paths: &AgentHubPaths) -> Result<PushResult> {
    let _lock = versions::lock(paths)?;
    push_committed_inner(paths)
}
fn push_committed_inner(paths: &AgentHubPaths) -> Result<PushResult> {
    let settings = git::remote_settings(&paths.root)?;
    let url = settings.url.context("connect a repository first")?;
    git::validate_remote(&url)?;
    push_to(paths, &url, &settings.branch)
}
fn push_to(paths: &AgentHubPaths, url: &str, branch: &str) -> Result<PushResult> {
    let before = git::snapshot(&paths.root)?
        .head
        .context("save a local version before push")?;
    let index = versions::index_digest(paths)?;
    let digest = versions::digest(&paths.root)?;
    let stage = paths.root.join(format!("runtime/push-{}", Uuid::new_v4()));
    let result = (|| {
        versions::checkout(paths, &stage, &before, false)?;
        // Fast-forwarding or pushing existing commits does not create a version
        // and must work on a freshly restored device without a local author.
        // Git still requires an actual configured identity for a merge commit.
        let author = git::identity(&paths.root)?;
        if let Some(name) = author.name.filter(|value| !value.trim().is_empty()) {
            git::run(&stage, &["config", "user.name", &name])?;
        }
        if let Some(email) = author.email.filter(|value| !value.trim().is_empty()) {
            git::run(&stage, &["config", "user.email", &email])?;
        }
        for attempt in 0..2 {
            let remote_ref = format!("refs/heads/{branch}");
            let remote = git::run_authenticated(
                &stage,
                &paths.root,
                &["ls-remote", "--heads", url, &remote_ref],
            )?;
            if !remote.is_empty() {
                git::run_authenticated(
                    &stage,
                    &paths.root,
                    &["fetch", "--no-tags", url, &remote_ref],
                )?;
                // All reachable history, not just the final tree, must be portable and credential-free.
                git::run(&stage, &["branch", "-f", "agenthub-incoming", "FETCH_HEAD"])?;
                git::check_upload_history_ref(&stage, "FETCH_HEAD")?;
                // Retain the exact validated remote commit for native conflict review.
                let incoming = git::run(&stage, &["rev-parse", "FETCH_HEAD"])?;
                git::run(
                    &paths.root,
                    &[
                        "fetch",
                        "--no-tags",
                        &git::git_path_argument(&stage)?,
                        &incoming,
                    ],
                )?;

                git::run(
                    &stage,
                    &[
                        "merge",
                        "--no-edit",
                        "--allow-unrelated-histories",
                        "FETCH_HEAD",
                    ],
                )
                .context("remote merge conflicts; local version and pending edits preserved")?;
            }
            let snapshot = AgentHubPaths::new(paths.user_home.clone(), stage.clone());
            canonical::validate(&snapshot)?;
            git::check_upload_history(&stage)?;
            let after = git::snapshot(&stage)?
                .head
                .context("remote candidate HEAD")?;
            // Import objects only: does not alter the current index, branch or working files.
            git::run(
                &paths.root,
                &[
                    "fetch",
                    "--no-tags",
                    &git::git_path_argument(&stage)?,
                    &after,
                ],
            )?;
            anyhow::ensure!(
                git::snapshot(&paths.root)?.head.as_deref() == Some(&before)
                    && versions::index_digest(paths)? == index
                    && versions::digest(&paths.root)? == digest,
                "local library changed during remote reconciliation; retry"
            );
            let pending = git::pending_paths(&paths.root)?;
            let incoming = git::run(
                &stage,
                &["diff", "--name-only", "--no-renames", "-z", &before, &after],
            )?;
            let owner = |p: &str| p.split('/').take(2).collect::<Vec<_>>().join("/");
            anyhow::ensure!(
                !incoming
                    .split('\0')
                    .filter(|p| !p.is_empty())
                    .any(|p| pending.iter().any(|q| owner(p) == owner(q))),
                "remote changes overlap a pending capability; save or review it first"
            );
            git::run(
                &paths.root,
                &["read-tree", "-n", "-m", "-u", &before, &after],
            )
            .context("remote changes would overwrite pending local content")?;
            let destination = format!("HEAD:{remote_ref}");
            match git::run_authenticated(&stage, &paths.root, &["push", url, &destination]) {
                Ok(_) => {}
                Err(e) if attempt == 0 && e.to_string().contains("[rejected]") => continue,
                Err(e) => return Err(e),
            }
            // Git two-tree checkout keeps unrelated index/worktree edits and refuses overwrites.
            git::run(&paths.root, &["read-tree", "-m", "-u", &before, &after])?;
            if let Err(e) = git::run(&paths.root, &["update-ref", "HEAD", &after, &before]) {
                let _ = git::run(&paths.root, &["read-tree", "-m", "-u", &after, &before]);
                return Err(e);
            }
            crate::git_auth::remember_state(&paths.root, url, branch, "synced")?;
            return Ok(PushResult {
                head: after.clone(),
                branch: branch.into(),
                merged: after != before,
            });
        }
        anyhow::bail!("remote changed again; retry push")
    })();
    let _ = fs::remove_dir_all(stage);
    result
}
/// Explicit automatic projection uses HEAD and saved profiles, never pending files.
pub fn project_head(
    paths: &AgentHubPaths,
    store: &Store,
) -> Result<Vec<crate::models::AutoSyncOutcome>> {
    if !store
        .auto_sync_profiles()?
        .iter()
        .any(|p| p.enabled && !p.needs_review)
    {
        return Ok(Vec::new());
    }
    let _lock = versions::lock(paths)?;
    let head = git::snapshot(&paths.root)?
        .head
        .context("save a local version before projecting to tools")?;
    let stage = paths
        .root
        .join(format!("runtime/project-{}", Uuid::new_v4()));
    let result = (|| {
        versions::checkout(paths, &stage, &head, false)?;
        let mut committed = AgentHubPaths::new(paths.user_home.clone(), stage.clone());
        committed.backups = paths.backups.clone();
        transaction::run_auto_sync_snapshot(&committed, store)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentHub;
    use tempfile::TempDir;
    fn skill(hub: &AgentHub, id: &str, body: &str) {
        let p = hub.paths.skills.join(id);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), format!("# {id}\n{body}\n")).unwrap();
    }
    fn mcp(hub: &AgentHub, id: &str) {
        let p = hub.paths.mcp.join(id);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("server.json"),format!(r#"{{"schemaVersion":1,"id":"{id}","display_name":"{id}","transport":"stdio","command":"node"}}"#)).unwrap();
    }
    fn device(temp: &TempDir) -> AgentHub {
        let h = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        git::commit(
            &h.paths.root,
            "Initial",
            Some("Test"),
            Some("test@example.com"),
        )
        .unwrap();
        h
    }
    fn selected(id: &str) -> SaveOptions {
        SaveOptions {
            only: Some(vec![parse_key(&format!("skill:{id}")).unwrap()]),
            host_sync: "none".into(),
            ..Default::default()
        }
    }
    #[test]
    fn selective_save_keeps_five_staged_mcps_and_pushes_only_skill() {
        let temp = TempDir::new().unwrap();
        let hub = device(&temp);
        let remote = temp.path().join("remote.git");
        fs::create_dir(&remote).unwrap();
        git::run(&remote, &["init", "--bare"]).unwrap();
        for id in [
            "chrome-devtools",
            "context7",
            "esp-component-registry",
            "espressif-docs",
            "node-repl",
        ] {
            mcp(&hub, id);
        }
        git::run(&hub.paths.root, &["add", "mcp"]).unwrap();
        let staged = git::run(&hub.paths.root, &["diff", "--cached", "--binary"]).unwrap();
        skill(&hub, "agenthub-manager", "selected");
        let plan = preview(
            &hub.paths,
            &hub.store,
            "Skill only",
            None,
            None,
            selected("agenthub-manager"),
        )
        .unwrap();
        assert_eq!(plan.changes.len(), 1);
        assert_eq!(plan.excluded_pending_changes.len(), 5);
        assert!(plan.host_plans.is_empty());
        let result = apply(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert!(result.auto_sync.is_empty());
        assert!(!result.remote_synced);
        assert_eq!(
            git::run(&hub.paths.root, &["diff", "--cached", "--binary"]).unwrap(),
            staged
        );
        let pushed = push_to(&hub.paths, &remote.to_string_lossy(), "main").unwrap();
        assert_eq!(pushed.head, result.commit_hash);
        assert_eq!(
            git::run(&hub.paths.root, &["diff", "--cached", "--binary"]).unwrap(),
            staged
        );
        let files = git::run(&remote, &["ls-tree", "-r", "--name-only", "main"]).unwrap();
        assert!(files.contains("skills/agenthub-manager/SKILL.md"));
        assert!(!files.contains("mcp/"));
        for target in [".codex", ".cursor", ".claude", ".agents"] {
            assert!(!temp.path().join(target).exists());
        }
    }
    #[test]
    fn stale_index_or_excluded_content_blocks_apply() {
        let temp = TempDir::new().unwrap();
        let hub = device(&temp);
        skill(&hub, "one", "pending");
        mcp(&hub, "server");
        let plan = preview(
            &hub.paths,
            &hub.store,
            "Only one",
            None,
            None,
            selected("one"),
        )
        .unwrap();
        fs::write(hub.paths.mcp.join("server/server.json"), "changed").unwrap();
        assert!(apply(&hub.paths, &hub.store, plan.id, true)
            .unwrap_err()
            .to_string()
            .contains("stale"));
        assert_eq!(git::snapshot(&hub.paths.root).unwrap().head, plan.head);
    }
    #[test]
    fn first_selective_save_and_deletion_preserve_other_pending_capabilities() {
        let temp = TempDir::new().unwrap();
        let hub = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        skill(&hub, "one", "selected");
        skill(&hub, "two", "pending");
        let plan = preview(
            &hub.paths,
            &hub.store,
            "Initial one",
            Some("Test"),
            Some("test@example.com"),
            selected("one"),
        )
        .unwrap();
        apply(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert!(
            git::run(&hub.paths.root, &["ls-tree", "-r", "--name-only", "HEAD"])
                .unwrap()
                .contains("agenthub.toml")
        );
        fs::remove_dir_all(hub.paths.skills.join("one")).unwrap();
        let plan = preview(
            &hub.paths,
            &hub.store,
            "Delete one",
            None,
            None,
            selected("one"),
        )
        .unwrap();
        assert_eq!(plan.changes[0].action, "delete");
        apply(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert!(hub.paths.skills.join("two/SKILL.md").exists());
        assert!(
            !git::run(&hub.paths.root, &["ls-tree", "-r", "--name-only", "HEAD"])
                .unwrap()
                .contains("skills/")
        );
    }
}

#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use crate::{models::AutoSyncProfile, AgentHub};
    use tempfile::TempDir;
    fn skill(h: &AgentHub, id: &str, body: &str) {
        let dir = h.paths.skills.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), format!("# {id}\n{body}\n")).unwrap();
    }
    fn hub(home: &Path) -> AgentHub {
        let h = AgentHub::open(AgentHubPaths::for_home(home)).unwrap();
        git::commit(
            &h.paths.root,
            "Init",
            Some("Test"),
            Some("test@example.com"),
        )
        .unwrap();
        h
    }
    #[test]
    fn dirty_mcp_survives_remote_skill_merge_and_overlapping_mcp_blocks_push() {
        let t = TempDir::new().unwrap();
        let a = hub(&t.path().join("a"));
        let b = hub(&t.path().join("b"));
        let remote = t.path().join("remote.git");
        fs::create_dir(&remote).unwrap();
        git::run(&remote, &["init", "--bare"]).unwrap();
        let url = remote.to_string_lossy();
        push_to(&a.paths, &url, "main").unwrap();
        push_to(&b.paths, &url, "main").unwrap();
        skill(&a, "remote-skill", "remote change");
        git::commit(&a.paths.root, "Remote skill", None, None).unwrap();
        push_to(&a.paths, &url, "main").unwrap();
        let dir = b.paths.mcp.join("local");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("server.json"),r#"{"schemaVersion":1,"id":"local","display_name":"Local","transport":"stdio","command":"node"}"#).unwrap();
        git::run(&b.paths.root, &["add", "mcp"]).unwrap();
        let index = git::run(&b.paths.root, &["diff", "--cached", "--binary"]).unwrap();
        assert!(push_to(&b.paths, &url, "main").unwrap().merged);
        assert!(b.paths.skills.join("remote-skill/SKILL.md").exists());
        assert_eq!(
            git::run(&b.paths.root, &["diff", "--cached", "--binary"]).unwrap(),
            index
        );
        let dir = a.paths.mcp.join("local");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("server.json"),r#"{"schemaVersion":1,"id":"local","display_name":"Local","transport":"stdio","command":"remote"}"#).unwrap();
        git::commit(&a.paths.root, "Remote MCP", None, None).unwrap();
        push_to(&a.paths, &url, "main").unwrap();
        let before = git::snapshot(&b.paths.root).unwrap().head;
        assert!(push_to(&b.paths, &url, "main")
            .unwrap_err()
            .to_string()
            .contains("overlap"));
        assert_eq!(git::snapshot(&b.paths.root).unwrap().head, before);
        assert_eq!(
            git::run(&b.paths.root, &["diff", "--cached", "--binary"]).unwrap(),
            index
        );
    }
    #[test]
    fn selected_projection_excludes_pending_mcp_and_blocks_user_host_edits() {
        let t = TempDir::new().unwrap();
        let h = hub(t.path());
        skill(&h, "one", "initial");
        git::commit(&h.paths.root, "One", None, None).unwrap();
        let scope = SyncSelection {
            skills: vec!["one".into()],
            mcp: vec![],
            ..Default::default()
        };
        transaction::sync_once(&h.paths, &h.store, Target::Codex, Some(&scope)).unwrap();
        h.store
            .set_auto_sync_profile(&AutoSyncProfile {
                target: Target::Codex,
                enabled: true,
                needs_review: false,
                selection: SyncSelection {
                    mcp: vec!["pending".into()],
                    ..scope
                },
            })
            .unwrap();
        let dir = h.paths.mcp.join("pending");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("server.json"),r#"{"schemaVersion":1,"id":"pending","display_name":"Pending","transport":"stdio","command":"node"}"#).unwrap();
        skill(&h, "one", "updated");
        let options = SaveOptions {
            only: Some(vec![parse_key("skill:one").unwrap()]),
            host_sync: "enabled".into(),
            ..Default::default()
        };
        let plan = preview(&h.paths, &h.store, "One only", None, None, options.clone()).unwrap();
        assert!(plan
            .host_plans
            .iter()
            .flat_map(|p| &p.steps)
            .all(|s| s.capability_kind == Some(CapabilityKind::Skill)));
        let result = apply(&h.paths, &h.store, plan.id, true).unwrap();
        assert!(result.auto_sync[0].changed);
        assert!(!t.path().join(".codex/config.toml").exists());
        assert!(
            fs::read_to_string(t.path().join(".codex/skills/one/SKILL.md"))
                .unwrap()
                .contains("updated")
        );
        // Automatic runs are HEAD-only even when MCP is included in the profile.
        let outcomes = project_head(&h.paths, &h.store).unwrap();
        assert!(!outcomes[0].changed);
        assert!(!t.path().join(".codex/config.toml").exists());
        fs::write(
            t.path().join(".codex/skills/one/SKILL.md"),
            "# user host edit",
        )
        .unwrap();
        skill(&h, "one", "new canonical");
        let plan = preview(&h.paths, &h.store, "Needs review", None, None, options).unwrap();
        let before = git::snapshot(&h.paths.root).unwrap().head;
        assert!(apply(&h.paths, &h.store, plan.id, true)
            .unwrap_err()
            .to_string()
            .contains("host content changed"));
        assert_eq!(git::snapshot(&h.paths.root).unwrap().head, before);
        assert!(
            fs::read_to_string(t.path().join(".codex/skills/one/SKILL.md"))
                .unwrap()
                .contains("user host edit")
        );
    }
    #[test]
    fn altered_plan_and_unsupported_scope_are_rejected() {
        let t = TempDir::new().unwrap();
        let h = hub(t.path());
        skill(&h, "one", "initial");
        let options = SaveOptions {
            only: Some(vec![parse_key("skill:one").unwrap()]),
            host_sync: "none".into(),
            ..Default::default()
        };
        let mut plan = preview(&h.paths, &h.store, "One", None, None, options).unwrap();
        plan.message = "tampered".into();
        fs::write(
            plan_path(&h.paths, plan.id),
            serde_json::to_vec(&plan).unwrap(),
        )
        .unwrap();
        assert!(apply(&h.paths, &h.store, plan.id, true).is_err());
        assert!(parse_key("skill:../../mcp").is_err());
        assert!(parse_key("skill:One").is_err());
        assert!(preview(
            &h.paths,
            &h.store,
            "No",
            None,
            None,
            SaveOptions {
                only: Some(vec![]),
                ..Default::default()
            }
        )
        .is_err());
    }
}

#[cfg(test)]
mod windows_clean_tests {
    use super::*;
    use crate::AgentHub;
    use tempfile::TempDir;
    #[test]
    fn selective_save_uses_windows_clean_rules_and_keeps_unstaged_mcp() {
        let temp = TempDir::new().unwrap();
        let hub = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        git::commit(
            &hub.paths.root,
            "Init",
            Some("Test"),
            Some("test@example.com"),
        )
        .unwrap();
        git::run(&hub.paths.root, &["config", "core.autocrlf", "true"]).unwrap();
        let p = hub.paths.skills.join("windows");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), "# Windows\r\nCRLF instruction\r\n").unwrap();
        let other = hub.paths.mcp.join("pending");
        fs::create_dir_all(&other).unwrap();
        fs::write(
            other.join("server.json"),
            "pending invalid excluded content",
        )
        .unwrap();
        let options = SaveOptions {
            only: Some(vec![parse_key("skill:windows").unwrap()]),
            host_sync: "none".into(),
            ..Default::default()
        };
        let plan = preview(&hub.paths, &hub.store, "Windows skill", None, None, options).unwrap();
        let result = apply(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert!(result.local_saved);
        let saved = git::run(&hub.paths.root, &["show", "HEAD:skills/windows/SKILL.md"]).unwrap();
        assert!(!saved.contains('\r'));
        assert_eq!(
            fs::read_to_string(p.join("SKILL.md")).unwrap(),
            "# Windows\r\nCRLF instruction\r\n"
        );
        assert_eq!(
            fs::read_to_string(other.join("server.json")).unwrap(),
            "pending invalid excluded content"
        );
        assert!(
            !git::run(&hub.paths.root, &["ls-tree", "-r", "--name-only", "HEAD"])
                .unwrap()
                .contains("mcp/")
        );
    }
}
