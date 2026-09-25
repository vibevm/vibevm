//! `vibe doc check` — the thin surface over the documentation library
//! (PROP-057 `##PIPE-LIBRARY`, `##INV-LOGIC-IN-THE-LIBRARY`).
//!
//! Everything with content in it lives in `vibe-doc`. This module does
//! three things and no more: it turns flags into the library's options,
//! it turns the composition root's environment into the library's
//! `RunnerEnv` (the library reads no ambient environment of its own), and
//! it prints the report and sets the exit code.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

mod env;
mod observe;
pub mod shell;
pub mod site;
pub mod surface;
pub mod todo;
pub use env::DocEnv;
// The environment helpers live in `env`, the module named for exactly
// that question, and are re-exported here so the sibling command
// modules keep reaching them as `super::spec_sources`. Out of line per
// the file-length budget.
use env::{STORE_DIR, preferred_language, self_coordinate, settings_home, spec_sources};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use chrono::Utc;
use vibe_core::progress::Progress;
use vibe_doc::build;
use vibe_doc::chapters;
use vibe_doc::citations::{self, SpecSources};
use vibe_doc::coverage;
use vibe_doc::derived;
use vibe_doc::examples::{self, RunnerEnv};
use vibe_doc::llms;
use vibe_doc::manifest;
use vibe_doc::media;
use vibe_doc::prompts;
use vibe_doc::style;
use vibe_doc::translations;

use crate::cli::{
    DocArgs, DocBuildArgs, DocCheckArgs, DocCommand, DocManifestArgs, DocServeArgs,
    ProgressCommonArgs,
};
use crate::output;

/// Run `vibe doc …`.
pub fn run(ctx: &output::Context, args: DocArgs, env: DocEnv) -> Result<()> {
    match args.command {
        DocCommand::Build(build) => run_build(build, env),
        DocCommand::BuildSite(site) => site::run(site, env),
        DocCommand::Check(check) => run_check(check, env),
        DocCommand::Manifest(manifest) => run_manifest(manifest, env),
        DocCommand::Serve(serve) => run_serve(serve, env),
        DocCommand::Shell(args) => shell::run(ctx, args, env),
        DocCommand::Surface(record) => surface::run_surface(record, env),
        DocCommand::Diff(diff) => surface::run_diff(diff, env),
        DocCommand::Todo(queue) => todo::run_todo(queue, env),
    }
}

/// `vibe doc build` — render the package into a directory that can be
/// served as it stands.
fn run_build(args: DocBuildArgs, env: DocEnv) -> Result<()> {
    if let Some(lang) = &args.lang {
        observe::phase(&env.progress, "Checking documentation language", || {
            build::expect_language(&args.path, lang)
        })?;
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
        observe::skipped(
            &env.progress,
            "Generating derived documentation",
            "disabled by --no-derived",
        );
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
    let built = build::build_observed(&args.path, &sources, &options, &env.progress)?;
    observe::phase(&env.progress, "Writing documentation output", || {
        build::write(&built, &args.out)
    })?;
    print!("{}", built.render());
    println!("  written under {}", args.out.display());
    Ok(())
}

/// `vibe doc manifest` — the machine's view of the package.
fn run_manifest(args: DocManifestArgs, env: DocEnv) -> Result<()> {
    if let Some(lang) = &args.lang {
        observe::phase(&env.progress, "Checking documentation language", || {
            build::expect_language(&args.path, lang)
        })?;
    }
    let sources = spec_sources(&env.cwd, settings_home(&env.settings, &env.home).as_deref());
    let options = manifest::Options::at(Utc::now());
    let built = observe::phase(&env.progress, "Building documentation manifest", || {
        manifest::build(&args.path, &sources, &options)
    })?;

    if let Some(tier) = &args.llms {
        let tier = match tier.as_str() {
            "index" => llms::Tier::Index,
            "small" => llms::Tier::Small,
            "medium" => llms::Tier::Medium,
            _ => llms::Tier::Full,
        };
        let (set, content) = observe::phase(&env.progress, "Reading documentation pages", || {
            build::content(&args.path, &sources, &args.base, BTreeMap::new())
        })?;
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
    // The shell is resolved before anything else, because `--print-shell`
    // is a question about this binary and not about any package: it must
    // answer on a machine with no documentation to point at.
    let shell_task = env.progress.task("Resolving documentation reader shell");
    let dressed = shell::resolve(&env, args.bare_shell);
    shell_task.finish();
    if args.print_shell {
        println!("{}", serde_json::to_string_pretty(&dressed.report())?);
        return Ok(());
    }
    let config = vibe_doc_server::Config {
        package_dir: args.path.clone(),
        base: args.base.clone(),
        port: args.port,
        frame_ancestor: args.frame_ancestor.clone(),
        // «The language preference comes from the project's
        // `[i18n].preferred` when present» (`##LOCAL-SERVE`), and the flag
        // overrides it. One reader serves one package, so a preference has
        // nothing to choose between — what it does is state which language
        // was expected, and the refusal names the adaptation that holds it.
        lang: args
            .lang
            .clone()
            .or_else(|| env.cwd.as_deref().and_then(preferred_language)),
    };
    let sources = spec_sources(&env.cwd, settings_home(&env.settings, &env.home).as_deref());
    let reader = observe::phase(&env.progress, "Preparing documentation reader", || {
        vibe_doc_server::Reader::wearing(&config, sources, Utc::now(), dressed)
    })?;
    let derived = if args.no_derived {
        observe::skipped(
            &env.progress,
            "Generating derived documentation",
            "disabled by --no-derived",
        );
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
    Ok(
        observe::phase(&env.progress, "Generating derived documentation", || {
            derived::generate(package_dir, &derived_env)
        })?
        .into_iter()
        .map(|block| {
            (
                vibe_doc::content::Content::derived_key(block.kind, &block.reference),
                block.text,
            )
        })
        .collect(),
    )
}

fn run_check(args: DocCheckArgs, env: DocEnv) -> Result<()> {
    if !args.examples
        && !args.derived
        && !args.citations
        && !args.translations
        && !args.chapters
        && !args.coverage
        && !args.media
        && !args.style
        && !args.prompts
    {
        bail!(
            "`vibe doc check` needs a check to run: `--examples`, `--derived`, \
             `--citations`, `--translations`, `--chapters`, `--coverage`, `--media`, \
             `--style`, `--prompts`, or any combination \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY)"
        );
    }
    let progress = env.progress.clone();
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
        let report = examples::check_observed(&args.path, &runner, &options, &progress)?;
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
        let report = observe::phase(&progress, "Checking documentation citations", || {
            citations::check(&args.path, &coordinate, &sources)
        })?;
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
        let report = observe::phase(&progress, "Checking documentation translations", || {
            translations::check(&args.path, &sources)
        })?;
        print!("{}", report.render());
        if !report.ok() {
            bail!(
                "a translation does not mirror the documentation it adapts (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR; \
                 fix: repair the translation against its source — never the other way round)"
            );
        }
    }

    // Beside the mirror check because both read the `[navigation]` table
    // across a package boundary, and before the expensive ones because it
    // runs no product and builds no sandbox. It is the only block here
    // that does not `bail!` on what it found: the norm rules the forward
    // links a measurement and not a gate
    // (`##NAV-CHAPTERS-CHECKED`), because an orientation page points
    // ahead on purpose.
    if args.chapters {
        let report = observe::phase(
            &progress,
            "Measuring the documentation learning path",
            || chapters::check(&args.path),
        )?;
        if args.json {
            print!("{}", chapters::to_json(&report));
        } else {
            print!("{}", report.render());
        }
    }

    if args.prompts {
        let report = observe::phase(&progress, "Checking documented prompts", || {
            prompts::check(
                &args.path,
                &runner,
                &prompts::Options {
                    runner: args.runner.clone(),
                    sample: args.sample,
                    only: args.only.clone(),
                },
            )
        })?;
        print!("{}", report.render());
        if !report.ok() {
            let counts = report.counts();
            bail!(
                "a documented prompt no longer gets the result the page promises: {} broke, \
                 {} could not run, {} page(s) unreadable (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST; \
                 fix: repair the product, or the prompt — never the assert)",
                counts.broke,
                counts.failed,
                report.unreadable.len()
            );
        }
    }

    if args.style {
        let report = observe::phase(&progress, "Checking documentation style", || {
            style::check(&args.path, args.min)
        })?;
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
        let report = observe::phase(&progress, "Checking documentation media", || {
            media::check(&args.path)
        })?;
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
            &progress,
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
        let report = observe::phase(&progress, "Checking derived documentation", || {
            derived::check(&args.path, &env, args.accept)
        })?;
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
    progress: &Progress,
) -> Result<coverage::Report> {
    let Some(root) = repo_root.clone() else {
        bail!(
            "`--coverage` needs the tree whose specifications state the obligations, and \
             this run has no working directory to read one from \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE; \
             fix: run it from the project whose `facts.toml` names the observed corpus)"
        );
    };
    let grounded = observe::phase(progress, "Grounding documentation obligations", || {
        crate::commands::progress::grounding::ground(&ProgressCommonArgs {
            path: root,
            campaign: None,
            no_cache: false,
        })
    })?;
    let obligations = coverage::obligations(grounded.docs.iter());
    Ok(observe::phase(
        progress,
        "Checking documentation coverage",
        || {
            coverage::check(
                package_dir,
                coordinate,
                sources,
                &grounded.root,
                obligations,
                min,
            )
        },
    )?)
}

#[cfg(test)]
mod tests;
