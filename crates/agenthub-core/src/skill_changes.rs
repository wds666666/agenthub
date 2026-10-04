//! Read-only, bounded-frequency checks for edits to known Skills in tool roots.
use crate::{
    canonical,
    models::{CapabilityKind, ScanItem},
    paths::AgentHubPaths,
    scanner, skill_content,
};
use anyhow::Result;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

pub const CHECK_INTERVAL: Duration = Duration::from_secs(120);
const CONTENT_INTERVAL: Duration = Duration::from_secs(1800);

#[derive(Clone, Debug, Serialize)]
pub struct SkillChange {
    pub canonical_id: String,
    pub item: ScanItem,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct SkillChangeReport {
    pub changes: Vec<SkillChange>,
    pub errors: Vec<String>,
}
struct CachedSkill {
    fingerprint: String,
    checked: Instant,
    item: ScanItem,
}
#[derive(Default)]
pub struct SkillChangeCache {
    roots: Option<(PathBuf, PathBuf)>,
    checked: Option<Instant>,
    report: SkillChangeReport,
    skills: BTreeMap<PathBuf, CachedSkill>,
}

impl SkillChangeCache {
    pub const fn new() -> Self {
        Self {
            roots: None,
            checked: None,
            report: SkillChangeReport {
                changes: Vec::new(),
                errors: Vec::new(),
            },
            skills: BTreeMap::new(),
        }
    }
    pub fn check(&mut self, paths: &AgentHubPaths, refresh: bool) -> Result<SkillChangeReport> {
        let roots = (paths.root.clone(), paths.user_home.clone());
        if self.roots.as_ref() != Some(&roots) {
            *self = Self::default();
            self.roots = Some(roots);
        }
        let now = Instant::now();
        if !refresh
            && self
                .checked
                .is_some_and(|checked| now.duration_since(checked) < CHECK_INTERVAL)
        {
            return Ok(self.report.clone());
        }
        let mut report = SkillChangeReport::default();
        let mut active = BTreeSet::new();
        let mut library = BTreeMap::new();
        for entry in fs::read_dir(&paths.skills)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if !canonical::valid_id(&id) || !entry.file_type()?.is_dir() {
                continue;
            }
            match self.read(entry.path(), "library", now, &mut active) {
                Ok(item) if item.importable => {
                    library.insert(id, item.digest);
                }
                _ => report.errors.push(entry.path().display().to_string()),
            }
        }
        let known: BTreeSet<_> = library.values().cloned().collect();
        for (source, relative) in [
            ("agents", ".agents/skills"),
            ("cursor", ".cursor/skills"),
            ("codex", ".codex/skills"),
            ("claude", ".claude/skills"),
        ] {
            let root = paths.user_home.join(relative);
            match fs::symlink_metadata(&root) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Ok(meta) if meta.file_type().is_dir() => {}
                _ => {
                    report.errors.push(root.display().to_string());
                    continue;
                }
            }
            for (id, digest) in &library {
                let path = root.join(id);
                match fs::symlink_metadata(&path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Ok(meta) if meta.file_type().is_dir() => {}
                    _ => {
                        report.errors.push(path.display().to_string());
                        continue;
                    }
                }
                match self.read(path.clone(), source, now, &mut active) {
                    Ok(item) if item.importable => {
                        if item.digest != *digest && !known.contains(&item.digest) {
                            report.changes.push(SkillChange {
                                canonical_id: id.clone(),
                                item,
                            });
                        }
                    }
                    _ => report.errors.push(path.display().to_string()),
                }
            }
        }
        self.skills.retain(|path, _| active.contains(path));
        self.checked = Some(now);
        self.report = report.clone();
        Ok(report)
    }

    fn read(
        &mut self,
        path: PathBuf,
        source: &str,
        now: Instant,
        active: &mut BTreeSet<PathBuf>,
    ) -> Result<ScanItem> {
        active.insert(path.clone());
        let fingerprint = skill_content::fingerprint(&path)?;
        if let Some(cached) = self.skills.get(&path) {
            if cached.fingerprint == fingerprint
                && now.duration_since(cached.checked) < CONTENT_INTERVAL
            {
                return Ok(cached.item.clone());
            }
        }
        let item = scanner::item(CapabilityKind::Skill, source, path.clone(), None)?;
        // Do not cache a digest read while an editor was still changing the tree.
        anyhow::ensure!(
            skill_content::fingerprint(&path)? == fingerprint,
            "skill changed during check"
        );
        self.skills.insert(
            path,
            CachedSkill {
                fingerprint,
                checked: now,
                item: item.clone(),
            },
        );
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn skill(path: &std::path::Path, body: &str) {
        fs::create_dir_all(path).unwrap();
        fs::write(path.join("SKILL.md"), body).unwrap();
    }
    #[test]
    fn detects_all_sources_and_support_files_with_cooldown_and_deduplication() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(temp.path());
        paths.ensure_runtime().unwrap();
        skill(&paths.skills.join("example"), "original");
        for root in [".agents", ".cursor", ".codex", ".claude"] {
            skill(&temp.path().join(root).join("skills/example"), "original");
        }
        let mut cache = SkillChangeCache::default();
        assert!(cache.check(&paths, false).unwrap().changes.is_empty());
        let host = temp.path().join(".agents/skills/example");
        fs::write(host.join("SKILL.md"), "updated").unwrap();
        assert!(cache.check(&paths, false).unwrap().changes.is_empty());
        assert_eq!(cache.check(&paths, true).unwrap().changes.len(), 1);
        skill(&paths.skills.join("example-2"), "updated");
        assert!(cache.check(&paths, true).unwrap().changes.is_empty());
        fs::write(host.join("helper.py"), "updated script").unwrap();
        assert_eq!(cache.check(&paths, true).unwrap().changes.len(), 1);
        for root in [".cursor", ".codex", ".claude"] {
            fs::write(
                temp.path().join(root).join("skills/example/SKILL.md"),
                "different",
            )
            .unwrap();
        }
        assert_eq!(cache.check(&paths, true).unwrap().changes.len(), 4);
        fs::remove_file(host.join("helper.py")).unwrap();
        skill(&host, "original");
        fs::create_dir(host.join(".venv")).unwrap();
        fs::write(host.join(".venv/local"), "ignored").unwrap();
        let result = cache.check(&paths, true).unwrap();
        assert_eq!(result.changes.len(), 3);
        assert!(result.errors.is_empty());
        skill(&temp.path().join(".agents/skills/host-only"), "unmanaged");
        assert_eq!(cache.check(&paths, true).unwrap().changes.len(), 3);
    }
    #[test]
    fn reuses_unchanged_digests_and_invalidates_on_library_change() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(temp.path());
        paths.ensure_runtime().unwrap();
        skill(&paths.skills.join("example"), "one");
        skill(&temp.path().join(".agents/skills/example"), "two");
        let mut cache = SkillChangeCache::default();
        assert_eq!(cache.check(&paths, true).unwrap().changes.len(), 1);
        let checked = cache
            .skills
            .get(&paths.skills.join("example"))
            .unwrap()
            .checked;
        cache.check(&paths, true).unwrap();
        assert_eq!(
            cache
                .skills
                .get(&paths.skills.join("example"))
                .unwrap()
                .checked,
            checked
        );
        skill(&paths.skills.join("example"), "two");
        assert!(cache.check(&paths, true).unwrap().changes.is_empty());
    }
    #[test]
    #[ignore = "local performance fixture"]
    fn thousand_skills_cache_cost() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(temp.path());
        paths.ensure_runtime().unwrap();
        let body = "Skill instruction text\n".repeat(256);
        for i in 0..1000 {
            let id = format!("skill-{i}");
            skill(&paths.skills.join(&id), &body);
            for root in [".agents", ".cursor", ".codex", ".claude"] {
                skill(&temp.path().join(root).join("skills").join(&id), &body);
            }
        }
        let mut cache = SkillChangeCache::default();
        let first = Instant::now();
        assert!(cache.check(&paths, true).unwrap().changes.is_empty());
        let cold = first.elapsed();
        let checked: Vec<_> = cache.skills.values().map(|c| c.checked).collect();
        let next = Instant::now();
        assert!(cache.check(&paths, true).unwrap().changes.is_empty());
        let warm = next.elapsed();
        assert_eq!(
            checked,
            cache.skills.values().map(|c| c.checked).collect::<Vec<_>>()
        );
        let next = Instant::now();
        cache.check(&paths, false).unwrap();
        let cooldown = next.elapsed();
        println!("1000 library Skills + 4 x 1000 tool copies (~28 MB): cold={cold:?}, cached_metadata={warm:?}, cooldown={cooldown:?}");
    }

    #[test]
    #[cfg(unix)]
    fn unsafe_resource_is_partial_error_and_links_are_not_followed() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AgentHubPaths::for_home(temp.path());
        paths.ensure_runtime().unwrap();
        skill(&paths.skills.join("example"), "one");
        let host = temp.path().join(".agents/skills/example");
        skill(&host, "two");
        std::os::unix::fs::symlink(temp.path(), host.join("escape")).unwrap();
        let report = SkillChangeCache::default().check(&paths, true).unwrap();
        assert!(report.changes.is_empty());
        assert_eq!(report.errors, vec![host.display().to_string()]);
    }
}
