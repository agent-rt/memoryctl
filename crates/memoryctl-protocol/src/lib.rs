//! 协议表面 wire schema。

#![forbid(unsafe_code)]

use chrono::{DateTime, FixedOffset};
use memoryctl_core::EntryType;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub protocol: u32,
    pub count: usize,
    pub topics: Vec<TopicSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    pub name: String,
    pub entries: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<DateTime<FixedOffset>>,
    pub types: Vec<EntryType>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResponse {
    pub protocol: u32,
    pub topic: String,
    pub scope: String,
    pub count: usize,
    pub entries: Vec<EntryView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryView {
    pub timestamp: DateTime<FixedOffset>,
    pub entry_type: EntryType,
    pub source: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResponse {
    pub protocol: u32,
    pub success: bool,
    pub action: String,
    pub topic: String,
    pub entry_type: EntryType,
    pub scope: String,
    pub path: camino::Utf8PathBuf,
    pub entry_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub protocol: u32,
    pub success: bool,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ErrorEnvelope {
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self { protocol: PROTOCOL_VERSION, success: false, error: code.into(), hint: None }
    }
}
