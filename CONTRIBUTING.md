# Contributing to plugcheck

Thanks for your interest in contributing. plugcheck validates plugins against the
[Agent Plugins 1.0](https://agent-plugins.org) specification, so the first step
is always: **read the relevant part of the spec**.

## Setup

```sh
git clone https://github.com/Zc-scarecrow/plugcheck.git
cd plugcheck
cargo build
cargo test
```

## Adding a validation rule

1. Find the right module in `src/validate/` (`plugin.rs`, `mcp.rs`, `skills.rs`).
2. Add unit tests for pure logic (name rules, version parsing).
3. Add an integration test in `tests/cli.rs` for end-to-end behavior.
4. Run `cargo fmt && cargo clippy -- -D warnings && cargo test`.

## Ground rules

- Diagnostics must reference the **file path relative to the plugin root**
  (e.g. `plugin.json`, `skills/foo/SKILL.md`).
- Follow the spec's failure-isolation principle: problems inside one skill
  are warnings, never errors that invalidate the whole plugin.
- Keep dependencies minimal. A single static binary with no runtime
  dependencies is a feature, not an accident.
- New validation rules need a fixture-based test covering both the valid
  and the invalid case.
