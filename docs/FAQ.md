# 常见问题（FAQ）

[English README](../README.md) | [中文 README](../README_zh.md)

## 什么是 Agent Plugins 1.0？

一个开放标准，让插件一次打包即可在 VS Code、Cursor、GitHub Copilot、
ChatGPT & Codex、Kiro 等实现了该标准的客户端中运行。插件的核心是一个
`plugin.json` manifest，可选配 `mcp.json` 和 `skills/`。

## plugcheck 和通用 JSON Schema 校验有什么不同？

plugcheck 是**面向规范的结构化校验器**，除了字段类型，还会检查命名规则、
`$schema` URL 与版本一致性、MCP 传输类型是否合法、skill frontmatter 是否完整，
并给出带 `hint` 的可操作诊断。它不联网拉取 schema，因此是纯静态、毫秒级的。

## 为什么有些问题是 warning 而不是 error？

规范遵循**失败隔离**：单个 skill 损坏、建议字段缺失、未知字段等只报告警告，
不会让整个插件失效。只有违反硬性规则（缺少 `plugin.json`、非法名称、
MCP 版本不一致等）才会让退出码变成 1。

## 支持哪些操作系统？

Linux（x86_64 / aarch64）、macOS（x86_64 / aarch64）、Windows（x86_64）。
发布包在每次 tag 推送时由 GitHub Actions 自动构建，可从 Releases 下载。

## 怎么在 CI 中使用？

```yaml
- name: Validate plugin
  run: |
    cargo install plugcheck
    plugcheck check --format json
```

退出码 `0` 表示没有 error，可直接作为流水线门禁。

## `mcp.json` 的版本为什么要和 `plugin.json` 一致？

客户端可能因为版本不匹配而拒绝加载 MCP 配置。plugcheck 会把不一致报为 error，
并在 `mcp.json` 缺失或版本无法识别时给出提示。

## `plugcheck pack` 生成什么？

一个**不压缩的 ZIP 归档**（`.zip`），使用标准 ZIP 格式，可直接被常见解压工具打开。
为了保持可复现与整洁，会排除 `.git`、`target`、`node_modules` 和 `*.log`。
时间戳固定为 1980-01-01，因此相同输入生成相同字节。

## `plugcheck watch` 是怎么检测变更的？

默认每 2 秒轮询一次目录下所有文件的修改时间与大小（可用 `--interval` 调整）。
采用轮询是为了不引入额外的文件系统监听依赖，保持单一静态二进制。
按 `Ctrl-C` 停止。

## `plugcheck compat` 是真正的兼容性矩阵吗？

目前是**自声明一致性检查**：汇总 `extensions` 中声明的目标客户端与 `mcp.json`
中实际使用的传输类型，并标出不一致（例如用了 MCP 传输却没有声明任何客户端）。
项目内置客户端能力数据库属于路线图项，尚未实现。

## 可以用自己的插件模板吗？

可以。`plugcheck new my-plugin --template ./my-template` 会从本地目录复制，
文本文件中的 `{{name}}` 会被替换为插件名；二进制文件原样复制。

## 如何贡献？

参见 [CONTRIBUTING.md](../CONTRIBUTING.md)。新增校验规则请同时提供单元测试和集成测试。
