use super::{
    json_extensions, optional_string, parse_jsonc, required_string, string_array, string_map,
    unknown_json_warnings,
};
use crate::diagnostic::{Diagnostic, codes};
use crate::model::{
    CanonicalConfig, CanonicalServer, Client, ParsedConfig, SourceDocument, Transport,
};
use indexmap::IndexMap;
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use std::path::PathBuf;

pub fn parse(path: PathBuf, source: &str) -> Result<ParsedConfig, Vec<Diagnostic>> {
    let value = parse_jsonc(source).map_err(|error| vec![error])?;
    let JsonValue::Object(mut root) = value else {
        return Err(vec![Diagnostic::error(
            codes::ROOT_TYPE,
            "Claude Code .mcp.json 顶层必须是对象",
        )]);
    };
    let Some(JsonValue::Object(server_values)) = root.remove("mcpServers") else {
        return Err(vec![
            Diagnostic::error(codes::CONTAINER, "Claude Code 配置必须包含 mcpServers 对象")
                .field("mcpServers"),
        ]);
    };

    let mut diagnostics = unknown_json_warnings(&root, &[], "$");
    let mut servers = IndexMap::new();
    for (name, value) in server_values {
        let JsonValue::Object(object) = value else {
            diagnostics.push(
                Diagnostic::error(codes::SERVER_TYPE, "MCP Server 定义必须是对象")
                    .field(format!("mcpServers.{name}")),
            );
            continue;
        };
        match parse_server(&name, object) {
            Ok((server, warnings)) => {
                servers.insert(name, server);
                diagnostics.extend(warnings);
            }
            Err(error) => diagnostics.push(error),
        }
    }
    if diagnostics
        .iter()
        .any(|item| item.severity == crate::Severity::Error)
    {
        return Err(diagnostics);
    }

    let canonical = CanonicalConfig {
        servers,
        extensions: json_extensions(Client::Claude, root),
    };
    Ok(ParsedConfig::new(
        Client::Claude,
        path,
        canonical,
        diagnostics,
        SourceDocument::Jsonc(source.to_owned()),
    ))
}

fn parse_server(
    name: &str,
    mut object: JsonMap<String, JsonValue>,
) -> Result<(CanonicalServer, Vec<Diagnostic>), Diagnostic> {
    let field = format!("mcpServers.{name}");
    let transport_name = optional_string(&mut object, "type", &format!("{field}.type"))?;
    let transport_name = match transport_name.as_deref() {
        Some(value) => value,
        None if object.contains_key("command") => "stdio",
        None if object.contains_key("url") => {
            return Err(Diagnostic::error(
                codes::FIELD_TYPE,
                "Claude HTTP Server 必须显式声明 type",
            )
            .field(format!("{field}.type")));
        }
        None => {
            return Err(
                Diagnostic::error(codes::FIELD_TYPE, "无法判断 Server transport").field(field),
            );
        }
    };

    let transport = match transport_name {
        "stdio" => {
            if object.contains_key("url") {
                return Err(Diagnostic::error(
                    codes::TRANSPORT_CONFLICT,
                    "stdio Server 不能同时声明 url",
                )
                .field(format!("{field}.url")));
            }
            let command = required_string(&mut object, "command", &format!("{field}.command"))?;
            let args = string_array(&mut object, "args", &format!("{field}.args"))?;
            let env = string_map(&mut object, "env", &format!("{field}.env"))?;
            Transport::Stdio {
                command,
                args,
                env,
                working_directory: None,
            }
        }
        "http" | "streamable-http" => {
            if object.contains_key("command") {
                return Err(Diagnostic::error(
                    codes::TRANSPORT_CONFLICT,
                    "HTTP Server 不能同时声明 command",
                )
                .field(format!("{field}.command")));
            }
            let url = required_string(&mut object, "url", &format!("{field}.url"))?;
            let headers = string_map(&mut object, "headers", &format!("{field}.headers"))?;
            let dynamic_headers_command = optional_string(
                &mut object,
                "headersHelper",
                &format!("{field}.headersHelper"),
            )?;
            Transport::StreamableHttp {
                url,
                headers,
                dynamic_headers_command,
            }
        }
        "sse" | "websocket" | "ws" => {
            return Err(Diagnostic::error(
                codes::UNSUPPORTED_TRANSPORT,
                format!("Milestone 1 不支持 transport {transport_name}"),
            )
            .field(format!("{field}.type")));
        }
        other => {
            return Err(Diagnostic::error(
                codes::UNSUPPORTED_TRANSPORT,
                format!("未知 transport {other}"),
            )
            .field(format!("{field}.type")));
        }
    };

    let warnings = unknown_json_warnings(&object, &["alwaysLoad", "timeout", "oauth"], &field);
    Ok((
        CanonicalServer {
            transport,
            extensions: json_extensions(Client::Claude, object),
        },
        warnings,
    ))
}

pub fn render_new(config: &CanonicalConfig) -> String {
    let servers: JsonMap<String, JsonValue> = config
        .servers
        .iter()
        .map(|(name, server)| {
            let value = match &server.transport {
                Transport::Stdio {
                    command, args, env, ..
                } => {
                    let mut value = json!({"type": "stdio", "command": command});
                    let object = value.as_object_mut().expect("object literal");
                    if !args.is_empty() {
                        object.insert("args".to_owned(), json!(args));
                    }
                    if !env.is_empty() {
                        object.insert("env".to_owned(), json!(env));
                    }
                    value
                }
                Transport::StreamableHttp {
                    url,
                    headers,
                    dynamic_headers_command,
                } => {
                    let mut value = json!({"type": "http", "url": url});
                    let object = value.as_object_mut().expect("object literal");
                    if !headers.is_empty() {
                        object.insert("headers".to_owned(), json!(headers));
                    }
                    if let Some(command) = dynamic_headers_command {
                        object.insert("headersHelper".to_owned(), json!(command));
                    }
                    value
                }
            };
            (name.clone(), value)
        })
        .collect();
    serde_json::to_string_pretty(&json!({"mcpServers": servers})).expect("serializable") + "\n"
}
