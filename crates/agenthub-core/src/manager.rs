//! Embedded content maintenance is separate from application initialization.
use crate::{paths::AgentHubPaths, storage::Store};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs;
include!(concat!(env!("OUT_DIR"), "/manager_bundle.rs"));
pub const RESOURCE_VERSION: u64 = 1;
pub fn digest() -> String {
    let mut h = Sha256::new();
    for (path, bytes) in FILES {
        h.update(path.as_bytes());
        h.update(bytes);
    }
    hex::encode(h.finalize())
}
/// Called by Desktop lifecycle/discovery, never by AgentHub::open or installers.
pub fn ensure(paths: &AgentHubPaths, store: &Store) -> Result<bool> {
    if !store.initialized()? {
        return Ok(false);
    }
    let _lock = crate::versions::lock(paths)?;
    let destination = paths.skills.join("agenthub-manager");
    let bundled = digest();
    let current = if destination.exists() {
        Some(crate::skill_content::inspect(&destination)?.digest)
    } else {
        None
    };
    if current.as_ref() == Some(&bundled) {
        store.set_meta("manager_digest", &bundled)?;
        store.set_meta("manager_version", &RESOURCE_VERSION.to_string())?;
        store.set_meta("manager_error", "")?;
        return Ok(false);
    }
    if let Some(current) = &current {
        let known: Vec<String> = serde_json::from_str(include_str!("manager_known_digests.json"))?;
        let owned = store.meta("manager_digest")?.as_ref() == Some(current);
        let newer = store
            .meta("manager_version")?
            .and_then(|s| s.parse::<u64>().ok())
            .is_some_and(|v| v > RESOURCE_VERSION);
        anyhow::ensure!(
            !newer && (owned || known.contains(current)),
            "agenthub-manager has unknown or edited content; review before updating"
        );
    }
    let staging = paths
        .root
        .join("runtime")
        .join(format!("manager-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging)?;
    let result = (|| -> Result<()> {
        for (relative, bytes) in FILES {
            let file = staging.join(relative);
            fs::create_dir_all(file.parent().unwrap())?;
            fs::write(file, bytes)?;
        }
        anyhow::ensure!(
            crate::skill_content::inspect(&staging)?.digest == bundled,
            "bundle validation failed"
        );
        let fresh = if destination.exists() {
            Some(crate::skill_content::inspect(&destination)?.digest)
        } else {
            None
        };
        anyhow::ensure!(
            fresh == current,
            "manager Skill changed during update; retry after reviewing edits"
        );
        let backup = paths
            .backups
            .join(format!("manager-{}", uuid::Uuid::new_v4()));
        if current.is_some() {
            fs::create_dir_all(&backup)?;
            fs::rename(&destination, backup.join("agenthub-manager"))?;
        }
        if let Err(error) = fs::rename(&staging, &destination) {
            if current.is_some() {
                fs::rename(backup.join("agenthub-manager"), &destination)?;
            }
            return Err(error.into());
        }
        store.set_meta("manager_digest", &bundled)?;
        store.set_meta("manager_version", &RESOURCE_VERSION.to_string())?;
        store.set_meta("manager_error", "")?;
        crate::transaction::with_trigger("manager_update", || {
            crate::transaction::run_auto_sync_on_change(paths, store)
        })?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(staging);
    result?;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_is_idempotent_and_preserves_edits_and_uninitialized_libraries() {
        let temp = tempfile::tempdir().unwrap();
        let h = crate::AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        assert!(!ensure(&h.paths, &h.store).unwrap());
        h.store.set_initialized(true).unwrap();
        assert!(ensure(&h.paths, &h.store).unwrap());
        assert!(!ensure(&h.paths, &h.store).unwrap());
        assert!(!temp.path().join(".agents/skills").exists());
        fs::write(
            h.paths.skills.join("agenthub-manager/SKILL.md"),
            "# User edit",
        )
        .unwrap();
        assert!(ensure(&h.paths, &h.store).is_err());
        assert_eq!(
            fs::read_to_string(h.paths.skills.join("agenthub-manager/SKILL.md")).unwrap(),
            "# User edit"
        );
    }
    #[test]
    fn recognized_upgrade_is_backed_up() {
        let temp = tempfile::tempdir().unwrap();
        let h = crate::AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        h.store.set_initialized(true).unwrap();
        let dir = h.paths.skills.join("agenthub-manager");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "# Old owned bundle").unwrap();
        h.store
            .set_meta(
                "manager_digest",
                &crate::skill_content::inspect(&dir).unwrap().digest,
            )
            .unwrap();
        assert!(ensure(&h.paths, &h.store).unwrap());
        assert_eq!(fs::read_dir(&h.paths.backups).unwrap().count(), 1);
    }
}
