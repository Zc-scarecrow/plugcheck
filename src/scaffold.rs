use std::fs;
use std::io;
use std::path::Path;

use crate::validate::plugin::{SCHEMA_URL_PREFIX, SPEC_VERSION, validate_name};

/// Errors that can occur while scaffolding a new plugin.
#[derive(Debug)]
pub enum ScaffoldError {
    InvalidName(String),
    Io(io::Error),
}

impl From<io::Error> for ScaffoldError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(msg) => write!(f, "invalid plugin name: {}", msg),
            Self::Io(err) => write!(f, "{}", err),
        }
    }
}

fn plugin_json(name: &str) -> String {
    format!(
        r#"{{
  "$schema": "{}{}/plugin.schema.json",
  "name": "{}",
  "version": "0.1.0",
  "description": "A short description of what this plugin does.",
  "author": "Your Name",
  "license": "MIT",
  "keywords": ["agent-plugins"]
}}
"#,
        SCHEMA_URL_PREFIX, SPEC_VERSION, name
    )
}

fn mcp_json() -> String {
    format!(
        r#"{{
  "$schema": "{}{}/mcp.schema.json",
  "mcpServers": {{}}
}}
"#,
        SCHEMA_URL_PREFIX, SPEC_VERSION
    )
}

fn skill_md(name: &str) -> String {
    format!(
        r#"---
name: {}-example
description: A starter skill. Replace or delete this directory.
---

# {name} example skill

This is a starter skill scaffolded by `plugcheck new`.
Edit this file, and add optional `scripts/` or `references/` directories.

See the Agent Skills specification for authoring guidance.
"#,
        name,
    )
}

fn readme_md(name: &str) -> String {
    format!(
        r#"# {name}

A plugin for agent clients supporting the [Agent Plugins 1.0](https://agent-plugins.org) standard
(VS Code, Cursor, GitHub Copilot, ChatGPT & Codex, Kiro, and others).

## Contents

- `plugin.json` — plugin manifest
- `skills/` — agent skills (progressive disclosure instructions)
- `mcp.json` — optional MCP server configuration

## Validate

```sh
plugcheck check
```

## License

MIT
"#,
        name = name
    )
}

const GITIGNORE: &str = "/target\n*.log\n";

const LICENSE_MIT: &str = r#"MIT License

Copyright (c) 2026 Your Name

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#;

/// Scaffold a new Agent Plugins 1.0 plugin into `dir`.
pub fn scaffold(name: &str, dir: &Path) -> Result<(), ScaffoldError> {
    validate_name(name).map_err(ScaffoldError::InvalidName)?;

    if dir.exists() {
        return Err(ScaffoldError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target directory already exists: {}", dir.display()),
        )));
    }

    fs::create_dir_all(dir)?;
    fs::create_dir_all(dir.join("skills").join("example"))?;

    write(dir, "plugin.json", &plugin_json(name))?;
    write(dir, "mcp.json", &mcp_json())?;
    write(dir, "README.md", &readme_md(name))?;
    write(dir, "LICENSE", LICENSE_MIT)?;
    write(dir, ".gitignore", GITIGNORE)?;
    write(dir, "skills/example/SKILL.md", &skill_md(name))?;

    Ok(())
}

/// Scaffold from a local template directory.
///
/// Text files may contain `{{name}}` placeholders, replaced by the plugin
/// name. Non-UTF-8 files are copied verbatim. `.git` and `target` are skipped.
pub fn scaffold_from_template(
    name: &str,
    dir: &Path,
    template: &Path,
) -> Result<(), ScaffoldError> {
    validate_name(name).map_err(ScaffoldError::InvalidName)?;
    if dir.exists() {
        return Err(ScaffoldError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target directory already exists: {}", dir.display()),
        )));
    }
    if !template.is_dir() {
        return Err(ScaffoldError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("template directory not found: {}", template.display()),
        )));
    }
    copy_template(template, dir, name)?;
    Ok(())
}

fn copy_template(src: &Path, dst: &Path, name: &str) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.filter_map(Result::ok) {
        let from = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" {
            continue;
        }
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_template(&from, &to, name)?;
        } else if from.is_file() {
            let data = fs::read(&from)?;
            match String::from_utf8(data.clone()) {
                Ok(text) => fs::write(&to, text.replace("{{name}}", name))?,
                Err(_) => fs::write(&to, &data)?,
            }
        }
    }
    Ok(())
}

fn write(dir: &Path, rel: &str, content: &str) -> io::Result<()> {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::{check_plugin_dir, skills::Frontmatter};

    /// Dogfood test: a scaffolded plugin must pass plugcheck's own validator.
    #[test]
    fn template_substitutes_name_and_copies_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let template = tmp.path().join("template");
        fs::create_dir_all(template.join("skills/hello")).unwrap();
        fs::write(template.join("plugin.json"), "{\"name\": \"{{name}}\"}").unwrap();
        fs::write(template.join("skills/hello/SKILL.md"), "name: {{name}}").unwrap();
        fs::create_dir_all(template.join(".git")).unwrap();
        fs::write(template.join(".git/config"), "skip").unwrap();

        let target = tmp.path().join("out");
        scaffold_from_template("demo-plugin", &target, &template).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("plugin.json")).unwrap(),
            "{\"name\": \"demo-plugin\"}"
        );
        assert_eq!(
            fs::read_to_string(target.join("skills/hello/SKILL.md")).unwrap(),
            "name: demo-plugin"
        );
        assert!(!target.join(".git").exists());
    }

    #[test]
    fn template_rejects_invalid_name_and_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(scaffold_from_template("Bad", &tmp.path().join("a"), tmp.path()).is_err());
        assert!(
            scaffold_from_template("ok", &tmp.path().join("b"), &tmp.path().join("nope")).is_err()
        );
    }

    #[test]
    fn default_scaffold_still_works() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("demo");
        scaffold("demo-plugin", &dir).unwrap();
        let (report, skills) = check_plugin_dir(&dir);
        assert!(report.ok(), "diagnostics: {:?}", report.diagnostics);
        assert_eq!(skills, 1);
    }

    #[test]
    fn rejects_existing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("existing");
        fs::create_dir(&dir).unwrap();
        match scaffold("x-plugin", &dir) {
            Err(ScaffoldError::Io(_)) => {}
            other => panic!("expected Io error, got {:?}", other),
        }
    }

    #[test]
    fn rejects_invalid_names() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in ["Bad", "under_score", "-lead", "trail-", "a..b", "a--b", ""] {
            assert!(
                scaffold(bad, &tmp.path().join("x")).is_err(),
                "name {:?} should be rejected",
                bad
            );
        }
    }

    #[test]
    fn templates_are_valid_json() {
        for text in [plugin_json("acme.analytics"), mcp_json()] {
            serde_json::from_str::<serde_json::Value>(&text).unwrap();
        }
    }

    #[test]
    fn skill_template_has_valid_frontmatter() {
        let skill = skill_md("acme.analytics");
        let fm = Frontmatter::parse(&skill).expect("template must have frontmatter");
        assert_eq!(fm.get("name"), Some("acme.analytics-example"));
        assert!(fm.get("description").is_some());
    }

    #[test]
    fn readme_template_mentions_plugin_name() {
        let readme = readme_md("cool-plugin");
        assert!(readme.contains("cool-plugin"));
        assert!(readme.contains("plugcheck check"));
    }
}
