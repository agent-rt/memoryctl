//! `memoryctl search <query>` — 跨 topic 全文搜索。

use memoryctl_core::{EntryType, Result};
use memoryctl_search::{search as do_search, Match, SearchOpts};
use memoryctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    pub query: String,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long)]
    pub r#type: Option<String>,
    #[arg(long)]
    pub max_per_topic: Option<usize>,
}

#[derive(Serialize)]
struct SearchOut<'a> {
    success: bool,
    count: usize,
    matches: Vec<MatchView<'a>>,
}

#[derive(Serialize)]
struct MatchView<'a> {
    topic: &'a str,
    scope: String,
    timestamp: &'a str,
    r#type: &'a str,
    snippet: &'a str,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let opts = SearchOpts {
        query: args.query,
        scope: args.scope.as_deref().map(util::parse_scope).transpose()?,
        entry_type: args.r#type.as_deref().map(EntryType::parse).transpose()?,
        max_per_topic: args.max_per_topic,
    };
    let matches = do_search(&store, &opts)?;

    if matches!(fmt, super::OutputFormat::Tsv) {
        println!("TOPIC\tTIMESTAMP\tTYPE\tSCOPE\tMATCH");
        for m in &matches {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                util::tsv_clean(&m.topic),
                m.timestamp,
                m.entry_type.as_str(),
                m.scope.label(),
                util::tsv_clean(&m.snippet)
            );
        }
        return Ok(());
    }

    let view: Vec<MatchView<'_>> = matches
        .iter()
        .map(|m: &Match| MatchView {
            topic: &m.topic,
            scope: m.scope.label(),
            timestamp: &m.timestamp,
            r#type: m.entry_type.as_str(),
            snippet: &m.snippet,
        })
        .collect();

    let out = SearchOut { success: true, count: view.len(), matches: view };
    util::emit(fmt, &out, |o| {
        for m in &o.matches {
            println!("[{}] {} ({}, {}) {}", m.timestamp, m.topic, m.r#type, m.scope, m.snippet);
        }
        Ok(())
    })
}
