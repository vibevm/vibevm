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
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_doc::citations::{self, SpecSources};
use vibe_doc::coverage;
use vibe_doc::derived;
use vibe_doc::examples::{self, RunnerEnv};
use vibe_doc::translations;

use crate::cli::{DocArgs, DocCheckArgs, DocCommand, ProgressCommonArgs};

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
    if !args.examples && !args.derived && !args.citations && !args.translations && !args.coverage {
        bail!(
            "`vibe doc check` needs a check to run: `--examples`, `--derived`, \
             `--citations`, `--translations`, `--coverage`, or any combination \
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

    if args.citations {
        let coordinate = derived::coordinate_of(&args.path)?;
        let sources = spec_sources(&runner.repo_root, runner.settings_home.as_deref());
        let report = citations::check(&args.path, &coordinate, &sources)?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "a documented rule cites an address that no longer resolves (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED; \
                 fix: correct the address, or leave a tombstone where the rule was renamed)"
            );
        }
    }

    if args.translations {
        let sources = spec_sources(&runner.repo_root, runner.settings_home.as_deref());
        let report = translations::check(&args.path, &sources)?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "a translation does not mirror the documentation it adapts (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR; \
                 fix: repair the translation against its source — never the other way round)"
            );
        }
    }

    if args.coverage {
        let coordinate = derived::coordinate_of(&args.path)?;
        let sources = spec_sources(&runner.repo_root, runner.settings_home.as_deref());
        let report = coverage_report(
            &args.path,
            args.min,
            &runner.repo_root,
            &coordinate,
            &sources,
        )?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "the documentation does not tell everything the specifications promised: \
                 {}% of {} audience pair(s) covered, {} required, {} page(s) unreadable \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE; \
                 fix: write the page — the gate closes on a page for that audience citing \
                 the rule, never on a list of pages)",
                report.percent(),
                report.owed(),
                report.min_percent,
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

/// The coverage gate's two halves, each fetched from the one place that
/// owns it.
///
/// The obligations come from the grounding cell every `vibe facts` verb
/// enters through — the same include globs, the same exclusions, the same
/// per-package vocabulary dispatch. Enumerating the corpus a second time
/// here would give the gate a corpus nobody else can see, and the day the
/// two lists disagreed the gate would be measuring the disagreement.
///
/// A check run outside a checkout has no corpus at all: `--coverage` then
/// refuses rather than reporting a green nothing, because «no obligations
/// found» and «no obligations» print the same number.
fn coverage_report(
    package_dir: &Path,
    min: u8,
    repo_root: &Option<PathBuf>,
    coordinate: &str,
    sources: &SpecSources,
) -> Result<coverage::Report> {
    let Some(root) = repo_root.clone() else {
        bail!(
            "`--coverage` needs the tree whose specifications state the obligations, and \
             this run has no working directory to read one from \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE; \
             fix: run it from the project whose `facts.toml` names the observed corpus)"
        );
    };
    let grounded = crate::commands::progress::grounding::ground(&ProgressCommonArgs {
        path: root,
        campaign: None,
        no_cache: false,
    })?;
    let obligations = coverage::obligations(grounded.docs.iter());
    Ok(coverage::check(
        package_dir,
        coordinate,
        sources,
        &grounded.root,
        obligations,
        min,
    )?)
}

/// The world a `rule` citation resolves against (PROP-057
/// `##LOCAL-WARMUP`): the checkout the operator is standing in, the
/// packages it authors in-tree, the instances its lock selected, and the
/// machine store `vibe cache add` warms. The library discovers none of
/// them — naming them here is what keeps a documentation build
/// reproducible by inspection.
///
/// Without a working directory there is no checkout, and the store alone
/// answers: that is the local reader's own situation, reading
/// documentation for a project it is not inside.
fn spec_sources(repo_root: &Option<PathBuf>, settings_home: Option<&Path>) -> SpecSources {
    let sources = match repo_root {
        Some(root) => {
            let (group, name) = self_coordinate(root);
            SpecSources::for_checkout(root, group.as_deref(), &name)
        }
        None => SpecSources::new(),
    };
    match settings_home {
        Some(home) => sources.with_store(home.join(STORE_DIR)),
        None => sources,
    }
}

/// The machine store's directory under the settings home — the same
/// `<settings>/cache` [`vibe_registry::store::store_root`] resolves, named
/// here because this command hands the path down instead of letting a
/// library read the environment for it.
const STORE_DIR: &str = "cache";

/// The checkout's own `[project]` group and name — the coordinate a
/// `spec://` address must carry to reach its authored specs (B-031).
///
/// Read as TOML data: the one question is «what coordinate does this
/// directory answer to», and a strict manifest parse would make it fail
/// over a field it never reads. A directory with no manifest answers to
/// no coordinate, which is a legitimate state — then only the other three
/// sources speak.
fn self_coordinate(root: &Path) -> (Option<String>, String) {
    let Ok(text) = std::fs::read_to_string(root.join("vibe.toml")) else {
        return (None, String::new());
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return (None, String::new());
    };
    let read = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let group = read("project", "group").or_else(|| read("package", "group"));
    let name = read("project", "name")
        .or_else(|| read("package", "name"))
        .unwrap_or_default();
    (group, name)
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
            citations: false,
            translations: false,
            coverage: false,
            min: vibe_doc::coverage::FULL_COVERAGE,
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
        assert!(e.to_string().contains("--translations"), "{e}");
        assert!(e.to_string().contains("--coverage"), "{e}");
        assert!(e.to_string().contains("PROP-057#PIPE-LIBRARY"), "{e}");
    }

    /// The gate measures a corpus, and a run with no tree to read one
    /// from has none. It refuses rather than reporting the green nothing
    /// an empty obligation list would print — «zero promises found» and
    /// «zero promises» are the same number and not the same fact.
    #[test]
    fn coverage_without_a_tree_refuses_instead_of_reporting_nothing() {
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            tmp.path().join("vibe.toml"),
            "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
        )
        .expect("write");
        let checked = DocCheckArgs {
            coverage: true,
            path: tmp.path().to_path_buf(),
            ..args()
        };
        let e = run_check(checked, DocEnv::default()).expect_err("refused");
        assert!(e.to_string().contains("OBS-COVERAGE-GATE"), "{e}");
        assert!(e.to_string().contains("facts.toml"), "{e}");
    }

    /// A package that adapts nothing passes `--translations` and says
    /// why. The surface is thin on purpose: the verdict, the wording and
    /// the exit code all come from the library, and this proves the flag
    /// reaches it.
    #[test]
    fn the_translations_check_is_green_on_a_source_documentation() {
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            tmp.path().join("vibe.toml"),
            "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
        )
        .expect("write");
        let checked = DocCheckArgs {
            translations: true,
            path: tmp.path().to_path_buf(),
            ..args()
        };
        run_check(checked, DocEnv::default()).expect("nothing to mirror is a green state");
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

    /// The composition root NAMES the four citation sources; the library
    /// discovers none of them. Without a working directory there is no
    /// checkout — the local reader's own situation.
    #[test]
    fn without_a_working_directory_the_citation_world_has_no_checkout() {
        let world = spec_sources(&None, Some(Path::new("/home/u/.vibe")));
        assert!(!world.has_checkout());
    }

    /// A directory that carries no manifest answers to no coordinate,
    /// which is a state, not a failure.
    #[test]
    fn a_directory_with_no_manifest_answers_to_no_coordinate() {
        let tmp = tempfile::tempdir().expect("temp dir");
        assert_eq!(self_coordinate(tmp.path()), (None, String::new()));
    }

    /// The project's own coordinate is what a `spec://` address must
    /// carry to reach its authored specs.
    #[test]
    fn the_checkout_coordinate_is_read_from_the_project_table() {
        let tmp = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            tmp.path().join("vibe.toml"),
            "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\n",
        )
        .expect("write");
        assert_eq!(
            self_coordinate(tmp.path()),
            (Some("org.vibevm.core".to_string()), "vibevm".to_string())
        );
    }
}
