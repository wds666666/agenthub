use agenthub_core::{
    canonical, comparison, host,
    models::{
        AutoSyncProfile, CapabilityKind, HostResourceRelation, RuleDocument, SyncMode,
        SyncSelection, Target,
    },
    paths::AgentHubPaths,
    planner, scanner, transaction, AgentHub,
};
use std::fs;
use tempfile::TempDir;
fn fixture() -> (TempDir, AgentHub) {
    let temp = TempDir::new().unwrap();
    let hub = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
    (temp, hub)
}
fn skill(root: &std::path::Path, name: &str, text: &str) {
    fs::create_dir_all(root.join(name)).unwrap();
    fs::write(root.join(name).join("SKILL.md"), text).unwrap();
}
fn rule(hub: &AgentHub, id: &str, body: &str) {
    canonical::save_rule(
        &hub.paths,
        &RuleDocument {
            schema_version: 1,
            id: id.into(),
            display_name: id.into(),
            activation: "always".into(),
            paths: vec![],
            targets: Target::HOSTS.to_vec(),
            body: body.into(),
        },
        true,
    )
    .unwrap();
}
fn mcp(hub: &AgentHub) {
    fs::create_dir_all(hub.paths.mcp.join("server")).unwrap();
    fs::write(hub.paths.mcp.join("server/server.json"),r#"{"schemaVersion":1,"id":"server","display_name":"Server","transport":"stdio","command":"node","args":[],"env":{"MODE":"local"},"headers":{}}"#).unwrap();
}
#[test]
fn preserve_and_replace_never_expand_selected_categories() {
    let (_temp, hub) = fixture();
    skill(&hub.paths.skills, "selected", "# Library");
    skill(&hub.paths.skills, "not-selected", "# Other library");
    for target in Target::ALL {
        let root = hub
            .paths
            .user_home
            .join(format!(".{}/skills", target.as_str()));
        skill(&root, "selected", "# Old");
        skill(&root, "host-only", "# Own");
        let scope = SyncSelection {
            skills_managed: true,
            skills: vec!["selected".into()],
            ..Default::default()
        };
        let run = transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        assert!(run.changed);
        assert_eq!(
            fs::read_to_string(root.join("selected/SKILL.md")).unwrap(),
            "# Library"
        );
        assert!(root.join("host-only/SKILL.md").exists());
        assert!(!root.join("not-selected").exists());
        assert!(scanner::scan_global(&hub.paths, &[target])
            .unwrap()
            .iter()
            .filter(|i| i.kind == CapabilityKind::Skill && i.path.ends_with("selected"))
            .all(|i| i.comparison_digest
                == canonical::inventory(&hub.paths)
                    .unwrap()
                    .iter()
                    .find(|c| c.id == "selected")
                    .unwrap()
                    .comparison_digest));
        let scope = SyncSelection {
            mode: SyncMode::Replace,
            ..scope
        };
        let plan = planner::create_with_selection(&hub.paths, target, Some(&scope)).unwrap();
        assert!(plan
            .steps
            .iter()
            .any(|s| s.action == agenthub_core::models::PlanAction::Delete));
        transaction::apply(&hub.paths, &hub.store, &plan).unwrap();
        assert!(!root.join("host-only").exists());
        assert!(!root.join("not-selected").exists());
    }
}
#[test]
fn mcp_round_trip_uses_content_identity_and_preserves_unselected_servers() {
    let (_temp, hub) = fixture();
    mcp(&hub);
    let canonical = canonical::inventory(&hub.paths).unwrap()[0]
        .comparison_digest
        .clone();
    for target in Target::HOSTS {
        let path = hub.paths.user_home.join(match target {
            Target::Codex => ".codex/config.toml",
            Target::Cursor => ".cursor/mcp.json",
            _ => ".claude.json",
        });
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            if target == Target::Codex {
                "theme = 'keep'\n[mcp_servers.own]\ncommand='own'\n"
            } else {
                r#"{"theme":"keep","mcpServers":{"own":{"command":"own"}}}"#
            },
        )
        .unwrap();
        let scope = SyncSelection {
            mcp_managed: true,
            mcp: vec!["server".into()],
            ..Default::default()
        };
        transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        let mut items = scanner::scan_global(&hub.paths, &[target]).unwrap();
        let item = items
            .iter_mut()
            .find(|i| i.source_key.as_deref() == Some("server:server"))
            .unwrap();
        assert_eq!(item.comparison_digest, canonical);
        item.selected = true;
        assert!(canonical::import_scan_items_atomic(&hub.paths, &items)
            .unwrap()
            .is_empty());
        assert_eq!(canonical::inventory(&hub.paths).unwrap().len(), 1);
        assert!(fs::read_to_string(&path).unwrap().contains("own"));
        assert!(host::inventory(&hub.paths, target)
            .unwrap()
            .iter()
            .any(|r| r.display_name == "server"
                && r.relation == HostResourceRelation::CanonicalMatch));
        let scope = SyncSelection {
            mode: SyncMode::Replace,
            ..scope
        };
        let plan = planner::create_with_selection(&hub.paths, target, Some(&scope)).unwrap();
        assert!(plan
            .removed_resources
            .iter()
            .any(|item| item.kind == CapabilityKind::Mcp && item.id == "own"));
        transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        assert!(!fs::read_to_string(&path).unwrap().contains("own"));
        assert!(fs::read_to_string(&path).unwrap().contains("theme"));
    }
}
#[test]
fn invalid_mcp_configuration_cannot_be_overwritten() {
    let (_temp, hub) = fixture();
    mcp(&hub);
    for target in Target::HOSTS {
        let path = hub.paths.user_home.join(match target {
            Target::Codex => ".codex/config.toml",
            Target::Cursor => ".cursor/mcp.json",
            _ => ".claude.json",
        });
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "malformed config {").unwrap();
        let scope = SyncSelection {
            mcp: vec!["server".into()],
            ..Default::default()
        };
        assert!(transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "malformed config {");
        // An invalid unmanaged MCP file must not prevent a Skills-only sync.
        skill(&hub.paths.skills, "selected", "# Library");
        let scope = SyncSelection {
            skills: vec!["selected".into()],
            ..Default::default()
        };
        transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        fs::remove_dir_all(hub.paths.skills.join("selected")).unwrap();
    }
}
#[test]
fn rules_are_selected_individually_and_generated_wrappers_do_not_reverse_import() {
    let (_temp, hub) = fixture();
    rule(&hub, "selected", "# Rule");
    rule(&hub, "unselected", "# Leave out");
    for target in Target::HOSTS {
        let scope = SyncSelection {
            rules_managed: true,
            rule_ids: vec!["selected".into()],
            ..Default::default()
        };
        if target == Target::Codex {
            let path = hub.paths.user_home.join(".codex/AGENTS.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "# User rules\nKeep me\n").unwrap();
        }
        transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        let mut items = scanner::scan_global(&hub.paths, &[target]).unwrap();
        assert!(!items.iter().any(|i| i.kind == CapabilityKind::Plugin));
        assert_eq!(
            items
                .iter()
                .filter(|i| i.kind == CapabilityKind::Rule && i.importable)
                .count(),
            1
        );
        for item in &mut items {
            item.selected = item.importable;
        }
        assert!(canonical::import_scan_items_atomic(&hub.paths, &items)
            .unwrap()
            .is_empty());
        if target == Target::Codex {
            assert!(
                fs::read_to_string(hub.paths.user_home.join(".codex/AGENTS.md"))
                    .unwrap()
                    .contains("Keep me")
            );
        }
    }
}
#[test]
fn creating_rules_never_writes_hosts_or_expands_auto_scope() {
    let (_temp, hub) = fixture();
    rule(&hub, "selected", "# Rule");
    hub.store
        .set_auto_sync_profile(&AutoSyncProfile {
            target: Target::Codex,
            enabled: true,
            needs_review: false,
            selection: SyncSelection {
                rule_ids: vec!["selected".into()],
                rules_managed: true,
                ..Default::default()
            },
        })
        .unwrap();
    let draft = RuleDocument {
        schema_version: 1,
        id: "new-rule".into(),
        display_name: "New".into(),
        activation: "always".into(),
        paths: vec![],
        targets: Target::HOSTS.to_vec(),
        body: "# New".into(),
    };
    let saved = transaction::save_rule(&hub.paths, &hub.store, &draft, true).unwrap();
    assert!(saved.auto_sync.is_empty());
    assert!(!hub.paths.user_home.join(".codex/AGENTS.md").exists());
    transaction::run_auto_sync(&hub.paths, &hub.store).unwrap();
    let body = fs::read_to_string(hub.paths.user_home.join(".codex/AGENTS.md")).unwrap();
    assert!(body.contains("# Rule"));
    assert!(!body.contains("# New"));
}
#[test]
fn legacy_auto_profiles_pause_and_snapshot_existing_rule_ids() {
    let (_temp, hub) = fixture();
    rule(&hub, "one", "One");
    hub.store.set_meta("auto_sync_profile_codex",r#"{"target":"codex","enabled":true,"selection":{"skills":[],"mcp":[],"plugins":[],"rules":true,"authoritative":true}}"#).unwrap();
    let paths = hub.paths.clone();
    drop(hub);
    let hub = AgentHub::open(paths).unwrap();
    let profile = hub
        .store
        .auto_sync_profiles()
        .unwrap()
        .into_iter()
        .find(|p| p.target == Target::Codex)
        .unwrap();
    assert!(!profile.enabled);
    assert!(profile.needs_review);
    assert_eq!(profile.selection.rule_ids, vec!["one"]);
    assert!(!profile.selection.authoritative);
    rule(&hub, "two", "Two");
    assert_eq!(
        hub.store
            .auto_sync_profiles()
            .unwrap()
            .into_iter()
            .find(|p| p.target == Target::Codex)
            .unwrap()
            .selection
            .rule_ids,
        vec!["one"]
    );
    assert!(transaction::run_auto_sync(&hub.paths, &hub.store)
        .unwrap()
        .is_empty());
}
#[test]
fn plugins_compare_complete_payload_and_import_once_across_sources() {
    let (_temp, hub) = fixture();
    for source in ["cursor", "claude"] {
        let root = hub.paths.user_home.join(format!(
            ".{source}/plugins/{}",
            if source == "cursor" {
                "local/example"
            } else {
                "example"
            }
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("plugin.json"),
            r#"{"name":"example","version":"1","description":"Complete plugin"}"#,
        )
        .unwrap();
        fs::write(root.join("script.js"), "export const complete = true").unwrap();
    }
    let mut items = scanner::scan_global(&hub.paths, &Target::HOSTS).unwrap();
    for item in &mut items {
        item.selected = true;
    }
    assert_eq!(
        canonical::import_scan_items_atomic(&hub.paths, &items)
            .unwrap()
            .len(),
        1
    );
    let mut items = scanner::scan_global(&hub.paths, &Target::HOSTS).unwrap();
    let cap = canonical::inventory(&hub.paths).unwrap().remove(0);
    assert!(items
        .iter()
        .all(|i| i.comparison_digest == cap.comparison_digest));
    for item in &mut items {
        item.selected = true;
    }
    assert!(canonical::import_scan_items_atomic(&hub.paths, &items)
        .unwrap()
        .is_empty());
    assert!(cap.path.join("payload/script.js").exists());
    assert_eq!(
        comparison::portable_tree(&cap.path.join("payload")).unwrap(),
        cap.comparison_digest
    );
}
#[test]
fn same_named_modified_mcp_is_never_labelled_identical() {
    let (_temp, hub) = fixture();
    mcp(&hub);
    let path = hub.paths.user_home.join(".cursor/mcp.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, r#"{"mcpServers":{"server":{"command":"different"}}}"#).unwrap();
    assert_eq!(
        host::inventory(&hub.paths, Target::Cursor).unwrap()[0].relation,
        HostResourceRelation::Modified
    );
}

#[test]
fn rule_updates_are_idempotent_and_preserve_user_text_and_unselected_blocks() {
    let (_temp, hub) = fixture();
    rule(&hub, "selected", "Selected");
    rule(&hub, "other", "Other");
    let path = hub.paths.user_home.join(".codex/AGENTS.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "# User\nKeep me exactly.\n").unwrap();
    let all = SyncSelection {
        rules_managed: true,
        rule_ids: vec!["selected".into(), "other".into()],
        ..Default::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&all)).unwrap();
    let scope = SyncSelection {
        rules_managed: true,
        rule_ids: vec!["selected".into()],
        ..Default::default()
    };
    let first = fs::read(&path).unwrap();
    assert!(
        !transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&scope))
            .unwrap()
            .changed
    );
    assert_eq!(fs::read(&path).unwrap(), first);
    let mut rule = canonical::read_rule(&hub.paths, "selected").unwrap();
    rule.body = "Updated".into();
    canonical::save_rule(&hub.paths, &rule, false).unwrap();
    transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&scope)).unwrap();
    let body = fs::read_to_string(&path).unwrap();
    assert!(body.starts_with("# User\nKeep me exactly.\n"));
    assert!(body.contains("Other"));
    assert!(body.contains("Updated"));
    assert!(
        !transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&scope))
            .unwrap()
            .changed
    );
}
#[test]
fn replacement_preview_names_removed_rule_blocks_and_user_text() {
    let (_temp, hub) = fixture();
    rule(&hub, "keep", "Keep");
    rule(&hub, "remove", "Remove");
    let path = hub.paths.user_home.join(".codex/AGENTS.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "# User text\n").unwrap();
    let scope = SyncSelection {
        rule_ids: vec!["keep".into(), "remove".into()],
        ..Default::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&scope)).unwrap();
    let scope = SyncSelection {
        mode: SyncMode::Replace,
        rule_ids: vec!["keep".into()],
        ..Default::default()
    };
    let plan = planner::create_with_selection(&hub.paths, Target::Codex, Some(&scope)).unwrap();
    assert!(plan
        .removed_resources
        .iter()
        .any(|item| item.id == "remove"));
    assert!(plan
        .removed_resources
        .iter()
        .any(|item| item.id == "__user_text__"));
}
#[test]
fn rule_activation_and_paths_participate_in_comparison() {
    let (_temp, hub) = fixture();
    rule(&hub, "conditional", "Same body");
    let mut doc = canonical::read_rule(&hub.paths, "conditional").unwrap();
    doc.activation = "paths".into();
    doc.paths = vec!["src/**".into()];
    canonical::save_rule(&hub.paths, &doc, false).unwrap();
    let scope = SyncSelection {
        rule_ids: vec!["conditional".into()],
        ..Default::default()
    };
    assert!(planner::create_with_selection(&hub.paths, Target::Codex, Some(&scope)).is_err());
    for target in [Target::Cursor, Target::Claude] {
        transaction::sync_once(&hub.paths, &hub.store, target, Some(&scope)).unwrap();
        let mut items = scanner::scan_global(&hub.paths, &[target]).unwrap();
        for item in &mut items {
            item.selected = item.importable;
        }
        assert!(canonical::import_scan_items_atomic(&hub.paths, &items)
            .unwrap()
            .is_empty());
        let cap = canonical::inventory(&hub.paths).unwrap()[0].clone();
        assert!(items
            .iter()
            .filter(|i| i.kind == CapabilityKind::Rule)
            .all(|i| i.comparison_digest == cap.comparison_digest));
    }
    let path = hub
        .paths
        .user_home
        .join(".cursor/plugins/local/agenthub-rules/rules/conditional.mdc");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("globs: [\"src/**\"]", "globs: [\"other/**\"]");
    fs::write(&path, text).unwrap();
    let resources = host::inventory(&hub.paths, Target::Cursor).unwrap();
    assert!(resources
        .iter()
        .any(|r| r.kind == CapabilityKind::Rule && r.relation == HostResourceRelation::Modified));
    assert!(resources
        .iter()
        .any(|r| r.relation == HostResourceRelation::Generated && !r.deletable));
}
#[test]
fn malformed_generated_blocks_block_writes_and_stale_plans_are_rejected() {
    let (_temp, hub) = fixture();
    rule(&hub, "selected", "Rule");
    let path = hub.paths.user_home.join(".codex/AGENTS.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "<!-- agenthub:rule selected --> missing closing marker",
    )
    .unwrap();
    let scope = SyncSelection {
        rule_ids: vec!["selected".into()],
        ..Default::default()
    };
    assert!(planner::create_with_selection(&hub.paths, Target::Codex, Some(&scope)).is_err());
    fs::write(&path, "# User").unwrap();
    let plan = planner::create_with_selection(&hub.paths, Target::Codex, Some(&scope)).unwrap();
    fs::write(&path, "# Concurrent edit").unwrap();
    assert!(transaction::apply(&hub.paths, &hub.store, &plan).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "# Concurrent edit");
    assert!(hub.store.recent_transactions(10).unwrap().is_empty());
}

#[test]
fn cursor_preserve_rules_keeps_unselected_rules_and_protected_skills_survive_replace() {
    let (_temp, hub) = fixture();
    rule(&hub, "one", "One");
    rule(&hub, "two", "Two");
    let all = SyncSelection {
        rule_ids: vec!["one".into(), "two".into()],
        ..Default::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Cursor, Some(&all)).unwrap();
    let path = hub
        .paths
        .user_home
        .join(".cursor/plugins/local/agenthub-rules/rules");
    fs::write(path.join("own.mdc"), "Own rule").unwrap();
    let scope = SyncSelection {
        rule_ids: vec!["one".into()],
        ..Default::default()
    };
    assert!(
        !transaction::sync_once(&hub.paths, &hub.store, Target::Cursor, Some(&scope))
            .unwrap()
            .changed
    );
    assert!(path.join("two.mdc").exists());
    assert_eq!(
        fs::read_to_string(path.join("own.mdc")).unwrap(),
        "Own rule"
    );
    let codex = hub.paths.user_home.join(".codex/skills");
    skill(&codex, ".system/vendor", "# Vendor");
    skill(&codex, "own", "# Own");
    let clear = SyncSelection {
        skills_managed: true,
        mode: SyncMode::Replace,
        ..Default::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Codex, Some(&clear)).unwrap();
    assert!(codex.join(".system/vendor/SKILL.md").exists());
    assert!(!codex.join("own").exists());
}
#[cfg(unix)]
#[test]
fn preserve_keeps_broken_unselected_links_and_blocks_linked_ancestors() {
    let (_temp, hub) = fixture();
    skill(&hub.paths.skills, "one", "One");
    let root = hub.paths.user_home.join(".agents/skills");
    fs::create_dir_all(&root).unwrap();
    std::os::unix::fs::symlink("/missing-user-resource", root.join("own-link")).unwrap();
    let scope = SyncSelection {
        skills: vec!["one".into()],
        ..Default::default()
    };
    transaction::sync_once(&hub.paths, &hub.store, Target::Agents, Some(&scope)).unwrap();
    assert_eq!(
        fs::read_link(root.join("own-link")).unwrap(),
        std::path::PathBuf::from("/missing-user-resource")
    );
    let outside = TempDir::new().unwrap();
    std::os::unix::fs::symlink(outside.path(), hub.paths.user_home.join(".cursor")).unwrap();
    assert!(planner::create_with_selection(&hub.paths, Target::Cursor, Some(&scope)).is_err());
    assert!(!outside.path().join("skills").exists());
}

#[test]
fn import_rejects_changed_source_and_reports_existing_duplicates_without_deleting() {
    let (_temp, hub) = fixture();
    let host = hub.paths.user_home.join(".agents/skills");
    skill(&host, "changed", "# Before scan");
    let mut items = scanner::scan_global(&hub.paths, &[Target::Agents]).unwrap();
    items[0].selected = true;
    skill(&host, "changed", "# Changed after scan");
    assert!(canonical::import_scan_items_atomic(&hub.paths, &items).is_err());
    assert!(canonical::inventory(&hub.paths).unwrap().is_empty());
    skill(&hub.paths.skills, "first", "# Same portable content");
    skill(&hub.paths.skills, "second", "# Same portable content");
    let inventory = canonical::inventory(&hub.paths).unwrap();
    assert_eq!(inventory.len(), 2);
    assert_eq!(
        inventory
            .iter()
            .filter(|item| item.duplicate_of.is_some())
            .count(),
        1
    );
    assert!(hub.paths.skills.join("second/SKILL.md").exists());
}
