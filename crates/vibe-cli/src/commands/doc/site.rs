//! `vibe doc build-site` — the registry builder's surface
//! (PROP-057 `##SITE-TWO-SOURCES`, campaign atom A5.1).
//!
//! The thin half, like every other `vibe doc` verb: it turns flags into
//! the library's options, hands down the ambient values the composition
//! root resolved, and prints the report. Everything that decides
//! anything lives in `vibe_doc::site`.
//!
//! The first thing a run prints is the configuration as it was RESOLVED —
//! every value, including the ones nobody wrote down. That is deliberate
//! and it is the report's main job: the configuration is read by a
//! generated type, which is permissive like every generated reader in
//! this tree, so a misspelled `[souce.host]` is not a refusal but a host
//! that is not there. Saying which sources were understood, at the top of
//! the run, catches that in the place an operator is already looking.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-TWO-SOURCES");

use anyhow::Result;
use vibe_doc::site::Site;

use super::DocEnv;
use crate::cli::DocBuildSiteArgs;

/// Run `vibe doc build-site`.
pub fn run(args: DocBuildSiteArgs, _env: DocEnv) -> Result<()> {
    let site = Site::read(&args.config)?;
    print!("{}", site.render());
    if args.dry_run {
        println!("  dry run — nothing was written");
        return Ok(());
    }
    println!("  output {}", args.out.display());
    Ok(())
}

#[cfg(test)]
mod tests;
