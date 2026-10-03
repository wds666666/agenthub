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
