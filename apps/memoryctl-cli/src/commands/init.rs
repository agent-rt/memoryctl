//! `memoryctl init` — 起手项目（建 .memoryctl/ + AGENTS.md 块）。

use camino::Utf8PathBuf;
use memoryctl_core::{Error, Result};
use memoryctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// 已存在 .memoryctl 时强制重写 AGENTS.md 块。
    #[arg(long)]
    pub force: bool,
}

#[derive(Serialize)]
struct Out {
    success: bool,
    action: &'static str,
    project: Utf8PathBuf,
    updated: Vec<Utf8PathBuf>,
}

pub fn run(_args: Args, fmt: super::OutputFormat) -> Result<()> {
    let cwd = util::cwd()?;
    let project_root = cwd.clone();
    let memoryctl_dir = project_root.join(".memoryctl").join("topics");
    fs_err::create_dir_all(memoryctl_dir.as_std_path())
        .map_err(|e| Error::Io { path: memoryctl_dir.clone(), source: e })?;

    let agents_md = project_root.join("AGENTS.md");
    memoryctl_agent::upsert(&agents_md, &memoryctl_agent::default_block())?;

    // 顺便确保全局根存在
    let store = Store {
        global_root: Store::default_open()?.global_root,
        project_root: Some(project_root.clone()),
    };
    store.ensure_global_dirs()?;

    let out = Out {
        success: true,
        action: "init",
        project: project_root,
        updated: vec![memoryctl_dir, agents_md],
    };
    util::emit(fmt, &out, |o| {
        println!("initialized memoryctl in {}", o.project);
        Ok(())
    })
}
