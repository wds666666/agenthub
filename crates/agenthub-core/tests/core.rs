use agenthub_core::{
    adapters, canonical, git, host,
    models::{
        CapabilityKind, GitSnapshot, McpServer, RuleDocument, ScanItem, SyncSelection, Target,
        Transaction, TransactionMode,
    },
    paths::AgentHubPaths,
    planner, scanner, secrets, transaction, AgentHub,
};
use std::{collections::BTreeMap, fs};
use tempfile::TempDir;

fn fixture() -> (TempDir, AgentHub) {
    let temp = TempDir::new().unwrap();
    let paths = AgentHubPaths::for_home(temp.path());
    let hub = AgentHub::open(paths).unwrap();
    (temp, hub)
}
fn seed(hub: &AgentHub) {
    let skill = hub.paths.skills.join("review");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: review\ndescription: Review code\n---\n# Review",
    )
    .unwrap();
    let rule = hub.paths.rules.join("safe");
    fs::create_dir_all(&rule).unwrap();
    fs::write(rule.join("rule.md"), "Never reveal secrets.").unwrap();
    fs::write(
        rule.join("rule.json"),
        r#"{"schemaVersion":1,"id":"safe","activation":"always","paths":[]}"#,
    )
    .unwrap();
    let mcp = hub.paths.mcp.join("local");
    fs::create_dir_all(&mcp).unwrap();
    let server = McpServer {
        schema_version: 1,
        id: "local".into(),
        display_name: "Local".into(),
        transport: "stdio".into(),
        command: Some("server".into()),
        args: vec!["--stdio".into()],
        url: None,
        env: BTreeMap::new(),
        headers: BTreeMap::new(),
    };
    fs::write(
        mcp.join("server.json"),
        serde_json::to_vec(&server).unwrap(),
    )
    .unwrap();
}

#[test]
fn scanner_is_strictly_user_global() {
    let (temp, hub) = fixture();
    let project = temp.path().join("work/project/.cursor/skills/evil");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("SKILL.md"), "evil").unwrap();
    let global = temp.path().join(".cursor/skills/good");
    fs::create_dir_all(&global).unwrap();
    fs::write(global.join("SKILL.md"), "good").unwrap();
    let hidden = temp.path().join(".cursor/skills/.system");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("SKILL.md"), "built in").unwrap();
    let git = temp.path().join(".agents/skills/.git");
    fs::create_dir_all(&git).unwrap();
    fs::write(git.join("SKILL.md"), "not a skill").unwrap();
    fs::create_dir_all(temp.path().join(".cursor/plugins/local/cache")).unwrap();
    std::env::set_current_dir(project.parent().unwrap()).unwrap();
    let found = scanner::scan_global(&hub.paths, &[Target::Cursor]).unwrap();
    assert_eq!(found.len(), 1);
    assert!(found[0].path.ends_with("good"));
    assert!(!scanner::allowed_roots(temp.path())
        .iter()
        .any(|p| p.starts_with(temp.path().join("work"))));
}

#[test]
fn scanner_finds_nested_agent_skills_without_entering_hidden_directories() {
    let (temp, hub) = fixture();
    let nested = temp.path().join(".agents/skills/team/review");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("SKILL.md"), "# Review").unwrap();
    let hidden = temp.path().join(".agents/skills/.cache/hidden");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(hidden.join("SKILL.md"), "# Hidden").unwrap();

    let found = scanner::scan_global(&hub.paths, &[Target::Agents]).unwrap();
    assert_eq!(found.len(), 1);
    assert!(found[0].path.ends_with("team/review"));
}

#[test]
fn shared_agents_skills_can_be_explicitly_cleared() {
    let (temp, hub) = fixture();
    seed(&hub);
    let host_skill = temp.path().join(".agents/skills/host-only");
    fs::create_dir_all(&host_skill).unwrap();
    fs::write(host_skill.join("SKILL.md"), "# Host only").unwrap();
    let selection = SyncSelection {
        skills_managed: true,
        skills: Vec::new(),
        ..SyncSelection::default()
    };

    let run =
        transaction::sync_once(&hub.paths, &hub.store, Target::Agents, Some(&selection)).unwrap();
    assert!(run.changed);
    assert!(!temp.path().join(".agents/skills/host-only").exists());
    assert!(temp.path().join(".agents/skills").is_dir());
}

#[test]
fn scanner_reports_invalid_rules_without_allowing_import() {
    let (temp, hub) = fixture();
    let rules = temp.path().join(".claude/rules");
    fs::create_dir_all(&rules).unwrap();
    fs::write(rules.join("empty.md"), "  \r\n").unwrap();
    fs::write(rules.join("binary.md"), [0xff, 0xfe]).unwrap();
    fs::write(rules.join("valid.md"), "# Valid\r\nKeep this rule.").unwrap();

    let found = scanner::scan_global(&hub.paths, &[Target::Claude]).unwrap();
    assert_eq!(found.len(), 3);
    let empty = found
        .iter()
        .find(|item| item.path.ends_with("empty.md"))
        .unwrap();
    assert!(!empty.importable);
    assert_eq!(empty.warning.as_deref(), Some("rule_empty"));
    let binary = found
        .iter()
        .find(|item| item.path.ends_with("binary.md"))
        .unwrap();
    assert!(!binary.importable);
    assert_eq!(binary.warning.as_deref(), Some("rule_invalid_encoding"));
    assert!(
        found
            .iter()
            .find(|item| item.path.ends_with("valid.md"))
            .unwrap()
            .importable
    );

    let mut selected = empty.clone();
    selected.selected = true;
    assert!(canonical::import_initial_atomic(&hub.paths, &[selected]).is_err());
    assert!(canonical::canonical_dirs_empty(&hub.paths).unwrap());
}

#[test]
fn scanner_and_import_keep_each_mcp_server_distinct() {
    let (temp, hub) = fixture();
    let cursor = temp.path().join(".cursor");
    fs::create_dir_all(&cursor).unwrap();
    fs::write(
        cursor.join("mcp.json"),
        r#"{"theme":"night","mcpServers":{"alpha":{"command":"a"},"beta":{"command":"b","args":["--stdio"]}}}"#,
    )
    .unwrap();

    let mut found = scanner::scan_global(&hub.paths, &[Target::Cursor]).unwrap();
    assert_eq!(found.len(), 2);
    assert_ne!(found[0].digest, found[1].digest);
    found.iter_mut().for_each(|item| item.selected = true);
    let imported = canonical::import_initial_atomic(&hub.paths, &found).unwrap();
    assert_eq!(imported.len(), 2);
    let ids: Vec<_> = canonical::inventory(&hub.paths)
        .unwrap()
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(ids, vec!["alpha", "beta"]);
}

#[test]
fn host_inventory_is_target_specific_and_cleanup_preserves_other_mcp_servers() {
    let (temp, hub) = fixture();
    let cursor = temp.path().join(".cursor");
    fs::create_dir_all(cursor.join("skills/host-only")).unwrap();
    fs::write(cursor.join("skills/host-only/SKILL.md"), "# Host only").unwrap();
    fs::create_dir_all(temp.path().join(".agents/skills/shared")).unwrap();
    fs::write(
        temp.path().join(".agents/skills/shared/SKILL.md"),
        "# Shared",
    )
    .unwrap();
    fs::write(
        cursor.join("mcp.json"),
        r#"{"theme":"night","mcpServers":{"alpha":{"command":"a"},"beta":{"command":"b"}}}"#,
    )
    .unwrap();

    let items = host::inventory(&hub.paths, Target::Cursor).unwrap();
    assert!(items.iter().any(|item| item.display_name == "host-only"));
    assert!(!items.iter().any(|item| item.display_name == "shared"));
    let alpha = items
        .iter()
        .find(|item| item.display_name == "alpha")
        .unwrap();
    let result =
        host::cleanup(&hub.paths, Target::Cursor, std::slice::from_ref(&alpha.id)).unwrap();
    assert!(result.backup_path.join("manifest.json").is_file());
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(cursor.join("mcp.json")).unwrap()).unwrap();
    assert_eq!(value["theme"], "night");
    assert!(value["mcpServers"].get("alpha").is_none());
    assert!(value["mcpServers"].get("beta").is_some());
}

#[test]
fn cursor_plugin_projection_flattens_payload_and_omits_agenthub_manifest() {
    let (_temp, hub) = fixture();
    let plugin = hub.paths.plugins.join("portable");
    fs::create_dir_all(plugin.join("payload/.cursor-plugin")).unwrap();
    fs::create_dir_all(plugin.join("payload/skills/demo")).unwrap();
    fs::write(plugin.join("agenthub.plugin.json"), r#"{"schemaVersion":1,"id":"portable","displayName":"Portable","sourceFormat":"cursor","components":[]}"#).unwrap();
    fs::write(
        plugin.join("payload/.cursor-plugin/plugin.json"),
        r#"{"name":"portable"}"#,
    )
    .unwrap();
    fs::write(plugin.join("payload/skills/demo/SKILL.md"), "# Demo").unwrap();

    let domains = adapters::projection(&hub.paths, Target::Cursor).unwrap();
    let plugins = domains
        .iter()
        .find(|domain| domain.name == "plugins")
        .unwrap();
    assert!(plugins
        .files
        .contains_key(std::path::Path::new("portable/.cursor-plugin/plugin.json")));
    assert!(plugins
        .files
        .contains_key(std::path::Path::new("portable/skills/demo/SKILL.md")));
    assert!(!plugins
        .files
        .keys()
        .any(|path| path.to_string_lossy().contains("payload")));
    assert!(!plugins
        .files
        .keys()
        .any(|path| path.ends_with("agenthub.plugin.json")));
}

#[test]
fn codex_plugin_store_is_preserved_and_reported_as_constraint() {
    let (temp, hub) = fixture();
    let plugin = hub.paths.plugins.join("portable");
    fs::create_dir_all(plugin.join("payload")).unwrap();
    fs::write(
        plugin.join("agenthub.plugin.json"),
        r#"{"schemaVersion":1,"id":"portable","displayName":"Portable","components":[]}"#,
    )
    .unwrap();
    fs::write(plugin.join("payload/plugin.json"), r#"{"name":"portable"}"#).unwrap();
    let installed = temp.path().join(".codex/plugins/cache/vendor/plugin/1.0.0");
    fs::create_dir_all(&installed).unwrap();
    fs::write(installed.join("state.json"), "keep").unwrap();
    fs::write(installed.join("plugin.json"), r#"{"name":"plugin"}"#).unwrap();

    let host_plugins = host::inventory(&hub.paths, Target::Codex).unwrap();
    let cached = host_plugins
        .iter()
        .find(|item| item.path == installed)
        .unwrap();
    assert_eq!(cached.display_name, "plugin");
    assert!(!cached.deletable);

    let plan = planner::create(&hub.paths, Target::Codex).unwrap();
    assert!(plan
        .warnings
        .iter()
        .any(|warning| warning == "codex_plugins_marketplace_managed"));
    assert!(!plan
        .steps
        .iter()
        .any(|step| step.path.starts_with(temp.path().join(".codex/plugins"))));
    assert_eq!(
        fs::read_to_string(installed.join("state.json")).unwrap(),
        "keep"
    );
}

#[test]
fn canonical_ids_are_portable_to_windows() {
    assert!(!canonical::valid_id("con"));
    assert!(!canonical::valid_id("nul"));
    assert!(!canonical::valid_id("com1"));
    assert!(!canonical::valid_id("lpt9"));
    assert!(canonical::valid_id("console"));
}

#[test]
fn plan_is_read_only_and_apply_preserves_unrelated_mcp_settings() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let cursor = hub.paths.user_home.join(".cursor");
    fs::create_dir_all(cursor.join("skills/host-only")).unwrap();
    fs::write(cursor.join("skills/host-only/SKILL.md"), "drift").unwrap();
    fs::write(
        cursor.join("mcp.json"),
        r#"{"theme":"night","mcpServers":{"old":{"command":"old"}}}"#,
    )
    .unwrap();
    let before = fs::read(cursor.join("mcp.json")).unwrap();
    let plan = planner::create(&hub.paths, Target::Cursor).unwrap();
    assert_eq!(fs::read(cursor.join("mcp.json")).unwrap(), before);
    let skills = plan
        .summary
        .iter()
        .find(|item| item.kind == CapabilityKind::Skill)
        .unwrap();
    assert_eq!(skills.affected, 2);
    assert_eq!(skills.create, 1);
    assert_eq!(skills.delete, 1);
    assert_eq!(skills.files, 2);
    assert!(plan.steps.iter().any(|step| {
        step.capability_kind == Some(CapabilityKind::Skill)
            && step.capability_id.as_deref() == Some("host-only")
            && step.action == agenthub_core::models::PlanAction::Delete
    }));
    hub.store.save_plan(&plan).unwrap();
    let tx = transaction::apply(&hub.paths, &hub.store, &plan).unwrap();
    assert_eq!(tx.status, "applied");
    assert_eq!(tx.mode, TransactionMode::Reviewed);
    assert_eq!(hub.store.enabled_targets().unwrap(), vec![Target::Cursor]);
    assert!(!cursor.join("skills/host-only").exists());
    assert!(cursor.join("skills/review/SKILL.md").exists());
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(cursor.join("mcp.json")).unwrap()).unwrap();
    assert_eq!(json["theme"], "night");
    assert!(json["mcpServers"].get("old").is_none());
    assert!(json["mcpServers"].get("local").is_some());
}

#[test]
fn codex_mcp_projection_uses_documented_env_and_http_header_fields() {
    let (temp, hub) = fixture();
    seed(&hub);
    let path = hub.paths.mcp.join("local/server.json");
    let mut server: McpServer = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    server.env.insert("MODE".into(), "safe".into());
    server.headers.insert("X-Tenant".into(), "demo".into());
    server.transport = "http".into();
    server.command = None;
    server.args.clear();
    server.url = Some("https://example.test/mcp".into());
    fs::write(&path, serde_json::to_vec(&server).unwrap()).unwrap();
    let stdio_dir = hub.paths.mcp.join("stdio");
    fs::create_dir_all(&stdio_dir).unwrap();
    fs::write(
        stdio_dir.join("server.json"),
        serde_json::to_vec(&McpServer {
            schema_version: 1,
            id: "stdio".into(),
            display_name: "Stdio".into(),
            transport: "stdio".into(),
            command: Some("stdio-server".into()),
            args: Vec::new(),
            url: None,
            env: BTreeMap::from([("MODE".into(), "safe".into())]),
            headers: BTreeMap::new(),
        })
        .unwrap(),
    )
    .unwrap();
    fs::create_dir_all(temp.path().join(".codex")).unwrap();
    fs::write(
        temp.path().join(".codex/config.toml"),
        "model = \"gpt-test\"\n",
    )
    .unwrap();

    transaction::sync_once(&hub.paths, &hub.store, Target::Codex, None).unwrap();
    let config = fs::read_to_string(temp.path().join(".codex/config.toml")).unwrap();
    assert!(config.contains("model = \"gpt-test\""));
    assert!(config.contains("http_headers = { X-Tenant = \"demo\" }"));
    assert_eq!(config.matches("env = { MODE = \"safe\" }").count(), 1);
}

#[test]
fn claude_apply_preserves_cli_managed_plugin_store_and_reports_constraint() {
    let (temp, hub) = fixture();
    seed(&hub);
    let canonical_plugin = hub.paths.plugins.join("example");
    fs::create_dir_all(&canonical_plugin).unwrap();
    fs::write(
        canonical_plugin.join("agenthub.plugin.json"),
        r#"{"schemaVersion":1,"id":"example","displayName":"Example","components":[]}"#,
    )
    .unwrap();
    let installed = temp.path().join(".claude/plugins/cache/vendor/example");
    fs::create_dir_all(&installed).unwrap();
    fs::write(installed.join("state.json"), "keep").unwrap();

    let plan = planner::create_with_selection(&hub.paths, Target::Claude, None).unwrap();
    assert!(plan
        .warnings
        .iter()
        .any(|warning| warning == "claude_plugins_cli_managed"));
    hub.store.save_plan(&plan).unwrap();
    transaction::apply(&hub.paths, &hub.store, &plan).unwrap();
    assert_eq!(
        fs::read_to_string(installed.join("state.json")).unwrap(),
        "keep"
    );
}

#[test]
fn manual_rollback_restores_pre_apply_state_and_records_a_new_transaction() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let cursor = hub.paths.user_home.join(".cursor");
    fs::create_dir_all(cursor.join("skills/host-only")).unwrap();
    fs::write(cursor.join("skills/host-only/SKILL.md"), "before apply").unwrap();

    let plan = planner::create(&hub.paths, Target::Cursor).unwrap();
    hub.store.save_plan(&plan).unwrap();
    let applied = transaction::apply(&hub.paths, &hub.store, &plan).unwrap();
    assert!(cursor.join("skills/review/SKILL.md").exists());
    assert!(!cursor.join("skills/host-only").exists());

    let rollback = transaction::rollback(&hub.paths, &hub.store, &applied.id).unwrap();
    assert_eq!(rollback.status, "rollback_applied");
    assert_eq!(rollback.mode, TransactionMode::Rollback);
    assert_eq!(rollback.target, Target::Cursor);
    assert_ne!(rollback.id, applied.id);
    assert!(!cursor.join("skills/review").exists());
    assert_eq!(
        fs::read_to_string(cursor.join("skills/host-only/SKILL.md")).unwrap(),
        "before apply"
    );
    assert!(hub.store.transaction(&rollback.id).unwrap().is_some());
}

#[test]
fn selected_sync_scope_only_projects_selected_domains_and_resources() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let cursor = hub.paths.user_home.join(".cursor");
    fs::create_dir_all(cursor.join("skills/host-only")).unwrap();
    fs::write(cursor.join("skills/host-only/SKILL.md"), "host drift").unwrap();
    fs::write(
        cursor.join("mcp.json"),
        r#"{"theme":"night","mcpServers":{"old":{"command":"old"}}}"#,
    )
    .unwrap();
    fs::create_dir_all(cursor.join("plugins/local/host-plugin")).unwrap();
    fs::write(cursor.join("plugins/local/host-plugin/file.txt"), "keep").unwrap();
    fs::create_dir_all(cursor.join("plugins/local/agenthub-rules/rules")).unwrap();
    fs::write(
        cursor.join("plugins/local/agenthub-rules/rules/keep.mdc"),
        "keep rule",
    )
    .unwrap();

    let selection = SyncSelection {
        skills: vec!["review".into()],
        plugins: Vec::new(),
        mcp: Vec::new(),
        rules: false,
        ..SyncSelection::default()
    };
    let plan =
        planner::create_with_selection(&hub.paths, Target::Cursor, Some(&selection)).unwrap();
    assert!(plan.selection.as_ref().unwrap() == &selection);
    assert!(plan
        .steps
        .iter()
        .all(|step| step.capability_kind == Some(CapabilityKind::Skill)));
    hub.store.save_plan(&plan).unwrap();
    let transaction =
        transaction::apply_with_mode(&hub.paths, &hub.store, &plan, TransactionMode::AutoSync)
            .unwrap();
    assert_eq!(transaction.mode, TransactionMode::AutoSync);
    assert!(cursor.join("skills/review/SKILL.md").exists());
    assert!(!cursor.join("skills/host-only").exists());
    assert_eq!(
        fs::read_to_string(cursor.join("plugins/local/host-plugin/file.txt")).unwrap(),
        "keep"
    );
    assert!(fs::read_to_string(cursor.join("mcp.json"))
        .unwrap()
        .contains("old"));
    assert!(cursor
        .join("plugins/local/agenthub-rules/rules/keep.mdc")
        .exists());
}

#[test]
fn encrypted_secret_detects_wrong_key() {
    let (_temp, hub) = fixture();
    secrets::set(&hub.store, &hub.paths, "TOKEN", b"value").unwrap();
    assert_eq!(
        secrets::get(&hub.store, &hub.paths, "TOKEN")
            .unwrap()
            .unwrap(),
        b"value"
    );
    fs::write(&hub.paths.master_key, [7u8; 32]).unwrap();
    assert!(secrets::get(&hub.store, &hub.paths, "TOKEN").is_err());
}

#[test]
fn database_can_rebuild_inventory_from_files() {
    let (temp, hub) = fixture();
    seed(&hub);
    assert_eq!(canonical::inventory(&hub.paths).unwrap().len(), 3);
    drop(hub);
    fs::remove_file(temp.path().join(".agenthub/state/agenthub.db")).unwrap();
    let reopened = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
    assert_eq!(canonical::inventory(&reopened.paths).unwrap().len(), 3);
}

#[test]
fn capability_details_preview_canonical_content_and_file_inventory() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let skill =
        canonical::read_capability_detail(&hub.paths, CapabilityKind::Skill, "review").unwrap();
    assert_eq!(skill.preview_path, "SKILL.md");
    assert!(skill.preview.contains("# Review"));
    assert_eq!(skill.files[0].path, "SKILL.md");

    let server_path = hub.paths.mcp.join("local/server.json");
    let mut server: serde_json::Value =
        serde_json::from_slice(&fs::read(&server_path).unwrap()).unwrap();
    server["env"] = serde_json::json!({"API_TOKEN":"super-secret-value"});
    fs::write(&server_path, serde_json::to_vec(&server).unwrap()).unwrap();
    let mcp = canonical::read_capability_detail(&hub.paths, CapabilityKind::Mcp, "local").unwrap();
    assert_eq!(mcp.preview_path, "server.json");
    assert!(mcp.preview.contains("stdio"));
    assert!(mcp.preview.contains("[REDACTED]"));
    assert!(!mcp.preview.contains("super-secret-value"));
    assert_eq!(mcp.files.len(), 1);
}

#[test]
fn plugin_import_rejects_unknown_components_and_traversal() {
    let (temp, hub) = fixture();
    let plugin = temp.path().join("plugin");
    fs::create_dir_all(&plugin).unwrap();
    let scan = |path: &std::path::Path| ScanItem {
        id: "scan".into(),
        kind: CapabilityKind::Plugin,
        source: "cursor".into(),
        path: path.to_path_buf(),
        digest: "digest".into(),
        selected: true,
        importable: true,
        source_key: None,
        warning: None,
    };
    fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"bad","mystery":"./x"}"#,
    )
    .unwrap();
    assert!(canonical::import_scan_items(&hub.paths, &[scan(&plugin)]).is_err());
    fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"bad","skills":["../escape"]}"#,
    )
    .unwrap();
    assert!(canonical::import_scan_items(&hub.paths, &[scan(&plugin)]).is_err());
    assert!(canonical::import_initial_atomic(&hub.paths, &[scan(&plugin)]).is_err());
    assert_eq!(fs::read_dir(&hub.paths.plugins).unwrap().count(), 0);
}

#[test]
fn rule_editor_writes_canonical_and_git_commit_records_it() {
    let (_temp, hub) = fixture();
    let mut rule = RuleDocument {
        schema_version: 1,
        id: "team-safety".into(),
        display_name: "Team safety".into(),
        activation: "paths".into(),
        paths: vec!["src/**".into()],
        targets: Target::HOSTS.to_vec(),
        body: "# Safety\n\nDo not expose secrets.".into(),
    };
    let capability = canonical::save_rule(&hub.paths, &rule, true).unwrap();
    assert_eq!(capability.display_name, "Team safety");
    assert_eq!(
        canonical::read_rule(&hub.paths, "team-safety").unwrap(),
        rule
    );

    rule.body.push_str("\nValidate generated files.");
    canonical::save_rule(&hub.paths, &rule, false).unwrap();
    assert!(canonical::read_rule(&hub.paths, "team-safety")
        .unwrap()
        .body
        .contains("Validate generated files"));

    assert!(git::commit(
        &hub.paths.root,
        "",
        Some("AgentHub Test"),
        Some("test@example.com")
    )
    .is_err());
    git::commit(
        &hub.paths.root,
        "add team safety rule",
        Some("AgentHub Test"),
        Some("test@example.com"),
    )
    .unwrap();
    assert!(git::log(&hub.paths.root)
        .unwrap()
        .contains("add team safety rule"));
    let identity = git::identity(&hub.paths.root).unwrap();
    assert_eq!(identity.name.as_deref(), Some("AgentHub Test"));
}

#[test]
fn legacy_applied_transaction_enables_missing_target_on_reopen() {
    let (temp, hub) = fixture();
    hub.store
        .save_transaction(&Transaction {
            id: "legacy-transaction".into(),
            plan_id: "legacy-plan".into(),
            target: Target::Claude,
            status: "applied".into(),
            mode: Default::default(),
            backup_path: hub.paths.backups.join("legacy-transaction"),
            git: GitSnapshot {
                head: None,
                dirty: true,
            },
            canonical_digest: "digest".into(),
            verification: Some("expected state matched".into()),
            created_at: "2026-09-27T12:00:00Z".into(),
        })
        .unwrap();
    assert!(hub.store.enabled_targets().unwrap().is_empty());
    drop(hub);

    let reopened = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
    assert_eq!(
        reopened.store.enabled_targets().unwrap(),
        vec![Target::Claude]
    );
}

#[test]
fn automatic_sync_profiles_run_after_mutation_and_skip_noop_transactions() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let selection = SyncSelection {
        skills: vec!["review".into()],
        plugins: Vec::new(),
        mcp: Vec::new(),
        rules: false,
        ..SyncSelection::default()
    };

    let first =
        transaction::sync_once(&hub.paths, &hub.store, Target::Cursor, Some(&selection)).unwrap();
    assert!(first.changed);
    assert_eq!(
        first.transaction.as_ref().unwrap().mode,
        TransactionMode::AutoSync
    );
    hub.store
        .set_auto_sync_profile(&agenthub_core::models::AutoSyncProfile {
            target: Target::Cursor,
            enabled: true,
            selection,
        })
        .unwrap();
    fs::write(
        hub.paths.skills.join("review/SKILL.md"),
        "---\nname: review\ndescription: Updated review\n---\n# Review",
    )
    .unwrap();
    let changed = transaction::run_auto_sync(&hub.paths, &hub.store).unwrap();
    assert_eq!(changed.len(), 1);
    assert!(changed[0].changed);
    assert!(changed[0].error.is_none());

    let no_change = transaction::run_auto_sync(&hub.paths, &hub.store).unwrap();
    assert_eq!(no_change.len(), 1);
    assert!(!no_change[0].changed);
    assert!(no_change[0].transaction_id.is_none());
    assert_eq!(hub.store.recent_transactions(10).unwrap().len(), 2);
}

#[test]
fn strict_authority_expands_saved_auto_scope_to_all_canonical_resources() {
    let (temp, hub) = fixture();
    seed(&hub);
    let second = hub.paths.skills.join("second");
    fs::create_dir_all(&second).unwrap();
    fs::write(second.join("SKILL.md"), "# Second").unwrap();
    hub.store
        .set_auto_sync_profile(&agenthub_core::models::AutoSyncProfile {
            target: Target::Cursor,
            enabled: true,
            selection: SyncSelection {
                skills_managed: true,
                skills: vec!["review".into()],
                ..SyncSelection::default()
            },
        })
        .unwrap();
    hub.store
        .set_policy_settings(&agenthub_core::models::PolicySettings {
            strict_authoritative: true,
            sync_after_reverse_import: false,
        })
        .unwrap();

    let outcomes = transaction::run_auto_sync(&hub.paths, &hub.store).unwrap();
    assert!(outcomes[0].changed, "{:?}", outcomes[0]);
    assert!(temp.path().join(".cursor/skills/review/SKILL.md").is_file());
    assert!(temp.path().join(".cursor/skills/second/SKILL.md").is_file());
}

#[test]
fn deleting_canonical_capability_runs_saved_auto_sync_scope() {
    let (_temp, hub) = fixture();
    seed(&hub);
    let selection = SyncSelection {
        skills: vec!["review".into()],
        plugins: Vec::new(),
        mcp: Vec::new(),
        rules: false,
        ..SyncSelection::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Cursor, Some(&selection)).unwrap();
    hub.store
        .set_auto_sync_profile(&agenthub_core::models::AutoSyncProfile {
            target: Target::Cursor,
            enabled: true,
            selection,
        })
        .unwrap();

    canonical::delete_capability(&hub.paths, CapabilityKind::Skill, "review").unwrap();
    let outcomes = transaction::run_auto_sync(&hub.paths, &hub.store).unwrap();

    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].changed);
    assert!(outcomes[0].error.is_none());
    assert!(!hub.paths.user_home.join(".cursor/skills/review").exists());
    assert!(canonical::inventory(&hub.paths)
        .unwrap()
        .iter()
        .all(|capability| capability.id != "review"));
}

#[test]
fn legacy_default_sync_mode_deserializes_as_automatic_sync() {
    let json = r#"{
        "id":"tx","plan_id":"plan","target":"codex","status":"applied",
        "mode":"default_sync","backup_path":"/tmp/backup",
        "git":{"head":null,"dirty":false},"canonical_digest":"digest",
        "verification":"ok","created_at":"2026-09-28T00:00:00Z"
    }"#;
    let transaction: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(transaction.mode, TransactionMode::AutoSync);
}
