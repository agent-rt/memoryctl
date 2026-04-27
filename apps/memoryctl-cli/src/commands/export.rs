//! `memoryctl export` — 导出 topic 为可编辑 markdown，用于提升为 SKILL.md 起手。

use memoryctl_core::{Error, Result};
use memoryctl_store::Store;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long)]
    pub topic: String,
    #[arg(long, default_value = "global")]
    pub scope: String,
}

pub fn run(args: Args, _fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scope = util::parse_scope(&args.scope)?;
    let path = store.topic_path(&scope, &args.topic)?;
    let topic =
        memoryctl_topic::read(&path)?.ok_or_else(|| Error::TopicNotFound(args.topic.clone()))?;

    // 简单合并：按 type 分组，组内按时间升序
    use std::collections::BTreeMap;
    let mut by_type: BTreeMap<&'static str, Vec<&memoryctl_entry::Entry>> = BTreeMap::new();
    for entry in &topic.entries {
        by_type.entry(entry.entry_type.as_str()).or_default().push(entry);
    }
    for entries in by_type.values_mut() {
        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    println!("# {}", topic.name);
    println!();
    println!("> Exported from memoryctl on {}", chrono::Local::now().format("%Y-%m-%d"));
    println!();
    for (ty, entries) in by_type {
        println!("## {}", ty);
        println!();
        for e in entries {
            println!(
                "- {} _(by {})_",
                e.content.lines().next().unwrap_or(""),
                e.source.formatted()
            );
            for line in e.content.lines().skip(1) {
                println!("  {line}");
            }
        }
        println!();
    }
    Ok(())
}
