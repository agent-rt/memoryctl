//! 共享辅助。

use camino::Utf8PathBuf;
use memctl_core::{Error, Result, Scope};
use serde::Serialize;

use super::OutputFormat;

pub fn cwd() -> Result<Utf8PathBuf> {
    let std = std::env::current_dir().map_err(|e| Error::other(format!("cwd: {e}")))?;
    Utf8PathBuf::from_path_buf(std).map_err(|p| Error::other(format!("non-utf8 cwd: {p:?}")))
}

#[must_use]
pub fn tsv_clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

pub fn emit<T: Serialize>(
    fmt: OutputFormat,
    value: &T,
    other: impl FnOnce(&T) -> Result<()>,
) -> Result<()> {
    match fmt {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(value)
                .map_err(|e| Error::other(format!("json: {e}")))?;
            println!("{s}");
            Ok(())
        }
        OutputFormat::Human | OutputFormat::Tsv => other(value),
    }
}

/// 从字符串解析 scope；`agent:<name>` 解析为 Agent 变体。
pub fn parse_scope(s: &str) -> Result<Scope> {
    if s == "global" {
        return Ok(Scope::Global);
    }
    if s == "project" {
        return Ok(Scope::Project);
    }
    if let Some(rest) = s.strip_prefix("agent:") {
        if rest.is_empty() {
            return Err(Error::InvalidScope("agent: requires a name".into()));
        }
        return Ok(Scope::Agent(rest.to_owned()));
    }
    Err(Error::InvalidScope(s.to_owned()))
}

#[must_use]
pub fn agent_name() -> String {
    std::env::var("MEMCTL_AGENT").unwrap_or_else(|_| "unknown".into())
}

/// 把 cwd 转成 source 中记录的简短项目路径（HOME-relative 或 basename）。
#[must_use]
pub fn project_label() -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return String::new();
    };
    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = cwd.strip_prefix(&home) {
            return rel.to_string_lossy().into_owned();
        }
    }
    cwd.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}
