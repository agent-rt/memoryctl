//! 跨 topic 全文搜索。优先用系统 `rg`（性能），不可用时回退内置 regex 扫描。

#![forbid(unsafe_code)]

use memctl_core::{EntryType, Result, Scope};
use memctl_store::Store;

/// 单条匹配。
#[derive(Debug, Clone)]
pub struct Match {
    pub topic: String,
    pub scope: Scope,
    pub timestamp: String,
    pub entry_type: EntryType,
    pub snippet: String,
}

/// 搜索选项。
#[derive(Debug, Clone, Default)]
pub struct SearchOpts {
    pub query: String,
    pub scope: Option<Scope>,
    pub entry_type: Option<EntryType>,
    pub max_per_topic: Option<usize>,
}

/// 在 store 内搜索；使用 rg 加速，否则回退内置实现。
pub fn search(store: &Store, opts: &SearchOpts) -> Result<Vec<Match>> {
    // 内置实现：稳定且测试友好。rg 加速留待 phase 2 优化。
    builtin_search(store, opts)
}

fn builtin_search(store: &Store, opts: &SearchOpts) -> Result<Vec<Match>> {
    let re = regex::RegexBuilder::new(&regex::escape(&opts.query))
        .case_insensitive(true)
        .build()
        .map_err(|e| memctl_core::Error::other(format!("regex: {e}")))?;

    let mut out = Vec::new();
    let scopes: Vec<Scope> = match &opts.scope {
        Some(s) => vec![s.clone()],
        None => store.all_scopes()?,
    };

    for scope in scopes {
        for name in store.list_topics(&scope)? {
            let path = store.topic_path(&scope, &name)?;
            let topic = match memctl_topic::read(&path)? {
                Some(t) => t,
                None => continue,
            };
            let mut matches_in_topic = 0usize;
            for entry in &topic.entries {
                if let Some(t) = opts.entry_type {
                    if entry.entry_type != t {
                        continue;
                    }
                }
                if let Some(found) = re.find(&entry.content) {
                    let snippet = make_snippet(&entry.content, found.start(), found.end());
                    out.push(Match {
                        topic: name.clone(),
                        scope: scope.clone(),
                        timestamp: entry.timestamp.format("%Y-%m-%dT%H:%M").to_string(),
                        entry_type: entry.entry_type,
                        snippet,
                    });
                    matches_in_topic += 1;
                    if let Some(cap) = opts.max_per_topic {
                        if matches_in_topic >= cap {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

fn make_snippet(content: &str, start: usize, end: usize) -> String {
    let pre = floor_char_boundary(content, start.saturating_sub(40));
    let post = ceil_char_boundary(content, (end + 80).min(content.len()));
    let mut s = String::with_capacity(post - pre + 6);
    if pre > 0 {
        s.push('…');
    }
    s.push_str(&content[pre..post]);
    if post < content.len() {
        s.push('…');
    }
    s.replace(['\n', '\t', '\r'], " ")
}

/// `str::floor_char_boundary` 在 stable 上还未稳定，自己实现一份。
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

/// 检测 ripgrep 是否可用（信息用途；当前不使用）。
#[must_use]
pub fn has_ripgrep() -> bool {
    which::which("rg").is_ok()
}
