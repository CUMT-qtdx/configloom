use clap::{Args, Parser, Subcommand};
use configloom::model::Client;
use configloom::redact::{redact_canonical, redact_value};
use configloom::{
    ConversionStatus, Diagnostic, convert, discover_project_config, parse_file, render_canonical,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "configloom", version, about = "只读检查和转换项目级 MCP 配置")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 读取并显示脱敏后的 Canonical Model
    Inspect(ReadArgs),
    /// 校验配置语法和已支持字段
    Validate(ReadArgs),
    /// 转换到另一个客户端；只向 stdout 输出无损结果
    Convert {
        #[command(flatten)]
        source: ReadArgs,
        #[arg(long, value_enum)]
        to: Client,
        /// 显式允许把 env/header 中的疑似凭据写到 stdout
        #[arg(long)]
        show_secrets: bool,
    },
}

#[derive(Debug, Args)]
struct ReadArgs {
    #[arg(value_enum)]
    client: Client,
    /// 显式配置路径；测试时可避免读取真实项目配置
    #[arg(long)]
    config: Option<PathBuf>,
    /// 未提供 --config 时用于发现项目级配置的根目录
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

fn run(cli: Cli) -> Result<(), u8> {
    match cli.command {
        Command::Inspect(args) => {
            let parsed = read(&args)?;
            println!("Client: {}", parsed.client);
            println!("Config: {}", parsed.path().display());
            println!("Servers: {}", parsed.canonical.servers.len());
            let value = serde_json::to_value(&parsed.canonical).map_err(|error| {
                eprintln!("无法序列化 Canonical Model: {error}");
                1
            })?;
            println!(
                "{}",
                serde_json::to_string_pretty(&redact_value(&value)).expect("JSON value 可序列化")
            );
            print_diagnostics(&parsed.diagnostics);
            Ok(())
        }
        Command::Validate(args) => {
            let parsed = read(&args)?;
            println!(
                "VALID: {} 个 MCP Server，配置 {}",
                parsed.canonical.servers.len(),
                parsed.path().display()
            );
            print_diagnostics(&parsed.diagnostics);
            Ok(())
        }
        Command::Convert {
            source,
            to,
            show_secrets,
        } => {
            let parsed = read(&source)?;
            if parsed.client == to {
                eprintln!("CNV001 source 和 target 必须是不同客户端");
                return Err(2);
            }
            let report = convert(&parsed, to);
            eprintln!(
                "Conversion: {} → {}\nStatus: {}",
                report.source, report.target, report.status
            );
            print_diagnostics(&report.diagnostics);
            match (report.status, report.rendered) {
                (ConversionStatus::Lossless, Some(raw_rendered)) => {
                    let rendered = if show_secrets {
                        raw_rendered
                    } else {
                        eprintln!("Output credentials: REDACTED");
                        render_canonical(to, &redact_canonical(&parsed.canonical))
                    };
                    print!("{rendered}");
                    Ok(())
                }
                _ => Err(2),
            }
        }
    }
}

fn read(args: &ReadArgs) -> Result<configloom::ParsedConfig, u8> {
    let path = args
        .config
        .clone()
        .unwrap_or_else(|| discover_project_config(args.client, &args.root));
    parse_file(args.client, &path).map_err(|diagnostics| {
        print_diagnostics(&diagnostics);
        1
    })
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!("{diagnostic}");
    }
}
