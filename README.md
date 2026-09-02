# ConfigLoom

不同 AI 客户端以不同格式和位置保存 MCP Server 配置。ConfigLoom 读取、校验并编译这些定义，同时保留源文件中不属于 MCP 的设置、未知字段与注释。

## Current status

当前为早期开发版本，仅完成 Milestone 1 的只读能力。它不会修改任何真实配置文件。

## Supported clients

- Claude Code 项目级 `.mcp.json`
- VS Code 项目级 `.vscode/mcp.json`
- Codex 项目级 `.codex/config.toml`

仅实现 stdio 与 Streamable HTTP。OAuth、SSE 和 WebSocket 不在当前范围内。

## Current commands

```text
configloom inspect <claude|vscode|codex> [--config <path>] [--root <path>]
configloom validate <claude|vscode|codex> [--config <path>] [--root <path>]
configloom convert <claude|vscode|codex> --to <client> [--config <path>]
```

`inspect` 和 `convert` 默认脱敏疑似凭据。`convert` 只在转换被判定为 `LOSSLESS` 时向 stdout 输出配置；`LOSSY` 或 `UNSUPPORTED` 不输出目标配置。只有显式传入 `--show-secrets` 才会输出 env/header 的字面值。

## Development

```powershell
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Fixtures 和依据说明见 `tests/fixtures/README.md`，产品决策见 `docs/research.md`。

## Known limitations

- 尚未实现 `apply`、`sync`、drift detection、backup、restore。
- 同客户端 round-trip 对未修改的语法文档提供字节级保真；修改 Canonical Model 后重写原文属于下一阶段。
- Claude Code 官方文档只承诺 JSON；JSONC 注释 fixture 仅证明 ConfigLoom 自身能保真读取，不宣称 Claude Code 会接受它。
- 当前仅发现项目级路径，不读取 user/global scope。

## License

MIT
