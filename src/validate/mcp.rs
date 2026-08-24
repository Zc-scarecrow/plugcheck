use serde_json::Value;

use crate::report::{Diagnostic, Report};
use crate::validate::plugin::{SCHEMA_URL_PREFIX, parse_schema_version};

/// Transport types recognized by Agent Plugins 1.0.
const KNOWN_TRANSPORTS: &[&str] = &["stdio", "streamable-http", "sse"];

/// Validate `mcp.json`. The MCP schema version declared by `mcp.json` must
/// match the Agent Plugins version declared by `plugin.json`; a mismatch is
/// reported as an error because clients may reject the MCP configuration.
pub fn check_mcp_json(report: &mut Report, value: &Value, plugin_version: Option<&str>) {
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            report.push(Diagnostic::error("mcp.json", "root must be a JSON object"));
            return;
        }
    };

    // --- $schema version consistency with plugin.json ---
    let mcp_version = match obj.get("$schema").and_then(Value::as_str) {
        Some(s) => parse_schema_version(s).ok(),
        None => None,
    };
    match (plugin_version, mcp_version.as_deref()) {
        (Some(pv), Some(mv)) if pv == mv => {}
        (Some(pv), Some(mv)) => {
            report.push(Diagnostic::error(
                "mcp.json",
                format!(
                    "schema version mismatch: plugin.json declares `{}` but mcp.json declares `{}`",
                    pv, mv
                ),
            )
            .with_hint(
                "clients may reject the MCP configuration when the versions do not match",
            ));
        }
        (Some(pv), None) => {
            report.push(Diagnostic::warning(
                "mcp.json",
                format!(
                    "`$schema` missing or unrecognized; expected version `{}` to match plugin.json",
                    pv
                ),
            )
            .with_hint(format!(
                "expected `{}{}/mcp.schema.json`",
                SCHEMA_URL_PREFIX, pv
            )));
        }
        (None, _) => {
            // plugin.json was already reported as invalid; stay silent here.
        }
    }

    // --- mcpServers ---
    let servers = match obj.get("mcpServers") {
        None => {
            report.push(Diagnostic::warning(
                "mcp.json",
                "missing `mcpServers` field (a plugin may declare zero servers)",
            ));
            return;
        }
        Some(v) => v,
    };

    let Some(servers_obj) = servers.as_object() else {
        report.push(Diagnostic::error(
            "mcp.json",
            "`mcpServers` must be an object keyed by server name",
        ));
        return;
    };

    for (name, spec) in servers_obj {
        check_server(report, name, spec);
    }
}

fn check_server(report: &mut Report, name: &str, spec: &Value) {
    let Some(spec_obj) = spec.as_object() else {
        report.push(Diagnostic::error(
            "mcp.json",
            format!("mcpServers.{} must be an object", name),
        ));
        return;
    };

    let Some(transport) = spec_obj.get("type").and_then(Value::as_str) else {
        report.push(
            Diagnostic::error("mcp.json", format!("mcpServers.{} is missing `type`", name))
                .with_hint("expected one of: stdio, streamable-http, sse"),
        );
        return;
    };

    if !KNOWN_TRANSPORTS.contains(&transport) {
        report.push(
            Diagnostic::error(
                "mcp.json",
                format!(
                    "mcpServers.{} has unknown transport type `{}`",
                    name, transport
                ),
            )
            .with_hint("expected one of: stdio, streamable-http, sse"),
        );
        return;
    }

    match transport {
        "stdio" => {
            let has_command = spec_obj.get("command").and_then(Value::as_str).is_some();
            if !has_command {
                report.push(
                    Diagnostic::error(
                        "mcp.json",
                        format!("mcpServers.{} (stdio) is missing `command`", name),
                    )
                    .with_hint("stdio servers launch a local process and require a `command`"),
                );
            }
        }
        "streamable-http" | "sse" => {
            let has_url = spec_obj.get("url").and_then(Value::as_str).is_some();
            if !has_url {
                report.push(Diagnostic::error(
                    "mcp.json",
                    format!("mcpServers.{} ({}) is missing `url`", name, transport),
                ));
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Report {
        Report::new(std::path::Path::new("."))
    }

    #[test]
    fn valid_stdio_server() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": {
                    "local-tools": { "type": "stdio", "command": "./bin/server" }
                }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert!(r.ok());
    }

    #[test]
    fn version_mismatch_is_error() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.1.0/mcp.schema.json",
                "mcpServers": {}
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("mismatch"));
    }

    #[test]
    fn stdio_requires_command() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "broken": { "type": "stdio" } }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
    }

    #[test]
    fn http_requires_url() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "broken": { "type": "streamable-http" } }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
    }

    #[test]
    fn sse_requires_url() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "broken": { "type": "sse" } }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("url"));
    }

    #[test]
    fn unknown_transport_is_error() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "x": { "type": "websocket", "url": "ws://x" } }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("unknown transport"));
    }

    #[test]
    fn mcp_servers_must_be_object() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": [1, 2]
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
    }

    #[test]
    fn server_spec_must_be_object() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "x": "not-an-object" }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
    }

    #[test]
    fn missing_type_is_error() {
        let json: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
                "mcpServers": { "x": { "command": "./x" } }
            }"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("type"));
    }

    #[test]
    fn missing_servers_is_warning_only() {
        let json: Value = serde_json::from_str(
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json"}"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, Some("1.0.0"));
        assert!(r.ok());
        assert_eq!(r.warnings(), 1);
    }

    #[test]
    fn root_not_object_is_error() {
        let mut r = report();
        check_mcp_json(&mut r, &Value::Null, Some("1.0.0"));
        assert_eq!(r.errors(), 1);
    }

    #[test]
    fn silent_when_plugin_version_unknown() {
        let json: Value = serde_json::from_str(
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{}}"#,
        )
        .unwrap();
        let mut r = report();
        check_mcp_json(&mut r, &json, None);
        assert!(r.ok());
    }

    /// Robustness test: deterministic pseudo-random garbage must never panic
    /// the MCP validator, whether it parses as JSON or not.
    #[test]
    fn malformed_input_never_panics() {
        let mut seed: u64 = 0xCAFE_BABE;
        for _ in 0..500 {
            seed = seed
                .wrapping_mul(2862933555777941757)
                .wrapping_add(3037000493);
            let len = (seed % 128) as usize;
            let bytes: Vec<u8> = (0..len).map(|i| (seed >> (i % 32)) as u8).collect();
            if let Ok(text) = String::from_utf8(bytes)
                && let Ok(v) = serde_json::from_str::<Value>(&text)
            {
                let mut r = report();
                check_mcp_json(&mut r, &v, Some("1.0.0")); // must not panic
            }
        }
    }
}
