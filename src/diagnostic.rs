use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            field: None,
            reason: None,
        }
    }

    #[must_use]
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            field: None,
            reason: None,
        }
    }

    #[must_use]
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    #[must_use]
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.code, self.message)?;
        if let Some(field) = &self.field {
            write!(formatter, "\n  Field: {field}")?;
        }
        if let Some(reason) = &self.reason {
            write!(formatter, "\n  Reason: {reason}")?;
        }
        Ok(())
    }
}

pub mod codes {
    pub const IO_READ: &str = "IO001";
    pub const MALFORMED: &str = "CFG001";
    pub const ROOT_TYPE: &str = "CFG002";
    pub const CONTAINER: &str = "CFG003";
    pub const SERVER_TYPE: &str = "CFG004";
    pub const FIELD_TYPE: &str = "CFG005";
    pub const TRANSPORT_CONFLICT: &str = "CFG006";
    pub const UNKNOWN_FIELD: &str = "CFG007";
    pub const UNSUPPORTED_TRANSPORT: &str = "TRN001";
    pub const UNSUPPORTED_CONVERSION: &str = "CNV001";
    pub const LOSSY_CONVERSION: &str = "CNV002";
}
