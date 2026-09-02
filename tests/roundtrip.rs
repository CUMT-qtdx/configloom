use configloom::{Client, parse_source};
use std::fs;
use std::path::{Path, PathBuf};

const JSON_FIXTURES: &[&str] = &[
    "empty",
    "single-stdio",
    "multiple-servers",
    "http",
    "env",
    "headers",
    "client-specific-fields",
    "unrelated-settings",
    "comments",
    "unknown-fields",
];

#[test]
fn claude_fixtures_have_byte_exact_and_semantic_round_trip() {
    assert_round_trip(Client::Claude, "claude", "json", "jsonc");
}

#[test]
fn vscode_fixtures_have_byte_exact_and_semantic_round_trip() {
    assert_round_trip(Client::Vscode, "vscode", "jsonc", "jsonc");
}

#[test]
fn codex_fixtures_have_byte_exact_and_semantic_round_trip() {
    for name in JSON_FIXTURES {
        let path = fixture("codex", &format!("{name}.toml"));
        let source = fs::read_to_string(&path).unwrap();
        let parsed = parse_source(Client::Codex, path.clone(), &source).unwrap();
        let rendered = parsed.render().unwrap();
        assert_eq!(rendered, source, "{} 不是字节级保真", path.display());
        let reparsed = parse_source(Client::Codex, path, &rendered).unwrap();
        assert_eq!(reparsed.canonical, parsed.canonical);
    }
}

fn assert_round_trip(client: Client, directory: &str, default_extension: &str, comments: &str) {
    for name in JSON_FIXTURES {
        let extension = if *name == "comments" {
            comments
        } else {
            default_extension
        };
        let path = fixture(directory, &format!("{name}.{extension}"));
        let source = fs::read_to_string(&path).unwrap();
        let parsed = parse_source(client, path.clone(), &source).unwrap();
        let rendered = parsed.render().unwrap();
        assert_eq!(rendered, source, "{} 不是字节级保真", path.display());
        let reparsed = parse_source(client, path, &rendered).unwrap();
        assert_eq!(reparsed.canonical, parsed.canonical);
    }
}

fn fixture(client: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(client)
        .join(file)
}
