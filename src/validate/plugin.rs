use serde_json::Value;

use crate::report::{Diagnostic, Report};

/// Official schema URL prefix for Agent Plugins 1.0.
pub const SCHEMA_URL_PREFIX: &str = "https://agent-plugins.org/schemas/";
pub const SPEC_VERSION: &str = "1.0.0";

/// Top-level fields defined by the Agent Plugins 1.0 `plugin.json` schema.
/// The schema is intentionally closed; unknown top-level fields are warnings.
const KNOWN_FIELDS: &[&str] = &[
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

/// Validate `plugin.json` against the Agent Plugins 1.0 manifest rules.
/// Returns the declared spec version (e.g. "1.0.0") on success.
pub fn check_plugin_json(report: &mut Report, value: &Value) -> Option<String> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            report.push(Diagnostic::error(
                "plugin.json",
                "root must be a JSON object",
            ));
            return None;
        }
    };

    // --- $schema (required) ---
    let schema = match obj.get("$schema").and_then(Value::as_str) {
        Some(s) => s,
        None => {
            report.push(
                Diagnostic::error("plugin.json", "missing required field `$schema`").with_hint(
                    format!(
                        "expected `{}{}/plugin.schema.json`",
                        SCHEMA_URL_PREFIX, SPEC_VERSION
                    ),
                ),
            );
            return None;
        }
    };

    let version = match parse_schema_version(schema) {
        Ok(v) => v,
        Err(msg) => {
            report.push(Diagnostic::error("plugin.json", msg).with_hint(format!(
                "expected `{}{}/plugin.schema.json`",
                SCHEMA_URL_PREFIX, SPEC_VERSION
            )));
            return None;
        }
    };

    // --- name (required) ---
    match obj.get("name").and_then(Value::as_str) {
        None => {
            report.push(
                Diagnostic::error("plugin.json", "missing required field `name`").with_hint(
                    "names may contain lowercase letters, digits, hyphens and periods (e.g. `research-tools`, `acme.analytics`)",
                ),
            );
        }
        Some(name) => {
            if let Err(msg) = validate_name(name) {
                report.push(Diagnostic::error("plugin.json", msg));
            }
        }
    }

    // --- closed schema: unknown top-level fields ---
    for key in obj.keys() {
        if !KNOWN_FIELDS.contains(&key.as_str()) {
            report.push(
                Diagnostic::warning(
                    "plugin.json",
                    format!("unknown top-level field `{}` (the schema is closed)", key),
                )
                .with_hint(
                    "client-specific metadata belongs under `extensions.<reverse-domain>` instead",
                ),
            );
        }
    }

    // --- extensions must be a map of reverse-domain names ---
    if let Some(ext) = obj.get("extensions") {
        match ext.as_object() {
            None => {
                report.push(Diagnostic::error(
                    "plugin.json",
                    "`extensions` must be an object keyed by reverse-domain client identifiers",
                ));
            }
            Some(map) => {
                for key in map.keys() {
                    if !is_reverse_domain(key) {
                        report.push(Diagnostic::warning(
                            "plugin.json",
                            format!(
                                "extensions key `{}` does not look like a reverse-domain identifier",
                                key
                            ),
                        )
                        .with_hint("expected a reverse-domain name, e.g. `com.example.client`"));
                    }
                }
            }
        }
    }

    // --- keywords (optional) must be an array of strings ---
    if let Some(kw) = obj.get("keywords")
        && !kw
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_string))
    {
        report.push(Diagnostic::warning(
            "plugin.json",
            "`keywords` should be an array of strings",
        ));
    }

    // --- recommended optional fields ---
    for field in ["version", "description", "license"] {
        if !obj.contains_key(field) {
            report.push(Diagnostic::warning(
                "plugin.json",
                format!(
                    "optional field `{}` is not set (recommended before distribution)",
                    field
                ),
            ));
        }
    }

    Some(version)
}

/// Parse the spec version out of a `$schema` URL.
/// Accepts URLs of the form `https://agent-plugins.org/schemas/<version>/plugin.schema.json`.
pub fn parse_schema_version(schema: &str) -> Result<String, String> {
    let rest = schema
        .strip_prefix(SCHEMA_URL_PREFIX)
        .ok_or_else(|| format!("`$schema` must start with `{}`", SCHEMA_URL_PREFIX))?;
    let version = rest
        .split('/')
        .next()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "`$schema` is missing a specification version".to_string())?;
    if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(format!(
            "invalid specification version `{}` in `$schema`",
            version
        ));
    }
    Ok(version.to_string())
}

/// Agent Plugins 1.0 plugin naming rules:
/// lowercase letters, digits, hyphens, periods; must not begin or end with
/// punctuation; must not contain repeated `--` or `..`.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("`name` must not be empty".to_string());
    }
    for (i, c) in name.chars().enumerate() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.';
        if !ok {
            return Err(format!(
                "`name` contains invalid character `{}` at position {}",
                c, i
            ));
        }
    }
    // The loop above already rejects uppercase; keep an explicit check for clarity.
    if name.starts_with(['-', '.']) || name.ends_with(['-', '.']) {
        return Err("`name` must not begin or end with `-` or `.`".to_string());
    }
    if name.contains("--") || name.contains("..") {
        return Err("`name` must not contain repeated `--` or `..`".to_string());
    }
    Ok(())
}

fn is_reverse_domain(key: &str) -> bool {
    !key.is_empty()
        && key.split('.').count() >= 2
        && key.split('.').all(|seg| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_valid() {
        assert!(validate_name("research-tools").is_ok());
        assert!(validate_name("acme.analytics").is_ok());
        assert!(validate_name("deploy3").is_ok());
    }

    #[test]
    fn name_invalid() {
        assert!(validate_name("Research-Tools").is_err()); // uppercase
        assert!(validate_name("-leading").is_err()); // leading punctuation
        assert!(validate_name("trailing.").is_err()); // trailing punctuation
        assert!(validate_name("double--dash").is_err());
        assert!(validate_name("double..dot").is_err());
        assert!(validate_name("under_score").is_err());
    }

    #[test]
    fn schema_version_parsing() {
        assert_eq!(
            parse_schema_version("https://agent-plugins.org/schemas/1.0.0/plugin.schema.json")
                .unwrap(),
            "1.0.0"
        );
        assert!(
            parse_schema_version("https://example.com/schemas/1.0.0/plugin.schema.json").is_err()
        );
        assert!(
            parse_schema_version("https://agent-plugins.org/schemas//plugin.schema.json").is_err()
        );
    }

    #[test]
    fn name_edge_cases() {
        // valid single characters, digit-leading names, multi-segment dotted names
        assert!(validate_name("a").is_ok());
        assert!(validate_name("3d").is_ok());
        assert!(validate_name("a.b.c").is_ok());
        assert!(validate_name("x1-y2.z3").is_ok());
        // invalid: empty, punctuation at edges, repeated punctuation
        assert!(validate_name("").is_err());
        assert!(validate_name("a-").is_err());
        assert!(validate_name("-a").is_err());
        assert!(validate_name("a.").is_err());
        assert!(validate_name(".a").is_err());
        // invalid: unicode letters are not allowed
        assert!(validate_name("aé").is_err());
    }

    #[test]
    fn full_valid_manifest_returns_version() {
        let v: Value = serde_json::from_str(
            r#"{
                "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                "name": "ok-plugin",
                "version": "1.2.3",
                "description": "hi",
                "keywords": ["a", "b"],
                "extensions": { "com.acme.x": { "n": 1 } }
            }"#,
        )
        .unwrap();
        let mut r = Report::new(std::path::Path::new("."));
        assert_eq!(check_plugin_json(&mut r, &v).as_deref(), Some("1.0.0"));
        assert!(r.ok(), "diagnostics: {:?}", r.diagnostics);
    }

    #[test]
    fn manifest_missing_name_is_error() {
        let v: Value = serde_json::from_str(
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"}"#,
        )
        .unwrap();
        let mut r = Report::new(std::path::Path::new("."));
        check_plugin_json(&mut r, &v);
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("name"));
    }

    #[test]
    fn manifest_bad_schema_url_is_error() {
        let v: Value =
            serde_json::from_str(r#"{"$schema":"https://evil.com/x.json","name":"ok"}"#).unwrap();
        let mut r = Report::new(std::path::Path::new("."));
        assert_eq!(check_plugin_json(&mut r, &v), None);
        assert!(r.errors() >= 1);
    }

    #[test]
    fn extensions_must_be_object() {
        let v: Value = serde_json::from_str(
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"ok","extensions":42}"#,
        )
        .unwrap();
        let mut r = Report::new(std::path::Path::new("."));
        check_plugin_json(&mut r, &v);
        assert_eq!(r.errors(), 1);
        assert!(r.diagnostics[0].message.contains("extensions"));
    }

    #[test]
    fn root_not_object_is_error() {
        let v = Value::Array(vec![]);
        let mut r = Report::new(std::path::Path::new("."));
        assert_eq!(check_plugin_json(&mut r, &v), None);
        assert_eq!(r.errors(), 1);
    }

    /// Robustness test: deterministic pseudo-random garbage must never panic
    /// the manifest validator, whether it parses as JSON or not.
    #[test]
    fn malformed_input_never_panics() {
        let mut seed: u64 = 0xDEAD_BEEF;
        for _ in 0..500 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let len = (seed % 128) as usize;
            let bytes: Vec<u8> = (0..len).map(|i| (seed >> (i % 32)) as u8).collect();
            if let Ok(text) = String::from_utf8(bytes)
                && let Ok(v) = serde_json::from_str::<Value>(&text)
            {
                let mut r = Report::new(std::path::Path::new("."));
                check_plugin_json(&mut r, &v); // must not panic
            }
        }
    }

    #[test]
    fn missing_recommended_fields_are_warnings() {
        let v: Value = serde_json::from_str(
            r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"ok"}"#,
        )
        .unwrap();
        let mut r = Report::new(std::path::Path::new("."));
        check_plugin_json(&mut r, &v);
        assert!(r.ok());
        let warnings: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::report::Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 3, "diagnostics: {:?}", r.diagnostics);
        assert!(warnings.iter().any(|d| d.message.contains("`version`")));
    }
}
