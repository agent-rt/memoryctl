//! `memctl save` — 添加一条记忆。

use std::io::Read;

use memctl_core::{validate_topic_name, EntryType, Error, Result, Scope};
use memctl_entry::{Entry, EntrySource};
use memctl_protocol::{SaveResponse, PROTOCOL_VERSION};
use memctl_store::Store;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// 7 类之一：lesson / decision / fact / feedback / reference / user / project
    #[arg(long, value_enum)]
    pub r#type: TypeArg,
    /// topic 名（自动创建）
    #[arg(long)]
    pub topic: String,
    /// 记忆内容（位置参数）。与 --from-stdin 二选一。
    pub content: Option<String>,
    /// 从 stdin 读取内容。
    #[arg(long)]
    pub from_stdin: bool,
    /// 作用域：global / project / agent:<name>。默认 global。
    #[arg(long, default_value = "global")]
    pub scope: String,
    /// 覆盖默认来源标记（agent 名 + 项目路径）。
    #[arg(long)]
    pub source: Option<String>,
    /// 跳过交互确认。
    #[arg(long)]
    pub no_confirm: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TypeArg {
    Lesson,
    Decision,
    Fact,
    Feedback,
    Reference,
    User,
    Project,
}

impl From<TypeArg> for EntryType {
    fn from(t: TypeArg) -> Self {
        match t {
            TypeArg::Lesson => EntryType::Lesson,
            TypeArg::Decision => EntryType::Decision,
            TypeArg::Fact => EntryType::Fact,
            TypeArg::Feedback => EntryType::Feedback,
            TypeArg::Reference => EntryType::Reference,
            TypeArg::User => EntryType::User,
            TypeArg::Project => EntryType::Project,
        }
    }
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    validate_topic_name(&args.topic)?;
    let entry_type: EntryType = args.r#type.into();
    let scope = util::parse_scope(&args.scope)?;

    let content = match (args.content.clone(), args.from_stdin) {
        (Some(c), false) => c,
        (None, true) => read_stdin()?,
        (Some(_), true) => {
            return Err(Error::other("pass either positional content or --from-stdin, not both"))
        }
        (None, false) => {
            return Err(Error::other("missing content; pass it as positional arg or --from-stdin"))
        }
    };
    if content.trim().is_empty() {
        return Err(Error::other("content is empty"));
    }

    let store = Store::default_open()?;
    let path = store.topic_path(&scope, &args.topic)?;

    // ensure parent dirs by scope
    match &scope {
        Scope::Global | Scope::Agent(_) => store.ensure_global_dirs()?,
        Scope::Project => {
            if store.project_root.is_none() {
                return Err(Error::other(
                    "no project found; run `memctl init` first or use --scope global",
                ));
            }
            store.ensure_project_dirs()?;
        }
    }

    if !args.no_confirm && !is_non_interactive() {
        // 简单 stdin 确认
        eprintln!("Save to {}? [y/N]", path);
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        if !matches!(buf.trim(), "y" | "Y" | "yes") {
            return Err(Error::other("aborted by user"));
        }
    }

    let source = build_source(args.source.as_deref());
    let entry = Entry::now(entry_type, source, content);
    memctl_topic::append(&path, &args.topic, &entry)?;

    // 计数：重新读 topic 看 entries 长度
    let topic = memctl_topic::read(&path)?.ok_or_else(|| Error::other("topic write failed"))?;
    let resp = SaveResponse {
        protocol: PROTOCOL_VERSION,
        success: true,
        action: "save".into(),
        topic: args.topic,
        entry_type,
        scope: scope.label(),
        path: path.clone(),
        entry_index: topic.entries.len(),
    };
    util::emit(fmt, &resp, |r| {
        println!("saved entry #{} to {} ({})", r.entry_index, r.path, r.scope);
        Ok(())
    })
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).map_err(|e| Error::other(format!("stdin: {e}")))?;
    Ok(buf)
}

fn is_non_interactive() -> bool {
    // 非常简单的启发：如果 stdin 不是 tty，则认为非交互
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

fn build_source(override_str: Option<&str>) -> EntrySource {
    if let Some(s) = override_str {
        return EntrySource::parse(s);
    }
    EntrySource { agent: util::agent_name(), project: util::project_label() }
}
