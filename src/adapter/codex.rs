use crate::diagnostic::{Diagnostic, codes};
use crate::model::{
    CanonicalConfig, CanonicalServer, Client, ClientExtensions, ParsedConfig, SourceDocument,
    Transport,
};
use indexmap::IndexMap;
use std::path::PathBuf;
use toml::{Table, Value};
use toml_edit::{Array, DocumentMut, Item, Table as EditTable, Value as EditValue};

pub fn parse(path: PathBuf, source: &str) -> Result<ParsedConfig, Vec<Diagnostic>> {
    source.parse::<DocumentMut>().map_err(|error| {
        vec![Diagnostic::error(
            codes::MALFORMED,
            format!("TOML 语法错误: {error}"),
        )]
    })?;
    let value = toml::from_str::<Value>(source).map_err(|error| {
        vec![Diagnostic::error(
            codes::MALFORMED,
            format!("TOML 语法错误: {error}"),
        )]
    })?;
    let Value::Table(mut root) = value else {
        return Err(vec![Diagnostic::error(
            codes::ROOT_TYPE,
            "Codex .codex/config.toml 顶层必须是 table",
        )]);
    };
    let Some(Value::Table(server_values)) = root.remove("mcp_servers") else {
        return Err(vec![
            Diagnostic::error(codes::CONTAINER, "Codex 配置必须包含 mcp_servers table")
                .field("mcp_servers"),
        ]);
    };

    let mut diagnostics = unknown_toml_warnings(&root, KNOWN_TOP_LEVEL, "$.");
    let mut servers = IndexMap::new();
    for (name, value) in server_values {
        let Value::Table(table) = value else {
            diagnostics.push(
                Diagnostic::error(codes::SERVER_TYPE, "MCP Server 定义必须是 table")
                    .field(format!("mcp_servers.{name}")),
            );
            continue;
        };
        match parse_server(&name, table) {
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

    let extensions = if root.is_empty() {
        ClientExtensions::default()
    } else {
        ClientExtensions {
            codex: Some(Value::Table(root)),
            ..ClientExtensions::default()
        }
    };
    let canonical = CanonicalConfig {
        servers,
        extensions,
    };
    Ok(ParsedConfig::new(
        Client::Codex,
        path,
        canonical,
        diagnostics,
        SourceDocument::Toml(source.to_owned()),
    ))
}

fn parse_server(
    name: &str,
    mut table: Table,
) -> Result<(CanonicalServer, Vec<Diagnostic>), Diagnostic> {
    let field = format!("mcp_servers.{name}");
    let has_command = table.contains_key("command");
    let has_url = table.contains_key("url");
    if has_command == has_url {
        return Err(Diagnostic::error(
            codes::TRANSPORT_CONFLICT,
            "Codex Server 必须且只能声明 command 或 url 之一",
        )
        .field(field));
    }

    let transport = if has_command {
        let command = take_string(&mut table, "command", &format!("{field}.command"))?;
        let args = take_string_array(&mut table, "args", &format!("{field}.args"))?;
        let env = take_string_table(&mut table, "env", &format!("{field}.env"))?;
        let working_directory = take_optional_string(&mut table, "cwd", &format!("{field}.cwd"))?;
        Transport::Stdio {
            command,
            args,
            env,
            working_directory,
        }
    } else {
        let url = take_string(&mut table, "url", &format!("{field}.url"))?;
        let headers =
            take_string_table(&mut table, "http_headers", &format!("{field}.http_headers"))?;
        let dynamic_headers_command = take_optional_string(
            &mut table,
            "http_headers_helper",
            &format!("{field}.http_headers_helper"),
        )?;
        Transport::StreamableHttp {
            url,
            headers,
            dynamic_headers_command,
        }
    };

    let warnings = unknown_toml_warnings(&table, KNOWN_SERVER_EXTENSIONS, &format!("{field}."));
    let extensions = if table.is_empty() {
        ClientExtensions::default()
    } else {
        ClientExtensions {
            codex: Some(Value::Table(table)),
            ..ClientExtensions::default()
        }
    };
    Ok((
        CanonicalServer {
            transport,
            extensions,
        },
        warnings,
    ))
}

fn take_string(table: &mut Table, key: &str, field: &str) -> Result<String, Diagnostic> {
    match table.remove(key) {
        Some(Value::String(value)) => Ok(value),
        Some(_) => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串")).field(field))
        }
        None => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("缺少必填字段 {field}")).field(field))
        }
    }
}

fn take_optional_string(
    table: &mut Table,
    key: &str,
    field: &str,
) -> Result<Option<String>, Diagnostic> {
    match table.remove(key) {
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串")).field(field))
        }
        None => Ok(None),
    }
}

fn take_string_array(table: &mut Table, key: &str, field: &str) -> Result<Vec<String>, Diagnostic> {
    let Some(value) = table.remove(key) else {
        return Ok(Vec::new());
    };
    let Value::Array(values) = value else {
        return Err(
            Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串数组")).field(field),
        );
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            Value::String(value) => Ok(value),
            _ => Err(Diagnostic::error(
                codes::FIELD_TYPE,
                format!("{field}[{index}] 必须是字符串"),
            )
            .field(format!("{field}[{index}]"))),
        })
        .collect()
}

fn take_string_table(
    table: &mut Table,
    key: &str,
    field: &str,
) -> Result<IndexMap<String, String>, Diagnostic> {
    let Some(value) = table.remove(key) else {
        return Ok(IndexMap::new());
    };
    let Value::Table(values) = value else {
        return Err(
            Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是 table")).field(field),
        );
    };
    values
        .into_iter()
        .map(|(key, value)| match value {
            Value::String(value) => Ok((key, value)),
            _ => Err(
                Diagnostic::error(codes::FIELD_TYPE, format!("{field}.{key} 必须是字符串"))
                    .field(format!("{field}.{key}")),
            ),
        })
        .collect()
}

fn unknown_toml_warnings(
    table: &Table,
    known_extensions: &[&str],
    prefix: &str,
) -> Vec<Diagnostic> {
    table
        .keys()
        .filter(|key| !known_extensions.contains(&key.as_str()))
        .map(|key| {
            Diagnostic::warning(
                codes::UNKNOWN_FIELD,
                "未知字段已保存在 client extensions 中",
            )
            .field(format!("{prefix}{key}"))
        })
        .collect()
}

const KNOWN_SERVER_EXTENSIONS: &[&str] = &[
    "env_vars",
    "env_http_headers",
    "bearer_token_env_var",
    "environment_id",
    "auth",
    "startup_timeout_sec",
    "startup_timeout_ms",
    "tool_timeout_sec",
    "enabled",
    "required",
    "supports_parallel_tool_calls",
    "omit_tools_from",
    "default_tools_approval_mode",
    "enabled_tools",
    "disabled_tools",
    "scopes",
    "oauth",
    "oauth_resource",
    "name",
    "tools",
];

const KNOWN_TOP_LEVEL: &[&str] = &[
    "model",
    "model_provider",
    "approval_policy",
    "sandbox_mode",
    "features",
    "projects",
];

pub fn render_new(config: &CanonicalConfig) -> String {
    let mut document = DocumentMut::new();
    let mut servers = EditTable::new();
    servers.set_implicit(false);
    for (name, server) in &config.servers {
        let mut table = EditTable::new();
        match &server.transport {
            Transport::Stdio {
                command,
                args,
                env,
                working_directory,
            } => {
                table.insert("command", Item::Value(EditValue::from(command.as_str())));
                if !args.is_empty() {
                    table.insert("args", Item::Value(string_array(args)));
                }
                if !env.is_empty() {
                    let mut env_table = EditTable::new();
                    for (key, value) in env {
                        env_table.insert(key, Item::Value(EditValue::from(value.as_str())));
                    }
                    table.insert("env", Item::Table(env_table));
                }
                if let Some(cwd) = working_directory {
                    table.insert("cwd", Item::Value(EditValue::from(cwd.as_str())));
                }
            }
            Transport::StreamableHttp {
                url,
                headers,
                dynamic_headers_command,
            } => {
                table.insert("url", Item::Value(EditValue::from(url.as_str())));
                if !headers.is_empty() {
                    let mut header_table = EditTable::new();
                    for (key, value) in headers {
                        header_table.insert(key, Item::Value(EditValue::from(value.as_str())));
                    }
                    table.insert("http_headers", Item::Table(header_table));
                }
                if let Some(command) = dynamic_headers_command {
                    table.insert(
                        "http_headers_helper",
                        Item::Value(EditValue::from(command.as_str())),
                    );
                }
            }
        }
        servers.insert(name, Item::Table(table));
    }
    document["mcp_servers"] = Item::Table(servers);
    document.to_string()
}

fn string_array(values: &[String]) -> EditValue {
    let mut array = Array::new();
    for value in values {
        array.push(value.as_str());
    }
    EditValue::Array(array)
}
