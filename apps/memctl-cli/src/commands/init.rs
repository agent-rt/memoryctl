//! `memctl init` — 起手项目（建 .memctl/ + AGENTS.md 块）。

use camino::Utf8PathBuf;
use memctl_core::{Error, Result};
use memctl_store::Store;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// 已存在 .memctl 时强制重写 AGENTS.md 块。
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
    let memctl_dir = project_root.join(".memctl").join("topics");
    fs_err::create_dir_all(memctl_dir.as_std_path())
        .map_err(|e| Error::Io { path: memctl_dir.clone(), source: e })?;

    let agents_md = project_root.join("AGENTS.md");
    memctl_agent::upsert(&agents_md, &memctl_agent::default_block())?;

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
        updated: vec![memctl_dir, agents_md],
    };
    util::emit(fmt, &out, |o| {
        println!("initialized memctl in {}", o.project);
        Ok(())
    })
}
