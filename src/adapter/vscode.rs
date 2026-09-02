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
            "VS Code .vscode/mcp.json 顶层必须是对象",
        )]);
    };
    let Some(JsonValue::Object(server_values)) = root.remove("servers") else {
        return Err(vec![
            Diagnostic::error(codes::CONTAINER, "VS Code 配置必须包含 servers 对象")
                .field("servers"),
        ]);
    };

    let mut diagnostics = unknown_json_warnings(&root, &["inputs", "sandbox"], "$");
    let mut servers = IndexMap::new();
    for (name, value) in server_values {
        let JsonValue::Object(object) = value else {
            diagnostics.push(
                Diagnostic::error(codes::SERVER_TYPE, "MCP Server 定义必须是对象")
                    .field(format!("servers.{name}")),
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
        extensions: json_extensions(Client::Vscode, root),
    };
    Ok(ParsedConfig::new(
        Client::Vscode,
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
    let field = format!("servers.{name}");
    let transport_name = optional_string(&mut object, "type", &format!("{field}.type"))?;
    let transport_name = match transport_name.as_deref() {
        Some(value) => value,
        None if object.contains_key("command") => "stdio",
        None => {
            return Err(Diagnostic::error(
                codes::FIELD_TYPE,
                "VS Code Server 必须声明 type，或提供可推断的 command",
            )
            .field(format!("{field}.type")));
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
            let env_value = object.remove("env");
            let mut env = IndexMap::new();
            if let Some(value) = env_value {
                let JsonValue::Object(values) = value else {
                    return Err(Diagnostic::error(codes::FIELD_TYPE, "env 必须是对象")
                        .field(format!("{field}.env")));
                };
                let mut non_string = JsonMap::new();
                for (key, value) in values {
                    if let JsonValue::String(value) = value {
                        env.insert(key, value);
                    } else {
                        non_string.insert(key, value);
                    }
                }
                if !non_string.is_empty() {
                    object.insert("env".to_owned(), JsonValue::Object(non_string));
                }
            }
            let working_directory = optional_string(&mut object, "cwd", &format!("{field}.cwd"))?;
            Transport::Stdio {
                command,
                args,
                env,
                working_directory,
            }
        }
        "http" => {
            if object.contains_key("command") {
                return Err(Diagnostic::error(
                    codes::TRANSPORT_CONFLICT,
                    "HTTP Server 不能同时声明 command",
                )
                .field(format!("{field}.command")));
            }
            let url = required_string(&mut object, "url", &format!("{field}.url"))?;
            let headers = string_map(&mut object, "headers", &format!("{field}.headers"))?;
            Transport::StreamableHttp {
                url,
                headers,
                dynamic_headers_command: None,
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

    let warnings = unknown_json_warnings(
        &object,
        &["env", "envFile", "dev", "sandboxEnabled", "oauth"],
        &field,
    );
    Ok((
        CanonicalServer {
            transport,
            extensions: json_extensions(Client::Vscode, object),
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
                    command,
                    args,
                    env,
                    working_directory,
                } => {
                    let mut value = json!({"type": "stdio", "command": command});
                    let object = value.as_object_mut().expect("object literal");
                    if !args.is_empty() {
                        object.insert("args".to_owned(), json!(args));
                    }
                    if !env.is_empty() {
                        object.insert("env".to_owned(), json!(env));
                    }
                    if let Some(cwd) = working_directory {
                        object.insert("cwd".to_owned(), json!(cwd));
                    }
                    value
                }
                Transport::StreamableHttp { url, headers, .. } => {
                    let mut value = json!({"type": "http", "url": url});
                    if !headers.is_empty() {
                        value
                            .as_object_mut()
                            .expect("object literal")
                            .insert("headers".to_owned(), json!(headers));
                    }
                    value
                }
            };
            (name.clone(), value)
        })
        .collect();
    serde_json::to_string_pretty(&json!({"servers": servers})).expect("serializable") + "\n"
}
