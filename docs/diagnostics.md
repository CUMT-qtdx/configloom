# Diagnostic codes

这些 code 同时服务于当前人类可读 CLI 和未来结构化 JSON 输出。已分配 code 不因文案变化而改变语义。

| Code | 含义 |
| --- | --- |
| `IO001` | 配置文件读取失败 |
| `CFG001` | JSON/JSONC/TOML 语法错误 |
| `CFG002` | 顶层值类型错误 |
| `CFG003` | 缺少客户端配置容器 |
| `CFG004` | Server 定义不是 object/table |
| `CFG005` | 字段缺失或类型错误 |
| `CFG006` | transport 字段互相冲突 |
| `CFG007` | 未知字段已保存在 client extension |
| `TRN001` | 未知或本阶段不支持的 transport |
| `CNV001` | 目标客户端没有已验证的等价字段 |
| `CNV002` | 可以近似转换但会改变语义或类型 |

`CNV001` 与 `CNV002` 默认都阻止输出目标配置。
