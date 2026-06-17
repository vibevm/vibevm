//! Sub-command implementations. Each module keeps `pub fn run(&Context, args) -> anyhow::Result<()>`.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#cli-surface");

pub mod check;
pub mod init;
pub mod install;
pub mod list;
pub mod mcp;
pub mod outdated;
pub mod registry;
pub mod reinstall;
pub mod search;
pub mod short_name;
pub mod show;
pub mod skill;
pub mod uninstall;
pub mod update;
pub mod workspace;
