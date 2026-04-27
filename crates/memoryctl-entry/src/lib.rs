//! Entry header 格式：
//!
//! ```text
//! ## 2026-04-27T14:32 [type=decision source=claude-code @ ai-workspace/skillctl]
//! 内容多行 markdown
//! ```
//!
//! 见 REQ.md §4.1。

#![forbid(unsafe_code)]

use chrono::{DateTime, FixedOffset, Local};
use memoryctl_core::{EntryType, Error, Result};
use serde::{Deserialize, Serialize};

/// 单条记忆条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub timestamp: DateTime<FixedOffset>,
    pub entry_type: EntryType,
    pub source: EntrySource,
    pub content: String,
}

/// 来源标记：`<agent> @ <project-path>`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntrySource {
    pub agent: String,
    /// 写入时的项目路径（相对或绝对）；空字符串表示无项目（全局）。
    pub project: String,
}

impl EntrySource {
    #[must_use]
    pub fn formatted(&self) -> String {
        let project = if self.project.is_empty() { "-".to_owned() } else { self.project.clone() };
        format!("{} @ {}", self.agent, project)
    }

    /// 解析 `<agent> @ <project>`。空时填充默认。
    pub fn parse(s: &str) -> Self {
        if let Some((agent, rest)) = s.split_once('@') {
            let project = rest.trim();
            let project = if project == "-" { String::new() } else { project.to_owned() };
            EntrySource { agent: agent.trim().to_owned(), project }
        } else {
            EntrySource { agent: s.trim().to_owned(), project: String::new() }
        }
    }
}

impl Entry {
    /// 序列化为 markdown：H2 头 + 空行 + 内容 + 末尾换行。
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let ts = self.timestamp.format("%Y-%m-%dT%H:%M");
        let mut out = String::with_capacity(self.content.len() + 128);
        out.push_str(&format!(
            "## {} [type={} source={}]\n",
            ts,
            self.entry_type.as_str(),
            self.source.formatted(),
        ));
        out.push_str(self.content.trim_end());
        out.push('\n');
        out
    }

    /// 立刻构造一条以 Local 时区为时间戳的 entry。
    #[must_use]
    pub fn now(entry_type: EntryType, source: EntrySource, content: String) -> Self {
        let ts: DateTime<FixedOffset> = Local::now().fixed_offset();
        Self { timestamp: ts, entry_type, source, content }
    }
}

/// 解析 topic 文件全文，返回所有 entry 与文件标题。
///
/// 文件结构：
/// - 第一行可选 `# <topic>` 标题
/// - 之后 `## ` 开头的每段是一条 entry
pub fn parse_file(input: &str) -> Result<(Option<String>, Vec<Entry>)> {
    let mut lines = input.lines().peekable();

    // 提取 H1 标题
    let title = if let Some(line) = lines.peek() {
        line.strip_prefix("# ").map(|s| s.trim().to_owned())
    } else {
        None
    };
    if title.is_some() {
        lines.next();
    }

    let mut entries = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_body = String::new();

    let flush = |header: &Option<String>, body: &str, out: &mut Vec<Entry>| -> Result<()> {
        if let Some(h) = header {
            out.push(parse_entry(h, body)?);
        }
        Ok(())
    };

    for line in lines {
        if let Some(rest) = line.strip_prefix("## ") {
            flush(&current_header, &current_body, &mut entries)?;
            current_header = Some(rest.to_owned());
            current_body.clear();
        } else if current_header.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
        // header 出现前的内容（除 H1）忽略
    }
    flush(&current_header, &current_body, &mut entries)?;

    Ok((title, entries))
}

fn parse_entry(header: &str, body: &str) -> Result<Entry> {
    // header: `2026-04-27T14:32 [type=decision source=claude-code @ skillctl]`
    let (ts_part, kvs_part) = match header.split_once(" [") {
        Some((ts, rest)) => {
            let kvs = rest.trim_end_matches(']');
            (ts.trim(), kvs)
        }
        None => return Err(Error::InvalidEntry(format!("missing `[`: {header}"))),
    };

    let timestamp = parse_timestamp(ts_part)?;
    let mut entry_type: Option<EntryType> = None;
    let mut source = EntrySource::default();

    // kvs 形如：`type=decision source=claude-code @ skillctl`
    // source 值可能含空格（"@" 周围）；按空格分词不行。
    // 策略：找到 `type=`，再找 `source=`（取剩下全部到 `]`）
    if let Some(t_idx) = kvs_part.find("type=") {
        let after_type = &kvs_part[t_idx + 5..];
        // type 值到下一个空格或字符串末尾
        let type_end = after_type.find(' ').unwrap_or(after_type.len());
        let type_str = &after_type[..type_end];
        entry_type = Some(EntryType::parse(type_str)?);
    }
    if let Some(s_idx) = kvs_part.find("source=") {
        let after_src = &kvs_part[s_idx + 7..].trim();
        source = EntrySource::parse(after_src);
    }

    let entry_type =
        entry_type.ok_or_else(|| Error::InvalidEntry(format!("missing type: {header}")))?;

    Ok(Entry { timestamp, entry_type, source, content: body.trim_end().to_owned() })
}

fn parse_timestamp(s: &str) -> Result<DateTime<FixedOffset>> {
    // 接受 `2026-04-27T14:32` 或 `2026-04-27T14:32:00` 或带时区
    // 优先尝试带时区
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Ok(t);
    }
    // 否则补秒、用本地时区
    let with_seconds = if s.matches(':').count() == 1 { format!("{s}:00") } else { s.to_owned() };
    let naive = chrono::NaiveDateTime::parse_from_str(&with_seconds, "%Y-%m-%dT%H:%M:%S")
        .map_err(|e| Error::InvalidEntry(format!("timestamp `{s}`: {e}")))?;
    let local = naive
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| Error::InvalidEntry(format!("ambiguous local time: {s}")))?;
    Ok(local.fixed_offset())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# skillctl-design

## 2026-04-27T14:32 [type=decision source=claude-code @ ai-workspace/skillctl]
TSV beats JSON for Agent list output.

Multi-line content allowed.

## 2026-04-27T13:55 [type=lesson source=user @ -]
Second entry with no project source.
"#;

    #[test]
    fn parses_sample() {
        let (title, entries) = parse_file(SAMPLE).expect("ok");
        assert_eq!(title.as_deref(), Some("skillctl-design"));
        assert_eq!(entries.len(), 2);

        let e1 = &entries[0];
        assert_eq!(e1.entry_type, EntryType::Decision);
        assert_eq!(e1.source.agent, "claude-code");
        assert_eq!(e1.source.project, "ai-workspace/skillctl");
        assert!(e1.content.contains("TSV beats JSON"));
        assert!(e1.content.contains("Multi-line"));

        let e2 = &entries[1];
        assert_eq!(e2.entry_type, EntryType::Lesson);
        assert_eq!(e2.source.project, "");
    }

    #[test]
    fn entry_to_markdown_roundtrip() {
        let (_, entries) = parse_file(SAMPLE).unwrap();
        let md = entries[0].to_markdown();
        assert!(md.starts_with(
            "## 2026-04-27T14:32 [type=decision source=claude-code @ ai-workspace/skillctl]"
        ));
    }

    #[test]
    fn rejects_missing_type() {
        let bad = "# t\n\n## 2026-04-27T14:32 [source=x @ -]\ncontent\n";
        assert!(parse_file(bad).is_err());
    }

    #[test]
    fn handles_empty_topic_file() {
        let empty = "# my-topic\n";
        let (title, entries) = parse_file(empty).unwrap();
        assert_eq!(title.as_deref(), Some("my-topic"));
        assert!(entries.is_empty());
    }
}
