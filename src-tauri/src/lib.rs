use agenthub_core::{
    canonical, git,
    models::{
        AutoSyncProfile, AutoSyncUpdateResult, CapabilityDetail, CapabilityKind,
        CapabilityMutationResult, Dashboard, GitIdentity, Plan, RuleDocument, ScanItem,
        SyncSelection, Target, Transaction,
    },
    planner, scanner, transaction, AgentHub,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

#[derive(serde::Serialize)]
struct RuntimeDiagnostics {
    log_dir: String,
    canonical_root: String,
    git_available: bool,
    platform: &'static str,
}

static INITIALIZING: AtomicBool = AtomicBool::new(false);
struct InitGuard;
impl InitGuard {
    fn acquire() -> Result<Self, String> {
        INITIALIZING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "initialization is already running".to_string())?;
        Ok(Self)
    }
}
impl Drop for InitGuard {
    fn drop(&mut self) {
        INITIALIZING.store(false, Ordering::Release);
    }
}

fn hub() -> Result<AgentHub, String> {
    AgentHub::open_default().map_err(err)
}
fn err(e: impl std::fmt::Display) -> String {
    agenthub_core::secrets::redact(&e.to_string())
}

fn trace(area: &str, event: &str, context: impl std::fmt::Display) {
    let context = agenthub_core::secrets::redact(&context.to_string().replace(['\n', '\r'], " "));
    let context: String = context.chars().take(180).collect();
    log::info!("[agenthub][{area}] {event} {context}");
}

#[tauri::command]
fn runtime_diagnostics(app: tauri::AppHandle) -> Result<RuntimeDiagnostics, String> {
    let h = hub()?;
    Ok(RuntimeDiagnostics {
        log_dir: app.path().app_log_dir().map_err(err)?.display().to_string(),
        canonical_root: h.paths.root.display().to_string(),
        git_available: git::available(),
        platform: std::env::consts::OS,
    })
}

#[tauri::command]
fn debug_event(event: String, context: Option<String>) -> Result<(), String> {
    if event.is_empty()
        || event.len() > 64
        || !event
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("invalid debug event".into());
    }
    trace("ui", &event, context.unwrap_or_default());
    Ok(())
}

#[tauri::command]
fn dashboard() -> Result<Dashboard, String> {
    let h = hub()?;
    let initialized = h.store.initialized().map_err(err)?;
    let mut counts = BTreeMap::new();
    if initialized {
        for c in canonical::inventory(&h.paths).map_err(err)? {
            *counts.entry(c.kind.as_str().to_string()).or_insert(0) += 1;
        }
    }
    let auto_sync_targets = h
        .store
        .auto_sync_profiles()
        .map_err(err)?
        .into_iter()
        .filter(|profile| profile.enabled)
        .map(|profile| profile.target)
        .collect();
    Ok(Dashboard {
        initialized,
        inventory: counts,
        enabled_targets: h.store.enabled_targets().map_err(err)?,
        auto_sync_targets,
        dirty: if initialized {
            git::snapshot(&h.paths.root).map_err(err)?.dirty
        } else {
            false
        },
        recent_transactions: h.store.recent_transactions(5).map_err(err)?,
    })
}
#[tauri::command]
fn inventory() -> Result<Vec<agenthub_core::models::Capability>, String> {
    let h = hub()?;
    canonical::inventory(&h.paths).map_err(err)
}
#[tauri::command]
fn capability_detail(kind: CapabilityKind, id: String) -> Result<CapabilityDetail, String> {
    let h = hub()?;
    canonical::read_capability_detail(&h.paths, kind, &id).map_err(err)
}
#[tauri::command]
fn read_rule(id: String) -> Result<RuleDocument, String> {
    let h = hub()?;
    if !h.store.initialized().map_err(err)? {
        return Err("AgentHub must be initialized before editing rules".into());
    }
    canonical::read_rule(&h.paths, &id).map_err(err)
}
#[tauri::command]
fn save_rule(rule: RuleDocument, create: bool) -> Result<CapabilityMutationResult, String> {
    trace(
        "rules",
        "save_start",
        format_args!("id={} create={create}", rule.id),
    );
    let h = hub()?;
    if !h.store.initialized().map_err(err)? {
        return Err("AgentHub must be initialized before adding rules".into());
    }
    let result = canonical::save_rule(&h.paths, &rule, create).map_err(err);
    trace(
        "rules",
        if result.is_ok() {
            "save_ok"
        } else {
            "save_failed"
        },
        format_args!("id={}", rule.id),
    );
    let capability = result?;
    let auto_sync = transaction::run_auto_sync(&h.paths, &h.store).map_err(err)?;
    trace(
        "rules",
        "auto_sync_complete",
        format_args!(
            "id={} targets={} failures={}",
            rule.id,
            auto_sync.len(),
            auto_sync
                .iter()
                .filter(|outcome| outcome.error.is_some())
                .count()
        ),
    );
    Ok(CapabilityMutationResult {
        capability,
        auto_sync,
    })
}
#[tauri::command]
fn initial_scan() -> Result<Vec<ScanItem>, String> {
    let h = hub()?;
    if !h.store.initialized().map_err(err)?
        && !canonical::canonical_dirs_empty(&h.paths).map_err(err)?
    {
        return Err("incomplete Canonical import exists; discard it before retrying".into());
    }
    let items = scanner::scan_global(&h.paths, &Target::ALL).map_err(err)?;
    trace(
        "init",
        "scan_complete",
        format_args!(
            "found={} importable={} rejected={}",
            items.len(),
            items.iter().filter(|item| item.importable).count(),
            items.iter().filter(|item| !item.importable).count()
        ),
    );
    Ok(items)
}
#[tauri::command]
fn finish_init(selected_ids: Vec<String>) -> Result<Vec<String>, String> {
    let _guard = InitGuard::acquire()?;
    let h = hub()?;
    if h.store.initialized().map_err(err)? {
        return Err("AgentHub is already initialized".into());
    }
    if !git::available() {
        return Err("Git is required to initialize AgentHub; install Git and retry".into());
    }
    trace(
        "init",
        "import_start",
        format_args!("selected={}", selected_ids.len()),
    );
    let selected: std::collections::BTreeSet<_> = selected_ids.into_iter().collect();
    let mut items = scanner::scan_global(&h.paths, &Target::ALL).map_err(err)?;
    for item in &mut items {
        item.selected = selected.contains(&item.id);
    }
    let matched = items.iter().filter(|item| item.selected).count();
    if matched != selected.len() {
        return Err("scan result changed; scan again before importing".into());
    }
    let ids = canonical::import_initial_atomic(&h.paths, &items).map_err(|error| {
        trace("init", "import_failed", err(&error));
        err(error)
    })?;
    git::ensure_repo(&h.paths.root).map_err(err)?;
    h.store.set_initialized(true).map_err(err)?;
    trace(
        "init",
        "import_complete",
        format_args!("imported={}", ids.len()),
    );
    Ok(ids)
}
#[tauri::command]
fn discard_incomplete_init() -> Result<(), String> {
    let _guard = InitGuard::acquire()?;
    let h = hub()?;
    if h.store.initialized().map_err(err)? {
        return Err("cannot discard a completed initialization".into());
    }
    canonical::discard_incomplete_import(&h.paths).map_err(err)
}

#[tauri::command]
fn reset_failed_initialization() -> Result<(), String> {
    let _guard = InitGuard::acquire()?;
    let h = hub()?;
    if !h.store.initialized().map_err(err)? {
        return Err("AgentHub initialization is not marked complete".into());
    }
    let validation_error = match canonical::inventory(&h.paths) {
        Ok(_) => {
            return Err("Canonical is valid; refusing to reset a completed initialization".into())
        }
        Err(error) => error,
    };
    let snapshot = git::snapshot(&h.paths.root).map_err(err)?;
    if snapshot.head.is_some() {
        return Err("Canonical has committed history; refusing automatic reset".into());
    }
    trace("init", "legacy_reset_start", err(&validation_error));
    canonical::discard_incomplete_import(&h.paths).map_err(err)?;
    h.store.set_initialized(false).map_err(err)?;
    trace("init", "legacy_reset_complete", "canonical_copies_removed");
    Ok(())
}
#[tauri::command]
fn set_target(target: Target, enabled: bool) -> Result<(), String> {
    let h = hub()?;
    h.store.set_target(target, enabled).map_err(err)
}
#[tauri::command]
fn create_plan(target: Target, selection: Option<SyncSelection>) -> Result<Plan, String> {
    trace("plan", "create_start", target.as_str());
    let h = hub()?;
    let p = planner::create_with_selection(&h.paths, target, selection.as_ref()).map_err(err)?;
    h.store.save_plan(&p).map_err(err)?;
    trace(
        "plan",
        "create_ok",
        format_args!(
            "target={} id={} steps={}",
            target.as_str(),
            p.id,
            p.steps.len()
        ),
    );
    Ok(p)
}
#[tauri::command]
fn apply_plan(plan_id: String) -> Result<Transaction, String> {
    trace("apply", "start", format_args!("plan_id={plan_id}"));
    let h = hub()?;
    let p = h
        .store
        .plan(&plan_id)
        .map_err(err)?
        .ok_or_else(|| "plan not found".to_string())?;
    let result = transaction::apply(&h.paths, &h.store, &p).map_err(err);
    trace(
        "apply",
        if result.is_ok() { "ok" } else { "failed" },
        format_args!("plan_id={plan_id} target={}", p.target.as_str()),
    );
    result
}
#[tauri::command]
fn auto_sync_profiles() -> Result<Vec<AutoSyncProfile>, String> {
    let h = hub()?;
    h.store.auto_sync_profiles().map_err(err)
}
#[tauri::command]
fn set_auto_sync(
    target: Target,
    selection: SyncSelection,
    enabled: bool,
) -> Result<AutoSyncUpdateResult, String> {
    trace(
        "auto_sync",
        "configure_start",
        format_args!("target={} enabled={enabled}", target.as_str()),
    );
    let h = hub()?;
    let profile = AutoSyncProfile {
        target,
        enabled,
        selection,
    };
    if !enabled {
        h.store.set_auto_sync_profile(&profile).map_err(err)?;
        trace("auto_sync", "disabled", target.as_str());
        return Ok(AutoSyncUpdateResult {
            profile,
            initial_sync: None,
        });
    }
    let has_scope = !profile.selection.skills.is_empty()
        || !profile.selection.plugins.is_empty()
        || !profile.selection.mcp.is_empty()
        || profile.selection.rules;
    if !has_scope {
        return Err("select at least one capability before enabling automatic sync".into());
    }
    let initial_sync = transaction::sync_once(&h.paths, &h.store, target, Some(&profile.selection))
        .map_err(err)?;
    h.store.set_auto_sync_profile(&profile).map_err(err)?;
    trace(
        "auto_sync",
        "enabled",
        format_args!(
            "target={} changed={} transaction={}",
            target.as_str(),
            initial_sync.changed,
            initial_sync
                .transaction
                .as_ref()
                .map(|transaction| transaction.id.as_str())
                .unwrap_or("none")
        ),
    );
    Ok(AutoSyncUpdateResult {
        profile,
        initial_sync: Some(initial_sync),
    })
}
#[tauri::command]
fn transaction_history(limit: Option<usize>) -> Result<Vec<Transaction>, String> {
    let h = hub()?;
    h.store
        .recent_transactions(limit.unwrap_or(100).clamp(1, 500))
        .map_err(err)
}
#[tauri::command]
fn rollback_transaction(transaction_id: String) -> Result<Transaction, String> {
    trace(
        "rollback",
        "start",
        format_args!("transaction_id={transaction_id}"),
    );
    let h = hub()?;
    let result = transaction::rollback(&h.paths, &h.store, &transaction_id).map_err(err);
    trace(
        "rollback",
        if result.is_ok() { "ok" } else { "failed" },
        format_args!("transaction_id={transaction_id}"),
    );
    result
}
#[tauri::command]
fn git_status() -> Result<String, String> {
    let h = hub()?;
    git::status(&h.paths.root).map_err(err)
}
#[tauri::command]
fn git_diff() -> Result<String, String> {
    let h = hub()?;
    git::diff(&h.paths.root).map_err(err)
}
#[tauri::command]
fn git_identity() -> Result<GitIdentity, String> {
    let h = hub()?;
    git::identity(&h.paths.root).map_err(err)
}
#[tauri::command]
fn git_log() -> Result<String, String> {
    let h = hub()?;
    git::log(&h.paths.root).map_err(err)
}
#[tauri::command]
fn git_commit(
    message: String,
    name: Option<String>,
    email: Option<String>,
) -> Result<String, String> {
    let h = hub()?;
    git::commit(&h.paths.root, &message, name.as_deref(), email.as_deref()).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("agenthub-desktop".into()),
                    },
                ))
                .max_file_size(5_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(4))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|_| {
            std::panic::set_hook(Box::new(|panic_info| {
                log::error!(
                    "desktop panic: {}",
                    agenthub_core::secrets::redact(&panic_info.to_string())
                );
            }));
            log::info!(
                "AgentHub desktop started version={}",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard,
            runtime_diagnostics,
            debug_event,
            inventory,
            capability_detail,
            read_rule,
            save_rule,
            initial_scan,
            finish_init,
            discard_incomplete_init,
            reset_failed_initialization,
            set_target,
            create_plan,
            apply_plan,
            auto_sync_profiles,
            set_auto_sync,
            transaction_history,
            rollback_transaction,
            git_status,
            git_diff,
            git_identity,
            git_log,
            git_commit
        ])
        .run(tauri::generate_context!())
        .expect("error while running AgentHub")
}
