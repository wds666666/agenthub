use agenthub_core::{
    canonical, git,
    paths::AgentHubPaths,
    versions::{self, VersionAction},
    AgentHub,
};
use std::{fs, process::Command};
use tempfile::TempDir;
fn fixture() -> (TempDir, AgentHub) {
    let temp = TempDir::new().unwrap();
    let hub = AgentHub::open(AgentHubPaths::for_home(
        temp.path().join("home with spaces"),
    ))
    .unwrap();
    fs::create_dir_all(hub.paths.skills.join("one")).unwrap();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Original\n").unwrap();
    git::commit(
        &hub.paths.root,
        "original",
        Some("Tester"),
        Some("test@example.com"),
    )
    .unwrap();
    (temp, hub)
}
fn command(hub: &AgentHub, args: &[&str]) {
    assert!(Command::new("git")
        .arg("-C")
        .arg(&hub.paths.root)
        .args(args)
        .output()
        .unwrap()
        .status
        .success());
}
#[test]
fn clean_crlf_worktree_is_not_reported_as_a_capability_change() {
    let (_temp, hub) = fixture();
    command(&hub, &["config", "core.autocrlf", "true"]);
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Original\r\n").unwrap();
    command(&hub, &["add", "--renormalize", "."]);
    assert!(!git::snapshot(&hub.paths.root).unwrap().dirty);
    assert!(versions::changes(&hub.paths).unwrap().is_empty());
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Edited\r\n").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    let stage = hub.paths.root.join(format!("runtime/version-{}", plan.id));
    assert_eq!(
        fs::read_to_string(stage.join("skills/one/SKILL.md")).unwrap(),
        "# Original\n"
    );
    let result = versions::apply(&hub.paths, plan.id, "DISCARD").unwrap();
    assert!(
        !result.pending_changes,
        "{}\n{}",
        git::status(&hub.paths.root).unwrap(),
        git::diff(&hub.paths.root).unwrap()
    );
}
#[test]
fn recovery_preview_ignores_clean_crlf_skills_but_lists_staged_mcp_and_ignored_files() {
    let (_temp, hub) = fixture();
    command(&hub, &["config", "core.autocrlf", "true"]);
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Original\r\n").unwrap();
    for id in [
        "chrome-devtools",
        "context7",
        "esp-component-registry",
        "espressif-docs",
        "node_repl",
    ] {
        fs::create_dir_all(hub.paths.mcp.join(id)).unwrap();
        fs::write(hub.paths.mcp.join(id).join("server.json"), format!(r#"{{"schemaVersion":1,"id":"{id}","display_name":"{id}","transport":"stdio","command":"node","args":[],"env":{{}},"headers":{{}}}}"#)).unwrap();
    }
    command(&hub, &["add", "mcp"]);
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    assert_eq!(plan.changes.len(), 5);
    assert!(plan.changes.iter().all(|change| change.kind
        == Some(agenthub_core::models::CapabilityKind::Mcp)
        && change.action == "delete"));
    // Actual edits remain visible even alongside unchanged CRLF files.
    fs::write(hub.paths.skills.join("one/script.py"), "print('new')\r\n").unwrap();
    fs::write(hub.paths.skills.join("one/.gitignore"), "local.txt\n").unwrap();
    fs::write(hub.paths.skills.join("one/local.txt"), "local content\n").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    let skill = plan
        .changes
        .iter()
        .find(|change| change.id == "one")
        .unwrap();
    assert_eq!(skill.files.len(), 3);
    assert!(!skill.files.iter().any(|path| path.ends_with("SKILL.md")));
    assert!(skill.files.iter().any(|path| path.ends_with("local.txt")));
}

#[test]
fn recovery_preview_respects_binary_git_attributes_and_keeps_raw_stale_checks() {
    let (_temp, hub) = fixture();
    command(&hub, &["config", "core.autocrlf", "true"]);
    fs::write(
        hub.paths.skills.join("one/.gitattributes"),
        "asset.bin -text\n",
    )
    .unwrap();
    fs::write(hub.paths.skills.join("one/asset.bin"), b"binary\0\n").unwrap();
    git::commit(
        &hub.paths.root,
        "binary",
        Some("Tester"),
        Some("test@example.com"),
    )
    .unwrap();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Original\r\n").unwrap();
    fs::write(hub.paths.skills.join("one/asset.bin"), b"binary\0\r\n").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].files, ["skills/one/asset.bin"]);
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Original\n").unwrap();
    // Even an equivalent line-ending-only concurrent edit invalidates Apply.
    assert!(versions::apply(&hub.paths, plan.id, "DISCARD").is_err());
}

#[test]
fn capability_changes_include_staged_unstaged_new_and_deleted_files() {
    let (_temp, hub) = fixture();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Staged\n").unwrap();
    command(&hub, &["add", "skills/one"]);
    fs::write(hub.paths.skills.join("one/script.py"), "print('new')\n").unwrap();
    fs::create_dir_all(hub.paths.skills.join("two")).unwrap();
    fs::write(hub.paths.skills.join("two/SKILL.md"), "# Two\n").unwrap();
    let changes = versions::changes(&hub.paths).unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(
        changes.iter().find(|c| c.id == "one").unwrap().files.len(),
        2
    );
    assert_eq!(
        changes.iter().find(|c| c.id == "two").unwrap().action,
        "create"
    );
    assert!(git::diff(&hub.paths.root).unwrap().contains("Staged"));
    fs::remove_dir_all(hub.paths.skills.join("one")).unwrap();
    assert_eq!(
        versions::changes(&hub.paths)
            .unwrap()
            .iter()
            .find(|c| c.id == "one")
            .unwrap()
            .action,
        "delete"
    );
}
#[test]
fn discard_backs_up_all_edits_and_index_without_touching_hosts_state_or_history() {
    let (_temp, hub) = fixture();
    let original = git::snapshot(&hub.paths.root).unwrap().head;
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Edited\n").unwrap();
    command(&hub, &["add", "skills"]);
    let index = fs::read(hub.paths.root.join(".git/index")).unwrap();
    fs::create_dir_all(hub.paths.skills.join("new")).unwrap();
    fs::write(hub.paths.skills.join("new/SKILL.md"), "# New\n").unwrap();
    let host = hub.paths.user_home.join(".agents/skills/one");
    fs::create_dir_all(&host).unwrap();
    fs::write(host.join("SKILL.md"), "# Host\n").unwrap();
    hub.store.set_meta("device_fixture", "untouched").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    assert!(versions::apply(&hub.paths, plan.id, "REMOTE").is_err());
    assert!(git::snapshot(&hub.paths.root).unwrap().dirty);
    let result = versions::apply(&hub.paths, plan.id, "DISCARD").unwrap();
    assert!(!result.pending_changes);
    assert!(!hub.paths.skills.join("new").exists());
    assert_eq!(
        fs::read_to_string(hub.paths.skills.join("one/SKILL.md")).unwrap(),
        "# Original\n"
    );
    assert_eq!(
        fs::read_to_string(host.join("SKILL.md")).unwrap(),
        "# Host\n"
    );
    assert_eq!(
        hub.store.meta("device_fixture").unwrap().as_deref(),
        Some("untouched")
    );
    assert_eq!(git::snapshot(&hub.paths.root).unwrap().head, original);
    let backup = std::path::Path::new(&result.backup_path);
    assert_eq!(fs::read(backup.join("index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(backup.join("skills/new/SKILL.md")).unwrap(),
        "# New\n"
    );
}
#[test]
fn discard_rejects_stale_local_or_candidate_content_before_writing() {
    let (_temp, hub) = fixture();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Edited\n").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Newer\n").unwrap();
    assert!(versions::apply(&hub.paths, plan.id, "DISCARD").is_err());
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    fs::write(
        hub.paths
            .root
            .join(format!("runtime/version-{}/skills/one/SKILL.md", plan.id)),
        "# Tampered\n",
    )
    .unwrap();
    assert!(versions::apply(&hub.paths, plan.id, "DISCARD").is_err());
    assert_eq!(
        fs::read_to_string(hub.paths.skills.join("one/SKILL.md")).unwrap(),
        "# Newer\n"
    );
}
#[test]
fn recovery_rolls_back_interrupted_activation_on_open() {
    let (_temp, hub) = fixture();
    fs::write(hub.paths.skills.join("one/SKILL.md"), "# Recover me\n").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    versions::apply(&hub.paths, plan.id, "DISCARD").unwrap();
    fs::write(
        hub.paths.root.join("runtime/version-recovery.json"),
        serde_json::to_vec(&serde_json::json!({"id":plan.id})).unwrap(),
    )
    .unwrap();
    let paths = hub.paths.clone();
    drop(hub);
    let reopened = AgentHub::open(paths).unwrap();
    assert_eq!(
        fs::read_to_string(reopened.paths.skills.join("one/SKILL.md")).unwrap(),
        "# Recover me\n"
    );
    assert!(git::snapshot(&reopened.paths.root).unwrap().dirty);
    canonical::validate(&reopened.paths).unwrap();
}
#[test]
fn discard_requires_saved_version_but_invalid_pending_config_can_be_recovered() {
    let temp = TempDir::new().unwrap();
    let hub = AgentHub::open(AgentHubPaths::for_home(temp.path())).unwrap();
    assert!(versions::preview(&hub.paths, VersionAction::Discard).is_err());
    git::commit(
        &hub.paths.root,
        "empty",
        Some("Tester"),
        Some("test@example.com"),
    )
    .unwrap();
    fs::write(hub.paths.root.join("agenthub.toml"), "broken {").unwrap();
    let plan = versions::preview(&hub.paths, VersionAction::Discard).unwrap();
    versions::apply(&hub.paths, plan.id, "DISCARD").unwrap();
    canonical::validate(&hub.paths).unwrap();
}
