use crate::{
    adapters::{actual_domain_digest, projection_with_selection, DomainProjection},
    canonical::canonical_digest,
    models::{
        AutoSyncOutcome, Plan, SyncRunResult, SyncSelection, Target, Transaction, TransactionMode,
    },
    paths::{set_private_file, AgentHubPaths},
    storage::Store,
};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
struct BackupEntry {
    target_path: PathBuf,
    backup_name: String,
    existed: bool,
    was_file: bool,
    mode: Option<u32>,
    link_target: Option<PathBuf>,
}
#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    transaction_id: String,
    entries: Vec<BackupEntry>,
}

pub fn apply(paths: &AgentHubPaths, store: &Store, plan: &Plan) -> Result<Transaction> {
    apply_with_mode(paths, store, plan, TransactionMode::Reviewed)
}

pub fn sync_once(
    paths: &AgentHubPaths,
    store: &Store,
    target: Target,
    selection: Option<&SyncSelection>,
) -> Result<SyncRunResult> {
    let plan = crate::planner::create_with_selection(paths, target, selection)?;
    if plan.steps.is_empty() {
        return Ok(SyncRunResult {
            changed: false,
            plan,
            transaction: None,
        });
    }
    store.save_plan(&plan)?;
    let transaction = apply_with_mode(paths, store, &plan, TransactionMode::AutoSync)?;
    Ok(SyncRunResult {
        changed: true,
        plan,
        transaction: Some(transaction),
    })
}

fn auto_sync_result(paths: &AgentHubPaths, store: &Store) -> crate::models::RemoteSyncResult {
    match run_auto_sync(paths, store) {
        Ok(auto_sync) => crate::models::RemoteSyncResult {
            auto_sync,
            auto_sync_error: None,
        },
        Err(error) => crate::models::RemoteSyncResult {
            auto_sync: Vec::new(),
            auto_sync_error: Some(crate::secrets::redact(&format!("{error:#}"))),
        },
    }
}

pub fn save_version(
    paths: &AgentHubPaths,
    store: &Store,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
) -> Result<crate::git::CommitResult> {
    let mut result = crate::git::commit_and_sync(&paths.root, message, name, email)?;
    let sync = auto_sync_result(paths, store);
    result.auto_sync = sync.auto_sync;
    result.auto_sync_error = sync.auto_sync_error;
    Ok(result)
}

pub fn sync_remote(
    paths: &AgentHubPaths,
    store: &Store,
) -> Result<crate::models::RemoteSyncResult> {
    let before = crate::canonical::canonical_digest(paths)?;
    crate::git::sync_remote(&paths.root)?;
    if crate::canonical::canonical_digest(paths)? == before {
        return Ok(Default::default());
    }
    Ok(auto_sync_result(paths, store))
}

pub fn save_rule(
    paths: &AgentHubPaths,
    store: &Store,
    rule: &crate::models::RuleDocument,
    create: bool,
) -> Result<crate::models::CapabilityMutationResult> {
    let capability = crate::canonical::save_rule(paths, rule, create)?;
    let auto_sync = if create {
        Vec::new()
    } else {
        run_auto_sync(paths, store)?
    };
    Ok(crate::models::CapabilityMutationResult {
        capability,
        auto_sync,
    })
}

pub fn run_auto_sync(paths: &AgentHubPaths, store: &Store) -> Result<Vec<AutoSyncOutcome>> {
    let profiles = store.auto_sync_profiles()?;
    if !profiles.iter().any(|p| p.enabled && !p.needs_review) {
        return Ok(Vec::new());
    }
    let ids: std::collections::BTreeSet<_> = crate::canonical::inventory(paths)?
        .into_iter()
        .map(|c| (c.kind, c.id))
        .collect();

    Ok(profiles
        .into_iter()
        .filter(|profile| profile.enabled && !profile.needs_review)
        .map(|profile| {
            let mut selection = profile.selection;
            selection.skills_managed = selection.manages_skills();
            selection.mcp_managed = selection.manages_mcp();
            selection.plugins_managed = selection.manages_plugins();
            selection.rules_managed = selection.manages_rules();
            selection
                .skills
                .retain(|id| ids.contains(&(crate::models::CapabilityKind::Skill, id.clone())));
            selection
                .mcp
                .retain(|id| ids.contains(&(crate::models::CapabilityKind::Mcp, id.clone())));
            selection
                .plugins
                .retain(|id| ids.contains(&(crate::models::CapabilityKind::Plugin, id.clone())));
            selection
                .rule_ids
                .retain(|id| ids.contains(&(crate::models::CapabilityKind::Rule, id.clone())));
            match sync_once(paths, store, profile.target, Some(&selection)) {
                Ok(result) => AutoSyncOutcome {
                    target: profile.target,
                    changed: result.changed,
                    transaction_id: result.transaction.map(|transaction| transaction.id),
                    error: None,
                },
                Err(error) => AutoSyncOutcome {
                    target: profile.target,
                    changed: false,
                    transaction_id: None,
                    error: Some(crate::secrets::redact(&format!("{error:#}"))),
                },
            }
        })
        .collect())
}

pub fn apply_with_mode(
    paths: &AgentHubPaths,
    store: &Store,
    plan: &Plan,
    mode: TransactionMode,
) -> Result<Transaction> {
    anyhow::ensure!(
        canonical_digest(paths)? == plan.canonical_digest,
        "canonical state changed; generate a new plan"
    );
    let current =
        crate::planner::create_with_selection(paths, plan.target, plan.selection.as_ref())?;
    anyhow::ensure!(
        current.steps == plan.steps,
        "target drift changed; generate a new plan"
    );
    let id = Uuid::new_v4().to_string();
    let backup_path = paths.backups.join(&id);
    fs::create_dir_all(&backup_path)?;
    let domains = projection_with_selection(paths, plan.target, plan.selection.as_ref())?;
    let manifest = backup(&id, &backup_path, &domains)?;
    let mut tx = Transaction {
        id: id.clone(),
        plan_id: plan.id.clone(),
        target: plan.target,
        status: "applying".into(),
        mode,
        backup_path: backup_path.clone(),
        git: plan.git.clone(),
        canonical_digest: plan.canonical_digest.clone(),
        verification: None,
        created_at: Utc::now().to_rfc3339(),
    };
    store.save_transaction(&tx)?;
    let result = (|| -> Result<()> {
        for d in &domains {
            write_domain(d)?;
        }
        for d in &domains {
            anyhow::ensure!(
                actual_domain_digest(d)? == d.digest(),
                "verification failed for {}",
                d.name
            );
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            tx.status = "applied".into();
            tx.verification = Some("expected state matched".into());
            if let Err(error) = store.save_applied_transaction(&tx) {
                let rollback_result = restore_manifest(&backup_path, &manifest);
                tx.status = if rollback_result.is_ok() {
                    "rolled_back".into()
                } else {
                    "rollback_failed".into()
                };
                tx.verification = Some(format!("state persistence failed: {error:#}"));
                let _ = store.save_transaction(&tx);
                rollback_result?;
                return Err(error);
            }
        }
        Err(error) => {
            let rollback_result = restore_manifest(&backup_path, &manifest);
            tx.status = if rollback_result.is_ok() {
                "rolled_back".into()
            } else {
                "rollback_failed".into()
            };
            tx.verification = Some(format!("apply failed: {error:#}"));
            store.save_transaction(&tx)?;
            rollback_result?;
            return Err(error);
        }
    }
    Ok(tx)
}

fn backup(id: &str, root: &Path, domains: &[DomainProjection]) -> Result<BackupManifest> {
    let mut entries = Vec::new();
    for (index, d) in domains.iter().enumerate() {
        let name = format!("{index}-{}", d.name);
        let dest = root.join(&name);
        let existed = d.target_path.exists();
        let was_file = d.target_path.is_file();
        if existed {
            copy_path(&d.target_path, &dest)?;
        }
        let mode = mode(&d.target_path);
        let link_target = if d
            .target_path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            fs::read_link(&d.target_path).ok()
        } else {
            None
        };
        entries.push(BackupEntry {
            target_path: d.target_path.clone(),
            backup_name: name,
            existed,
            was_file,
            mode,
            link_target,
        });
    }
    let manifest = BackupManifest {
        transaction_id: id.into(),
        entries,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    fs::write(root.join("manifest.json"), bytes)?;
    set_private_file(&root.join("manifest.json"))?;
    Ok(manifest)
}
fn write_domain(d: &DomainProjection) -> Result<()> {
    if d.target_path.is_file()
        || (!d.target_path.exists() && d.files.len() == 1 && d.files.contains_key(Path::new("")))
    {
        let bytes = d
            .files
            .get(Path::new(""))
            .context("file projection payload")?;
        if let Some(parent) = d.target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = d.target_path.with_extension("agenthub-tmp");
        fs::write(&temp, bytes)?;
        if d.sensitive {
            set_private_file(&temp)?;
        }
        replace_file(&temp, &d.target_path)?;
        return Ok(());
    }
    let parent = d.target_path.parent().context("domain parent")?;
    fs::create_dir_all(parent)?;
    let stage = parent.join(format!(".agenthub-stage-{}", Uuid::new_v4()));
    fs::create_dir_all(&stage)?;
    for (rel, bytes) in &d.files {
        let path = stage.join(rel);
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(path, bytes)?;
    }
    for name in d
        .preserve_names
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
    {
        let source = d.target_path.join(name);
        match fs::symlink_metadata(&source) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                crate::backup::copy_link(&source, &stage.join(name))?
            }
            Ok(_) => copy_path(&source, &stage.join(name))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let old = parent.join(format!(".agenthub-old-{}", Uuid::new_v4()));
    if d.target_path.exists() {
        fs::rename(&d.target_path, &old)?;
    }
    if let Err(e) = fs::rename(&stage, &d.target_path) {
        if old.exists() {
            let _ = fs::rename(&old, &d.target_path);
        }
        return Err(e.into());
    }
    if old.exists() {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    if !target.exists() {
        fs::rename(source, target)?;
        return Ok(());
    }

    let parent = target.parent().context("file projection parent")?;
    let old = parent.join(format!(".agenthub-old-{}", Uuid::new_v4()));
    fs::rename(target, &old)?;
    if let Err(error) = fs::rename(source, target) {
        let _ = fs::rename(&old, target);
        return Err(error.into());
    }
    if let Err(error) = fs::remove_file(&old) {
        let _ = fs::remove_file(target);
        let _ = fs::rename(&old, target);
        return Err(error.into());
    }
    Ok(())
}
fn restore_manifest(root: &Path, m: &BackupManifest) -> Result<()> {
    for e in m.entries.iter().rev() {
        if e.target_path.exists() {
            if e.target_path.is_dir() {
                fs::remove_dir_all(&e.target_path)?
            } else {
                fs::remove_file(&e.target_path)?
            }
        }
        if e.existed {
            copy_path(&root.join(&e.backup_name), &e.target_path)?;
            set_mode(&e.target_path, e.mode)?;
        }
    }
    Ok(())
}
pub fn rollback(paths: &AgentHubPaths, store: &Store, transaction_id: &str) -> Result<Transaction> {
    let source = store
        .transaction(transaction_id)?
        .context("transaction not found")?;
    anyhow::ensure!(
        matches!(source.status.as_str(), "applied" | "rollback_applied"),
        "transaction status does not support manual rollback"
    );
    let source_root = paths.backups.join(transaction_id);
    let source_manifest: BackupManifest = serde_json::from_slice(
        &fs::read(source_root.join("manifest.json")).context("rollback backup is unavailable")?,
    )?;
    anyhow::ensure!(
        source_manifest.transaction_id == transaction_id,
        "rollback backup does not match transaction"
    );

    let id = Uuid::new_v4().to_string();
    let backup_path = paths.backups.join(&id);
    fs::create_dir_all(&backup_path)?;
    let current_manifest = backup_manifest_targets(&id, &backup_path, &source_manifest)?;
    let mut tx = Transaction {
        id,
        plan_id: source.plan_id.clone(),
        target: source.target,
        status: "rolling_back".into(),
        mode: TransactionMode::Rollback,
        backup_path: backup_path.clone(),
        git: crate::git::snapshot(&paths.root)?,
        canonical_digest: canonical_digest(paths).unwrap_or_else(|_| "unavailable".into()),
        verification: Some(format!("restoring pre-apply state from {transaction_id}")),
        created_at: Utc::now().to_rfc3339(),
    };
    store.save_transaction(&tx)?;

    let result = restore_manifest(&source_root, &source_manifest)
        .and_then(|_| verify_manifest(&source_root, &source_manifest));
    match result {
        Ok(()) => {
            tx.status = "rollback_applied".into();
            tx.verification = Some(format!(
                "restored and verified pre-apply state from {transaction_id}"
            ));
            store.save_transaction(&tx)?;
            Ok(tx)
        }
        Err(error) => {
            let recovery = restore_manifest(&backup_path, &current_manifest);
            tx.status = if recovery.is_ok() {
                "rollback_failed".into()
            } else {
                "rollback_recovery_failed".into()
            };
            tx.verification = Some(format!("manual rollback failed: {error:#}"));
            store.save_transaction(&tx)?;
            recovery.context("manual rollback failed and current state recovery failed")?;
            Err(error)
        }
    }
}

fn backup_manifest_targets(
    id: &str,
    root: &Path,
    source: &BackupManifest,
) -> Result<BackupManifest> {
    let domains: Vec<_> = source
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| DomainProjection {
            name: match index {
                0 => "rollback-domain-0",
                1 => "rollback-domain-1",
                2 => "rollback-domain-2",
                _ => "rollback-domain",
            },
            kind: crate::models::CapabilityKind::Skill,
            target_path: entry.target_path.clone(),
            files: Default::default(),
            preserve_names: Vec::new(),
            sensitive: true,
        })
        .collect();
    backup(id, root, &domains)
}

fn verify_manifest(root: &Path, manifest: &BackupManifest) -> Result<()> {
    for entry in &manifest.entries {
        anyhow::ensure!(
            entry.target_path.exists() == entry.existed,
            "restored path existence mismatch: {}",
            entry.target_path.display()
        );
        if !entry.existed {
            continue;
        }
        anyhow::ensure!(
            entry.target_path.is_file() == entry.was_file,
            "restored path type mismatch: {}",
            entry.target_path.display()
        );
        anyhow::ensure!(
            paths_equal(&root.join(&entry.backup_name), &entry.target_path)?,
            "restored path content mismatch: {}",
            entry.target_path.display()
        );
    }
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> Result<bool> {
    let left_type = fs::symlink_metadata(left)?.file_type();
    let right_type = fs::symlink_metadata(right)?.file_type();
    if left_type.is_symlink() || right_type.is_symlink() {
        return Ok(left_type.is_symlink()
            && right_type.is_symlink()
            && fs::read_link(left)? == fs::read_link(right)?);
    }
    if left.is_file() || right.is_file() {
        return Ok(left.is_file() && right.is_file() && fs::read(left)? == fs::read(right)?);
    }
    #[derive(PartialEq)]
    enum Node {
        Directory,
        File(Vec<u8>),
        Link(PathBuf),
    }
    let collect = |root: &Path| -> Result<Vec<(PathBuf, Node)>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry?;
            let node = if entry.file_type().is_symlink() {
                Node::Link(fs::read_link(entry.path())?)
            } else if entry.file_type().is_file() {
                Node::File(fs::read(entry.path())?)
            } else if entry.file_type().is_dir() {
                Node::Directory
            } else {
                anyhow::bail!("unsupported backup file type: {}", entry.path().display());
            };
            files.push((entry.path().strip_prefix(root)?.to_path_buf(), node));
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(files)
    };
    Ok(collect(left)? == collect(right)?)
}
fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    anyhow::ensure!(
        !fs::symlink_metadata(src)?.file_type().is_symlink(),
        "backup root must not be a symlink: {}",
        src.display()
    );
    if src.is_file() {
        if let Some(p) = dst.parent() {
            fs::create_dir_all(p)?;
        }
        fs::copy(src, dst)?;
        set_private_file(dst)?;
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for e in WalkDir::new(src).follow_links(false) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if e.file_type().is_symlink() {
            crate::backup::copy_link(e.path(), &target)?;
        } else if e.file_type().is_dir() {
            fs::create_dir_all(target)?
        } else if e.file_type().is_file() {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(e.path(), &target)?;
            set_private_file(&target)?;
        } else {
            anyhow::bail!("unsupported backup file type: {}", e.path().display());
        }
    }
    Ok(())
}
#[cfg(unix)]
fn mode(p: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    p.metadata().ok().map(|m| m.permissions().mode())
}
#[cfg(not(unix))]
fn mode(_: &Path) -> Option<u32> {
    None
}
#[cfg(unix)]
fn set_mode(p: &Path, m: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(m) = m {
        fs::set_permissions(p, fs::Permissions::from_mode(m))?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn set_mode(_: &Path, _: Option<u32>) -> Result<()> {
    Ok(())
}
