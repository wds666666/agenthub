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

pub fn run_auto_sync(paths: &AgentHubPaths, store: &Store) -> Result<Vec<AutoSyncOutcome>> {
    let profiles = store.auto_sync_profiles()?;
    let strict_authoritative = store.policy_settings()?.strict_authoritative;
    Ok(profiles
        .into_iter()
        .filter(|profile| profile.enabled)
        .map(|profile| {
            let mut selection = profile.selection;
            if strict_authoritative {
                selection.authoritative = true;
            }
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
    for name in &d.preserve_names {
        let source = d.target_path.join(name);
        if source.exists() {
            copy_path(&source, &stage.join(name))?;
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
    if left.is_file() || right.is_file() {
        return Ok(left.is_file() && right.is_file() && fs::read(left)? == fs::read(right)?);
    }
    let collect = |root: &Path| -> Result<Vec<(PathBuf, Vec<u8>)>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_symlink() {
                return Ok(Vec::new());
            }
            if entry.file_type().is_file() {
                files.push((
                    entry.path().strip_prefix(root)?.to_path_buf(),
                    fs::read(entry.path())?,
                ));
            }
        }
        Ok(files)
    };
    Ok(collect(left)? == collect(right)?)
}
fn copy_path(src: &Path, dst: &Path) -> Result<()> {
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
            anyhow::bail!(
                "refusing to back up escaping or unresolved symlink: {}",
                e.path().display()
            )
        } else if e.file_type().is_dir() {
            fs::create_dir_all(target)?
        } else if e.file_type().is_file() {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(e.path(), &target)?;
            set_private_file(&target)?;
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
