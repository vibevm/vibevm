//! `vibe doc todo` — the thin surface over the library's maintenance
//! queue (PROP-057 `##PIPE-LIBRARY`, `##OBS-MAINTENANCE-TOOLS`).
//!
//! It turns flags into the library's inputs, resolves the ambient values
//! the library refuses to read for itself — the clock, the checkout, the
//! obligations, the binary an example runs — and prints.
//!
//! It always returns success. That is the norm and not an oversight: no
//! technical gate binds a release of the product to its documentation,
//! because ten releases a day and a hundred pull requests make drift
//! between reconciliations an accepted risk, and the answer to an
//! accepted risk is a number somebody reads (`##OBS-NO-RELEASE-LOCK`).
//! The checks that DO stop a build are `vibe doc check`, and they stop it
//! for internal breakage of the documentation alone.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use vibe_doc::examples::RunnerEnv;
use vibe_doc::surface::SurfaceEnv;
use vibe_doc::todo;

use super::DocEnv;
use crate::cli::DocTodoArgs;

/// The host's debt file, by the name `MAINTENANCE.md` §3 gives it.
const BACKLOG: &str = "BACKLOG.md";

/// The journal, by the name D-26 gives it. It lives in the documentation
/// package after the campaign that wrote it ends; before then a caller
/// points `--journal` at the campaign zone.
const JOURNAL: &str = "JOURNAL.md";

/// `vibe doc todo`.
pub fn run_todo(args: DocTodoArgs, env: DocEnv) -> Result<()> {
    let binary = args
        .binary
        .clone()
        .or_else(|| env.current_exe.clone())
        .unwrap_or_else(|| PathBuf::from("vibe"));
    let settings = super::settings_home(&env.settings, &env.home);
    let inputs = todo::Inputs {
        coordinate: vibe_doc::derived::coordinate_of(&args.path)?,
        package_version: package_version(&args.path),
        // The product's own declared number. It is what the version
        // change section compares a recorded snapshot against, and it is
        // the only number this project compares versions by.
        product_version: env!("CARGO_PKG_VERSION").to_owned(),
        sources: super::spec_sources(&env.cwd, settings.as_deref()),
        corpus_root: env.cwd.clone(),
        obligations: super::surface::obligations(&env.cwd)?,
        min: args.min,
        // The clock is called HERE and nowhere below: the age of a page
        // is an input to the queue, and a library that read the clock
        // would give two answers to one tree.
        today: Utc::now().date_naive(),
        backlog: args
            .backlog
            .clone()
            .or_else(|| env.cwd.as_ref().map(|cwd| cwd.join(BACKLOG))),
        journal: args.journal.clone().or(Some(args.path.join(JOURNAL))),
        examples: args.examples.then(|| RunnerEnv {
            binary: binary.clone(),
            sandbox_root: args
                .sandbox
                .clone()
                .unwrap_or_else(|| env.temp.join("vdocs").join(env.pid.to_string())),
            repo_root: env.cwd.clone(),
            user_home: env.home.as_ref().map(PathBuf::from),
            settings_home: settings.clone(),
            cargo: PathBuf::from("cargo"),
            timeout_secs: args.timeout,
        }),
        surface: env.cwd.clone().map(|repo_root| SurfaceEnv {
            binary,
            repo_root,
            timeout_secs: args.timeout,
        }),
    };
    let queue = todo::build(&args.path, &inputs)?;
    if args.format == "json" {
        print!("{}", todo::to_json(&queue));
    } else {
        print!("{}", todo::report::render_md(&queue));
    }
    Ok(())
}

/// The package's own version, read as TOML data.
///
/// Data and not the typed model, for the reason the rest of this pipeline
/// reads a manifest that way: the one question is «what does this package
/// call itself», and a strict parse would fail over a field nobody here
/// looks at. A package that does not say is reported as not saying.
fn package_version(package_dir: &std::path::Path) -> String {
    let Ok(text) = std::fs::read_to_string(package_dir.join("vibe.toml")) else {
        return String::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return String::new();
    };
    value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests;
