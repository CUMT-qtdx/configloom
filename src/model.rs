use crate::diagnostic::{Diagnostic, codes};
use clap::ValueEnum;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::fmt;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Client {
    Claude,
    Vscode,
    Codex,
}

impl Client {
    #[must_use]
    pub const fn project_relative_path(self) -> &'static str {
        match self {
            Self::Claude => ".mcp.json",
            Self::Vscode => ".vscode/mcp.json",
            Self::Codex => ".codex/config.toml",
        }
    }
}

impl fmt::Display for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Claude => write!(formatter, "Claude Code"),
            Self::Vscode => write!(formatter, "VS Code"),
            Self::Codex => write!(formatter, "Codex"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct CanonicalConfig {
    pub servers: IndexMap<String, CanonicalServer>,
    #[serde(skip_serializing_if = "ClientExtensions::is_empty")]
    pub extensions: ClientExtensions,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanonicalServer {
    pub transport: Transport,
    #[serde(skip_serializing_if = "ClientExtensions::is_empty")]
    pub extensions: ClientExtensions,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        env: IndexMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
    },
    StreamableHttp {
        url: String,
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        headers: IndexMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dynamic_headers_command: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ClientExtensions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vscode: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex: Option<TomlValue>,
}

impl ClientExtensions {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.claude.is_none() && self.vscode.is_none() && self.codex.is_none()
    }

    #[must_use]
    pub fn for_client(&self, client: Client) -> Option<ExtensionRef<'_>> {
        match client {
            Client::Claude => self.claude.as_ref().map(ExtensionRef::Json),
            Client::Vscode => self.vscode.as_ref().map(ExtensionRef::Json),
            Client::Codex => self.codex.as_ref().map(ExtensionRef::Toml),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionRef<'a> {
    Json(&'a JsonValue),
    Toml(&'a TomlValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceDocument {
    Jsonc(String),
    Toml(String),
}

#[derive(Debug, Clone)]
pub struct ParsedConfig {
    pub client: Client,
    pub path: PathBuf,
    pub canonical: CanonicalConfig,
    pub diagnostics: Vec<Diagnostic>,
    source: SourceDocument,
    original: CanonicalConfig,
}

impl ParsedConfig {
    #[must_use]
    pub fn new(
        client: Client,
        path: PathBuf,
        canonical: CanonicalConfig,
        diagnostics: Vec<Diagnostic>,
        source: SourceDocument,
    ) -> Self {
        let original = canonical.clone();
        Self {
            client,
            path,
            canonical,
            diagnostics,
            source,
            original,
        }
    }

    pub fn render(&self) -> Result<String, Diagnostic> {
        if self.canonical != self.original {
            return Err(Diagnostic::error(
                codes::UNSUPPORTED_CONVERSION,
                "Milestone 1 不会重写已修改的原始语法树",
            )
            .reason("只允许未修改配置的保真 round-trip；安全修改属于 Milestone 2"));
        }

        match &self.source {
            SourceDocument::Jsonc(source) => {
                let root = jsonc_parser::cst::CstRootNode::parse(source, &Default::default())
                    .map_err(|error| Diagnostic::error(codes::MALFORMED, error.to_string()))?;
                Ok(root.to_string())
            }
            SourceDocument::Toml(source) => {
                let document = source
                    .parse::<toml_edit::DocumentMut>()
                    .map_err(|error| Diagnostic::error(codes::MALFORMED, error.to_string()))?;
                Ok(document.to_string())
            }
        }
    }

    #[must_use]
    pub fn source_text(&self) -> &str {
        match &self.source {
            SourceDocument::Jsonc(source) | SourceDocument::Toml(source) => source,
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversionStatus {
    Lossless,
    Lossy,
    Unsupported,
}

impl fmt::Display for ConversionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lossless => write!(formatter, "LOSSLESS"),
            Self::Lossy => write!(formatter, "LOSSY"),
            Self::Unsupported => write!(formatter, "UNSUPPORTED"),
        }
    }
}
