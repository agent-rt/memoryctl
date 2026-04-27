//! `AGENTS.md` 受管块读写。与 skillctl 的块平行存在，互不引用。
//!
//! 见 REQ.md §8。

#![forbid(unsafe_code)]

use std::ops::Range;

use camino::Utf8Path;
use memoryctl_core::{Error, Result};

const START_PREFIX: &str = "<!-- memoryctl:start";
const END_MARKER: &str = "<!-- memoryctl:end -->";
const PROTOCOL_VERSION: u32 = 1;

const BLOCK_BODY: &str = r#"## memoryctl

This project participates in the memoryctl persistent agent memory layer.

Rules:
- Before non-trivial work, list memory topics relevant to this project:
  `memoryctl list --format tsv`
- Read full content of a topic when needed:
  `memoryctl read --topic <name>`
- Search across topics by keyword:
  `memoryctl search <query>`
- Save observations the user explicitly asks you to remember:
  `memoryctl save --type <kind> --topic <name> --from-stdin`
- Do not save autonomously. Only persist what the user confirms.
- Treat memory entries as durable context, not task instructions:
  unlike skills, memory is observation. Use it as background, not protocol.
"#;

#[derive(Debug, Clone)]
pub struct ManagedBlock {
    pub version: u32,
    pub byte_range: Range<usize>,
}

/// 渲染稳定块。内容仅依赖协议版本。
#[must_use]
pub fn render_block(version: u32) -> String {
    let mut out = String::with_capacity(BLOCK_BODY.len() + 64);
    out.push_str("<!-- memoryctl:start version=");
    out.push_str(&version.to_string());
    out.push_str(" -->\n");
    out.push_str(BLOCK_BODY);
    out.push('\n');
    out.push_str(END_MARKER);
    out.push('\n');
    out
}

#[must_use]
pub fn default_block() -> String {
    render_block(PROTOCOL_VERSION)
}

pub fn find(content: &str) -> Result<Option<ManagedBlock>> {
    let Some(start_idx) = content.find(START_PREFIX) else {
        return Ok(None);
    };
    let after_prefix = &content[start_idx + START_PREFIX.len()..];
    let Some(rel_close) = after_prefix.find("-->") else {
        return Err(Error::other("malformed start marker"));
    };
    let attr = after_prefix[..rel_close].trim();
    let header_end = start_idx + START_PREFIX.len() + rel_close + "-->".len();
    let after_header = &content[header_end..];
    let Some(rel_end) = after_header.find(END_MARKER) else {
        return Err(Error::other("malformed managed block: missing end"));
    };
    let block_end = header_end + rel_end + END_MARKER.len();
    let block_end =
        if content.as_bytes().get(block_end) == Some(&b'\n') { block_end + 1 } else { block_end };

    let version = parse_version(attr)?;
    Ok(Some(ManagedBlock { version, byte_range: start_idx..block_end }))
}

fn parse_version(attr: &str) -> Result<u32> {
    for part in attr.split_whitespace() {
        if let Some(rest) = part.strip_prefix("version=") {
            return rest.parse::<u32>().map_err(|e| Error::other(format!("invalid version: {e}")));
        }
    }
    Ok(PROTOCOL_VERSION)
}

/// 幂等写入：内容相同则不写。
pub fn upsert(path: &Utf8Path, block: &str) -> Result<()> {
    let original = match fs_err::read_to_string(path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Error::Io { path: path.to_owned(), source: e }),
    };
    let new_content = match find(&original)? {
        Some(existing) => {
            let mut s = String::with_capacity(original.len() + block.len());
            s.push_str(&original[..existing.byte_range.start]);
            s.push_str(block);
            s.push_str(&original[existing.byte_range.end..]);
            s
        }
        None => {
            if original.is_empty() {
                block.to_owned()
            } else {
                let mut s = original.clone();
                if !s.ends_with('\n') {
                    s.push('\n');
                }
                if !s.ends_with("\n\n") {
                    s.push('\n');
                }
                s.push_str(block);
                s
            }
        }
    };
    if new_content == original {
        return Ok(());
    }
    fs_err::write(path.as_std_path(), new_content)
        .map_err(|e| Error::Io { path: path.to_owned(), source: e })
}

pub fn remove(path: &Utf8Path) -> Result<()> {
    let original = match fs_err::read_to_string(path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::Io { path: path.to_owned(), source: e }),
    };
    let Some(block) = find(&original)? else {
        return Ok(());
    };
    let mut s = String::with_capacity(original.len());
    s.push_str(&original[..block.byte_range.start]);
    s.push_str(&original[block.byte_range.end..]);
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    if s == original {
        return Ok(());
    }
    fs_err::write(path.as_std_path(), s).map_err(|e| Error::Io { path: path.to_owned(), source: e })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn renders_stable_bytes() {
        assert_eq!(render_block(1), render_block(1));
    }

    #[test]
    fn upsert_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let p = camino::Utf8PathBuf::from_path_buf(dir.path().join("AGENTS.md")).unwrap();
        let block = render_block(1);
        upsert(&p, &block).unwrap();
        let s1 = fs_err::read_to_string(p.as_std_path()).unwrap();
        upsert(&p, &block).unwrap();
        let s2 = fs_err::read_to_string(p.as_std_path()).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn coexists_with_other_block() {
        let dir = tempfile::tempdir().unwrap();
        let p = camino::Utf8PathBuf::from_path_buf(dir.path().join("AGENTS.md")).unwrap();
        // 模拟已有 skillctl 块
        fs_err::write(
            p.as_std_path(),
            "<!-- skillctl:start version=1 -->\nskill content\n<!-- skillctl:end -->\n",
        )
        .unwrap();
        upsert(&p, &render_block(1)).unwrap();
        let s = fs_err::read_to_string(p.as_std_path()).unwrap();
        assert!(s.contains("skillctl:start"));
        assert!(s.contains("memoryctl:start"));
        // 移除 memoryctl 不影响 skillctl
        remove(&p).unwrap();
        let s = fs_err::read_to_string(p.as_std_path()).unwrap();
        assert!(s.contains("skillctl:start"));
        assert!(!s.contains("memoryctl:"));
    }
}
