//! Reviewed library-only recovery; never resets history, pushes, or writes tools.
use crate::{
    canonical, git,
    models::{CapabilityKind, GitSnapshot},
    paths::{self, AgentHubPaths},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};
use uuid::Uuid;
const NAMES: [&str; 5] = ["skills", "mcp", "rules", "plugins", "agenthub.toml"];
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionAction {
    Discard,
    Remote,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityChange {
    pub kind: Option<CapabilityKind>,
    pub id: String,
    pub action: String,
    pub files: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPreview {
    pub id: Uuid,
    pub action: VersionAction,
    pub source_commit: String,
    pub changes: Vec<CapabilityChange>,
    pub git: GitSnapshot,
    pub before_digest: String,
    pub candidate_digest: String,
    pub index_digest: String,
    pub remote_url: Option<String>,
    pub remote_branch: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct VersionResult {
    pub backup_path: String,
    pub source_commit: String,
    pub pending_changes: bool,
}
fn runtime(paths: &AgentHubPaths) -> Result<()> {
    let root = paths.root.join("runtime");
    if root.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&root)?.file_type().is_dir(),
            "unsafe version runtime"
        );
    }
    fs::create_dir_all(root)?;
    Ok(())
}
fn lock(paths: &AgentHubPaths) -> Result<fs::File> {
    runtime(paths)?;
    let path = paths.root.join("runtime/versions.lock");
    if path.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&path)?.file_type().is_file(),
            "unsafe version lock"
        );
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    paths::set_private_file(&path)?;
    // A concurrently spawned Git process may briefly inherit an open lock
    // before exec closes it. Allow that transient handoff, but reject a real
    // overlapping recovery without writing anything.
    let started = std::time::Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock)
                if started.elapsed() < std::time::Duration::from_millis(100) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(error) => return Err(error).context("version recovery already in progress"),
        }
    }
    Ok(file)
}
fn files(root: &Path) -> Result<BTreeMap<String, String>> {
    files_filtered(root, None)
}
fn files_filtered(
    root: &Path,
    visible: Option<&std::collections::BTreeSet<String>>,
) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for name in NAMES {
        let path = root.join(name);
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        anyhow::ensure!(
            !meta.file_type().is_symlink(),
            "library domain is a symlink"
        );
        let relative = |entry: &walkdir::DirEntry| {
            entry
                .path()
                .strip_prefix(root)
                .map(|p| {
                    p.components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default()
        };
        for entry in walkdir::WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                visible.is_none_or(|names| {
                    let name = relative(entry);
                    if entry.file_type().is_dir() {
                        let prefix = format!("{name}/");
                        names
                            .range(prefix.clone()..)
                            .next()
                            .is_some_and(|next| next.starts_with(&prefix))
                    } else {
                        names.contains(&name)
                    }
                })
            })
        {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(root)?
                .components()
                .map(|p| p.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let digest = if entry.file_type().is_symlink() {
                canonical::sha256(fs::read_link(entry.path())?.as_os_str().as_encoded_bytes())
            } else {
                anyhow::ensure!(entry.file_type().is_file(), "unsupported library file");
                canonical::file_digest(entry.path())?
            };
            result.insert(rel, digest);
        }
    }
    Ok(result)
}
fn digest(root: &Path) -> Result<String> {
    Ok(canonical::sha256(&serde_json::to_vec(&files(root)?)?))
}
fn index_digest(paths: &AgentHubPaths) -> Result<String> {
    let index = paths.root.join(".git/index");
    if index.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&index)?.file_type().is_file(),
            "unsafe Git index"
        );
    }
    canonical::tree_digest(&index)
}
fn group(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    before_root: &Path,
    after_root: &Path,
) -> Vec<CapabilityChange> {
    let mut grouped: BTreeMap<(Option<CapabilityKind>, String), Vec<String>> = BTreeMap::new();
    for name in before.keys().chain(after.keys()) {
        if before.get(name) == after.get(name) {
            continue;
        }
        let mut parts = name.split('/');
        let kind = match parts.next() {
            Some("skills") => Some(CapabilityKind::Skill),
            Some("mcp") => Some(CapabilityKind::Mcp),
            Some("rules") => Some(CapabilityKind::Rule),
            Some("plugins") => Some(CapabilityKind::Plugin),
            _ => None,
        };
        let id = parts.next().unwrap_or("agenthub.toml").to_owned();
        grouped.entry((kind, id)).or_default().push(name.clone());
    }
    grouped
        .into_iter()
        .map(|((kind, id), mut names)| {
            names.sort();
            names.dedup();
            let path = kind.map_or_else(
                || id.clone(),
                |k| {
                    format!(
                        "{}s/{id}",
                        if k == CapabilityKind::Mcp {
                            "mcp"
                        } else {
                            k.as_str()
                        }
                    )
                },
            );
            let path = if kind == Some(CapabilityKind::Mcp) {
                format!("mcp/{id}")
            } else {
                path
            };
            let action = if !before_root.join(&path).exists() {
                "create"
            } else if !after_root.join(&path).exists() {
                "delete"
            } else {
                "update"
            };
            CapabilityChange {
                kind,
                id,
                action: action.into(),
                files: names,
            }
        })
        .collect()
}
fn checkout(paths: &AgentHubPaths, dest: &Path, sha: &str, remote: bool) -> Result<()> {
    let source = paths.root.canonicalize()?;
    git::run(
        &paths.root,
        &[
            "clone",
            "--shared",
            "--no-checkout",
            "--",
            &git::git_path_argument(&source)?,
            &git::git_path_argument(dest)?,
        ],
    )?;
    // Recovery snapshots must retain stored bytes even when the user's Git
    // defaults convert text to CRLF on Windows.
    git::run(dest, &["config", "core.autocrlf", "false"])?;
    git::run(dest, &["checkout", "--detach", sha])?;
    anyhow::ensure!(
        fs::symlink_metadata(dest.join("agenthub.toml"))
            .is_ok_and(|metadata| metadata.file_type().is_file()),
        "saved snapshot is not an AgentHub library"
    );
    let candidate = AgentHubPaths::new(paths.user_home.clone(), dest.to_path_buf());
    candidate.ensure_runtime()?;
    canonical::validate(&candidate)?;
    if remote {
        git::check_upload_history(dest)?;
    }
    Ok(())
}
pub fn changes(paths: &AgentHubPaths) -> Result<Vec<CapabilityChange>> {
    let _guard = lock(paths)?;
    let snap = git::snapshot(&paths.root)?;
    let stage = paths
        .root
        .join(format!("runtime/changes-{}", Uuid::new_v4()));
    let result = (|| {
        if let Some(head) = &snap.head {
            checkout(paths, &stage, head, false)?;
        } else {
            fs::create_dir_all(&stage)?;
        }
        let mut before = files(&stage)?;
        let listed = git::run(
            &paths.root,
            &["ls-files", "--others", "--exclude-standard", "-z"],
        )?;
        let mut visible: std::collections::BTreeSet<String> =
            listed.split('\0').map(str::to_owned).collect();
        if snap.head.is_some() {
            let changed = git::run(
                &paths.root,
                &[
                    "diff",
                    "HEAD",
                    "--name-only",
                    "--no-ext-diff",
                    "--no-textconv",
                    "-z",
                ],
            )?;
            visible.extend(changed.split('\0').map(str::to_owned));
        } else {
            visible.extend(before.keys().cloned());
            let tracked = git::run(&paths.root, &["ls-files", "--cached", "-z"])?;
            visible.extend(tracked.split('\0').map(str::to_owned));
        }
        before.retain(|name, _| visible.contains(name));
        let after = files_filtered(&paths.root, Some(&visible))?;
        Ok(group(&before, &after, &stage, &paths.root))
    })();
    let _ = fs::remove_dir_all(stage);
    result
}
fn replacement_changes(
    paths: &AgentHubPaths,
    stage: &Path,
    source_commit: &str,
) -> Result<Vec<CapabilityChange>> {
    // Git compares the working tree through its clean filters, including
    // autocrlf and .gitattributes. Raw snapshot hashes still guard the transaction,
    // but must not turn Windows checkout line endings into capability edits.
    let changed = git::run(
        &paths.root,
        &[
            "diff",
            source_commit,
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            "--",
        ],
    )?;
    // Include ignored, untracked canonical files as well: replacement removes
    // those too. The canonical allowlist excludes machine/runtime root paths.
    let untracked = git::run(&paths.root, &["ls-files", "--others", "-z"])?;
    let visible = changed
        .split('\0')
        .chain(untracked.split('\0'))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(group(
        &files_filtered(&paths.root, Some(&visible))?,
        &files_filtered(stage, Some(&visible))?,
        &paths.root,
        stage,
    ))
}

pub fn preview(paths: &AgentHubPaths, action: VersionAction) -> Result<VersionPreview> {
    let _guard = lock(paths)?;
    anyhow::ensure!(
        !paths.root.join(".git/MERGE_HEAD").exists(),
        "resolve the current merge before library recovery"
    );
    let snap = git::snapshot(&paths.root)?;
    let before = digest(&paths.root)?;
    let index = index_digest(paths)?;
    let (sha, url, branch) = match action {
        VersionAction::Discard => (
            snap.head
                .clone()
                .context("save a local version before discard")?,
            None,
            None,
        ),
        VersionAction::Remote => {
            let settings = git::remote_settings(&paths.root)?;
            let url = settings.url.context("connect a repository first")?;
            git::validate_remote(&url)?;
            let reference = format!("refs/heads/{}", settings.branch);
            anyhow::ensure!(
                !git::run(&paths.root, &["ls-remote", "--heads", &url, &reference])?.is_empty(),
                "remote branch is empty or missing"
            );
            git::run(&paths.root, &["fetch", "--no-tags", &url, &reference])?;
            (
                git::run(&paths.root, &["rev-parse", "FETCH_HEAD"])?,
                Some(url),
                Some(settings.branch),
            )
        }
    };
    let id = Uuid::new_v4();
    let stage = paths.root.join(format!("runtime/version-{id}"));
    let result = (|| {
        checkout(paths, &stage, &sha, action == VersionAction::Remote)?;
        anyhow::ensure!(
            digest(&paths.root)? == before
                && index_digest(paths)? == index
                && git::snapshot(&paths.root)? == snap,
            "local changes during preview; retry"
        );
        let plan = VersionPreview {
            id,
            action,
            source_commit: sha.clone(),
            changes: replacement_changes(paths, &stage, &sha)?,
            git: snap,
            before_digest: before,
            candidate_digest: digest(&stage)?,
            index_digest: index,
            remote_url: url,
            remote_branch: branch,
        };
        let file = paths.root.join(format!("runtime/version-{id}.json"));
        fs::write(&file, serde_json::to_vec(&plan)?)?;
        paths::set_private_file(&file)?;
        Ok(plan)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}
fn copy(source: &Path, dest: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(source)?;
    if meta.file_type().is_symlink() {
        return crate::backup::copy_link(source, dest);
    }
    if meta.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        anyhow::ensure!(meta.is_file(), "unsupported backup file");
        fs::copy(source, dest)?;
    }
    Ok(())
}
fn remove(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path)?,
        Ok(_) => fs::remove_file(path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct Journal {
    id: Uuid,
}
fn restore_backup(paths: &AgentHubPaths, id: Uuid) -> Result<()> {
    anyhow::ensure!(
        fs::symlink_metadata(&paths.backups)?.file_type().is_dir(),
        "unsafe recovery backup root"
    );
    let backup = paths.backups.join(format!("library-version-{id}"));
    anyhow::ensure!(
        fs::symlink_metadata(&backup)?.file_type().is_dir(),
        "unsafe or missing version backup"
    );
    for name in NAMES {
        let target = paths.root.join(name);
        remove(&target)?;
        let source = backup.join(name);
        if source.exists() {
            copy(&source, &target)?;
        }
    }
    let index = paths.root.join(".git/index");
    remove(&index)?;
    if backup.join("index").exists() {
        fs::copy(backup.join("index"), index)?;
    }
    Ok(())
}
pub fn recover_pending(paths: &AgentHubPaths) -> Result<()> {
    let journal = paths.root.join("runtime/version-recovery.json");
    if !journal.exists() {
        return Ok(());
    }
    let _guard = lock(paths)?;
    let entry: Journal = serde_json::from_slice(&fs::read(&journal)?)?;
    restore_backup(paths, entry.id)?;
    fs::remove_file(journal)?;
    Ok(())
}
pub fn apply(paths: &AgentHubPaths, id: Uuid, confirmation: &str) -> Result<VersionResult> {
    let _guard = lock(paths)?;
    let plan: VersionPreview = serde_json::from_slice(&fs::read(
        paths.root.join(format!("runtime/version-{id}.json")),
    )?)?;
    anyhow::ensure!(
        plan.id == id
            && confirmation
                == match plan.action {
                    VersionAction::Discard => "DISCARD",
                    VersionAction::Remote => "REMOTE",
                },
        "exact recovery confirmation required"
    );
    let stage = paths.root.join(format!("runtime/version-{id}"));
    anyhow::ensure!(
        digest(&paths.root)? == plan.before_digest
            && index_digest(paths)? == plan.index_digest
            && git::snapshot(&paths.root)? == plan.git
            && digest(&stage)? == plan.candidate_digest,
        "library changed; preview again"
    );
    if plan.action == VersionAction::Remote {
        let settings = git::remote_settings(&paths.root)?;
        anyhow::ensure!(
            settings.url == plan.remote_url && Some(settings.branch.clone()) == plan.remote_branch,
            "remote connection changed; preview again"
        );
        let latest = git::run(
            &paths.root,
            &[
                "ls-remote",
                "--heads",
                settings.url.as_deref().unwrap(),
                &format!("refs/heads/{}", settings.branch),
            ],
        )?;
        anyhow::ensure!(
            latest.split_whitespace().next() == Some(plan.source_commit.as_str()),
            "remote version changed; preview again"
        );
    }
    let candidate = AgentHubPaths::new(paths.user_home.clone(), stage.clone());
    canonical::validate(&candidate)?;
    anyhow::ensure!(
        fs::symlink_metadata(&paths.backups)?.file_type().is_dir(),
        "unsafe recovery backup root"
    );
    let backup = paths.backups.join(format!("library-version-{id}"));
    fs::create_dir(&backup)?;
    #[cfg(unix)]
    fs::set_permissions(&backup, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    for name in NAMES {
        let source = paths.root.join(name);
        if source.exists() {
            copy(&source, &backup.join(name))?;
        }
    }
    if paths.root.join(".git/index").exists() {
        fs::copy(paths.root.join(".git/index"), backup.join("index"))?;
    }
    anyhow::ensure!(
        digest(&backup)? == plan.before_digest,
        "recovery backup verification failed"
    );
    anyhow::ensure!(
        digest(&paths.root)? == plan.before_digest && index_digest(paths)? == plan.index_digest,
        "local changes during backup; preview again"
    );
    let journal = paths.root.join("runtime/version-recovery.json");
    fs::write(&journal, serde_json::to_vec(&Journal { id })?)?;
    paths::set_private_file(&journal)?;
    let result = (|| -> Result<()> {
        for name in NAMES {
            let target = paths.root.join(name);
            remove(&target)?;
            copy(&stage.join(name), &target)?;
        }
        if plan.git.head.is_some() {
            git::run(
                &paths.root,
                &[
                    "reset",
                    "HEAD",
                    "--",
                    "skills",
                    "mcp",
                    "rules",
                    "plugins",
                    "agenthub.toml",
                ],
            )?;
        }
        canonical::validate(paths)?;
        anyhow::ensure!(
            digest(&paths.root)? == plan.candidate_digest,
            "recovery content verification failed"
        );
        Ok(())
    })();
    if let Err(error) = result {
        restore_backup(paths, id).context("recovery rollback failed; keep private backup")?;
        fs::remove_file(journal)?;
        return Err(error);
    }
    fs::remove_file(journal)?;
    let _ = fs::remove_dir_all(stage);
    let _ = fs::remove_file(paths.root.join(format!("runtime/version-{id}.json")));
    Ok(VersionResult {
        backup_path: backup.display().to_string(),
        source_commit: plan.source_commit,
        pending_changes: git::snapshot(&paths.root)?.dirty,
    })
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn remote_snapshot_comparison_only_lists_real_skill_and_mcp_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let hub = crate::AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        for id in ["changed", "unchanged"] {
            fs::create_dir_all(hub.paths.skills.join(id)).unwrap();
            fs::write(hub.paths.skills.join(id).join("SKILL.md"), "# Original\n").unwrap();
        }
        git::commit(
            &hub.paths.root,
            "base",
            Some("Tester"),
            Some("test@example.com"),
        )
        .unwrap();
        fs::write(hub.paths.skills.join("changed/SKILL.md"), "# Remote edit\n").unwrap();
        git::commit(
            &hub.paths.root,
            "remote",
            Some("Tester"),
            Some("test@example.com"),
        )
        .unwrap();
        let remote = git::snapshot(&hub.paths.root).unwrap().head.unwrap();
        // Keep the local history at the previous commit while retaining the remote object.
        git::run(&hub.paths.root, &["reset", "--mixed", "HEAD~1"]).unwrap();
        git::run(&hub.paths.root, &["config", "core.autocrlf", "true"]).unwrap();
        for id in ["changed", "unchanged"] {
            fs::write(hub.paths.skills.join(id).join("SKILL.md"), "# Original\r\n").unwrap();
        }
        fs::create_dir_all(hub.paths.mcp.join("local-server")).unwrap();
        fs::write(hub.paths.mcp.join("local-server/server.json"), r#"{"schemaVersion":1,"id":"local-server","display_name":"Server","transport":"stdio","command":"node","args":[],"env":{},"headers":{}}"#).unwrap();
        git::run(&hub.paths.root, &["add", "mcp"]).unwrap();
        let stage = hub.paths.root.join("runtime/remote-test");
        checkout(&hub.paths, &stage, &remote, true).unwrap();
        let changes = replacement_changes(&hub.paths, &stage, &remote).unwrap();
        assert_eq!(changes.len(), 2);
        assert!(changes
            .iter()
            .any(|c| c.id == "changed" && c.action == "update"));
        assert!(changes
            .iter()
            .any(|c| c.id == "local-server" && c.action == "delete"));
        assert!(!changes.iter().any(|c| c.id == "unchanged"));
    }
}
