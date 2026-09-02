use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

#[test]
fn inspect_redacts_suspected_secrets() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["inspect", "claude", "--config"])
        .arg(fixture("claude", "env.json"))
        .assert()
        .success()
        .stdout(predicate::str::contains("<redacted>"))
        .stdout(predicate::str::contains("example-token").not());
}

#[test]
fn validate_reports_malformed_code() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["validate", "codex", "--config"])
        .arg(fixture("codex", "malformed.toml"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("CFG001"));
}

#[test]
fn convert_writes_only_lossless_config_to_stdout() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["convert", "claude", "--config"])
        .arg(fixture("claude", "single-stdio.json"))
        .args(["--to", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[mcp_servers.filesystem]"))
        .stderr(predicate::str::contains("Status: LOSSLESS"));
}

#[test]
fn convert_refuses_unsupported_output() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["convert", "claude", "--config"])
        .arg(fixture("claude", "client-specific-fields.json"))
        .args(["--to", "vscode"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("UNSUPPORTED"))
        .stderr(predicate::str::contains("CNV001"));
}

#[test]
fn convert_redacts_header_credentials_by_default() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["convert", "claude", "--config"])
        .arg(fixture("claude", "headers.json"))
        .args(["--to", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<redacted>"))
        .stdout(predicate::str::contains("example-token").not())
        .stderr(predicate::str::contains("Output credentials: REDACTED"));
}

#[test]
fn convert_requires_explicit_flag_to_show_header_credentials() {
    Command::cargo_bin("configloom")
        .unwrap()
        .args(["convert", "claude", "--config"])
        .arg(fixture("claude", "headers.json"))
        .args(["--to", "codex", "--show-secrets"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example-token"));
}

fn fixture(client: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(client)
        .join(file)
}
