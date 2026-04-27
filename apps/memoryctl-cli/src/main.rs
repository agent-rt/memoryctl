//! memoryctl 二进制入口。

#![forbid(unsafe_code)]
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod commands;

#[derive(Debug, Parser)]
#[command(
    name = "memoryctl",
    version,
    about = "Persistent agent memory layer (cross-tool / cross-project / cross-session)"
)]
struct Cli {
    /// 输出格式：human / json / tsv。tsv 推荐给 Agent。
    #[arg(long, global = true, default_value = "human")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    /// 人类视图。
    Human,
    /// 结构化 JSON。
    Json,
    /// 紧凑 TSV，Agent 推荐。
    Tsv,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 起手项目：创建 .memoryctl/ + 注入 AGENTS.md 块。
    Init(commands::init::Args),
    /// 写入 AGENTS.md memoryctl 协议入口块。
    Enable(commands::enable::Args),
    /// 移除 AGENTS.md memoryctl 协议入口块。
    Disable(commands::disable::Args),

    /// 添加一条记忆（必填 --type --topic）。
    Save(commands::save::Args),
    /// 列出 topic 摘要。
    List(commands::list::Args),
    /// 别名：list。
    Topics(commands::list::Args),
    /// 读取 topic 内容。
    Read(commands::read::Args),
    /// 跨 topic 全文搜索。
    Search(commands::search::Args),
    /// 移除 entry 或整个 topic。
    Forget(commands::forget::Args),
    /// 用 $EDITOR 打开 topic。
    Edit(commands::edit::Args),
    /// 在 scope 之间迁移 topic。
    Move(commands::move_cmd::Args),
    /// 校验 topic 文件格式。
    Validate(commands::validate::Args),
    /// 导出 topic（用于提升为 SKILL.md）。
    Export(commands::export::Args),
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let format = cli.format;
    match commands::dispatch(cli.command, format) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            emit_error(format, &err);
            ExitCode::FAILURE
        }
    }
}

fn emit_error(format: OutputFormat, err: &memoryctl_core::Error) {
    use memoryctl_core::Error;
    use memoryctl_protocol::{ErrorEnvelope, PROTOCOL_VERSION};

    let code = match err {
        Error::TopicNotFound(_) => "topic_not_found",
        Error::InvalidTopicName(_) => "invalid_topic_name",
        Error::InvalidType(_) => "invalid_type",
        Error::InvalidScope(_) => "invalid_scope",
        Error::InvalidEntry(_) => "invalid_entry",
        Error::NotAProject(_) => "not_a_project",
        Error::EntryOutOfRange(_) => "entry_out_of_range",
        Error::Io { .. } => "io_error",
        Error::Other(_) => "error",
    };

    match format {
        OutputFormat::Json => {
            let env = ErrorEnvelope {
                protocol: PROTOCOL_VERSION,
                success: false,
                error: code.into(),
                hint: Some(err.to_string()),
            };
            match serde_json::to_string_pretty(&env) {
                Ok(s) => println!("{s}"),
                Err(_) => eprintln!("error: {err}"),
            }
        }
        OutputFormat::Tsv => {
            let hint = err.to_string();
            println!("ERROR\t{code}\t{}", hint.replace(['\t', '\n', '\r'], " "));
        }
        OutputFormat::Human => eprintln!("error: {err}"),
    }
}
