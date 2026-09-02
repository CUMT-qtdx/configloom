use crate::adapter::render_canonical;
use crate::diagnostic::{Diagnostic, codes};
use crate::model::{Client, ConversionStatus, ExtensionRef, ParsedConfig, Transport};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone)]
pub struct ConversionReport {
    pub source: Client,
    pub target: Client,
    pub status: ConversionStatus,
    pub diagnostics: Vec<Diagnostic>,
    pub rendered: Option<String>,
}

#[must_use]
pub fn convert(parsed: &ParsedConfig, target: Client) -> ConversionReport {
    if parsed.client == target {
        return match parsed.render() {
            Ok(rendered) => ConversionReport {
                source: parsed.client,
                target,
                status: ConversionStatus::Lossless,
                diagnostics: Vec::new(),
                rendered: Some(rendered),
            },
            Err(error) => ConversionReport {
                source: parsed.client,
                target,
                status: ConversionStatus::Unsupported,
                diagnostics: vec![error],
                rendered: None,
            },
        };
    }

    let mut diagnostics = Vec::new();
    for (name, server) in &parsed.canonical.servers {
        match &server.transport {
            Transport::Stdio {
                working_directory: Some(_),
                ..
            } if target == Client::Claude => diagnostics.push(unsupported(
                parsed.client,
                target,
                format!("servers.{name}.working_directory"),
                "Claude Code 项目 MCP schema 没有等价的 cwd 字段",
            )),
            Transport::StreamableHttp {
                dynamic_headers_command: Some(_),
                ..
            } if target == Client::Vscode => diagnostics.push(unsupported(
                parsed.client,
                target,
                format!("servers.{name}.dynamic_headers_command"),
                "VS Code MCP schema 没有动态 Header 命令字段",
            )),
            _ => {}
        }

        if let Some(extension) = server.extensions.for_client(parsed.client) {
            diagnostics.extend(extension_diagnostics(
                extension,
                parsed.client,
                target,
                &format!("servers.{name}.extensions"),
            ));
        }
    }

    if let Some(extension) = parsed.canonical.extensions.for_client(parsed.client) {
        diagnostics.extend(extension_diagnostics(
            extension,
            parsed.client,
            target,
            "extensions",
        ));
    }

    let has_unsupported = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == codes::UNSUPPORTED_CONVERSION);
    let has_lossy = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == codes::LOSSY_CONVERSION);
    let status = if has_unsupported {
        ConversionStatus::Unsupported
    } else if has_lossy {
        ConversionStatus::Lossy
    } else {
        ConversionStatus::Lossless
    };
    let rendered =
        (status == ConversionStatus::Lossless).then(|| render_canonical(target, &parsed.canonical));

    ConversionReport {
        source: parsed.client,
        target,
        status,
        diagnostics,
        rendered,
    }
}

fn extension_diagnostics(
    extension: ExtensionRef<'_>,
    source: Client,
    target: Client,
    prefix: &str,
) -> Vec<Diagnostic> {
    match extension {
        ExtensionRef::Json(JsonValue::Object(object)) => object
            .iter()
            .flat_map(|(key, value)| {
                if source == Client::Vscode && key == "env" {
                    return non_string_env_diagnostics(value, source, target, prefix);
                }
                vec![unsupported(
                    source,
                    target,
                    format!("{prefix}.{key}"),
                    "该字段属于源客户端，目标客户端没有已验证的等价表示",
                )]
            })
            .collect(),
        ExtensionRef::Json(_) => vec![unsupported(
            source,
            target,
            prefix.to_owned(),
            "源客户端 extension 不是对象，无法映射",
        )],
        ExtensionRef::Toml(value) => value
            .as_table()
            .map(|table| {
                table
                    .keys()
                    .map(|key| {
                        unsupported(
                            source,
                            target,
                            format!("{prefix}.{key}"),
                            "该字段属于 Codex，目标客户端没有已验证的等价表示",
                        )
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![unsupported(
                    source,
                    target,
                    prefix.to_owned(),
                    "Codex extension 不是 table，无法映射",
                )]
            }),
    }
}

fn non_string_env_diagnostics(
    value: &JsonValue,
    source: Client,
    target: Client,
    prefix: &str,
) -> Vec<Diagnostic> {
    value
        .as_object()
        .map(|object| {
            object
                .keys()
                .map(|key| {
                    Diagnostic::error(
                        codes::LOSSY_CONVERSION,
                        format!("{source} → {target} 转换会改变 env 值类型"),
                    )
                    .field(format!("{prefix}.env.{key}"))
                    .reason("VS Code 允许 number/null；目标客户端只接受字符串，默认不做强制转换")
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                Diagnostic::error(
                    codes::LOSSY_CONVERSION,
                    format!("{source} → {target} 转换会改变 env 表示"),
                )
                .field(format!("{prefix}.env")),
            ]
        })
}

fn unsupported(source: Client, target: Client, field: String, reason: &str) -> Diagnostic {
    Diagnostic::error(
        codes::UNSUPPORTED_CONVERSION,
        format!("{source} → {target} 无法无损转换字段"),
    )
    .field(field)
    .reason(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;
    use std::path::PathBuf;

    #[test]
    fn simple_stdio_conversion_is_lossless() {
        let parsed = parse_source(
            Client::Claude,
            PathBuf::from(".mcp.json"),
            r#"{"mcpServers":{"files":{"command":"npx","args":["-y","server"]}}}"#,
        )
        .unwrap();
        let report = convert(&parsed, Client::Codex);
        assert_eq!(report.status, ConversionStatus::Lossless);
        assert!(report.rendered.unwrap().contains("[mcp_servers.files]"));
    }

    #[test]
    fn dynamic_headers_are_unsupported_by_vscode() {
        let parsed = parse_source(
            Client::Claude,
            PathBuf::from(".mcp.json"),
            r#"{"mcpServers":{"api":{"type":"http","url":"https://example.com/mcp","headersHelper":"get-headers"}}}"#,
        )
        .unwrap();
        let report = convert(&parsed, Client::Vscode);
        assert_eq!(report.status, ConversionStatus::Unsupported);
        assert_eq!(report.diagnostics[0].code, codes::UNSUPPORTED_CONVERSION);
    }

    #[test]
    fn vscode_non_string_env_is_lossy() {
        let parsed = parse_source(
            Client::Vscode,
            PathBuf::from("mcp.json"),
            r#"{"servers":{"files":{"command":"node","env":{"PORT":3000}}}}"#,
        )
        .unwrap();
        let report = convert(&parsed, Client::Claude);
        assert_eq!(report.status, ConversionStatus::Lossy);
        assert!(report.rendered.is_none());
    }
}
