//! `memctl enable` — 写入 AGENTS.md 协议入口块。

use camino::Utf8PathBuf;
use memctl_core::Result;
use serde::Serialize;

use super::util;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long, default_value = "AGENTS.md")]
    pub target: String,
}

#[derive(Serialize)]
struct Out<'a> {
    success: bool,
    action: &'static str,
    target: &'a str,
}

pub fn run(args: Args, fmt: super::OutputFormat) -> Result<()> {
    let cwd = util::cwd()?;
    let target_path: Utf8PathBuf = if std::path::Path::new(&args.target).is_absolute() {
        args.target.clone().into()
    } else {
        cwd.join(&args.target)
    };
    memctl_agent::upsert(&target_path, &memctl_agent::default_block())?;
    let out = Out { success: true, action: "enable", target: &args.target };
    util::emit(fmt, &out, |o| {
        println!("enabled memctl block in {}", o.target);
        Ok(())
    })
}
