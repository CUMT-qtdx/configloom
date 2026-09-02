use crate::model::{CanonicalConfig, Transport};
use serde_json::Value;

const REDACTED: &str = "<redacted>";

#[must_use]
pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let value = if is_sensitive_key(key) {
                        Value::String(REDACTED.to_owned())
                    } else {
                        redact_value(value)
                    };
                    (key.clone(), value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_value).collect()),
        _ => value.clone(),
    }
}

#[must_use]
pub fn redact_canonical(config: &CanonicalConfig) -> CanonicalConfig {
    let mut redacted = config.clone();
    for server in redacted.servers.values_mut() {
        match &mut server.transport {
            Transport::Stdio { env, .. } => {
                for (key, value) in env {
                    if is_sensitive_key(key) {
                        *value = REDACTED.to_owned();
                    }
                }
            }
            Transport::StreamableHttp {
                headers,
                dynamic_headers_command,
                ..
            } => {
                for (key, value) in headers {
                    if is_sensitive_key(key) {
                        *value = REDACTED.to_owned();
                    }
                }
                if dynamic_headers_command.is_some() {
                    *dynamic_headers_command = Some(REDACTED.to_owned());
                }
            }
        }
    }
    redacted
}

#[must_use]
pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized == "key"
        || normalized.ends_with("_key")
        || normalized.ends_with("-key")
        || normalized.contains("token")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("authorization")
        || normalized.contains("cookie")
        || normalized.contains("headershelper")
        || normalized.contains("headers_helper")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_nested_secret_values_but_not_commands() {
        let value = json!({
            "env": {"GITHUB_TOKEN": "secret", "MODE": "read-only"},
            "headers": {"Authorization": "Bearer secret"},
            "command": "monkey"
        });
        let redacted = redact_value(&value);
        assert_eq!(redacted["env"]["GITHUB_TOKEN"], REDACTED);
        assert_eq!(redacted["headers"]["Authorization"], REDACTED);
        assert_eq!(redacted["command"], "monkey");
    }

    #[test]
    fn redacts_canonical_env_headers_and_dynamic_header_commands() {
        let parsed = crate::parse_source(
            crate::Client::Claude,
            std::path::PathBuf::from(".mcp.json"),
            r#"{
              "mcpServers": {
                "local": {"command": "node", "env": {"API_TOKEN": "secret"}},
                "remote": {
                  "type": "http",
                  "url": "https://example.invalid/mcp",
                  "headers": {"Authorization": "Bearer secret"},
                  "headersHelper": "echo secret"
                }
              }
            }"#,
        )
        .unwrap();
        let redacted = redact_canonical(&parsed.canonical);
        let value = serde_json::to_value(redacted).unwrap();
        assert_eq!(
            value["servers"]["local"]["transport"]["env"]["API_TOKEN"],
            REDACTED
        );
        assert_eq!(
            value["servers"]["remote"]["transport"]["headers"]["Authorization"],
            REDACTED
        );
        assert_eq!(
            value["servers"]["remote"]["transport"]["dynamic_headers_command"],
            REDACTED
        );
    }
}
