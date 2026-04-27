//! `memoryctl forget` — 删除 entry 或整个 topic。

use chrono::{DateTime, FixedOffset};
use memoryctl_core::{Error, Result};
use memoryctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long)]
    pub topic: String,
    /// 删除第 N 条 entry（1-indexed），按时间升序。
    #[arg(long, conflicts_with = "before")]
    pub entry: Option<usize>,
    /// 删除指定时间之前的所有 entry：`30d` / `2026-01-01`。
    #[arg(long, conflicts_with = "entry")]
    pub before: Option<String>,
    /// 删除整个 topic 文件（需 --yes）。
    #[arg(long)]
    pub yes: bool,
    #[arg(long, default_value = "global")]
    pub scope: String,
}

#[derive(Serialize)]
struct Out {
    success: bool,
    action: &'static str,
    topic: String,
    removed: usize,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scope = util::parse_scope(&args.scope)?;
    let path = store.topic_path(&scope, &args.topic)?;
    let mut topic =
        memoryctl_topic::read(&path)?.ok_or_else(|| Error::TopicNotFound(args.topic.clone()))?;

    let removed = if let Some(idx) = args.entry {
        if idx == 0 || idx > topic.entries.len() {
            return Err(Error::EntryOutOfRange(idx));
        }
        topic.entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        topic.entries.remove(idx - 1);
        memoryctl_topic::write_full(&path, &topic)?;
        1
    } else if let Some(s) = args.before {
        let cutoff = parse_before(&s)?;
        let before_n = topic.entries.len();
        topic.entries.retain(|e| e.timestamp >= cutoff);
        let removed = before_n - topic.entries.len();
        if topic.entries.is_empty() {
            memoryctl_topic::remove(&path)?;
        } else {
            memoryctl_topic::write_full(&path, &topic)?;
        }
        removed
    } else {
        // 整个 topic
        if !args.yes {
            return Err(Error::other("removing entire topic requires --yes"));
        }
        let n = topic.entries.len();
        memoryctl_topic::remove(&path)?;
        n
    };

    let out = Out { success: true, action: "forget", topic: args.topic, removed };
    util::emit(fmt, &out, |o| {
        println!("forgot {} entries from {}", o.removed, o.topic);
        Ok(())
    })
}

fn parse_before(s: &str) -> Result<DateTime<FixedOffset>> {
    // 优先按相对时间 `30d`/`24h`，否则按 ISO 日期
    if s.ends_with('d') || s.ends_with('h') {
        let n: i64 = s
            .trim_end_matches(['d', 'h'])
            .parse()
            .map_err(|e| Error::other(format!("before `{s}`: {e}")))?;
        let dur =
            if s.ends_with('h') { chrono::Duration::hours(n) } else { chrono::Duration::days(n) };
        return Ok(chrono::Local::now().fixed_offset() - dur);
    }
    // ISO date / datetime
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt =
            d.and_hms_opt(0, 0, 0).ok_or_else(|| Error::other(format!("invalid date: {s}")))?;
        let local = dt
            .and_local_timezone(chrono::Local)
            .single()
            .ok_or_else(|| Error::other(format!("ambiguous: {s}")))?;
        return Ok(local.fixed_offset());
    }
    DateTime::parse_from_rfc3339(s).map_err(|e| Error::other(format!("before `{s}`: {e}")))
}
