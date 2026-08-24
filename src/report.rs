use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A single finding produced by validation.
#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Path relative to the plugin root.
    pub file: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn error(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            file: file.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn warning(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            file: file.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Aggregated validation result.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip)]
    pub root: PathBuf,
}

impl Report {
    pub fn new(root: &Path) -> Self {
        Self {
            diagnostics: Vec::new(),
            root: root.to_path_buf(),
        }
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn ok(&self) -> bool {
        self.errors() == 0
    }
}

#[derive(Debug, Serialize)]
pub struct JsonReport<'a> {
    pub tool: &'static str,
    pub version: &'static str,
    pub plugin_root: &'a str,
    pub valid: bool,
    pub summary: Summary,
    pub diagnostics: &'a [Diagnostic],
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub skills_checked: usize,
}

impl Report {
    /// Render the report as human-readable text.
    pub fn render_human(&self, skills_checked: usize) -> String {
        let mut out = String::new();
        for d in &self.diagnostics {
            let marker = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            out.push_str(&format!("{}: {}: {}\n", marker, d.file, d.message));
            if let Some(hint) = &d.hint {
                out.push_str(&format!("  hint: {}\n", hint));
            }
        }
        let e = self.errors();
        let w = self.warnings();
        if e == 0 && w == 0 {
            out.push_str(&format!(
                "plugcheck: no problems found ({} skills checked)\n",
                skills_checked
            ));
        } else {
            out.push_str(&format!(
                "plugcheck: {} problem(s) found ({} error(s), {} warning(s), {} skills checked)\n",
                e + w,
                e,
                w,
                skills_checked
            ));
        }
        out
    }

    /// Render the report as JSON.
    pub fn render_json(&self, skills_checked: usize) -> String {
        let plugin_root = self.root.to_string_lossy();
        let report = JsonReport {
            tool: "plugcheck",
            version: env!("CARGO_PKG_VERSION"),
            plugin_root: plugin_root.as_ref(),
            valid: self.ok(),
            summary: Summary {
                errors: self.errors(),
                warnings: self.warnings(),
                skills_checked,
            },
            diagnostics: &self.diagnostics,
        };
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    }
}
