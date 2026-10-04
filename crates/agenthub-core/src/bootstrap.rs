//! Read-only first-run restoration. Runtime state and credentials are never downloaded.
use crate::{canonical, git, git_auth, paths::AgentHubPaths, AgentHub};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize)]
pub struct RestoreResult {
    pub imported: usize,
    pub branch: String,
    pub recovery_path: String,
}

#[derive(Serialize, Deserialize)]
struct Journal {
    id: uuid::Uuid,
}

fn lock(paths: &AgentHubPaths) -> Result<fs::File> {
    let name = paths
        .root
        .file_name()
        .context("invalid library root")?
        .to_str()
        .context("invalid library root")?;
    let path = paths.root.with_file_name(format!(".{name}-restore.lock"));
    if path.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&path)?.file_type().is_file(),
            "invalid restoration lock"
        );
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)?;
    crate::paths::set_private_file(&path)?;
    file.try_lock()
        .context("library restoration is already in progress; retry after it finishes")?;
    Ok(file)
}

fn locations(paths: &AgentHubPaths, id: uuid::Uuid) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let name = paths
        .root
        .file_name()
        .context("library root needs a directory name")?
        .to_str()
        .context("invalid library root")?;
    let parent = paths.root.parent().context("library root needs a parent")?;
    Ok((
        parent.join(format!("{name}-restore-{id}")),
        parent.join(format!("{name}-before-restore-{id}")),
        parent.join(format!(".{name}-restore.json")),
    ))
}

/// Complete or roll back an interrupted root switch before opening SQLite.
pub(crate) fn recover_pending(paths: &AgentHubPaths) -> Result<()> {
    let (_, _, journal) = locations(paths, uuid::Uuid::nil())?;
    if !journal.exists() {
        return Ok(());
    }
    let _lock = lock(paths)?;
    if !journal.exists() {
        return Ok(());
    }
    anyhow::ensure!(
        fs::symlink_metadata(&journal)?.file_type().is_file(),
        "invalid restoration journal"
    );
    let entry: Journal = serde_json::from_slice(&fs::read(&journal)?)?;
    let (stage, backup, _) = locations(paths, entry.id)?;
    if !paths.root.exists() && backup.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&backup)?.file_type().is_dir(),
            "invalid restoration backup"
        );
        fs::rename(&backup, &paths.root).context("recover interrupted library restoration")?;
    }
    // The UUID stage is installer-independent and always private. Never follow a link.
    if stage.exists() {
        anyhow::ensure!(
            fs::symlink_metadata(&stage)?.file_type().is_dir(),
            "invalid restoration staging directory"
        );
        fs::remove_dir_all(&stage)?;
    }
    fs::remove_file(journal)?;
    Ok(())
}

fn assert_empty(paths: &AgentHubPaths) -> Result<()> {
    anyhow::ensure!(
        fs::symlink_metadata(&paths.root)?.file_type().is_dir(),
        "library root cannot be a symlink"
    );
    let store = crate::storage::Store::open(&paths.database)?;
    anyhow::ensure!(
        !store.initialized()? && canonical::canonical_dirs_empty(paths)?,
        "existing local library is preserved; restore requires an empty, uninitialized library"
    );
    anyhow::ensure!(
        git::snapshot(&paths.root)?.head.is_none(),
        "existing local versions are preserved; restore requires an empty library"
    );
    for entry in fs::read_dir(&paths.root)? {
        let name = entry?.file_name();
        anyhow::ensure!(
            [
                "skills",
                "plugins",
                "rules",
                "mcp",
                "state",
                "secrets",
                "backups",
                "projections",
                "runtime",
                ".git",
                ".gitignore",
                "agenthub.toml"
            ]
            .iter()
            .any(|known| name == *known),
            "unrecognized local files are preserved; use an empty library to restore"
        );
    }
    anyhow::ensure!(
        fs::read_to_string(paths.root.join("agenthub.toml"))? == "schema_version = 1\n",
        "existing local library configuration is preserved"
    );
    Ok(())
}

/// Caller must close all library handles and serialize operations before activation.
/// A missing branch uses `agenthub` when present, otherwise the remote's default branch.
pub fn restore(
    paths: &AgentHubPaths,
    url: &str,
    branch: &str,
    username: Option<&str>,
    token: Option<&str>,
) -> Result<RestoreResult> {
    git::validate_remote(url)?;
    anyhow::ensure!(
        git::available(),
        "Git is required to restore an AgentHub library; install Git and retry"
    );
    let _lock = lock(paths)?;
    assert_empty(paths)?;
    let id = uuid::Uuid::new_v4();
    let (stage_root, backup, journal) = locations(paths, id)?;
    let stage = AgentHubPaths::new(paths.user_home.clone(), stage_root.clone());
    let journal_temp = journal.with_extension(format!("{id}.tmp"));
    let write_journal = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&journal_temp)?;
        crate::paths::set_private_file(&journal_temp)?;
        serde_json::to_writer(&mut file, &Journal { id })?;
        file.sync_all()?;
        drop(file);
        anyhow::ensure!(
            !journal.exists(),
            "another restoration journal exists; reopen AgentHub to recover it"
        );
        fs::rename(&journal_temp, &journal)?;
        Ok(())
    })();
    if let Err(error) = write_journal {
        let _ = fs::remove_file(&journal_temp);
        return Err(error).context("record library restoration before download");
    }
    let mut switching = false;
    let result = (|| -> Result<RestoreResult> {
        let staged = AgentHub::open(stage.clone())?;
        git::ensure_repo(&stage_root)?;
        match (username, token) {
            (Some(user), Some(secret)) => {
                git_auth::login(&stage_root, url, "agenthub", user, secret)?;
            }
            (None, None) => {}
            _ => anyhow::bail!("username and access token must be supplied together"),
        }
        let refs = git::run(&stage_root, &["ls-remote", "--symref", url])?;
        let heads: Vec<_> = refs
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .filter_map(|(_, reference)| reference.strip_prefix("refs/heads/"))
            .collect();
        let branch = if branch.trim().is_empty() {
            if heads.contains(&"agenthub") {
                "agenthub".to_owned()
            } else {
                refs.lines()
                    .find_map(|line| {
                        line.strip_prefix("ref: refs/heads/")
                            .and_then(|line| line.strip_suffix("\tHEAD"))
                    })
                    .context(
                        "repository has no default branch; select an existing AgentHub branch",
                    )?
                    .to_owned()
            }
        } else {
            branch.trim().to_owned()
        };
        anyhow::ensure!(!branch.starts_with('-') && heads.contains(&branch.as_str()), "requested branch does not exist; check the branch or use a repository containing an AgentHub library");
        git::run(
            &stage_root,
            &["check-ref-format", &format!("refs/heads/{branch}")],
        )?;
        git::run(
            &stage_root,
            &["fetch", "--no-tags", url, &format!("refs/heads/{branch}")],
        )?;
        let tree = git::run(&stage_root, &["ls-tree", "-rz", "FETCH_HEAD"])?;
        let mut manifest = false;
        for entry in tree.split('\0').filter(|s| !s.is_empty()) {
            let (meta, path) = entry.split_once('\t').context("invalid remote tree")?;
            manifest |= path == "agenthub.toml";
            let allowed = path == "agenthub.toml"
                || path == ".gitignore"
                || ["skills/", "plugins/", "rules/", "mcp/"]
                    .iter()
                    .any(|prefix| path.starts_with(prefix));
            anyhow::ensure!(
                allowed && (meta.starts_with("100644 blob ") || meta.starts_with("100755 blob ")),
                "repository must contain only an AgentHub library, with no symlinks or submodules"
            );
        }
        anyhow::ensure!(manifest, "repository is not an AgentHub library: versioned agenthub.toml is missing; migrate a Skills-only repository on its original device first");
        fs::remove_file(stage_root.join("agenthub.toml"))?;
        fs::remove_file(stage_root.join(".gitignore"))?;
        git::run(&stage_root, &["checkout", "-B", &branch, "FETCH_HEAD"])?;
        // Missing empty domains are expected: Git cannot store empty directories.
        stage.ensure_runtime()?;
        let imported = canonical::validate(&stage)
            .context("restored library validation failed")?
            .len();
        git::check_upload_history(&stage_root).context("restored history is not portable")?;
        git::run(&stage_root, &["config", "remote.agenthub.url", url])?;
        git::run(&stage_root, &["config", "agenthub.remoteBranch", &branch])?;
        git_auth::remember_state(&stage_root, url, &branch, "read_verified")?;
        staged.store.set_initialized(true)?;
        drop(staged);
        assert_empty(paths)?;
        switching = true;
        if let Err(error) = fs::rename(&paths.root, &backup) {
            fs::remove_file(&journal)?;
            return Err(error).context("preserve original library before restore");
        }
        if let Err(error) = fs::rename(&stage_root, &paths.root) {
            fs::rename(&backup, &paths.root).context(
                "restore activation failed; recover original library from the private backup",
            )?;
            fs::remove_file(&journal)?;
            return Err(error).context("activate restored library; original data preserved");
        }
        // Activation succeeded; a leftover journal is safe and handled on next open.
        let _ = fs::remove_file(&journal);
        Ok(RestoreResult {
            imported,
            branch,
            recovery_path: backup.display().to_string(),
        })
    })();
    if !switching || !journal.exists() {
        if stage_root.exists() {
            fs::remove_dir_all(&stage_root).with_context(|| {
                format!(
                    "private restoration staging cleanup failed: {}",
                    stage_root.display()
                )
            })?;
        }
        if journal.exists() {
            fs::remove_file(&journal)?;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_switch_recovers_original_data_and_drops_private_stage() {
        let home = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(home.path());
        let hub = AgentHub::open(paths.clone()).unwrap();
        fs::write(paths.root.join("keep.txt"), "untouched").unwrap();
        drop(hub);
        let id = uuid::Uuid::new_v4();
        let (stage, backup, journal) = locations(&paths, id).unwrap();
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("private-credential"), "fixture-only").unwrap();
        fs::write(&journal, serde_json::to_vec(&Journal { id }).unwrap()).unwrap();
        fs::rename(&paths.root, &backup).unwrap();
        AgentHub::open(paths.clone()).unwrap();
        assert_eq!(
            fs::read_to_string(paths.root.join("keep.txt")).unwrap(),
            "untouched"
        );
        assert!(!stage.exists() && !journal.exists());
    }

    #[test]
    fn live_switch_is_not_recovered_by_a_second_process() {
        let home = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(home.path());
        drop(AgentHub::open(paths.clone()).unwrap());
        let id = uuid::Uuid::new_v4();
        let (stage, _, journal) = locations(&paths, id).unwrap();
        fs::create_dir(&stage).unwrap();
        let _lock = lock(&paths).unwrap();
        fs::write(&journal, serde_json::to_vec(&Journal { id }).unwrap()).unwrap();
        assert!(AgentHub::open(paths.clone()).is_err());
        assert!(stage.exists() && journal.exists());
    }

    #[test]
    fn existing_library_and_unknown_local_files_block_before_network_access() {
        let home = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(home.path());
        let hub = AgentHub::open(paths.clone()).unwrap();
        fs::write(paths.root.join("personal.txt"), "preserve").unwrap();
        drop(hub);
        assert!(
            restore(&paths, "https://127.0.0.1:1/repo.git", "", None, None)
                .unwrap_err()
                .to_string()
                .contains("unrecognized local files")
        );
        assert_eq!(
            fs::read_to_string(paths.root.join("personal.txt")).unwrap(),
            "preserve"
        );
        fs::remove_file(paths.root.join("personal.txt")).unwrap();
        let hub = AgentHub::open(paths.clone()).unwrap();
        hub.store.set_initialized(true).unwrap();
        drop(hub);
        assert!(
            restore(&paths, "https://127.0.0.1:1/repo.git", "", None, None)
                .unwrap_err()
                .to_string()
                .contains("existing local library")
        );
    }

    #[test]
    fn missing_database_reopens_valid_uncommitted_library_without_reimport() {
        let home = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(home.path());
        let hub = AgentHub::open(paths.clone()).unwrap();
        fs::create_dir(paths.skills.join("keep")).unwrap();
        fs::write(paths.skills.join("keep/SKILL.md"), "# Keep\n").unwrap();
        drop(hub);
        fs::remove_file(&paths.database).unwrap();
        let reopened = AgentHub::open(paths.clone()).unwrap();
        assert!(reopened.store.initialized().unwrap());
        assert_eq!(canonical::inventory(&paths).unwrap().len(), 1);
        assert!(reopened.store.enabled_targets().unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(paths.skills.join("keep/SKILL.md")).unwrap(),
            "# Keep\n"
        );
    }
}
