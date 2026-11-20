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

// ---------------------------------------------------------------------------
// Additional edge-case scenarios
// ---------------------------------------------------------------------------

fn write_valid_plugin(dir: &std::path::Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("plugin.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"test-plugin"}"#,
    )
    .unwrap();
}

#[test]
fn new_into_existing_directory_fails() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join("demo")).unwrap();
    let out = run(&["new", "demo", "--dir", "demo"], tmp.path());
    assert!(!out.status.success());
}

#[test]
fn check_non_directory_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("not-a-dir");
    fs::write(&file, "hi").unwrap();
    let out = run(&["check", "not-a-dir"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("not a directory"), "output: {}", stdout);
}

#[test]
fn malformed_plugin_json_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("plugin.json"), "{ this is not json").unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("invalid JSON"), "output: {}", stdout);
}

#[test]
fn malformed_mcp_json_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(tmp.path().join("mcp.json"), "not json").unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("invalid JSON"), "output: {}", stdout);
}

#[test]
fn empty_skills_dir_passes() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::create_dir(tmp.path().join("skills")).unwrap();
    let out = run(&["check"], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("0 skills checked"), "output: {}", stdout);
}

#[test]
fn subdir_without_skill_md_is_warning() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::create_dir_all(tmp.path().join("skills").join("no-skill")).unwrap();
    let out = run(&["check"], tmp.path());
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("does not contain SKILL.md"),
        "output: {}",
        stdout
    );
}

#[test]
fn regular_files_in_skills_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    let skills = tmp.path().join("skills");
    fs::create_dir(&skills).unwrap();
    fs::write(skills.join("README.txt"), "ignore me").unwrap();
    let out = run(&["check"], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
}

#[test]
fn dotted_name_is_valid() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["new", "acme.analytics", "--dir", "dot"], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
    let out = run(&["check"], &tmp.path().join("dot"));
    assert!(out.status.success());
}

#[test]
fn repeated_checks_are_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::create_dir_all(tmp.path().join("skills").join("ok")).unwrap();
    fs::write(
        tmp.path().join("skills").join("ok").join("SKILL.md"),
        "---\nname: ok\ndescription: fine\n---\n",
    )
    .unwrap();
    let first = run(&["check"], tmp.path());
    let second = run(&["check"], tmp.path());
    assert_eq!(first.status.code(), second.status.code());
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn json_output_marks_warning_only_plugin_valid() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    let out = run(&["check", "--format", "json"], tmp.path());
    assert!(out.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON");
    assert_eq!(parsed["valid"], true);
    assert!(parsed["summary"]["warnings"].as_u64().unwrap() >= 1);
}

#[test]
fn sse_server_missing_url_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"sse"}}}"#,
    )
    .unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("url"), "output: {}", stdout);
}

#[test]
fn unknown_transport_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"websocket","url":"ws://x"}}}"#,
    )
    .unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("unknown transport"), "output: {}", stdout);
}

#[test]
fn mcp_servers_array_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":[1]}"#,
    )
    .unwrap();
    let out = run(&["check"], tmp.path());
    assert!(!out.status.success());
}

#[test]
fn stress_many_skills() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    let skills = tmp.path().join("skills");
    fs::create_dir(&skills).unwrap();
    let n = 300;
    for i in 0..n {
        let dir = skills.join(format!("skill-{}", i));
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: skill-{}\ndescription: s\n---\n", i),
        )
        .unwrap();
    }
    let out = run(&["check"], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("300 skills checked"), "output: {}", stdout);
}

#[test]
fn stdio_server_with_command_and_args_passes() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"s":{"type":"stdio","command":"./bin/s","args":["--data","${PLUGIN_DATA}/s"]}}}"#,
    )
    .unwrap();
    let out = run(&["check"], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
}

#[test]
fn verbose_reports_missing_mcp_info() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    let quiet = run(&["check"], tmp.path());
    let loud = run(&["check", "--verbose"], tmp.path());
    let q = String::from_utf8_lossy(&quiet.stdout);
    let l = String::from_utf8_lossy(&loud.stdout);
    assert!(!q.contains("info: mcp.json"), "{}", q);
    assert!(l.contains("info: mcp.json"), "{}", l);
    assert!(loud.status.success());
}

#[test]
fn json_verbose_includes_info_diagnostics() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    let out = run(&["check", "--format", "json", "--verbose"], tmp.path());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["diagnostics"][0]["severity"], "info");
}

#[test]
fn non_target_schema_version_is_warning() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("plugin.json"),
        r#"{"$schema":"https://agent-plugins.org/schemas/2.0.0/plugin.schema.json","name":"demo"}"#,
    )
    .unwrap();
    let out = run(&["check", "--format", "json"], tmp.path());
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("schema version")
    );
}

#[test]
fn pack_creates_zip_and_excludes_git() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    fs::write(tmp.path().join(".git/config"), "secret").unwrap();
    let zip = tmp.path().join("demo.zip");
    let out = run(&["pack", "--output", zip.to_str().unwrap()], tmp.path());
    assert!(out.status.success(), "output: {:?}", out);
    let bytes = fs::read(&zip).unwrap();
    assert_eq!(&bytes[0..4], &0x0403_4b50u32.to_le_bytes());
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("plugin.json"));
    assert!(!text.contains(".git/config"));
}

#[test]
fn new_with_template_substitutes_name() {
    let tmp = tempfile::tempdir().unwrap();
    let tpl = tmp.path().join("tpl");
    fs::create_dir(&tpl).unwrap();
    fs::write(tpl.join("plugin.json"), r#"{"name":"{{name}}"}"#).unwrap();
    let out = run(
        &[
            "new",
            "demo",
            "--dir",
            "demo",
            "--template",
            tpl.to_str().unwrap(),
        ],
        tmp.path(),
    );
    assert!(out.status.success(), "output: {:?}", out);
    let content = fs::read_to_string(tmp.path().join("demo/plugin.json")).unwrap();
    assert_eq!(content, r#"{"name":"demo"}"#);
}

#[test]
fn compat_reports_transports() {
    let tmp = tempfile::tempdir().unwrap();
    write_valid_plugin(tmp.path());
    fs::write(
        tmp.path().join("mcp.json"),
        r#"{"mcpServers":{"s":{"type":"stdio"}}}"#,
    )
    .unwrap();
    let out = run(&["compat"], tmp.path());
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("stdio"), "{}", stdout);
}
