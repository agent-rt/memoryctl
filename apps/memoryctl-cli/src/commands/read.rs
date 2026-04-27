//! `memoryctl read` — 读取 topic 内容。

use chrono::{DateTime, FixedOffset};
use memoryctl_core::{EntryType, Error, Result, Scope};
use memoryctl_protocol::{EntryView, ReadResponse, PROTOCOL_VERSION};
use memoryctl_store::Store;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// topic 名。
    #[arg(long)]
    pub topic: String,
    /// 限制 scope（默认按优先级合并：agent > project > global）。
    #[arg(long)]
    pub scope: Option<String>,
    /// 仅指定类型。
    #[arg(long)]
    pub r#type: Option<String>,
    /// 时间过滤：`7d` / `24h`。
    #[arg(long)]
    pub since: Option<String>,
    /// 仅最新 N 条。
    #[arg(long)]
    pub limit: Option<usize>,
    /// 反序输出（最新→最旧）。
    #[arg(long)]
    pub reverse: bool,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scopes: Vec<Scope> = match args.scope.as_deref() {
        Some(s) => vec![util::parse_scope(s)?],
        None => store.all_scopes()?,
    };
    let type_filter = args.r#type.as_deref().map(EntryType::parse).transpose()?;
    let cutoff = args.since.as_deref().map(parse_since).transpose()?;

    // 合并多个 scope 的同名 topic
    let mut hits: Vec<(Scope, memoryctl_topic::Topic)> = Vec::new();
    for scope in &scopes {
        let path = store.topic_path(scope, &args.topic)?;
        if let Some(topic) = memoryctl_topic::read(&path)? {
            if !topic.entries.is_empty() {
                hits.push((scope.clone(), topic));
            }
        }
    }
    if hits.is_empty() {
        return Err(Error::TopicNotFound(args.topic));
    }

    // 合并 entries
    let mut entries: Vec<EntryView> = Vec::new();
    let display_scope = if hits.len() == 1 { hits[0].0.label() } else { "merged".to_owned() };

    for (_scope, topic) in hits {
        for e in topic.entries {
            if type_filter.map_or(false, |t| e.entry_type != t) {
                continue;
            }
            if cutoff.map_or(false, |c| e.timestamp < c) {
                continue;
            }
            entries.push(EntryView {
                timestamp: e.timestamp,
                entry_type: e.entry_type,
                source: e.source.formatted(),
                content: e.content,
            });
        }
    }
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    if args.reverse {
        entries.reverse();
    }
    if let Some(n) = args.limit {
        entries.truncate(n);
    }

    let resp = ReadResponse {
        protocol: PROTOCOL_VERSION,
        topic: args.topic.clone(),
        scope: display_scope,
        count: entries.len(),
        entries,
    };

    if matches!(fmt, super::OutputFormat::Tsv) {
        emit_tsv(&resp.entries);
        return Ok(());
    }
    util::emit(fmt, &resp, |r| {
        // human：直出 markdown
        println!("# {}\n", r.topic);
        for e in &r.entries {
            println!(
                "## {} [type={} source={}]",
                e.timestamp.format("%Y-%m-%dT%H:%M"),
                e.entry_type.as_str(),
                e.source
            );
            println!("{}\n", e.content);
        }
        Ok(())
    })
}

fn emit_tsv(entries: &[EntryView]) {
    println!("TIMESTAMP\tTYPE\tSOURCE\tCONTENT");
    for e in entries {
        let mut content = util::tsv_clean(&e.content);
        if content.chars().count() > 240 {
            let truncated: String = content.chars().take(239).collect();
            content = format!("{truncated}…");
        }
        println!(
            "{}\t{}\t{}\t{}",
            e.timestamp.format("%Y-%m-%dT%H:%M"),
            e.entry_type.as_str(),
            util::tsv_clean(&e.source),
            content
        );
    }
}

fn parse_since(s: &str) -> Result<DateTime<FixedOffset>> {
    let trimmed = s.trim_end_matches(['d', 'h']);
    let n: i64 = trimmed.parse().map_err(|e| Error::other(format!("since `{s}`: {e}")))?;
    let now = chrono::Local::now().fixed_offset();
    let dur = if s.ends_with('h') { chrono::Duration::hours(n) } else { chrono::Duration::days(n) };
    Ok(now - dur)
}
