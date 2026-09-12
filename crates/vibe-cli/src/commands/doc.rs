//! `vibe doc check` — the thin surface over the documentation library
//! (PROP-057 `##PIPE-LIBRARY`, `##INV-LOGIC-IN-THE-LIBRARY`).
//!
//! Everything with content in it lives in `vibe-doc`. This module does
//! three things and no more: it turns flags into the library's options,
//! it turns the composition root's environment into the library's
//! `RunnerEnv` (the library reads no ambient environment of its own), and
//! it prints the report and sets the exit code.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use chrono::Utc;
use vibe_doc::build;
use vibe_doc::citations::{self, SpecSources};
use vibe_doc::coverage;
use vibe_doc::derived;
use vibe_doc::examples::{self, RunnerEnv};
use vibe_doc::llms;
use vibe_doc::manifest;
use vibe_doc::media;
use vibe_doc::style;
use vibe_doc::translations;

use crate::cli::{
    DocArgs, DocBuildArgs, DocCheckArgs, DocCommand, DocManifestArgs, DocServeArgs,
    ProgressCommonArgs,
};

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
        DocCommand::Build(build) => run_build(build, env),
        DocCommand::Check(check) => run_check(check, env),
        DocCommand::Manifest(manifest) => run_manifest(manifest, env),
        DocCommand::Serve(serve) => run_serve(serve, env),
    }
}

/// `vibe doc build` — render the package into a directory that can be
/// served as it stands.
fn run_build(args: DocBuildArgs, env: DocEnv) -> Result<()> {
    if let Some(lang) = &args.lang {
        build::expect_language(&args.path, lang)?;
    }
    let sources = spec_sources(&env.cwd, settings_home(&env.settings, &env.home).as_deref());
    // clap admits only the three the parser lists, so this arm is
    // unreachable today; it is a refusal rather than a panic because the
    // parser's list and this match are two places, and the day they
    // disagree an operator should read a sentence, not a backtrace.
    let Some(format) = build::Format::parse(&args.format) else {
        bail!(
            "`--format {}` is not a projection this build writes: `html`, `md` or `xml` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY)",
            args.format
        );
    };
    let derived = if args.no_derived {
        BTreeMap::new()
    } else {
        generated_derived(&args.path, &env, args.binary.clone())?
    };
    let options = build::Options {
        format,
        base: args.base.clone(),
        // The clock is called HERE and nowhere below: the library
        // renders the same bytes from the same tree, and the instant is
        // an input to that (PROP-044 `##M-CANONICAL-BYTES`).
        manifest: manifest::Options::at(Utc::now()),
        derived,
    };
    let built = build::build(&args.path, &sources, &options)?;
    build::write(&built, &args.out)?;
    print!("{}", built.render());
    println!("  written under {}", args.out.display());
    Ok(())
}

/// `vibe doc manifest` — the machine's view of the package.
fn run_manifest(args: DocManifestArgs, env: DocEnv) -> Result<()> {
    if let Some(lang) = &args.lang {
        build::expect_language(&args.path, lang)?;
    }
    let sources = spec_sources(&env.cwd, settings_home(&env.settings, &env.home).as_deref());
    let options = manifest::Options::at(Utc::now());
    let built = manifest::build(&args.path, &sources, &options)?;

    if let Some(tier) = &args.llms {
        let tier = match tier.as_str() {
            "index" => llms::Tier::Index,
            "small" => llms::Tier::Small,
            "medium" => llms::Tier::Medium,
            _ => llms::Tier::Full,
        };
        let (set, content) = build::content(&args.path, &sources, &args.base, BTreeMap::new())?;
        let bodies = llms::bodies(&set, &content);
        print!(
            "{}",
            llms::render(tier, &built.manifest, &bodies, &args.base)
        );
        return Ok(());
    }

    if args.json {
        print!("{}", manifest::to_json(&built.manifest));
        return Ok(());
    }

    // The human form, which is a summary and not a second document:
    // anything that wants the whole thing asks for `--json`.
    let card = &built.manifest.package;
    println!(
        "{} {}/{}@{} ({}, {})",
        card.title,
        card.group.as_str(),
        card.name,
        card.version,
        card.lang,
        llms::status_word(&card.status)
    );
    println!("  {} page(s)", built.manifest.pages.len());
    for page in &built.manifest.pages {
        println!(
            "  {:<44} {} min  {}",
            page.path,
            page.reading_time_min,
            llms::audience_list(&page.audiences)
        );
    }
    for page in &built.unreadable {
        println!("  unreadable {page}");
    }
    Ok(())
}

/// `vibe doc serve` — the local reader.
fn run_serve(args: DocServeArgs, env: DocEnv) -> Result<()> {
    let config = vibe_doc_server::Config {
        package_dir: args.path.clone(),
        base: args.base.clone(),
        port: args.port,
        frame_ancestor: args.frame_ancestor.clone(),
        lang: args.lang.clone(),
    };
    let sources = spec_sources(&env.cwd, settings_home(&env.settings, &env.home).as_deref());
    let reader = vibe_doc_server::Reader::open(&config, sources, Utc::now())?;
    let derived = if args.no_derived {
        BTreeMap::new()
    } else {
        generated_derived(&args.path, &env, args.binary.clone())?
    };
    vibe_doc_server::serve(reader.with_derived(derived), args.port)?;
    Ok(())
}

/// Generate the `derived` blocks once, from the product this run is.
///
/// A block's text is a function of the binary, and the binary does not
/// move while a build runs or a reader is up — so it is generated once
/// here and handed down, rather than per page or per request.
fn generated_derived(
    package_dir: &Path,
    env: &DocEnv,
    binary: Option<PathBuf>,
) -> Result<BTreeMap<String, String>> {
    let derived_env = derived::DerivedEnv {
        binary: binary
            .or_else(|| env.current_exe.clone())
            .unwrap_or_else(|| PathBuf::from("vibe")),
        repo_root: env.cwd.clone().unwrap_or_else(|| package_dir.to_path_buf()),
        coordinate: derived::coordinate_of(package_dir)?,
        timeout_secs: 300,
    };
    // Keyed the way a backend asks for it — by kind and reference, not
    // by the record's per-page row id: one `vibe list --help` is one
    // text however many pages show it.
    Ok(derived::generate(package_dir, &derived_env)?
        .into_iter()
        .map(|block| {
            (
                vibe_doc::content::Content::derived_key(block.kind, &block.reference),
                block.text,
            )
        })
        .collect())
}

fn run_check(args: DocCheckArgs, env: DocEnv) -> Result<()> {
    if !args.examples
        && !args.derived
        && !args.citations
        && !args.translations
        && !args.coverage
        && !args.media
        && !args.style
    {
        bail!(
            "`vibe doc check` needs a check to run: `--examples`, `--derived`, \
             `--citations`, `--translations`, `--coverage`, `--media`, `--style`, \
             or any combination \
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

    if args.style {
        let report = style::check(&args.path, args.min)?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "the prose does not answer to the style law: {} of {} page(s) clean, {} \
                 required, {} error(s) (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT; \
                 fix: rewrite the sentence — a false positive is fixed in the linter's rule, \
                 with a BACKLOG entry, never worked around in the text)",
                report.clean_pages(),
                report.pages.len() + report.unreadable.len(),
                report.min_percent,
                report.errors()
            );
        }
    }

    if args.media {
        let report = media::check(&args.path)?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "the card declares an image the site cannot serve: {} error(s), {} warning(s) \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                 fix: repair the image, or drop the role — a placeholder is generated for a \
                 role that declares nothing)",
                report.errors(),
                report.warnings()
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
mod tests;
