mod claude;
mod codex;
mod vscode;

use crate::diagnostic::{Diagnostic, codes};
use crate::model::{Client, ClientExtensions, ParsedConfig};
use indexmap::IndexMap;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::fs;
use std::path::{Path, PathBuf};

#[must_use]
pub fn discover_project_config(client: Client, project_root: &Path) -> PathBuf {
    project_root.join(client.project_relative_path())
}

pub fn parse_file(client: Client, path: &Path) -> Result<ParsedConfig, Vec<Diagnostic>> {
    let source = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            codes::IO_READ,
            format!("无法读取配置 {}: {error}", path.display()),
        )]
    })?;
    parse_source(client, path.to_path_buf(), &source)
}

pub fn parse_source(
    client: Client,
    path: PathBuf,
    source: &str,
) -> Result<ParsedConfig, Vec<Diagnostic>> {
    match client {
        Client::Claude => claude::parse(path, source),
        Client::Vscode => vscode::parse(path, source),
        Client::Codex => codex::parse(path, source),
    }
}

pub(crate) fn parse_jsonc(source: &str) -> Result<JsonValue, Diagnostic> {
    jsonc_parser::cst::CstRootNode::parse(source, &Default::default()).map_err(|error| {
        Diagnostic::error(codes::MALFORMED, format!("JSON/JSONC 语法错误: {error}"))
    })?;
    jsonc_parser::parse_to_serde_value::<JsonValue>(source, &Default::default()).map_err(|error| {
        Diagnostic::error(codes::MALFORMED, format!("JSON/JSONC 语法错误: {error}"))
    })
}

pub(crate) fn required_string(
    object: &mut JsonMap<String, JsonValue>,
    key: &str,
    field: &str,
) -> Result<String, Diagnostic> {
    match object.remove(key) {
        Some(JsonValue::String(value)) => Ok(value),
        Some(_) => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串")).field(field))
        }
        None => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("缺少必填字段 {field}")).field(field))
        }
    }
}

pub(crate) fn optional_string(
    object: &mut JsonMap<String, JsonValue>,
    key: &str,
    field: &str,
) -> Result<Option<String>, Diagnostic> {
    match object.remove(key) {
        Some(JsonValue::String(value)) => Ok(Some(value)),
        Some(_) => {
            Err(Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串")).field(field))
        }
        None => Ok(None),
    }
}

pub(crate) fn string_array(
    object: &mut JsonMap<String, JsonValue>,
    key: &str,
    field: &str,
) -> Result<Vec<String>, Diagnostic> {
    let Some(value) = object.remove(key) else {
        return Ok(Vec::new());
    };
    let JsonValue::Array(values) = value else {
        return Err(
            Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是字符串数组")).field(field),
        );
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            JsonValue::String(value) => Ok(value),
            _ => Err(Diagnostic::error(
                codes::FIELD_TYPE,
                format!("{field}[{index}] 必须是字符串"),
            )
            .field(format!("{field}[{index}]"))),
        })
        .collect()
}

pub(crate) fn string_map(
    object: &mut JsonMap<String, JsonValue>,
    key: &str,
    field: &str,
) -> Result<IndexMap<String, String>, Diagnostic> {
    let Some(value) = object.remove(key) else {
        return Ok(IndexMap::new());
    };
    let JsonValue::Object(values) = value else {
        return Err(
            Diagnostic::error(codes::FIELD_TYPE, format!("{field} 必须是对象")).field(field),
        );
    };
    values
        .into_iter()
        .map(|(key, value)| match value {
            JsonValue::String(value) => Ok((key, value)),
            _ => Err(
                Diagnostic::error(codes::FIELD_TYPE, format!("{field}.{key} 必须是字符串"))
                    .field(format!("{field}.{key}")),
            ),
        })
        .collect()
}

pub(crate) fn json_extensions(
    client: Client,
    object: JsonMap<String, JsonValue>,
) -> ClientExtensions {
    if object.is_empty() {
        return ClientExtensions::default();
    }
    let value = JsonValue::Object(object);
    match client {
        Client::Claude => ClientExtensions {
            claude: Some(value),
            ..ClientExtensions::default()
        },
        Client::Vscode => ClientExtensions {
            vscode: Some(value),
            ..ClientExtensions::default()
        },
        Client::Codex => unreachable!("Codex 使用 TOML extensions"),
    }
}

pub(crate) fn unknown_json_warnings(
    object: &JsonMap<String, JsonValue>,
    known_extensions: &[&str],
    prefix: &str,
) -> Vec<Diagnostic> {
    object
        .keys()
        .filter(|key| !known_extensions.contains(&key.as_str()))
        .map(|key| {
            let field = format!("{prefix}.{key}");
            Diagnostic::warning(
                codes::UNKNOWN_FIELD,
                "未知字段已保存在 client extensions 中",
            )
            .field(field)
        })
        .collect()
}

pub fn render_canonical(client: Client, config: &crate::model::CanonicalConfig) -> String {
    match client {
        Client::Claude => claude::render_new(config),
        Client::Vscode => vscode::render_new(config),
        Client::Codex => codex::render_new(config),
    }
}
