use std::{fs, process::Command};
use tempfile::TempDir;

fn cli(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agenthub"))
        .env("AGENTHUB_HOME", root)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn initialized_library_cannot_be_reimported_and_validation_reports_broken_content() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("library");
    assert!(cli(&root, &["init", "--empty"]).status.success());
    assert!(!cli(&root, &["init", "--import-all"]).status.success());
    let valid = cli(&root, &["validate", "--json"]);
    assert!(valid.status.success());
    let report: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(report["valid"], true);
    let skill = root.join("skills/broken");
    fs::create_dir_all(&skill).unwrap();
    let invalid = cli(&root, &["validate", "--json"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("broken"));
}

#[cfg(unix)]
#[test]
fn selection_limits_plan_to_reviewed_skills_without_host_writes() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("library");
    assert!(cli(&root, &["init", "--empty"]).status.success());
    for id in ["one", "two"] {
        let skill = root.join("skills").join(id);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), format!("# {id}")).unwrap();
    }
    let selection = temporary.path().join("scope.json");
    fs::write(&selection, r#"{"skills":["one"]}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_agenthub"))
        .env("AGENTHUB_HOME", &root)
        .env("HOME", temporary.path())
        .args(["plan", "cursor", "--selection"])
        .arg(&selection)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["selection"]["skills"], serde_json::json!(["one"]));
    assert!(plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|step| { step["capability_id"] != "two" && step["capability_kind"] != "mcp" }));
    assert!(!temporary.path().join(".cursor").exists());
    fs::write(&selection, r#"{"skill":["one"]}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_agenthub"))
        .env("AGENTHUB_HOME", &root)
        .env("HOME", temporary.path())
        .args(["plan", "cursor", "--selection"])
        .arg(selection)
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn selective_version_cli_review_apply_and_untracked_diff() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("library");
    assert!(cli(&root, &["init", "--empty"]).status.success());
    for id in ["one", "two"] {
        let p = root.join("skills").join(id);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), format!("# {id}\nnew reviewed body\n")).unwrap();
    }
    let diff = cli(
        &root,
        &[
            "git",
            "diff",
            "--capability",
            "skill:one",
            "--include-untracked",
        ],
    );
    assert!(diff.status.success());
    let text = String::from_utf8_lossy(&diff.stdout);
    assert!(text.contains("new reviewed body"));
    assert!(!text.contains("skills/two"));
    let preview = cli(
        &root,
        &[
            "version",
            "plan",
            "--message",
            "Only one",
            "--only",
            "skill:one",
            "--name",
            "Fixture",
            "--email",
            "fixture@example.com",
            "--no-host-sync",
            "--json",
        ],
    );
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    let id = plan["id"].as_str().unwrap();
    assert_eq!(plan["excluded_pending_changes"][0]["id"], "two");
    assert_eq!(plan["options"]["push"], false);
    assert!(!cli(&root, &["version", "apply", id]).status.success());
    let saved = cli(&root, &["version", "apply", id, "--confirm"]);
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&saved.stdout).unwrap();
    assert_eq!(result["auto_sync"], serde_json::json!([]));
    assert_eq!(result["commit_hash"].as_str().unwrap().len(), 40);
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .unwrap();
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(tree.contains("skills/one/SKILL.md"));
    assert!(!tree.contains("skills/two/SKILL.md"));
    assert!(root.join("skills/two/SKILL.md").exists());
}
