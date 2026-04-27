//! `memctl validate` — 校验所有 topic 文件格式。

use memctl_core::{Result, Scope};
use memctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long)]
    pub topic: Option<String>,
    #[arg(long)]
    pub scope: Option<String>,
    #[arg(long)]
    pub strict: bool,
}

#[derive(Serialize)]
struct Out {
    success: bool,
    checked: usize,
    failed: Vec<Failure>,
}

#[derive(Serialize)]
struct Failure {
    scope: String,
    topic: String,
    error: String,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let store = Store::default_open()?;
    let scopes: Vec<Scope> = match args.scope.as_deref() {
        Some(s) => vec![util::parse_scope(s)?],
        None => store.all_scopes()?,
    };

    let mut checked = 0;
    let mut failed = Vec::new();
    for scope in scopes {
        let names = match &args.topic {
            Some(t) => vec![t.clone()],
            None => store.list_topics(&scope)?,
        };
        for name in names {
            let path = store.topic_path(&scope, &name)?;
            if !path.exists() {
                continue;
            }
            let raw = fs_err::read_to_string(path.as_std_path())
                .map_err(|e| memctl_core::Error::Io { path: path.clone(), source: e })?;
            checked += 1;
            if let Err(e) = memctl_entry::parse_file(&raw) {
                failed.push(Failure { scope: scope.label(), topic: name, error: e.to_string() });
            }
        }
    }

    let success = failed.is_empty();
    let out = Out { success, checked, failed };
    let _ = args.strict; // strict mode hook for future
    util::emit(fmt, &out, |o| {
        println!("checked {} topic files; {} failures", o.checked, o.failed.len());
        for f in &o.failed {
            println!("  ✗ [{}] {}: {}", f.scope, f.topic, f.error);
        }
        Ok(())
    })
}
