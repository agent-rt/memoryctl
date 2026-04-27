//! Topic 文件 IO：append-only 写入是默认；rewrite 是显式（forget/edit/move）。

#![forbid(unsafe_code)]

use std::io::Write;

use camino::Utf8Path;
use memoryctl_core::{Error, Result};
use memoryctl_entry::{parse_file, Entry};

/// Topic 在内存中的视图。
#[derive(Debug, Clone)]
pub struct Topic {
    pub name: String,
    pub entries: Vec<Entry>,
}

/// 读取 topic 文件。文件不存在返回空 topic。
pub fn read(path: &Utf8Path) -> Result<Option<Topic>> {
    let raw = match fs_err::read_to_string(path.as_std_path()) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::Io { path: path.to_owned(), source: e }),
    };
    let (title, entries) = parse_file(&raw)?;
    let name = title.unwrap_or_else(|| path.file_stem().unwrap_or("untitled").to_owned());
    Ok(Some(Topic { name, entries }))
}

/// 追加一条 entry。若文件不存在则先创建并写入 H1 标题。
///
/// **不会** 重写已有内容。这是默认写路径，避免并发冲突。
pub fn append(path: &Utf8Path, topic_name: &str, entry: &Entry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent.as_std_path())
            .map_err(|e| Error::Io { path: parent.to_owned(), source: e })?;
    }

    let exists = path.exists();
    let mut file = fs_err::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_std_path())
        .map_err(|e| Error::Io { path: path.to_owned(), source: e })?;

    if !exists {
        writeln!(file, "# {topic_name}\n")
            .map_err(|e| Error::Io { path: path.to_owned(), source: e })?;
    }
    // 在 entry 前确保有空行
    writeln!(file).map_err(|e| Error::Io { path: path.to_owned(), source: e })?;
    write!(file, "{}", entry.to_markdown())
        .map_err(|e| Error::Io { path: path.to_owned(), source: e })?;
    Ok(())
}

/// 整体重写 topic 文件（forget / edit / move 时使用）。
pub fn write_full(path: &Utf8Path, topic: &Topic) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent.as_std_path())
            .map_err(|e| Error::Io { path: parent.to_owned(), source: e })?;
    }
    let mut out = String::with_capacity(1024);
    out.push_str(&format!("# {}\n", topic.name));
    for entry in &topic.entries {
        out.push('\n');
        out.push_str(&entry.to_markdown());
    }
    fs_err::write(path.as_std_path(), out)
        .map_err(|e| Error::Io { path: path.to_owned(), source: e })
}

/// 删除 topic 文件。
pub fn remove(path: &Utf8Path) -> Result<()> {
    match fs_err::remove_file(path.as_std_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Io { path: path.to_owned(), source: e }),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use memoryctl_core::EntryType;
    use memoryctl_entry::EntrySource;

    fn tmp_path() -> (tempfile::TempDir, camino::Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = camino::Utf8PathBuf::from_path_buf(dir.path().join("topic.md")).unwrap();
        (dir, p)
    }

    #[test]
    fn append_creates_with_title() {
        let (_d, p) = tmp_path();
        let entry = Entry::now(
            EntryType::Decision,
            EntrySource { agent: "test".into(), project: String::new() },
            "first entry".into(),
        );
        append(&p, "my-topic", &entry).unwrap();
        let raw = fs_err::read_to_string(p.as_std_path()).unwrap();
        assert!(raw.starts_with("# my-topic\n"));
        assert!(raw.contains("first entry"));
    }

    #[test]
    fn append_preserves_existing() {
        let (_d, p) = tmp_path();
        let e1 = Entry::now(
            EntryType::Decision,
            EntrySource { agent: "a".into(), project: "p".into() },
            "one".into(),
        );
        let e2 = Entry::now(
            EntryType::Lesson,
            EntrySource { agent: "a".into(), project: "p".into() },
            "two".into(),
        );
        append(&p, "t", &e1).unwrap();
        append(&p, "t", &e2).unwrap();
        let topic = read(&p).unwrap().expect("exists");
        assert_eq!(topic.entries.len(), 2);
        assert_eq!(topic.entries[0].content, "one");
        assert_eq!(topic.entries[1].content, "two");
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = camino::Utf8PathBuf::from_path_buf(dir.path().join("none.md")).unwrap();
        assert!(read(&p).unwrap().is_none());
    }
}
