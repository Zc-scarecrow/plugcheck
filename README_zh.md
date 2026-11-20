# plugcheck

> 一个快速、轻量的 [Agent Plugins 1.0](https://agent-plugins.org) 开放标准校验器与脚手架 CLI。

[English](README.md) | [中文](README_zh.md)

[![CI](https://github.com/Zc-scarecrow/plugcheck/actions/workflows/ci.yml/badge.svg)](https://github.com/Zc-scarecrow/plugcheck/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**plugcheck** 用于校验和生成可移植的 Agent 插件——一次打包，即可运行于 VS Code、Cursor、
GitHub Copilot、ChatGPT & Codex、Kiro 等所有实现 Agent Plugins 1.0 标准的客户端。

- 极速：单一静态二进制，无运行时依赖
- 按 1.0 规范校验 `plugin.json`、`mcp.json` 与 `skills/`
- 遵循规范的失败隔离原则：单个 skill 损坏只会警告，绝不会使整个插件失效
- `--format json` 输出机器可读结果，方便接入 CI
- 一条命令生成符合规范的插件骨架

## 安装

### 预编译二进制

从 [Releases](https://github.com/Zc-scarecrow/plugcheck/releases) 下载对应平台的二进制，或使用：

```sh
cargo install plugcheck
```

### 从源码安装

```sh
cargo install --path .
```

## 快速开始

创建新插件：

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

校验已有插件（默认当前目录）：

```sh
plugcheck check /path/to/plugin
```

同时显示提示性信息：

```sh
plugcheck check --verbose
```

获取机器可读输出：

```sh
plugcheck check --format json
```

```json
{
  "tool": "plugcheck",
  "version": "0.1.1",
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

插件有效时退出码为 `0`，否则为 `1`。提示性信息（`--verbose` 显示）不影响退出码。

将插件打包为可分发的 ZIP 压缩包（排除 `.git`、`target`、`node_modules`
和 `*.log`；压缩包不压缩、内容可复现）：

```sh
plugcheck pack my-plugin -o my-plugin.zip
```

开发时自动重新校验（默认每 2 秒轮询一次）：

```sh
plugcheck watch my-plugin --interval 2
```

查看插件自声明的目标客户端与 MCP 传输类型：

```sh
plugcheck compat my-plugin --format json
```

从本地模板目录创建插件（文本文件中可用 `{{name}}` 占位符）：

```sh
plugcheck new my-plugin --template ./templates/basic
```

仓库中的 [`demo-plugin/`](demo-plugin) 是一个符合规范的示例——它由
`plugcheck new demo-plugin` 生成，并在每次 CI 运行中重新校验。

## 校验规则

| 文件 | 规则 |
|------|------|
| `plugin.json` | `$schema` 与 `name` 为必需字段；`$schema` URL 必须指向 `agent-plugins.org/schemas/<version>/plugin.schema.json`；name 仅允许小写字母、数字、连字符与点号（不能以标点开头或结尾，不允许连续的 `--` 或 `..`）；schema 封闭（未知顶层字段产生警告）；`extensions` 需以反向域名作为键 |
| `mcp.json` | `$schema` 版本必须与 `plugin.json` 声明的版本一致；`mcpServers` 条目必须使用已知传输类型（`stdio`、`streamable-http`、`sse`）；`stdio` 需要 `command`；HTTP 传输需要 `url` |
| `skills/` | 每个包含 `SKILL.md` 的直接子目录计为一个 skill；`SKILL.md` 必须以 `---` 分隔的 frontmatter 开头，且包含 `name` 与 `description`；损坏的 skill 会被报告但不会使插件失效（失败隔离） |

## 在 CI 中使用

开箱即用的 GitHub Action：

```yaml
- uses: Zc-scarecrow/plugcheck-action@v1
```

全部选项（`path`、`version`、`format`、`args`）见
[plugcheck-action](https://github.com/Zc-scarecrow/plugcheck-action)。也可以直接运行 CLI：

```yaml
- name: Validate plugin
  run: |
    cargo install plugcheck
    plugcheck check --format json
```

## 为什么做 plugcheck？

Agent Plugins 1.0 于 2026 年 8 月发布，但生态还很年轻：没有官方校验器、没有
linter、也没有脚手架工具。开发者手写 manifest，往往要到客户端拒绝加载插件时
才发现错误。plugcheck 让你在毫秒内获得反馈，在分享插件之前就确保它符合规范。

## 路线图

- [x] `plugcheck pack` — 将插件打包为可分发的压缩包
- [x] `plugcheck watch` — 文件变更时自动重新校验
- [x] `plugcheck compat` — 查看声明的客户端与 MCP 传输类型
- [x] 本地插件模板（`plugcheck new --template <dir>`）
- [ ] 社区模板仓库（托管式）
- [ ] 内置客户端能力数据库，实现真正的兼容性矩阵检查

## 参与贡献

欢迎贡献。请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)，再提交 issue 或 PR。
新增校验规则请附带测试。

## 状态

plugcheck 面向 **Agent Plugins 1.0.0** 规范。该规范目前仍被维护方标记为
*Working Draft*；规范演进时，plugcheck 会同步跟进。

## 许可证

MIT — 详见 [LICENSE](LICENSE)。
