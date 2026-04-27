//! `memoryctl edit` — `$EDITOR` 集成。

use std::process::Command;

use memoryctl_core::{Error, Result};
use memoryctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long)]
    pub topic: String,
    #[arg(long, default_value = "global")]
    pub scope: String,
    /// 不存在时创建。
    #[arg(long)]
    pub new: bool,
}

#[derive(Serialize)]
struct Out {
    success: bool,
    action: &'static str,
    topic: String,
    path: camino::Utf8PathBuf,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scope = util::parse_scope(&args.scope)?;
    let path = store.topic_path(&scope, &args.topic)?;

    if !path.exists() {
        if !args.new {
            return Err(Error::TopicNotFound(args.topic));
        }
        if let Some(parent) = path.parent() {
            fs_err::create_dir_all(parent.as_std_path())
                .map_err(|e| Error::Io { path: parent.to_owned(), source: e })?;
        }
        fs_err::write(path.as_std_path(), format!("# {}\n", args.topic))
            .map_err(|e| Error::Io { path: path.clone(), source: e })?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = Command::new(&editor)
        .arg(path.as_std_path())
        .status()
        .map_err(|e| Error::other(format!("{editor}: {e}")))?;
    if !status.success() {
        return Err(Error::other(format!("{editor} exited with non-zero")));
    }

    // 退出后做一次 validate
    let raw = fs_err::read_to_string(path.as_std_path())
        .map_err(|e| Error::Io { path: path.clone(), source: e })?;
    memoryctl_entry::parse_file(&raw)?; // 解析失败即报错

    let out = Out { success: true, action: "edit", topic: args.topic, path };
    util::emit(fmt, &out, |o| {
        println!("edited {}", o.path);
        Ok(())
    })
}
