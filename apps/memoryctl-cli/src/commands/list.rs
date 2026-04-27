//! `memoryctl list` / `memoryctl topics` — 列出 topic 摘要。

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use memoryctl_core::{EntryType, Result, Scope};
use memoryctl_protocol::{ListResponse, TopicSummary, PROTOCOL_VERSION};
use memoryctl_store::Store;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// 仅查指定 scope（global / project / agent:<name>）。
    #[arg(long)]
    pub scope: Option<String>,
    /// 仅含此类 entry 的 topic。
    #[arg(long)]
    pub r#type: Option<String>,
    /// 仅最近 N 天有更新（如 `7d`、`30d`）。
    #[arg(long)]
    pub recent: Option<String>,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scopes: Vec<Scope> = match args.scope.as_deref() {
        Some(s) => vec![util::parse_scope(s)?],
        None => store.all_scopes()?,
    };
    let type_filter = args.r#type.as_deref().map(EntryType::parse).transpose()?;
    let cutoff = args.recent.as_deref().map(parse_recent).transpose()?;

    let mut topics: Vec<TopicSummary> = Vec::new();
    for scope in scopes {
        for name in store.list_topics(&scope)? {
            let path = store.topic_path(&scope, &name)?;
            let Some(topic) = memoryctl_topic::read(&path)? else { continue };
            if topic.entries.is_empty() {
                continue;
            }

            // 过滤
            let entries_filtered: Vec<_> = topic
                .entries
                .iter()
                .filter(|e| type_filter.map_or(true, |t| e.entry_type == t))
                .filter(|e| cutoff.map_or(true, |c| e.timestamp >= c))
                .collect();
            if entries_filtered.is_empty() {
                continue;
            }

            let last_updated = entries_filtered.iter().map(|e| e.timestamp).max();
            let types: BTreeSet<EntryType> =
                entries_filtered.iter().map(|e| e.entry_type).collect();
            topics.push(TopicSummary {
                name,
                entries: entries_filtered.len(),
                last_updated,
                types: types.into_iter().collect(),
                scope: scope.label(),
            });
        }
    }

    topics.sort_by(|a, b| b.last_updated.cmp(&a.last_updated).then(a.name.cmp(&b.name)));

    let resp = ListResponse { protocol: PROTOCOL_VERSION, count: topics.len(), topics };

    if matches!(fmt, super::OutputFormat::Tsv) {
        emit_tsv(&resp.topics);
        return Ok(());
    }
    util::emit(fmt, &resp, |r| {
        for t in &r.topics {
            let last = t
                .last_updated
                .map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
                .unwrap_or_else(|| "-".into());
            let types: Vec<&str> = t.types.iter().map(EntryType::as_str).collect();
            println!(
                "{:<32} {:<5} {:<17} {:<48} {}",
                t.name,
                t.entries,
                last,
                types.join(","),
                t.scope
            );
        }
        Ok(())
    })
}

fn emit_tsv(topics: &[TopicSummary]) {
    println!("TOPIC\tENTRIES\tLAST_UPDATED\tTYPES\tSCOPE");
    for t in topics {
        let last = t
            .last_updated
            .map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_else(|| "-".into());
        let types: Vec<&str> = t.types.iter().map(EntryType::as_str).collect();
        println!(
            "{}\t{}\t{}\t{}\t{}",
            util::tsv_clean(&t.name),
            t.entries,
            last,
            util::tsv_clean(&types.join(",")),
            util::tsv_clean(&t.scope)
        );
    }
}

fn parse_recent(s: &str) -> Result<DateTime<FixedOffset>> {
    let trimmed = s.trim_end_matches(['d', 'h']);
    let n: i64 =
        trimmed.parse().map_err(|e| memoryctl_core::Error::other(format!("recent `{s}`: {e}")))?;
    let now = chrono::Local::now().fixed_offset();
    let dur = if s.ends_with('h') { chrono::Duration::hours(n) } else { chrono::Duration::days(n) };
    Ok(now - dur)
}
