//! `vibe doc check` — the thin surface over the documentation library
//! (PROP-057 `##PIPE-LIBRARY`, `##INV-LOGIC-IN-THE-LIBRARY`).
//!
//! Everything with content in it lives in `vibe-doc`. This module does
//! three things and no more: it turns flags into the library's options,
//! it turns the composition root's environment into the library's
//! `RunnerEnv` (the library reads no ambient environment of its own), and
//! it prints the report and sets the exit code.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Result, bail};
use vibe_doc::derived;
use vibe_doc::examples::{self, RunnerEnv};

use crate::cli::{DocArgs, DocCheckArgs, DocCommand};

/// The ambient values the composition root resolves and hands down, so no
/// module below `main` reads the environment (the conform ambient-env
/// gate, and the reason behind it: a check whose behaviour depends on an
/// unnamed variable is a check nobody can reproduce).
#[derive(Debug, Clone, Default)]
pub struct DocEnv {
    /// `$VIBE_SETTINGS`, when the operator relocated the settings dir.
    pub settings: Option<OsString>,
    /// The operator's home, for the real `~/.vibe` the tripwire guards.
    pub home: Option<OsString>,
    /// The system temporary directory — where sandboxes go by default.
    pub temp: PathBuf,
    /// The working directory, which is the source tree during a panel run.
    pub cwd: Option<PathBuf>,
    /// The running binary: the default subject of every example.
    pub current_exe: Option<PathBuf>,
    /// The process id, so two runs on one machine cannot share a sandbox.
    pub pid: u32,
}

/// Run `vibe doc …`.
pub fn run(args: DocArgs, env: DocEnv) -> Result<()> {
    match args.command {
        DocCommand::Check(check) => run_check(check, env),
    }
}

fn run_check(args: DocCheckArgs, env: DocEnv) -> Result<()> {
    if !args.examples && !args.derived {
        bail!(
            "`vibe doc check` needs a check to run: `--examples`, `--derived`, or both \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY)"
        );
    }
    let runner = RunnerEnv {
        binary: args
            .binary
            .or(env.current_exe)
            .unwrap_or_else(|| PathBuf::from("vibe")),
        sandbox_root: args
            .sandbox
            // Short by default: the sandbox holds a materialised
            // dependency tree, and Windows counts every character of the
            // path that leads to it.
            .unwrap_or_else(|| env.temp.join("vdocs").join(env.pid.to_string())),
        repo_root: env.cwd,
        user_home: env.home.as_ref().map(PathBuf::from),
        settings_home: settings_home(&env.settings, &env.home),
        cargo: PathBuf::from("cargo"),
        timeout_secs: args.timeout,
    };
    let options = examples::Options {
        accept: args.accept,
        force: args.force,
        only: args.only.clone(),
    };

    if args.examples {
        let report = examples::check(&args.path, &runner, &options)?;
        print!("{}", report.render());
        if !report.ok() {
            let counts = report.counts();
            bail!(
                "documented examples do not match the product: {} diverged, {} could not run, \
                 {} page(s) unreadable (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#INV-EXAMPLES-RUN; \
                 fix: repair the product, or the page — never the comparison)",
                counts.differ,
                counts.failed,
                report.unreadable.len()
            );
        }
    }

    if args.derived {
        let env = derived::DerivedEnv {
            binary: runner.binary.clone(),
            // The schemas and the format registry live in the tree, not
            // in the package: a `jtd-schema` reference addresses the
            // project that publishes the format.
            repo_root: runner
                .repo_root
                .clone()
                .unwrap_or_else(|| args.path.clone()),
            coordinate: derived::coordinate_of(&args.path)?,
            timeout_secs: args.timeout,
        };
        let report = derived::check(&args.path, &env, args.accept)?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "a `derived` reference no longer builds to what the record holds (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-045#ROW-DOCVOCAB-DERIVED-CHECK; \
                 fix: read what moved, update the prose around it, and re-run with --accept)"
            );
        }
    }
    Ok(())
}

/// The real per-user settings directory the tripwire guards: the
/// relocation variable when it is set, else `<home>/.vibe`.
fn settings_home(settings: &Option<OsString>, home: &Option<OsString>) -> Option<PathBuf> {
    if let Some(dir) = settings {
        return Some(PathBuf::from(dir));
    }
    home.as_ref().map(|h| PathBuf::from(h).join(".vibe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> DocCheckArgs {
        DocCheckArgs {
            examples: false,
            derived: false,
            accept: false,
            force: false,
            only: None,
            path: PathBuf::from("."),
            binary: None,
            sandbox: None,
            timeout: 300,
        }
    }

    #[test]
    fn a_check_with_no_check_named_says_which_ones_exist() {
        let e = run_check(args(), DocEnv::default()).expect_err("refused");
        assert!(e.to_string().contains("--examples"), "{e}");
        assert!(e.to_string().contains("PROP-057#PIPE-LIBRARY"), "{e}");
    }

    #[test]
    fn the_real_home_is_the_relocation_variable_when_the_operator_set_one() {
        let picked = settings_home(
            &Some(OsString::from("/tmp/elsewhere")),
            &Some(OsString::from("/home/u")),
        );
        assert_eq!(picked, Some(PathBuf::from("/tmp/elsewhere")));
    }

    #[test]
    fn without_a_relocation_the_guarded_home_is_dot_vibe_under_the_user() {
        let picked = settings_home(&None, &Some(OsString::from("/home/u")));
        assert_eq!(picked, Some(PathBuf::from("/home/u/.vibe")));
    }

    #[test]
    fn with_no_home_at_all_the_tripwire_simply_has_no_home_half() {
        assert_eq!(settings_home(&None, &None), None);
    }
}
