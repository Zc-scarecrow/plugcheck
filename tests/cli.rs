use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Path to the compiled binary under test.
fn bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/plugcheck.exe");
    if !p.exists() {
        p.set_file_name("plugcheck");
    }
    p
}

fn run(args: &[&str], cwd: &std::path::Path) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run plugcheck")
}

#[test]
fn scaffold_then_check_passes() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["new", "demo-plugin", "--dir", "demo"], tmp.path());
    assert!(out.status.success(), "scaffold failed: {:?}", out);

    let plugin_dir = tmp.path().join("demo");
    let out = run(&["check"], &plugin_dir);
    assert!(out.status.success(), "check failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no problems found"),
        "unexpected output: {}",
        stdout
    );
}

#[test]
fn scaffold_rejects_invalid_name() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["new", "Bad_Name"], tmp.path());
    assert!(!out.status.success());
}

#[test]
fn check_reports_missing_plugin_json() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("plugin.json"), "output: {}", stdout);
}

#[test]
fn check_reports_mcp_version_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("plugin.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"test-plugin"}"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/2.0.0/mcp.schema.json","mcpServers":{}}"#,
    )
    .unwrap();

    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("mismatch"),
        "expected version mismatch error, got: {}",
        stdout
    );
}

#[test]
fn skill_without_frontmatter_is_isolated_warning() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("plugin.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"test-plugin"}"#,
    )
    .unwrap();
    let skill_dir = tmp.path().join("skills").join("broken");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "no frontmatter here\n").unwrap();

    let out = run(&["check"], tmp.path());
    // A broken skill must not invalidate the whole plugin (failure isolation).
    assert!(out.status.success(), "check failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("frontmatter"), "output: {}", stdout);
}

#[test]
fn json_output_is_machine_readable() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("plugin.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"test-plugin"}"#,
    )
    .unwrap();

    let out = run(&["check", "--format", "json"], tmp.path());
    assert!(out.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(parsed["tool"], "plugcheck");
    assert_eq!(parsed["valid"], true);
    assert_eq!(parsed["summary"]["errors"], 0);
}

#[test]
fn json_output_reports_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["check", "--format", "json"], tmp.path());
    assert!(!out.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(parsed["valid"], false);
    assert!(parsed["summary"]["errors"].as_u64().unwrap() >= 1);
}
