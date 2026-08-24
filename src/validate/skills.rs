use std::fs;
use std::path::Path;

use crate::report::{Diagnostic, Report};

/// A minimal YAML-frontmatter extractor for `SKILL.md` files.
///
/// Agent Skills metadata lives in a fenced frontmatter block at the top of
/// the file, delimited by `---` lines, containing simple `key: value` pairs.
/// This parser intentionally supports only the flat key/value subset needed
/// for validation (`name`, `description`).
pub struct Frontmatter {
    pub fields: Vec<(String, String)>,
}

impl Frontmatter {
    pub fn parse(content: &str) -> Option<Self> {
        let mut lines = content.lines();
        if lines.next().is_none_or(|l| l.trim() != "---") {
            return None;
        }
        let mut fields = Vec::new();
        for line in lines.by_ref() {
            let line = line.trim();
            if line == "---" {
                return Some(Self { fields });
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                fields.push((key.trim().to_string(), value.to_string()));
            }
        }
        // No closing fence found.
        None
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Validate the `skills/` directory.
///
/// Every direct child directory containing a `SKILL.md` file is treated as
/// one skill. Following the failure-isolation principle of the
/// specification, an invalid skill is reported but must not invalidate the
/// whole plugin: problems inside a skill are downgraded to warnings here and
/// returned as the number of skills checked.
pub fn check_skills(report: &mut Report, root: &Path) -> usize {
    let skills_dir = root.join("skills");
    if !skills_dir.exists() {
        report.push(Diagnostic::warning(
            "skills/",
            "directory not found (a plugin may declare zero skills)",
        ));
        return 0;
    }
    if !skills_dir.is_dir() {
        report.push(Diagnostic::error(
            "skills/",
            "exists but is not a directory",
        ));
        return 0;
    }

    let mut checked = 0usize;
    let entries = match fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(err) => {
            report.push(Diagnostic::error(
                "skills/",
                format!("cannot read directory: {}", err),
            ));
            return 0;
        }
    };

    let mut dirs: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let rel = format!("skills/{}", dir.file_name().unwrap().to_string_lossy());
        let skill_md = dir.join("SKILL.md");
        if !skill_md.exists() {
            // Not a skill: only direct children with SKILL.md count.
            report.push(Diagnostic::warning(
                rel.clone(),
                "directory does not contain SKILL.md and is not treated as a skill",
            ));
            continue;
        }

        checked += 1;
        let rel_file = format!("{}/SKILL.md", rel);

        let content = match fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(err) => {
                report.push(Diagnostic::warning(
                    rel_file,
                    format!("cannot read SKILL.md: {}", err),
                ));
                continue;
            }
        };

        match Frontmatter::parse(&content) {
            None => {
                report.push(Diagnostic::warning(
                    rel_file,
                    "missing or malformed frontmatter (expected `---` delimited key/value block)",
                )
                .with_hint("add frontmatter with at least `name` and `description` fields"));
            }
            Some(fm) => {
                if fm.get("name").is_none_or(|v| v.is_empty()) {
                    report.push(Diagnostic::warning(
                        rel_file.clone(),
                        "frontmatter is missing required field `name`",
                    ));
                }
                if fm.get("description").is_none_or(|v| v.is_empty()) {
                    report.push(Diagnostic::warning(
                        rel_file,
                        "frontmatter is missing required field `description`",
                    ));
                }
            }
        }
    }

    checked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter() {
        let content = "---\nname: deploy\ndescription: \"Deploys the app\"\n---\n\nBody here.\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.get("name"), Some("deploy"));
        assert_eq!(fm.get("description"), Some("Deploys the app"));
    }

    #[test]
    fn rejects_missing_fence() {
        assert!(Frontmatter::parse("name: x\n---\nbody").is_none());
        assert!(Frontmatter::parse("---\nname: x").is_none()); // never closed
    }

    #[test]
    fn handles_quoted_values() {
        let content = "---\nname: 'single'\ndescription: \"double quoted\"\n---\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.get("name"), Some("single"));
        assert_eq!(fm.get("description"), Some("double quoted"));
    }

    #[test]
    fn keeps_colons_inside_values() {
        let content = "---\nname: x\nlink: https://example.com/path\n---\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.get("link"), Some("https://example.com/path"));
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let content = "---\n# a comment\n\nname: x\n---\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.fields.len(), 1);
        assert_eq!(fm.get("name"), Some("x"));
    }

    #[test]
    fn ignores_lines_without_colon() {
        let content = "---\nno-colon-here\nname: x\n---\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.get("name"), Some("x"));
        assert_eq!(fm.fields.len(), 1);
    }

    #[test]
    fn get_returns_none_for_absent_key() {
        let content = "---\nname: x\n---\n";
        let fm = Frontmatter::parse(content).unwrap();
        assert_eq!(fm.get("description"), None);
    }
}
