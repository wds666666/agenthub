//! Device-local scheduling. Never expands a saved target scope.
use crate::{models::AutoSyncOutcome, paths::AgentHubPaths, storage::Store};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct BackgroundSettings {
    pub close_behavior: String,
    pub check_seconds: u64,
    pub sync_seconds: u64,
    pub paused: bool,
}
impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            close_behavior: "ask".into(),
            check_seconds: 120,
            sync_seconds: 0,
            paused: false,
        }
    }
}
pub fn settings(store: &Store) -> Result<BackgroundSettings> {
    Ok(store
        .meta("background_settings")?
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default())
}
pub fn save_settings(store: &Store, value: &BackgroundSettings) -> Result<()> {
    anyhow::ensure!(
        ["ask", "exit", "tray"].contains(&value.close_behavior.as_str()),
        "invalid close behavior"
    );
    anyhow::ensure!(
        [60, 120, 300, 600, 1800].contains(&value.check_seconds),
        "invalid check interval"
    );
    anyhow::ensure!(
        [0, 60, 120, 300, 600, 1800].contains(&value.sync_seconds),
        "invalid delivery interval"
    );
    let previous = settings(store)?;
    if previous.sync_seconds != value.sync_seconds && status(store)?.pending {
        store.set_meta(
            "background_due",
            &(Utc::now().timestamp() + value.sync_seconds as i64).to_string(),
        )?;
    }
    store.set_meta("background_settings", &serde_json::to_string(value)?)
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackgroundStatus {
    pub running: bool,
    pub pending: bool,
    pub last_success: Option<String>,
    pub next_sync: Option<String>,
    #[serde(skip)]
    pub due_timestamp: Option<i64>,
    pub error: Option<String>,
    pub manager_error: Option<String>,
}
pub fn status(store: &Store) -> Result<BackgroundStatus> {
    let nonempty =
        |key| -> Result<Option<String>> { Ok(store.meta(key)?.filter(|s| !s.is_empty())) };
    let due = nonempty("background_due")?;
    Ok(BackgroundStatus {
        running: false,
        pending: due.is_some(),
        last_success: nonempty("background_success")?,
        due_timestamp: due.as_ref().and_then(|s| s.parse::<i64>().ok()),
        next_sync: due
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
            .map(|d| d.to_rfc3339()),
        error: nonempty("background_error")?,
        manager_error: nonempty("manager_error")?,
    })
}
/// Fixed due time: continued editing cannot indefinitely postpone delivery.
pub fn queue(store: &Store) -> Result<()> {
    if store.meta("background_due")?.is_none_or(|s| s.is_empty()) {
        let due = Utc::now().timestamp() + settings(store)?.sync_seconds as i64;
        store.set_meta("background_due", &due.to_string())?;
    }
    Ok(())
}
pub fn run_due(paths: &AgentHubPaths, store: &Store) -> Result<Vec<AutoSyncOutcome>> {
    if settings(store)?.paused {
        return Ok(vec![]);
    }
    let due = store
        .meta("background_due")?
        .and_then(|s| s.parse::<i64>().ok());
    if due.is_none_or(|due| due > Utc::now().timestamp()) {
        return Ok(vec![]);
    }
    let _lock = crate::versions::lock(paths)?;
    crate::transaction::with_trigger("scheduled", || {
        crate::transaction::deliver_changes(paths, store)
    })
}
pub fn record(store: &Store, outcomes: &[AutoSyncOutcome]) -> Result<()> {
    store.set_meta("background_due", "")?;
    let errors: Vec<_> = outcomes
        .iter()
        .filter_map(|o| o.error.as_ref())
        .cloned()
        .collect();
    store.set_meta("background_error", &errors.join("; "))?;
    if errors.is_empty() {
        store.set_meta("background_success", &Utc::now().to_rfc3339())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_queue_and_validation() {
        let t = tempfile::tempdir().unwrap();
        let h = crate::AgentHub::open(AgentHubPaths::for_home(t.path())).unwrap();
        assert_eq!(settings(&h.store).unwrap(), BackgroundSettings::default());
        let mut s = BackgroundSettings {
            sync_seconds: 60,
            ..Default::default()
        };
        save_settings(&h.store, &s).unwrap();
        queue(&h.store).unwrap();
        let before = h.store.meta("background_due").unwrap();
        queue(&h.store).unwrap();
        assert_eq!(before, h.store.meta("background_due").unwrap());
        assert!(run_due(&h.paths, &h.store).unwrap().is_empty());
        s.check_seconds = 1;
        assert!(save_settings(&h.store, &s).is_err());
    }
}

#[cfg(test)]
mod delivery_tests {
    use super::*;
    use crate::{
        models::{AutoSyncProfile, CapabilityKind, SyncSelection, Target},
        transaction, AgentHub,
    };
    use std::fs;
    #[test]
    fn scheduled_edits_pause_manual_refresh_drift_and_attribution() {
        let temp = tempfile::tempdir().unwrap();
        let h = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        h.store.set_initialized(true).unwrap();
        h.store
            .set_auto_sync_profile(&AutoSyncProfile {
                target: Target::Agents,
                enabled: true,
                needs_review: false,
                selection: SyncSelection {
                    all_kinds: vec![CapabilityKind::Skill],
                    sync_on_change: true,
                    ..Default::default()
                },
            })
            .unwrap();
        let source = h.paths.skills.join("one");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# One").unwrap();
        let mut prefs = BackgroundSettings {
            sync_seconds: 60,
            ..Default::default()
        };
        save_settings(&h.store, &prefs).unwrap();
        assert!(transaction::run_auto_sync_on_change(&h.paths, &h.store)
            .unwrap()
            .is_empty());
        assert!(!temp.path().join(".agents/skills").exists());
        h.store.set_meta("background_due", "1").unwrap();
        prefs.paused = true;
        save_settings(&h.store, &prefs).unwrap();
        assert!(run_due(&h.paths, &h.store).unwrap().is_empty());
        assert!(status(&h.store).unwrap().pending);
        prefs.paused = false;
        save_settings(&h.store, &prefs).unwrap();
        let outcomes = run_due(&h.paths, &h.store).unwrap();
        assert!(outcomes[0].changed);
        assert!(!status(&h.store).unwrap().pending);
        let tx = h
            .store
            .transaction(outcomes[0].transaction_id.as_ref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(tx.trigger.as_deref(), Some("scheduled"));
        assert_eq!(tx.source_state.as_deref(), Some("working_tree"));
        assert_eq!(tx.resource_changes[0].id, "one");
        prefs.paused = true;
        save_settings(&h.store, &prefs).unwrap();
        fs::write(source.join("SKILL.md"), "# Two").unwrap();
        let outcomes = transaction::run_auto_sync_pending(&h.paths, &h.store).unwrap();
        assert!(outcomes[0].changed);
        let host = temp.path().join(".agents/skills/one/SKILL.md");
        fs::write(&host, "# User host edit").unwrap();
        fs::write(source.join("SKILL.md"), "# Three").unwrap();
        let outcomes = transaction::run_auto_sync_pending(&h.paths, &h.store).unwrap();
        assert!(outcomes[0].error.is_some());
        assert_eq!(fs::read_to_string(host).unwrap(), "# User host edit");
    }
    #[test]
    fn manager_update_obeys_existing_scope_and_repeat_launch_never_refills_cleared_tools() {
        let temp = tempfile::tempdir().unwrap();
        let h = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
        h.store.set_initialized(true).unwrap();
        h.store
            .set_auto_sync_profile(&AutoSyncProfile {
                target: Target::Agents,
                enabled: true,
                needs_review: false,
                selection: SyncSelection {
                    all_kinds: vec![CapabilityKind::Skill],
                    sync_on_change: true,
                    ..Default::default()
                },
            })
            .unwrap();
        assert!(crate::manager::ensure(&h.paths, &h.store).unwrap());
        let host = temp.path().join(".agents/skills");
        assert!(host.join("agenthub-manager/SKILL.md").exists());
        fs::remove_dir_all(&host).unwrap();
        assert!(!crate::manager::ensure(&h.paths, &h.store).unwrap());
        let reopened = AgentHub::open(h.paths.clone()).unwrap();
        assert!(!crate::manager::ensure(&reopened.paths, &reopened.store).unwrap());
        assert!(!host.exists());
    }
}
