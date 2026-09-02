# Fixture corpus

这些文件只包含示例域名和示例凭据，不包含真实 secret。Milestone 1 的测试只读取这些路径。

## 覆盖范围

每个客户端包含 11 个 fixture：

- `empty`
- `single-stdio`
- `multiple-servers`
- `http`
- `env`
- `headers`
- `client-specific-fields`
- `unrelated-settings`
- `comments`
- `unknown-fields`
- `malformed`

总计 33 个 fixture，其中每个客户端有 10 个可解析 fixture 和 1 个 malformed fixture。

## 来源

- Claude Code：官方 [MCP 文档](https://code.claude.com/docs/en/mcp) 的项目 `.mcp.json`、stdio、HTTP、headers、`headersHelper` 和 `alwaysLoad` 结构。
- VS Code：官方 [MCP configuration reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration) 以及 `microsoft/vscode` 的 `mcpConfiguration.ts` schema。
- Codex：`openai/codex` 当前 `codex-rs/config/src/mcp_types.rs` 的 `RawMcpServerConfig` 和 transport 类型。

Fixtures 是压缩后的最小复现，不复制用户配置。`unknown-fields` 用于证明不静默删除未来字段，不代表客户端会接受这些字段。

## Comments 边界

- VS Code JSONC 和 Codex TOML 的 comments fixture 对应客户端真实格式。
- Claude 官方只承诺 JSON。`claude/comments.jsonc` 是 parser-only fixture，用来阻止 ConfigLoom 自身破坏用户交给它的 JSONC 文本；不把它列为 Claude Code 已验证接受的格式。
- 三个 comments fixture 都覆盖顶部、section、相邻字段和行尾注释，并要求未修改 round-trip 字节一致。
