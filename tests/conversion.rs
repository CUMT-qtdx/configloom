use configloom::diagnostic::codes;
use configloom::{Client, ConversionStatus, convert, parse_file};
use std::path::{Path, PathBuf};

#[test]
fn portable_stdio_converts_losslessly_to_both_targets() {
    let parsed = parse_file(Client::Claude, &fixture("claude", "single-stdio.json")).unwrap();
    for target in [Client::Vscode, Client::Codex] {
        let report = convert(&parsed, target);
        assert_eq!(report.status, ConversionStatus::Lossless);
        assert_generated_model(&parsed.canonical, target, report.rendered.unwrap());
    }
}

#[test]
fn portable_http_headers_convert_losslessly() {
    let parsed = parse_file(Client::Claude, &fixture("claude", "headers.json")).unwrap();
    for target in [Client::Vscode, Client::Codex] {
        let report = convert(&parsed, target);
        assert_eq!(report.status, ConversionStatus::Lossless);
        assert_generated_model(&parsed.canonical, target, report.rendered.unwrap());
    }
}

#[test]
fn cwd_is_lossless_between_vscode_and_codex_but_not_claude() {
    let source = r#"{"servers":{"dev":{"type":"stdio","command":"node","cwd":"./tools"}}}"#;
    let parsed =
        configloom::parse_source(Client::Vscode, PathBuf::from("mcp.json"), source).unwrap();
    assert_eq!(
        convert(&parsed, Client::Codex).status,
        ConversionStatus::Lossless
    );
    assert_eq!(
        convert(&parsed, Client::Claude).status,
        ConversionStatus::Unsupported
    );
}

#[test]
fn dynamic_headers_are_lossless_between_claude_and_codex() {
    let source = r#"{"mcpServers":{"api":{"type":"http","url":"https://example.invalid/mcp","headersHelper":"./headers"}}}"#;
    let parsed =
        configloom::parse_source(Client::Claude, PathBuf::from(".mcp.json"), source).unwrap();
    let report = convert(&parsed, Client::Codex);
    assert_eq!(report.status, ConversionStatus::Lossless);
    let rendered = report.rendered.unwrap();
    assert!(rendered.contains("http_headers_helper"));
    assert_generated_model(&parsed.canonical, Client::Codex, rendered);
}

#[test]
fn client_extensions_block_cross_client_output() {
    let parsed = parse_file(
        Client::Codex,
        &fixture("codex", "client-specific-fields.toml"),
    )
    .unwrap();
    let report = convert(&parsed, Client::Claude);
    assert_eq!(report.status, ConversionStatus::Unsupported);
    assert!(report.rendered.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .all(|item| item.code == codes::UNSUPPORTED_CONVERSION)
    );
}

#[test]
fn vscode_number_and_null_env_are_lossy_not_silent() {
    let parsed = parse_file(Client::Vscode, &fixture("vscode", "env.jsonc")).unwrap();
    let report = convert(&parsed, Client::Codex);
    assert_eq!(report.status, ConversionStatus::Lossy);
    assert!(report.rendered.is_none());
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|item| item.code == codes::LOSSY_CONVERSION)
            .count(),
        2
    );
}

fn fixture(client: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(client)
        .join(file)
}

fn assert_generated_model(
    expected: &configloom::CanonicalConfig,
    target: Client,
    rendered: String,
) {
    let reparsed = configloom::parse_source(target, PathBuf::from("generated"), &rendered).unwrap();
    assert_eq!(&reparsed.canonical, expected);
}
