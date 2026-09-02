use configloom::{Client, parse_source};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn claude_jsonc_parser_only_fixture_keeps_all_comment_positions() {
    assert_comment_markers(
        Client::Claude,
        fixture("claude", "comments.jsonc"),
        &["顶部注释", "MCP Server 区域", "相邻注释", "行尾注释"],
    );
}

#[test]
fn vscode_jsonc_keeps_all_comment_positions() {
    assert_comment_markers(
        Client::Vscode,
        fixture("vscode", "comments.jsonc"),
        &[
            "顶部注释",
            "Server section",
            "相邻字段注释",
            "行尾 transport",
        ],
    );
}

#[test]
fn codex_toml_keeps_all_comment_positions() {
    assert_comment_markers(
        Client::Codex,
        fixture("codex", "comments.toml"),
        &[
            "顶部注释",
            "行尾无关设置注释",
            "MCP Server section",
            "相邻字段注释",
            "行尾命令注释",
        ],
    );
}

fn assert_comment_markers(client: Client, path: PathBuf, markers: &[&str]) {
    let source = fs::read_to_string(&path).unwrap();
    let parsed = parse_source(client, path, &source).unwrap();
    let rendered = parsed.render().unwrap();
    assert_eq!(rendered, source);
    for marker in markers {
        assert_eq!(
            rendered.matches(marker).count(),
            1,
            "注释标记 {marker} 数量变化"
        );
    }
}

fn fixture(client: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(client)
        .join(file)
}
