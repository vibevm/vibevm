//! The example runner — «the docs cannot lie» made mechanical
//! (PROP-057 `##PIPE-EXAMPLE-RUNNER`, `##INV-EXAMPLES-RUN`).
//!
//! Every `<example>` on every page of a documentation package runs against
//! the built binary, in a fresh sandbox built from the fixture the example
//! names, and its three results — stdout, stderr and the exit code — are
//! compared EXACTLY with what the page promises, after the normalisation
//! the fixture declares.
//!
//! What makes it a check rather than a ritual:
//!
//! * **No match templates.** A line the product does not promise to keep
//!   stable is closed by a named rule in the fixture, where a reviewer
//!   reads it, or the product is fixed. A comparison is never loosened to
//!   turn a check green.
//! * **Silence is an assertion.** An absent `<stderr>` says «this command
//!   writes nothing to stderr», and an empty `<expect>` says «it prints
//!   nothing». Both are claims the runner tests, which is why an example
//!   that has not been captured yet is DECLARED in `examples/deferred.toml`
//!   with its reason and skipped, instead of sitting on a page as an empty
//!   golden that looks like an assertion (X-026).
//! * **The isolation cannot be assumed.** The real per-user home and the
//!   source tree are recorded before the run and compared after it. A
//!   runner that wrote outside its sandbox has already invalidated every
//!   green line it printed.
//! * **`--json` is checked against its schema**, document by document, by
//!   a validator this crate brings along — `vibe-wire` has none, because
//!   its schemas are codegen input. A document the fixture maps to no
//!   schema is reported UNCHECKED, never as passed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER");

pub mod accept;
pub mod command;
pub mod fixture;
pub mod jtd;
pub mod normalize;
pub mod report;
pub mod sandbox;
pub mod tripwire;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use vibe_specdoc::doc::{Block, BlockNode, Cond, CondOs, Section, SpecDoc};

use crate::error::{DocError, Result};
use crate::pages::{self, Page};

pub use report::{Counts, JsonVerdict, Outcome, Report, Verdict};
pub use sandbox::RunnerEnv;

/// How one run of the check behaves.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Record a capture on the page when the golden is empty.
    pub accept: bool,
    /// With `accept`, also replace a golden that already holds text. A
    /// deliberate, separate decision: silently re-blessing a divergence is
    /// the one move the norm forbids by name.
    pub force: bool,
    /// Run only the examples whose `page#id` contains this text.
    pub only: Option<String>,
}

/// One example, lifted off its page with everything the runner needs.
#[derive(Debug, Clone)]
struct Collected {
    page: String,
    path: PathBuf,
    id: String,
    fixture: String,
    exit: i32,
    run: String,
    expect: String,
    stderr: Option<String>,
    when: Option<Cond>,
}

/// Run every example of a documentation package and report.
pub fn check(package_dir: &Path, env: &RunnerEnv, opts: &Options) -> Result<Report> {
    let set = pages::read_package(package_dir)?;
    let deferred = fixture::read_deferred(package_dir)?;
    let before = tripwire::snapshot(env);

    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut built: BTreeMap<String, sandbox::Materialised> = BTreeMap::new();
    for page in &set.pages {
        for example in collect(page) {
            let address = format!("{}#{}", example.page, example.id);
            if opts.only.as_ref().is_some_and(|f| !address.contains(f)) {
                continue;
            }
            let skip = deferred
                .iter()
                .find(|d| d.page == example.page && d.id == example.id)
                .map(|d| format!("not captured yet ({}): {}", d.captured.as_str(), d.reason))
                .or_else(|| off_platform(example.when.as_ref()));
            let (verdict, json) = match skip {
                Some(reason) => (Verdict::Skipped { reason }, Vec::new()),
                None => one(package_dir, env, opts, &example, &mut built),
            };
            outcomes.push(Outcome {
                page: example.page.clone(),
                id: example.id.clone(),
                fixture: example.fixture.clone(),
                run: example.run.clone(),
                verdict,
                json,
            });
        }
    }

    tripwire::verify(&before, env)?;
    Ok(Report {
        outcomes,
        unreadable: set.unreadable,
        sandbox_root: env.sandbox_root.clone(),
    })
}

/// Run one example and judge it. Every failure here is an OUTCOME, never
/// an early return: one broken fixture must not hide the forty examples
/// behind it.
fn one(
    package_dir: &Path,
    env: &RunnerEnv,
    opts: &Options,
    example: &Collected,
    built: &mut BTreeMap<String, sandbox::Materialised>,
) -> (Verdict, Vec<JsonVerdict>) {
    match attempt(package_dir, env, opts, example, built) {
        Ok(pair) => pair,
        Err(e) => (
            Verdict::Failed {
                message: e.to_string(),
            },
            Vec::new(),
        ),
    }
}

fn attempt(
    package_dir: &Path,
    env: &RunnerEnv,
    opts: &Options,
    example: &Collected,
    built: &mut BTreeMap<String, sandbox::Materialised>,
) -> Result<(Verdict, Vec<JsonVerdict>)> {
    let decl = fixture::read_fixture(package_dir, &example.fixture)?;
    let base = sandbox::build_fixture(package_dir, env, &example.fixture, built)?;
    let run_dir = env
        .sandbox_root
        .join("r")
        .join(slug(&example.page, &example.id));
    let live = sandbox::clone_for_example(&base, run_dir)?;

    let cmd = command::parse(&example.run)?;
    let capture = sandbox::run(env, &cmd, &live.cwd, &live.root)?;

    let places = normalize::Placeholders {
        sandbox: Some(normalize::display_path(&live.root)),
        repo: env.repo_root.as_deref().map(normalize::display_path),
        home: env.user_home.as_deref().map(normalize::display_path),
    };
    let norm = normalize::Normalizer::compile(&example.fixture, &decl.normalize, places)?;
    let stdout = norm.apply(&capture.stdout);
    let stderr = norm.apply(&capture.stderr);
    let want_out = norm.apply(&example.expect);
    let want_err = norm.apply(example.stderr.as_deref().unwrap_or(""));

    let json = validate_json(package_dir, env, &decl.jtd, &capture.stdout);

    let mut problems: Vec<String> = Vec::new();
    if capture.code != example.exit {
        problems.push(format!(
            "  exit: golden {} — captured {}\n",
            example.exit, capture.code
        ));
    }
    if stdout != want_out {
        problems.push(report::diff("stdout", &want_out, &stdout));
    }
    if stderr != want_err {
        problems.push(report::diff("stderr", &want_err, &stderr));
    }
    if problems.is_empty() {
        return Ok((Verdict::Match, json));
    }
    if opts.accept {
        let filled = example.expect.is_empty();
        if filled || opts.force {
            accept::write(
                &example.path,
                &example.id,
                &accept::Golden {
                    expect: stdout,
                    stderr: (!stderr.is_empty()).then_some(stderr),
                    exit: capture.code,
                },
                opts.force,
            )?;
            return Ok((Verdict::Accepted { filled }, json));
        }
    }
    Ok((
        Verdict::Differ {
            diff: problems.concat(),
        },
        json,
    ))
}

/// Parse a `--json` stream document by document and check each against
/// the schema its `command` names in the fixture's map.
fn validate_json(
    package_dir: &Path,
    env: &RunnerEnv,
    map: &BTreeMap<String, String>,
    stdout: &str,
) -> Vec<JsonVerdict> {
    let trimmed = stdout.trim_start();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return Vec::new();
    }
    let base = env
        .repo_root
        .clone()
        .unwrap_or_else(|| package_dir.to_path_buf());
    // A schema refers by bare name to the SHARED vocabulary the formats
    // register keeps in one file; without it a perfectly valid document
    // would be reported as citing an undefined definition.
    let vocabulary = std::fs::read_to_string(base.join(SHARED_VOCABULARY))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    jtd::split_documents(stdout)
        .into_iter()
        .map(|doc| {
            let name = doc
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("<unnamed>")
                .to_owned();
            let Some(rel) = map.get(&name) else {
                return JsonVerdict {
                    command: name,
                    schema: None,
                    violations: Vec::new(),
                };
            };
            let path = base.join(rel);
            let violations = match std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            {
                Some(schema) => {
                    let schema = match &vocabulary {
                        Some(shared) => jtd::with_vocabulary(schema, shared),
                        None => schema,
                    };
                    jtd::validate(&schema, &doc)
                        .into_iter()
                        .map(|v| v.to_string())
                        .collect()
                }
                None => vec![format!(
                    "the schema `{}` is missing or not JSON",
                    path.display()
                )],
            };
            JsonVerdict {
                command: name,
                schema: Some(rel.clone()),
                violations,
            }
        })
        .collect()
}

/// Where the project keeps the shared JTD definitions its schemas refer
/// to by bare name.
const SHARED_VOCABULARY: &str = "formats/vocabularies.json";

/// An example whose slot is conditioned on another platform does not run
/// here, and says which platform it belongs to.
fn off_platform(when: Option<&Cond>) -> Option<String> {
    let Cond::Os(os) = when? else {
        return None;
    };
    let here = if cfg!(windows) {
        CondOs::Windows
    } else if cfg!(target_os = "macos") {
        CondOs::Macos
    } else {
        CondOs::Linux
    };
    (*os != here).then(|| {
        format!(
            "declared for {} — this machine is {}",
            os.as_str(),
            here.as_str()
        )
    })
}

/// Lift every example off one page, preamble and sections alike.
fn collect(page: &Page) -> Vec<Collected> {
    fn from_blocks(page: &Page, blocks: &[BlockNode], out: &mut Vec<Collected>) {
        for node in blocks {
            if let Block::Example {
                id,
                fixture,
                exit,
                run,
                expect,
                stderr,
                ..
            } = &node.block
            {
                out.push(Collected {
                    page: page.rel.clone(),
                    path: page.path.clone(),
                    id: id.clone(),
                    fixture: fixture.clone(),
                    exit: exit.unwrap_or(0),
                    run: run.clone(),
                    expect: expect.clone(),
                    stderr: stderr.clone(),
                    when: node.when,
                });
            }
        }
    }
    fn from_section(page: &Page, section: &Section, out: &mut Vec<Collected>) {
        from_blocks(page, &section.blocks, out);
        for sub in &section.sections {
            from_section(page, sub, out);
        }
    }
    let doc: &SpecDoc = &page.doc;
    let mut out = Vec::new();
    from_blocks(page, &doc.preamble, &mut out);
    for section in &doc.sections {
        from_section(page, section, &mut out);
    }
    out
}

/// A short, filesystem-safe name for one example's run directory. Short
/// on purpose: the sandbox holds a materialised dependency tree, and
/// Windows counts every character of the path that leads to it.
fn slug(page: &str, id: &str) -> String {
    let stem = page
        .rsplit('/')
        .next()
        .unwrap_or(page)
        .trim_end_matches(".xml");
    let raw = format!("{stem}-{id}");
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(40)
        .collect()
}

/// The error a surface renders when a package names no fixtures at all.
pub fn missing_fixtures(package_dir: &Path) -> DocError {
    DocError::Fixture {
        fixture: fixture::FIXTURES_DIR.to_owned(),
        message: format!(
            "`{}` holds no fixtures, so no example can run",
            package_dir.join(fixture::FIXTURES_DIR).display()
        ),
    }
}

#[cfg(test)]
mod tests;
