pub mod mcp;
pub mod plugin;
pub mod skills;

use std::fs;
use std::path::Path;

use crate::report::{Diagnostic, Report};

/// Validate a plugin directory against Agent Plugins 1.0.
///
/// Returns `(report, skills_checked)`.
pub fn check_plugin_dir(root: &Path) -> (Report, usize) {
    let mut report = Report::new(root);

    if !root.is_dir() {
        report.push(Diagnostic::error(
            ".",
            format!("not a directory: {}", root.display()),
        ));
        return (report, 0);
    }

    // plugin.json is required.
    let plugin_path = root.join("plugin.json");
    let plugin_value = match fs::read_to_string(&plugin_path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => Some(v),
            Err(err) => {
                report.push(Diagnostic::error(
                    "plugin.json",
                    format!("invalid JSON: {}", err),
                ));
                None
            }
        },
        Err(_) => {
            report.push(Diagnostic::error(
                "plugin.json",
                "file not found (a conformant plugin requires plugin.json)",
            ));
            None
        }
    };

    let plugin_version = plugin_value
        .as_ref()
        .and_then(|v| plugin::check_plugin_json(&mut report, v));

    // mcp.json is optional.
    let mcp_path = root.join("mcp.json");
    if mcp_path.exists() {
        match fs::read_to_string(&mcp_path) {
            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => mcp::check_mcp_json(&mut report, &v, plugin_version.as_deref()),
                Err(err) => {
                    report.push(Diagnostic::error(
                        "mcp.json",
                        format!("invalid JSON: {}", err),
                    ));
                }
            },
            Err(err) => {
                report.push(Diagnostic::error(
                    "mcp.json",
                    format!("cannot read file: {}", err),
                ));
            }
        }
    }

    let skills_checked = skills::check_skills(&mut report, root);

    (report, skills_checked)
}
