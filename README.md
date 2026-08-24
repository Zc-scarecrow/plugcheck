# plugcheck

> A fast validator and scaffolding CLI for the [Agent Plugins 1.0](https://agent-plugins.org) open standard.

[![CI](https://github.com/yourname/plugcheck/actions/workflows/ci.yml/badge.svg)](https://github.com/yourname/plugcheck/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**plugcheck** validates and scaffolds portable agent plugins — one package that runs across
VS Code, Cursor, GitHub Copilot, ChatGPT & Codex, Kiro, and other clients implementing the
Agent Plugins 1.0 standard.

- Blazing fast: a single static binary, no runtime dependencies
- Validates `plugin.json`, `mcp.json`, and `skills/` against the 1.0 rules
- Respects the spec's failure-isolation principle: a broken skill warns but never invalidates the whole plugin
- Machine-readable `--format json` output for CI pipelines
- Scaffolds a conformant plugin in one command

## Install

### Prebuilt binaries

Download the latest binary for your platform from
[Releases](https://github.com/yourname/plugcheck/releases), or install with:

```sh
cargo install plugcheck
```

### From source

```sh
cargo install --path .
```

## Quick start

Create a new plugin:

```sh
plugcheck new my-plugin
cd my-plugin
plugcheck check
```

```
Created plugin `my-plugin` in my-plugin

Next steps:
  cd my-plugin
  plugcheck check
```

Validate an existing plugin (current directory by default):

```sh
plugcheck check /path/to/plugin
```

Get machine-readable output:

```sh
plugcheck check --format json
```

```json
{
  "tool": "plugcheck",
  "version": "0.1.0",
  "plugin_root": "my-plugin",
  "valid": true,
  "summary": {
    "errors": 0,
    "warnings": 0,
    "skills_checked": 1
  },
  "diagnostics": []
}
```

Exit code is `0` when the plugin is valid, `1` otherwise.

A conformant example lives in [`demo-plugin/`](demo-plugin) — it was scaffolded
by `plugcheck new demo-plugin` and is re-validated on every CI run.

## What plugcheck validates

| File | Rules |
|------|-------|
| `plugin.json` | required `$schema` and `name` fields; `$schema` URL must point to `agent-plugins.org/schemas/<version>/plugin.schema.json`; name restricted to lowercase letters, digits, hyphens and periods (no leading/trailing punctuation, no repeated `--` or `..`); closed schema (unknown top-level fields are warnings); `extensions` keyed by reverse-domain identifiers |
| `mcp.json` | `$schema` version must match the version declared by `plugin.json`; `mcpServers` entries require a known transport (`stdio`, `streamable-http`, `sse`); `stdio` requires `command`; HTTP transports require `url` |
| `skills/` | every direct child directory with a `SKILL.md` counts as one skill; `SKILL.md` must start with `---`-delimited frontmatter containing `name` and `description`; a broken skill is reported but does not invalidate the plugin (failure isolation) |

## Use in CI

```yaml
- name: Validate plugin
  run: |
    cargo install plugcheck
    plugcheck check --format json
```

A dedicated GitHub Action is on the roadmap.

## Why plugcheck?

Agent Plugins 1.0 was released in August 2026, but the ecosystem is still young:
there is no official validator, no linter, no scaffolding tool. Developers hand-craft
manifests and discover mistakes only when a client rejects their plugin.
plugcheck gives you feedback in milliseconds, so your plugin is conformant before
you share it.

## Roadmap

- [ ] `plugcheck pack` — bundle a plugin into a distributable archive
- [ ] `plugcheck watch` — re-check on file changes
- [ ] GitHub Action for one-line CI integration
- [ ] Compatibility matrix checks (which clients support your transports)
- [ ] Plugin template registry (`plugcheck new` with community templates)

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and open an issue
or PR. Please add tests for new validation rules.

## Status

plugcheck targets the **Agent Plugins 1.0.0** specification. The specification is
still marked as a *Working Draft* by its maintainers; when it evolves, plugcheck
will follow.

## License

MIT — see [LICENSE](LICENSE).
