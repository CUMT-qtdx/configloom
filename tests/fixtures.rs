use configloom::diagnostic::codes;
use configloom::{Client, Transport, discover_project_config, parse_file};
use std::path::{Path, PathBuf};

#[test]
fn discovers_only_requested_project_paths() {
    let root = Path::new("example-project");
    assert_eq!(
        discover_project_config(Client::Claude, root),
        root.join(".mcp.json")
    );
    assert_eq!(
        discover_project_config(Client::Vscode, root),
        root.join(".vscode/mcp.json")
    );
    assert_eq!(
        discover_project_config(Client::Codex, root),
        root.join(".codex/config.toml")
    );
}

#[test]
fn parses_unicode_server_names_and_arguments() {
    let claude = parse_file(Client::Claude, &fixture("claude", "single-stdio.json")).unwrap();
    let Transport::Stdio { args, .. } = &claude.canonical.servers["filesystem"].transport else {
        panic!("expected stdio");
    };
    assert_eq!(args.last().unwrap(), "./文档");

    let codex = parse_file(Client::Codex, &fixture("codex", "multiple-servers.toml")).unwrap();
    assert!(codex.canonical.servers.contains_key("团队-api"));
}

#[test]
fn preserves_server_order() {
    for (client, path) in [
        (Client::Claude, fixture("claude", "multiple-servers.json")),
        (Client::Vscode, fixture("vscode", "multiple-servers.jsonc")),
        (Client::Codex, fixture("codex", "multiple-servers.toml")),
    ] {
        let parsed = parse_file(client, &path).unwrap();
        assert_eq!(
            parsed.canonical.servers.keys().next().unwrap(),
            "filesystem"
        );
    }
}

#[test]
fn parses_common_env_and_headers() {
    let env = parse_file(Client::Claude, &fixture("claude", "env.json")).unwrap();
    let Transport::Stdio { env, .. } = &env.canonical.servers["database"].transport else {
        panic!("expected stdio");
    };
    assert_eq!(env["NODE_ENV"], "production");

    let headers = parse_file(Client::Codex, &fixture("codex", "headers.toml")).unwrap();
    let Transport::StreamableHttp { headers, .. } = &headers.canonical.servers["github"].transport
    else {
        panic!("expected http");
    };
    assert_eq!(headers["X-Client"], "configloom-fixture");
}

#[test]
fn preserves_client_specific_and_unknown_fields_in_extensions() {
    let claude = parse_file(
        Client::Claude,
        &fixture("claude", "client-specific-fields.json"),
    )
    .unwrap();
    assert!(
        claude.canonical.servers["internal-api"]
            .extensions
            .claude
            .is_some()
    );

    let vscode = parse_file(Client::Vscode, &fixture("vscode", "unknown-fields.jsonc")).unwrap();
    assert!(
        vscode.canonical.servers["filesystem"]
            .extensions
            .vscode
            .is_some()
    );
    assert!(vscode.canonical.extensions.vscode.is_some());
    assert!(
        vscode
            .diagnostics
            .iter()
            .all(|item| item.code == codes::UNKNOWN_FIELD)
    );

    let codex = parse_file(Client::Codex, &fixture("codex", "unknown-fields.toml")).unwrap();
    assert!(
        codex.canonical.servers["filesystem"]
            .extensions
            .codex
            .is_some()
    );
    assert!(codex.canonical.extensions.codex.is_some());
}

#[test]
fn modified_canonical_model_cannot_rewrite_source_in_milestone_one() {
    let mut parsed = parse_file(Client::Claude, &fixture("claude", "single-stdio.json")).unwrap();
    parsed.canonical.servers.clear();
    let diagnostic = parsed.render().unwrap_err();
    assert_eq!(diagnostic.code, codes::UNSUPPORTED_CONVERSION);
}

#[test]
fn malformed_fixtures_return_stable_syntax_code() {
    for (client, path) in [
        (Client::Claude, fixture("claude", "malformed.json")),
        (Client::Vscode, fixture("vscode", "malformed.jsonc")),
        (Client::Codex, fixture("codex", "malformed.toml")),
    ] {
        let diagnostics = parse_file(client, &path).unwrap_err();
        assert_eq!(diagnostics[0].code, codes::MALFORMED);
    }
}

#[test]
fn invalid_command_type_returns_stable_field_code() {
    let source = r#"{"mcpServers":{"bad":{"command":["node","server.js"]}}}"#;
    let diagnostics =
        configloom::parse_source(Client::Claude, PathBuf::from(".mcp.json"), source).unwrap_err();
    assert_eq!(diagnostics[0].code, codes::FIELD_TYPE);
}

#[test]
fn sse_and_websocket_are_explicitly_unsupported() {
    for transport in ["sse", "websocket"] {
        let source = format!(
            r#"{{"servers":{{"remote":{{"type":"{transport}","url":"https://example.invalid"}}}}}}"#
        );
        let diagnostics =
            configloom::parse_source(Client::Vscode, PathBuf::from("mcp.json"), &source)
                .unwrap_err();
        assert_eq!(diagnostics[0].code, codes::UNSUPPORTED_TRANSPORT);
    }
}

fn fixture(client: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(client)
        .join(file)
}
