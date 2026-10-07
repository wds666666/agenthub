//! Selected reverse imports share the backend preflight across Desktop and CLI.
use crate::{
    canonical,
    models::{ScanImportResult, ScanItem, Target},
    paths::AgentHubPaths,
    scanner,
    storage::Store,
    versions,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub id: Uuid,
    pub items: Vec<ScanItem>,
    pub canonical_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSkillPreview {
    pub id: Uuid,
    pub item: ScanItem,
    pub files: Vec<crate::models::CapabilityFile>,
    pub excluded: Vec<std::path::PathBuf>,
    pub duplicate_of: Option<String>,
    pub canonical_digest: String,
}

/// Reads only a directory explicitly selected by the user, not a discovery root.
pub fn preview_local_skill(
    paths: &AgentHubPaths,
    store: &Store,
    source: &std::path::Path,
) -> Result<LocalSkillPreview> {
    let _lock = versions::lock(paths)?;
    anyhow::ensure!(
        store.initialized()?,
        "initialize the library before importing a Skill"
    );
    anyhow::ensure!(source.is_absolute(), "choose an absolute Skill directory");
    anyhow::ensure!(
        std::fs::symlink_metadata(source)?.file_type().is_dir(),
        "choose an ordinary Skill directory"
    );
    let source = std::fs::canonicalize(source)?;
    anyhow::ensure!(
        !source.starts_with(std::fs::canonicalize(&paths.root)?),
        "choose a Skill outside the AgentHub library"
    );
    let content = crate::skill_content::inspect(&source)?;
    let mut item = scanner::item(
        crate::models::CapabilityKind::Skill,
        "local",
        source.clone(),
        None,
    )?;
    crate::comparison::annotate(paths, std::slice::from_mut(&mut item))?;
    anyhow::ensure!(
        item.importable && item.digest == content.digest,
        "Skill source changed while creating the preview; choose it again"
    );
    let duplicate_of = canonical::inventory(paths)?
        .into_iter()
        .find(|cap| cap.kind == item.kind && cap.comparison_digest == item.comparison_digest)
        .map(|cap| cap.id);
    item.importable = item.importable && duplicate_of.is_none();
    let plan = LocalSkillPreview {
        id: Uuid::new_v4(),
        item,
        files: content.files(&source)?,
        excluded: content.excluded,
        duplicate_of,
        canonical_digest: canonical::canonical_digest(paths)?,
    };
    store.set_meta(
        &format!("local_skill_plan_{}", plan.id),
        &serde_json::to_string(&plan)?,
    )?;
    Ok(plan)
}

pub fn apply_local_skill(
    paths: &AgentHubPaths,
    store: &Store,
    id: Uuid,
    confirm: bool,
) -> Result<ScanImportResult> {
    anyhow::ensure!(confirm, "review the Skill and confirm import");
    let _lock = versions::lock(paths)?;
    let key = format!("local_skill_plan_{id}");
    let plan: LocalSkillPreview =
        serde_json::from_str(&store.meta(&key)?.context("Skill preview unavailable")?)?;
    anyhow::ensure!(
        store.initialized()?
            && plan.id == id
            && canonical::canonical_digest(paths)? == plan.canonical_digest,
        "library changed; choose the Skill again to refresh the preview"
    );
    anyhow::ensure!(
        plan.item.importable && plan.duplicate_of.is_none(),
        "identical Skill already exists in the library"
    );
    let mut fresh = scanner::item(
        crate::models::CapabilityKind::Skill,
        "local",
        plan.item.path.clone(),
        None,
    )?;
    crate::comparison::annotate(paths, std::slice::from_mut(&mut fresh))?;
    anyhow::ensure!(
        fresh.importable
            && fresh.digest == plan.item.digest
            && fresh.comparison_digest == plan.item.comparison_digest,
        "Skill source changed; choose the Skill again to refresh the preview"
    );
    fresh.selected = true;
    let imported = canonical::import_scan_items_atomic(paths, &[fresh])?;
    store.set_meta(&key, "null")?;
    Ok(ScanImportResult {
        skipped_duplicates: usize::from(imported.is_empty()),
        imported,
        auto_sync: Vec::new(),
    })
}
pub fn preview(paths: &AgentHubPaths, store: &Store, ids: &[String]) -> Result<ImportPreview> {
    let _lock = versions::lock(paths)?;
    anyhow::ensure!(
        store.initialized()?,
        "initialize the library before reverse import"
    );
    let requested: BTreeSet<_> = ids.iter().collect();
    anyhow::ensure!(
        !requested.is_empty() && requested.len() == ids.len(),
        "choose unique discovery IDs"
    );
    let found = scanner::scan_global(paths, &Target::ALL)?;
    let items: Vec<_> = found
        .into_iter()
        .filter(|i| requested.contains(&i.id))
        .collect();
    anyhow::ensure!(
        items.len() == requested.len(),
        "scan result changed; discover again"
    );
    anyhow::ensure!(
        items.iter().all(|i| i.importable),
        "selection contains unavailable or duplicate resources; discover again"
    );
    let plan = ImportPreview {
        id: Uuid::new_v4(),
        items,
        canonical_digest: canonical::canonical_digest(paths)?,
    };
    store.set_meta(
        &format!("import_plan_{}", plan.id),
        &serde_json::to_string(&plan)?,
    )?;
    Ok(plan)
}
pub fn apply(
    paths: &AgentHubPaths,
    store: &Store,
    id: Uuid,
    confirm: bool,
) -> Result<ScanImportResult> {
    anyhow::ensure!(confirm, "review selected resources and pass --confirm");
    let _lock = versions::lock(paths)?;
    let key = format!("import_plan_{id}");
    let plan: ImportPreview =
        serde_json::from_str(&store.meta(&key)?.context("import Plan unavailable")?)?;
    anyhow::ensure!(
        plan.id == id && canonical::canonical_digest(paths)? == plan.canonical_digest,
        "stale import Plan; discover again"
    );
    let mut found = scanner::scan_global(paths, &Target::ALL)?;
    for original in &plan.items {
        let fresh = found
            .iter_mut()
            .find(|i| i.id == original.id)
            .context("resource disappeared; discover again")?;
        anyhow::ensure!(
            fresh.importable
                && fresh.digest == original.digest
                && fresh.comparison_digest == original.comparison_digest
                && fresh.path == original.path,
            "resource changed; discover again"
        );
        fresh.selected = true;
    }
    let imported = canonical::import_scan_items_atomic(paths, &found)?;
    store.set_meta(&key, "null")?;
    Ok(ScanImportResult {
        skipped_duplicates: plan.items.len().saturating_sub(imported.len()),
        imported,
        auto_sync: Vec::new(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentHub;
    use std::fs;
    use tempfile::TempDir;
    #[test]
    fn local_skill_import_keeps_payload_excludes_runtime_and_blocks_duplicates() {
        let temp = TempDir::new().unwrap();
        let hub = AgentHub::open(AgentHubPaths::for_home(temp.path().join("home"))).unwrap();
        hub.store.set_initialized(true).unwrap();
        let source = temp.path().join("download/my-skill");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(source.join("scripts")).unwrap();
        fs::create_dir_all(source.join(".venv")).unwrap();
        fs::write(source.join("SKILL.md"), "# My Skill").unwrap();
        fs::write(source.join("nested/SKILL.md"), "# Part of parent").unwrap();
        fs::write(source.join("scripts/example.sh"), "echo sample").unwrap();
        fs::write(source.join(".venv/cache"), "excluded").unwrap();
        let plan = preview_local_skill(&hub.paths, &hub.store, &source).unwrap();
        assert_eq!(plan.files.len(), 3);
        assert_eq!(plan.excluded, vec![std::path::PathBuf::from(".venv")]);
        assert!(canonical::inventory(&hub.paths).unwrap().is_empty());
        assert!(apply_local_skill(&hub.paths, &hub.store, plan.id, false).is_err());
        let result = apply_local_skill(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert_eq!(result.imported, vec!["my-skill"]);
        assert!(result.auto_sync.is_empty());
        assert!(hub.paths.skills.join("my-skill/nested/SKILL.md").is_file());
        assert!(hub
            .paths
            .skills
            .join("my-skill/scripts/example.sh")
            .is_file());
        assert!(!hub.paths.skills.join("my-skill/.venv").exists());
        assert!(source.join(".venv/cache").is_file());
        assert_eq!(canonical::inventory(&hub.paths).unwrap().len(), 1);
        assert!(crate::git::snapshot(&hub.paths.root)
            .unwrap()
            .head
            .is_none());
        for target in [".agents", ".codex", ".cursor", ".claude"] {
            assert!(!hub.paths.user_home.join(target).exists());
        }
        let duplicate = preview_local_skill(&hub.paths, &hub.store, &source).unwrap();
        assert_eq!(duplicate.duplicate_of.as_deref(), Some("my-skill"));
        assert!(!duplicate.item.importable);
        assert!(apply_local_skill(&hub.paths, &hub.store, duplicate.id, true).is_err());
        assert!(apply_local_skill(&hub.paths, &hub.store, plan.id, true).is_err());
        fs::write(source.join("SKILL.md"), "# Revised").unwrap();
        let revised = preview_local_skill(&hub.paths, &hub.store, &source).unwrap();
        assert!(revised.item.importable);
        let result = apply_local_skill(&hub.paths, &hub.store, revised.id, true).unwrap();
        assert_ne!(result.imported[0], "my-skill");
        assert_eq!(
            fs::read_to_string(hub.paths.skills.join("my-skill/SKILL.md")).unwrap(),
            "# My Skill"
        );
    }

    #[test]
    fn local_skill_preview_rejects_invalid_source_and_stale_source_or_library() {
        let temp = TempDir::new().unwrap();
        let hub = AgentHub::open(AgentHubPaths::for_home(temp.path().join("home"))).unwrap();
        hub.store.set_initialized(true).unwrap();
        let source = temp.path().join("download/example");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested/SKILL.md"), "# Nested only").unwrap();
        assert!(preview_local_skill(&hub.paths, &hub.store, &source).is_err());
        fs::write(source.join("SKILL.md"), "# Example").unwrap();
        let plan = preview_local_skill(&hub.paths, &hub.store, &source).unwrap();
        fs::write(source.join("SKILL.md"), "# Changed").unwrap();
        assert!(apply_local_skill(&hub.paths, &hub.store, plan.id, true).is_err());
        assert!(canonical::inventory(&hub.paths).unwrap().is_empty());
        let plan = preview_local_skill(&hub.paths, &hub.store, &source).unwrap();
        fs::create_dir_all(hub.paths.skills.join("other")).unwrap();
        fs::write(hub.paths.skills.join("other/SKILL.md"), "# Other").unwrap();
        assert!(apply_local_skill(&hub.paths, &hub.store, plan.id, true).is_err());
        assert!(!hub.paths.skills.join("example").exists());
        assert!(
            preview_local_skill(&hub.paths, &hub.store, &hub.paths.skills.join("other")).is_err()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source.join("SKILL.md"), source.join("linked.md")).unwrap();
            assert!(preview_local_skill(&hub.paths, &hub.store, &source).is_err());
        }
    }
    #[test]
    fn selected_import_is_reviewed_stale_safe_and_never_projects_or_commits() {
        let temp = TempDir::new().unwrap();
        let hub = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        hub.store.set_initialized(true).unwrap();
        for id in ["one", "two"] {
            let root = temp.path().join(format!(".agents/skills/{id}"));
            fs::create_dir_all(&root).unwrap();
            fs::write(root.join("SKILL.md"), format!("# {id}")).unwrap();
        }
        let found = scanner::scan_global(&hub.paths, &[Target::Agents]).unwrap();
        let ids = vec![found
            .iter()
            .find(|i| i.path.ends_with("one"))
            .unwrap()
            .id
            .clone()];
        let plan = preview(&hub.paths, &hub.store, &ids).unwrap();
        assert!(apply(&hub.paths, &hub.store, plan.id, false).is_err());
        fs::write(temp.path().join(".agents/skills/one/SKILL.md"), "# changed").unwrap();
        assert!(apply(&hub.paths, &hub.store, plan.id, true).is_err());
        assert!(!hub.paths.skills.join("one").exists());
        let found = scanner::scan_global(&hub.paths, &[Target::Agents]).unwrap();
        let ids = vec![found
            .iter()
            .find(|i| i.path.ends_with("one"))
            .unwrap()
            .id
            .clone()];
        let plan = preview(&hub.paths, &hub.store, &ids).unwrap();
        let result = apply(&hub.paths, &hub.store, plan.id, true).unwrap();
        assert_eq!(result.imported.len(), 1);
        assert!(result.auto_sync.is_empty());
        assert!(!hub.paths.skills.join("two").exists());
        assert!(!temp.path().join(".codex").exists());
        assert!(crate::git::snapshot(&hub.paths.root)
            .unwrap()
            .head
            .is_none());
        assert!(apply(&hub.paths, &hub.store, plan.id, true).is_err());
    }
}
