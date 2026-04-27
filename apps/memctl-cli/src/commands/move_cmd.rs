//! `memctl move` — 在 scope 之间迁移 topic。

use memctl_core::{Error, Result};
use memctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long)]
    pub topic: String,
    #[arg(long, default_value = "global")]
    pub from_scope: String,
    #[arg(long)]
    pub to_scope: String,
}

#[derive(Serialize)]
struct Out {
    success: bool,
    action: &'static str,
    topic: String,
    from: String,
    to: String,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let from = util::parse_scope(&args.from_scope)?;
    let to = util::parse_scope(&args.to_scope)?;
    if from == to {
        return Err(Error::other("--from-scope and --to-scope are the same"));
    }
    let from_path = store.topic_path(&from, &args.topic)?;
    let to_path = store.topic_path(&to, &args.topic)?;

    let topic =
        memctl_topic::read(&from_path)?.ok_or_else(|| Error::TopicNotFound(args.topic.clone()))?;

    // 确保目标 dir 存在
    if let Some(parent) = to_path.parent() {
        fs_err::create_dir_all(parent.as_std_path())
            .map_err(|e| Error::Io { path: parent.to_owned(), source: e })?;
    }

    memctl_topic::write_full(&to_path, &topic)?;
    memctl_topic::remove(&from_path)?;

    let out = Out {
        success: true,
        action: "move",
        topic: args.topic,
        from: from.label(),
        to: to.label(),
    };
    util::emit(fmt, &out, |o| {
        println!("moved {} from {} to {}", o.topic, o.from, o.to);
        Ok(())
    })
}
