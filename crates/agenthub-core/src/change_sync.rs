//! Observe library edits only; startup and tool-side changes never deliver content.
use crate::{models::AutoSyncOutcome, paths::AgentHubPaths, storage::Store};
use anyhow::Result;
use std::{collections::BTreeMap, path::PathBuf, time::SystemTime};

type Files = BTreeMap<PathBuf, (u64, SystemTime)>;
#[derive(Default)]
pub struct LibraryChangeMonitor {
    scope: String,
    stable: Option<Files>,
    pending: Option<Files>,
}
impl LibraryChangeMonitor {
    pub fn has_pending_observation(&self) -> bool {
        self.pending.is_some()
    }
    /// Called periodically by Desktop. Two matching observations debounce saves.
    /// A new process/scope records a baseline without projecting anything.
    pub fn check(&mut self, paths: &AgentHubPaths, store: &Store) -> Result<Vec<AutoSyncOutcome>> {
        let profiles: Vec<_> = store
            .auto_sync_profiles()?
            .into_iter()
            .filter(|p| p.enabled && !p.needs_review && p.selection.sync_on_change)
            .collect();
        if profiles.is_empty() || !store.initialized()? {
            *self = Self::default();
            return Ok(vec![]);
        }
        let _lock = crate::versions::lock(paths)?;
        let scope = serde_json::to_string(&profiles)?;
        let mut files = Files::new();
        // Stat portable library files, never read the entire payload on idle ticks.
        // No tool directories, SQLite, Git, credentials or backups are observed.
        let mut roots = std::collections::BTreeSet::new();
        for profile in &profiles {
            for (kind, root, managed, ids) in [
                (
                    crate::models::CapabilityKind::Skill,
                    &paths.skills,
                    profile.selection.manages_skills(),
                    &profile.selection.skills,
                ),
                (
                    crate::models::CapabilityKind::Mcp,
                    &paths.mcp,
                    profile.selection.manages_mcp(),
                    &profile.selection.mcp,
                ),
                (
                    crate::models::CapabilityKind::Rule,
                    &paths.rules,
                    profile.selection.manages_rules(),
                    &profile.selection.rule_ids,
                ),
                (
                    crate::models::CapabilityKind::Plugin,
                    &paths.plugins,
                    profile.selection.manages_plugins(),
                    &profile.selection.plugins,
                ),
            ] {
                if !managed {
                    continue;
                }
                if profile.selection.includes_all(kind) {
                    roots.insert((kind, root.clone()));
                } else {
                    for id in ids {
                        roots.insert((kind, root.join(id)));
                    }
                }
            }
        }
        for (kind, root) in roots {
            if !root.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(&root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    kind != crate::models::CapabilityKind::Skill
                        || !crate::skill_content::is_runtime(e)
                })
            {
                let entry = entry?;
                if !entry.file_type().is_file() {
                    continue;
                }
                let meta = entry.metadata()?;
                files.insert(entry.path().to_path_buf(), (meta.len(), meta.modified()?));
            }
        }
        if self.scope != scope || self.stable.is_none() {
            self.scope = scope;
            self.stable = Some(files);
            self.pending = None;
            return Ok(vec![]);
        }
        if self.stable.as_ref() == Some(&files) {
            self.pending = None;
            return Ok(vec![]);
        }
        if self.pending.as_ref() != Some(&files) {
            self.pending = Some(files);
            return Ok(vec![]);
        }
        // Library deletion alone keeps the explicit cleanup contract.
        let changed = files
            .iter()
            .any(|(path, stamp)| self.stable.as_ref().unwrap().get(path) != Some(stamp));
        self.stable = Some(files);
        self.pending = None;
        if changed {
            crate::canonical::validate(paths)?;
            crate::transaction::with_trigger("library_edit", || {
                crate::transaction::run_auto_sync_on_change(paths, store)
            })
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{AutoSyncProfile, CapabilityKind, SyncSelection, Target},
        AgentHub,
    };
    use std::fs;
    #[test]
    fn monitor_debounces_library_edits_and_ignores_startup_tool_changes_and_deletions() {
        let temp = tempfile::TempDir::new().unwrap();
        let h = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        h.store.set_initialized(true).unwrap();
        let scope = SyncSelection {
            all_kinds: vec![CapabilityKind::Skill],
            sync_on_change: true,
            ..Default::default()
        };
        h.store
            .set_auto_sync_profile(&AutoSyncProfile {
                target: Target::Agents,
                enabled: true,
                needs_review: false,
                selection: scope,
            })
            .unwrap();
        let mut monitor = LibraryChangeMonitor::default();
        fs::create_dir_all(h.paths.skills.join("existing")).unwrap();
        fs::write(h.paths.skills.join("existing/SKILL.md"), "# Existing").unwrap();
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
        assert!(!temp.path().join(".agents/skills").exists());
        fs::write(
            h.paths.skills.join("existing/SKILL.md"),
            "# Edited canonical content",
        )
        .unwrap();
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
        assert!(monitor.check(&h.paths, &h.store).unwrap()[0].changed);
        fs::remove_dir_all(temp.path().join(".agents/skills")).unwrap();
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
        assert!(!temp.path().join(".agents/skills").exists());
        fs::remove_dir_all(h.paths.skills.join("existing")).unwrap();
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
        assert!(monitor.check(&h.paths, &h.store).unwrap().is_empty());
    }
}
