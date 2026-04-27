//! memctl 共享类型层。零 IO，不依赖业务 crate。

#![forbid(unsafe_code)]

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

/// memctl 全局错误类型。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error at {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("topic not found: {0}")]
    TopicNotFound(String),

    #[error("invalid topic name: {0}")]
    InvalidTopicName(String),

    #[error("invalid entry type: {0}")]
    InvalidType(String),

    #[error("invalid scope: {0}")]
    InvalidScope(String),

    #[error("invalid entry header: {0}")]
    InvalidEntry(String),

    #[error("project not initialized: {0}")]
    NotAProject(Utf8PathBuf),

    #[error("entry index out of range: {0}")]
    EntryOutOfRange(usize),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn other(s: impl Into<String>) -> Self {
        Self::Other(s.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// 记忆条目类型（固定 7 类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    /// 经验/教训/反复踩到的坑。
    Lesson,
    /// 设计决策 + 理由。
    Decision,
    /// 领域知识（非主观判断）。
    Fact,
    /// 协作偏好。
    Feedback,
    /// 外部资源指针。
    Reference,
    /// 用户身份/角色。
    User,
    /// 项目当前状态/上下文。
    Project,
}

impl EntryType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lesson => "lesson",
            Self::Decision => "decision",
            Self::Fact => "fact",
            Self::Feedback => "feedback",
            Self::Reference => "reference",
            Self::User => "user",
            Self::Project => "project",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "lesson" => Self::Lesson,
            "decision" => Self::Decision,
            "fact" => Self::Fact,
            "feedback" => Self::Feedback,
            "reference" => Self::Reference,
            "user" => Self::User,
            "project" => Self::Project,
            other => return Err(Error::InvalidType(other.to_owned())),
        })
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Lesson,
            Self::Decision,
            Self::Fact,
            Self::Feedback,
            Self::Reference,
            Self::User,
            Self::Project,
        ]
    }
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EntryType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

/// 作用域：决定 topic 文件存哪里、谁能看见。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// 跨项目、跨工具：`~/.memctl/global/`。
    Global,
    /// 项目级：`<project>/.memctl/`，跟代码一同 commit。
    Project,
    /// Agent 级：`~/.memctl/agents/<name>/`，仅特定 agent 应读到。
    Agent(String),
}

impl Scope {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Global => "global".into(),
            Self::Project => "project".into(),
            Self::Agent(name) => format!("agent:{name}"),
        }
    }

    /// 优先级：agent > project > global。
    #[must_use]
    pub fn priority(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Project => 1,
            Self::Agent(_) => 2,
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

/// 校验 topic 名：`[a-z0-9][a-z0-9-]{0,62}`。
pub fn validate_topic_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err(Error::InvalidTopicName(format!("length: {name}")));
    }
    let bytes = name.as_bytes();
    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return Err(Error::InvalidTopicName(format!("first char: {name}")));
    }
    if !bytes.iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') {
        return Err(Error::InvalidTopicName(format!("char set: {name}")));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn type_roundtrip() {
        for t in EntryType::all() {
            assert_eq!(EntryType::parse(t.as_str()).unwrap(), *t);
        }
    }

    #[test]
    fn topic_name_rules() {
        assert!(validate_topic_name("rust-error-patterns").is_ok());
        assert!(validate_topic_name("skillctl-design").is_ok());
        assert!(validate_topic_name("a").is_ok());
        assert!(validate_topic_name("").is_err());
        assert!(validate_topic_name("Rust-Errors").is_err());
        assert!(validate_topic_name("rust/errors").is_err());
        assert!(validate_topic_name("-leading-dash").is_err());
    }

    #[test]
    fn scope_priority() {
        assert!(Scope::Agent("x".into()).priority() > Scope::Project.priority());
        assert!(Scope::Project.priority() > Scope::Global.priority());
    }
}
