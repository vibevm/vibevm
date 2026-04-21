//! Sub-command implementations. Each module keeps `pub fn run(&Context, args) -> anyhow::Result<()>`.

pub mod init;
pub mod install;
pub mod list;
pub mod registry;
pub mod uninstall;
