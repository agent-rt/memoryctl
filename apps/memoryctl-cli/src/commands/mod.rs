//! 子命令分发。

use memoryctl_core::Result;

use super::{Command, OutputFormat};

pub mod util;

pub mod disable;
pub mod edit;
pub mod enable;
pub mod export;
pub mod forget;
pub mod init;
pub mod list;
pub mod move_cmd;
pub mod read;
pub mod save;
pub mod search;
pub mod validate;

pub fn dispatch(cmd: Command, fmt: OutputFormat) -> Result<()> {
    match cmd {
        Command::Init(a) => init::run(a, fmt),
        Command::Enable(a) => enable::run(a, fmt),
        Command::Disable(a) => disable::run(a, fmt),
        Command::Save(a) => save::run(a, fmt),
        Command::List(a) | Command::Topics(a) => list::run(a, fmt),
        Command::Read(a) => read::run(a, fmt),
        Command::Search(a) => search::run(a, fmt),
        Command::Forget(a) => forget::run(a, fmt),
        Command::Edit(a) => edit::run(a, fmt),
        Command::Move(a) => move_cmd::run(a, fmt),
        Command::Validate(a) => validate::run(a, fmt),
        Command::Export(a) => export::run(a, fmt),
    }
}
