# 研究与决策

记录日期：2026-09-02。

## 项目命名

检查范围包括 GitHub 仓库名、npm、crates.io、本机命令、公开同领域产品和明显品牌风险。

| 候选 | 检查结果 | 决定 |
| --- | --- | --- |
| `mcp-compile` | GitHub 已有精确同名仓库 | 排除 |
| `mcp-conform` | GitHub 已有两个精确同名仓库 | 排除 |
| `mcp-braid` | 无精确仓库名，但 `braid` 已是相邻 AI CLI/agent profile 产品 | 排除 |
| `mcp-seam` | 无精确仓库名，但 `SEAM` 已是带 CLI/MCP 的 agent memory 产品 | 排除 |
| `mcp-transit` | GitHub 已有精确同名仓库 | 排除 |
| `mcp-lens` | GitHub 和 npm 均有精确同名项目 | 排除 |
| `mcp-unison` | `Unison` 已有 CLI、MCP 和 AI 工具产品，语境冲突明显 | 排除 |
| `config-ferry` | 精确包名空闲，但 Ferry 软件名较多且定位不直观 | 排除 |
| `mcpcfg` | 既有 GitHub 项目，也有历史 Windows `MCPCFG` 命令 | 排除 |
| `configloom` | GitHub、npm、crates.io 和本机命令无精确占用；精确公开搜索未见同领域产品 | 采用 |

`Loom` 是拥挤的通用词，因此完整名称不能缩写为 `loom`。当前只使用 `ConfigLoom` / `configloom`。在公开发布 package 前应重新检查注册表和商标状态。

## 技术栈

选择 Rust，而不是沿用任何原型语言。

- `toml_edit` 明确保留 TOML 注释、空白和顺序，适合后续安全编辑。
- `jsonc-parser` 提供 concrete syntax tree；本阶段从 CST 原样渲染未修改文档。
- Rust 可发布单文件跨平台 CLI，运行时不需要 Node/Python。
- Cargo 自带测试、格式化、lint 和依赖锁定。
- 代价是首次构建需要 Rust toolchain，跨平台 binary 发布需要 CI；这属于安装成本风险。

## 真实格式依据

### Claude Code

依据：[Claude Code MCP 文档](https://code.claude.com/docs/en/mcp)。

- 项目路径是 `.mcp.json`，顶层键为 `mcpServers`。
- stdio 使用 `command`、`args`、`env`；官方示例允许省略 `type`。
- Streamable HTTP 使用 `type = http`（也接受 `streamable-http`）、`url`、`headers`。
- `headersHelper` 和 `alwaysLoad` 是 Claude 专用字段；前者与当前 Codex `http_headers_helper` 语义相符。
- 官方文档称其为 JSON，没有找到官方承诺 JSONC 注释可被 Claude Code 接受。

### VS Code

依据：[VS Code MCP configuration reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration) 和 [VS Code 源码 schema](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/mcp/common/mcpConfiguration.ts)。

- 项目路径是 `.vscode/mcp.json`，顶层键为 `servers`，并可能含 `inputs`、`sandbox`。
- stdio 字段包括 `type`、`command`、`args`、`cwd`、`env`、`envFile`、`dev`、`sandboxEnabled`。
- HTTP 字段包括 `type`、`url`、`headers`、`oauth`。
- VS Code 的 `env` 值允许 string、number、null；后两者不能无损放入 Claude/Codex 的字符串环境变量。

### Codex

依据：[Codex MCP 配置源码](https://github.com/openai/codex/blob/main/codex-rs/config/src/mcp_types.rs) 和 [Codex 配置结构](https://github.com/openai/codex/blob/main/codex-rs/config/src/config_toml.rs)。

- 本阶段按要求发现项目路径 `.codex/config.toml`，MCP table 为 `mcp_servers`。
- stdio 字段包括 `command`、`args`、`env`、`env_vars`、`cwd`。
- Streamable HTTP 字段包括 `url`、`http_headers`、`env_http_headers`、`bearer_token_env_var`、`http_headers_helper`。
- `enabled`、`required`、timeouts、tool filters、approval 和 OAuth 字段保存在 Codex extension 中。
- 当前源码对正式 schema 使用 deny-unknown-fields；ConfigLoom 仍先保留未知字段并给出 `CFG007`，避免只读 round-trip 删除数据。

## Problem / Evidence / Decision

### 注释不能依赖普通 parse/serialize

Problem：JSONC/TOML 反序列化为普通对象再序列化会删除注释和原格式。

Evidence：`toml_edit` 的公开文档承诺保留 comments/whitespace/order；`node-jsonc-parser` 仍有新增属性导致行尾注释重新附着的公开问题（microsoft/node-jsonc-parser#125）。

Decision：Milestone 1 保留原始 syntax document，并对未修改 Canonical Model 做 CST/Document round-trip；测试要求字节一致。修改源语法树留到 Safe Apply，不在本阶段伪装已经解决。

### 同名产品风险

Problem：`mcp-sync` 已覆盖 canonical config、多客户端、安全写入、backup/restore 等广泛方向。

Evidence：[EnjoyableWork/mcp-sync](https://github.com/EnjoyableWork/mcp-sync) 于 2026-08 创建，当前公开仓库已有完整安全模型，但尚无 star/fork 采用信号。

Decision：继续本 Milestone，但差异化必须收窄为项目级配置编译、稳定诊断、保真 round-trip 和明确的 loss 分类。进入 Milestone 2 前重新比较其实际实现；如果无法形成可验证差异，应停止。

### 动态 Header 能力发生变化

Problem：旧资料会把 Claude `headersHelper` → Codex 判断为不支持。

Evidence：当前 Codex `McpServerTransportConfig::StreamableHttp` 已包含 `http_headers_helper`，且同样是输出 JSON Header 对象的本地命令。

Decision：Canonical HTTP 模型加入 `dynamic_headers_command`。Claude 与 Codex 之间判为 lossless；VS Code 判为 unsupported。
