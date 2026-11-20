use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Self-declared compatibility overview derived from a plugin manifest.
///
/// No external client database is bundled: the report shows which clients a
/// plugin targets via `extensions.<reverse-domain>` and which MCP transports
/// it actually uses, then flags inconsistent declarations.
#[derive(Debug, Serialize)]
pub struct CompatReport {
    pub tool: &'static str,
    pub clients: Vec<String>,
    pub transports: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub message: String,
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|_| {
        format!(
            "cannot read {}",
            path.file_name().unwrap().to_string_lossy()
        )
    })?;
    serde_json::from_str(&text).map_err(|e| {
        format!(
            "invalid JSON in {}: {}",
            path.file_name().unwrap().to_string_lossy(),
            e
        )
    })
}

pub fn check_compat(root: &Path) -> Result<CompatReport, String> {
    let plugin = read_json(&root.join("plugin.json"))?;
    let mut clients: Vec<String> = plugin
        .get("extensions")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    clients.sort();

    let mcp_path = root.join("mcp.json");
    let mut transports: Vec<String> = Vec::new();
    if mcp_path.exists() {
        let mcp = read_json(&mcp_path)?;
        if let Some(servers) = mcp.get("mcpServers").and_then(Value::as_object) {
            for spec in servers.values() {
                if let Some(kind) = spec.get("type").and_then(Value::as_str) {
                    if !transports.iter().any(|t| t == kind) {
                        transports.push(kind.to_string());
                    }
                }
            }
        }
    }
    transports.sort();

    let mut diagnostics = Vec::new();
    let mut warn = |msg: &str| {
        diagnostics.push(Diagnostic {
            severity: "warning",
            message: msg.to_string(),
        })
    };
    if clients.is_empty() {
        warn("no target clients declared under `extensions`");
    }
    if transports.is_empty() {
        warn("no MCP transports declared (mcp.json is missing or has no servers)");
    } else if clients.is_empty() {
        warn("MCP transports are used but no target clients are declared");
    }
    for kind in &transports {
        if !matches!(kind.as_str(), "stdio" | "streamable-http" | "sse") {
            warn(&format!("unknown MCP transport `{}`", kind));
        }
    }

    Ok(CompatReport {
        tool: "plugcheck",
        clients,
        transports,
        diagnostics,
    })
}

pub fn render_human(report: &CompatReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("compat: clients: {}\n", report.clients.join(", ")));
    out.push_str(&format!(
        "compat: mcp transports: {}\n",
        report.transports.join(", ")
    ));
    for d in &report.diagnostics {
        out.push_str(&format!("{}: {}\n", d.severity, d.message));
    }
    if report.diagnostics.is_empty() {
        out.push_str("compat: no inconsistencies found\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for (name, content) in files {
            fs::write(tmp.path().join(name), content).unwrap();
        }
        tmp
    }

    const PLUGIN: &str = r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"demo","extensions":{"com.acme.client":{}}}"#;

    #[test]
    fn reports_declared_clients_and_transports() {
        let tmp = setup(&[
            ("plugin.json", PLUGIN),
            (
                "mcp.json",
                r#"{"mcpServers":{"a":{"type":"stdio"},"b":{"type":"sse"},"c":{"type":"stdio"}}}"#,
            ),
        ]);
        let report = check_compat(tmp.path()).unwrap();
        assert_eq!(report.clients, vec!["com.acme.client"]);
        assert_eq!(report.transports, vec!["sse", "stdio"]);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn warns_when_clients_are_missing() {
        let tmp = setup(&[("plugin.json", r#"{"name":"demo"}"#)]);
        let report = check_compat(tmp.path()).unwrap();
        assert!(report.clients.is_empty());
        assert_eq!(report.diagnostics.len(), 2);
        assert!(render_human(&report).contains("warning:"));
    }

    #[test]
    fn missing_manifest_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(check_compat(tmp.path()).is_err());
    }
}
