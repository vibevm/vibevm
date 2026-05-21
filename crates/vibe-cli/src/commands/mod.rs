//! Sub-command implementations. Each module keeps `pub fn run(&Context, args) -> anyhow::Result<()>`.

pub mod check;
pub mod init;
pub mod install;
pub mod list;
pub mod mcp;
pub mod outdated;
pub mod registry;
pub mod reinstall;
pub mod search;
pub mod search_cache;
pub mod search_full_scan;
pub mod show;
pub mod uninstall;
pub mod update;
pub mod workspace;
