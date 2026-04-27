//! 存储层：scope 路径解析与 topic 列表枚举。

#![forbid(unsafe_code)]

use camino::{Utf8Path, Utf8PathBuf};
use memoryctl_core::{validate_topic_name, Error, Result, Scope};

/// memoryctl 存储入口。
#[derive(Debug, Clone)]
pub struct Store {
    pub global_root: Utf8PathBuf,
    pub project_root: Option<Utf8PathBuf>,
}

impl Store {
    /// 默认全局根：`$HOME/.memoryctl`。项目根从 `cwd` 沿父目录查找 `.memoryctl/`。
    pub fn default_open() -> Result<Self> {
        let global_root = default_global_root()?;
        let cwd = std::env::current_dir().map_err(|e| Error::other(format!("cwd: {e}")))?;
        let cwd = Utf8PathBuf::from_path_buf(cwd)
            .map_err(|p| Error::other(format!("non-utf8 cwd: {p:?}")))?;
        let project_root = discover_project(&cwd);
        Ok(Self { global_root, project_root })
    }

    pub fn ensure_global_dirs(&self) -> Result<()> {
        let dir = self.global_root.join("global").join("topics");
        fs_err::create_dir_all(dir.as_std_path())
            .map_err(|e| Error::Io { path: dir, source: e })?;
        let agents = self.global_root.join("agents");
        fs_err::create_dir_all(agents.as_std_path())
            .map_err(|e| Error::Io { path: agents, source: e })?;
        Ok(())
    }

    pub fn ensure_project_dirs(&self) -> Result<()> {
        let root = self.project_root.as_ref().ok_or_else(|| Error::other("no project root"))?;
        let dir = root.join(".memoryctl").join("topics");
        fs_err::create_dir_all(dir.as_std_path())
            .map_err(|e| Error::Io { path: dir, source: e })?;
        Ok(())
    }

    /// scope 对应的 topics 根目录。
    pub fn topics_dir(&self, scope: &Scope) -> Result<Utf8PathBuf> {
        Ok(match scope {
            Scope::Global => self.global_root.join("global").join("topics"),
            Scope::Project => {
                let root = self
                    .project_root
                    .as_ref()
                    .ok_or_else(|| Error::NotAProject(self.global_root.clone()))?;
                root.join(".memoryctl").join("topics")
            }
            Scope::Agent(name) => self.global_root.join("agents").join(name).join("topics"),
        })
    }

    /// scope + topic 名 → 文件路径。
    pub fn topic_path(&self, scope: &Scope, topic: &str) -> Result<Utf8PathBuf> {
        validate_topic_name(topic)?;
        Ok(self.topics_dir(scope)?.join(format!("{topic}.md")))
    }

    /// 列出某 scope 下所有 topic 名。
    pub fn list_topics(&self, scope: &Scope) -> Result<Vec<String>> {
        let dir = self.topics_dir(scope)?;
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in fs_err::read_dir(dir.as_std_path())
            .map_err(|e| Error::Io { path: dir.clone(), source: e })?
        {
            let Ok(entry) = entry else { continue };
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                out.push(stem.to_owned());
            }
        }
        out.sort();
        Ok(out)
    }

    /// 列出所有 scope 中的 topic（去重，按名字）。
    pub fn list_all_topics(&self) -> Result<Vec<TopicLocation>> {
        let mut found: Vec<TopicLocation> = Vec::new();
        for scope in self.all_scopes()? {
            for name in self.list_topics(&scope)? {
                found.push(TopicLocation { name, scope: scope.clone() });
            }
        }
        Ok(found)
    }

    /// 列出已知的所有 scope 实例（含每个 agent）。
    pub fn all_scopes(&self) -> Result<Vec<Scope>> {
        let mut scopes = vec![Scope::Global];
        if self.project_root.is_some() {
            scopes.push(Scope::Project);
        }
        // agents/<name>/topics
        let agents_root = self.global_root.join("agents");
        if agents_root.exists() {
            for entry in fs_err::read_dir(agents_root.as_std_path())
                .map_err(|e| Error::Io { path: agents_root.clone(), source: e })?
            {
                let Ok(entry) = entry else { continue };
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        scopes.push(Scope::Agent(name.to_owned()));
                    }
                }
            }
        }
        Ok(scopes)
    }
}

/// Topic 在某 scope 下的位置。
#[derive(Debug, Clone)]
pub struct TopicLocation {
    pub name: String,
    pub scope: Scope,
}

fn default_global_root() -> Result<Utf8PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| Error::other("cannot resolve home"))?;
    Utf8PathBuf::from_path_buf(home.join(".memoryctl"))
        .map_err(|p| Error::other(format!("non-utf8 home: {p:?}")))
}

fn discover_project(start: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if dir.join(".memoryctl").is_dir() {
            return Some(dir.to_owned());
        }
        cur = dir.parent();
    }
    None
}

/// 强制以指定 cwd 构造 store（测试用）。
#[must_use]
pub fn with_roots(global_root: Utf8PathBuf, project_root: Option<Utf8PathBuf>) -> Store {
    Store { global_root, project_root }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn discovers_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(root.join(".memoryctl/topics").as_std_path()).unwrap();
        let nested = root.join("a/b/c");
        std::fs::create_dir_all(nested.as_std_path()).unwrap();
        assert_eq!(discover_project(&nested), Some(root));
    }

    #[test]
    fn topic_path_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let store = with_roots(root.clone(), Some(root.clone()));

        let p = store.topic_path(&Scope::Global, "foo").unwrap();
        assert_eq!(p, root.join("global/topics/foo.md"));

        let p = store.topic_path(&Scope::Project, "bar").unwrap();
        assert_eq!(p, root.join(".memoryctl/topics/bar.md"));

        let p = store.topic_path(&Scope::Agent("claude-code".into()), "baz").unwrap();
        assert_eq!(p, root.join("agents/claude-code/topics/baz.md"));
    }

    #[test]
    fn rejects_invalid_topic() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let store = with_roots(root, None);
        assert!(store.topic_path(&Scope::Global, "Bad-Name").is_err());
    }
}
