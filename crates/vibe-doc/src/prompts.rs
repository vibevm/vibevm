//! `vibe doc check --prompts` — running the half of the manual a reader
//! does not type (PROP-057 `##STYLE-PROMPT-FIRST`, PROP-045
//! `##ROW-DOCVOCAB-PROMPT-CHECK`).
//!
//! In VibeVM any action is done by an agent or by hand, and the agent is
//! the main road, so a task page opens with a request in the user's
//! voice. That request is documentation like any other, and documentation
//! that is never run goes stale in silence. The example runner cannot
//! help here: a prompt has no golden output. An agent says something
//! different every time it says it, and comparing what it said would make
//! this check fail on the weather.
//!
//! So the page states what must be TRUE afterwards. Each prompt carries
//! at least one `<assert>` — a command that must exit zero once the agent
//! has finished — and the check is: a clean sandbox built from the
//! prompt's fixture, the request handed to the configured agent, then the
//! asserts, in the order the page wrote them.
//!
//! ## Three properties this check keeps
//!
//! **It is outside the panel**, by the norm's own instruction. It calls a
//! real agent, it takes minutes, and it costs money; `tools/self-check.sh`
//! gets no step. It runs in the prose phase before acceptance, as a
//! sample in the monthly loop, and in full at a reconciliation.
//!
//! **The runner is configuration, not code.** The first is the campaign's
//! own worker tier; the second is a different vendor's agent, so that a
//! prompt which only works on one of them is a prompt this check can
//! catch. Neither is named here.
//!
//! **The sandbox is the example runner's.** Same fixtures, same recipes,
//! same retargeting, same tripwire over the real `~/.vibe` — because a
//! prompt and an example document the same product and must start from
//! the same state, and because an agent let loose in a working directory
//! is exactly what the isolation was built for.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST");

pub mod assert;
pub mod config;
pub mod report;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vibe_specdoc::doc::{Block, BlockNode, Section, SpecDoc};

use crate::error::{DocError, Result};
use crate::examples::fixture::HOME_DIR;
use crate::examples::normalize::{self, Normalizer};
use crate::examples::sandbox::{self, CLEARED, Capture, Materialised, RunnerEnv};
use crate::examples::{command, fixture, tripwire};
use crate::pages::{self, Page};

pub use config::Config;
pub use report::{Outcome, Report, Verdict};

/// The environment variables a scripted runner reads: the prompt on
/// disk as well as on its standard input, the directory it works in, the
/// binary under test, and what the page says the agent needs.
pub const ENV_PROMPT_FILE: &str = "VIBE_DOC_PROMPT_FILE";
pub const ENV_DIR: &str = "VIBE_DOC_DIR";
pub const ENV_BINARY: &str = "VIBE_DOC_BINARY";
pub const ENV_NEEDS: &str = "VIBE_DOC_NEEDS";

/// Where the prompt text is written inside the sandbox, for a runner
/// that would rather read a file than its standard input.
const PROMPT_FILE: &str = "prompt.txt";

/// How one run behaves.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The command to hand the prompt to, overriding the package's own.
    /// One corpus, two agents, for neutrality.
    pub runner: Option<String>,
    /// Take this many prompts instead of all of them. `None` uses the
    /// package's `sample`; `Some(0)` is every prompt.
    pub sample: Option<usize>,
    /// Run only the prompts whose `page#id` contains this text.
    pub only: Option<String>,
}

/// One prompt, lifted off its page.
#[derive(Debug, Clone)]
struct Collected {
    page: String,
    id: String,
    text: String,
    needs: Option<String>,
    asserts: Vec<String>,
}

/// Run a documentation package's prompts through the configured agent.
pub fn check(package_dir: &Path, env: &RunnerEnv, opts: &Options) -> Result<Report> {
    let config = config::read(package_dir)?;
    let runner = opts
        .runner
        .clone()
        .or_else(|| config.runner.clone())
        .ok_or_else(|| DocError::Prompt {
            message: format!(
                "no agent to hand the prompts to — declare `[doc.prompts] runner` in `{}`, \
                 or pass `--runner`",
                config::config_path(package_dir).display()
            ),
        })?;
    let set = pages::read_package(package_dir)?;
    // The tripwire matters more here than it does for the examples: an
    // example runs a command the page shows, and an AGENT is loose in a
    // directory with a machine around it.
    let before = tripwire::snapshot(env);

    let mut collected: Vec<Collected> = Vec::new();
    for page in &set.pages {
        collected.extend(collect(page));
    }
    let taken = choose(&collected, &config, opts);

    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut built: BTreeMap<String, Materialised> = BTreeMap::new();
    for prompt in &collected {
        let address = format!("{}#{}", prompt.page, prompt.id);
        if opts.only.as_ref().is_some_and(|f| !address.contains(f)) {
            continue;
        }
        let fixture = config.fixture_of(&prompt.page, &prompt.id).to_owned();
        let skip = skip_reason(prompt, &config, &taken, &address);
        let outcome = match skip {
            Some(reason) => Outcome {
                page: prompt.page.clone(),
                id: prompt.id.clone(),
                fixture,
                needs: prompt.needs.clone(),
                runner_code: None,
                tail: String::new(),
                asserts: Vec::new(),
                verdict: Verdict::Skipped { reason },
            },
            None => one(
                package_dir,
                env,
                &config,
                &runner,
                prompt,
                &fixture,
                &mut built,
            ),
        };
        outcomes.push(outcome);
    }

    tripwire::verify(&before, env)?;
    Ok(Report {
        outcomes,
        unreadable: set.unreadable.iter().map(|u| u.rel.clone()).collect(),
        runner,
    })
}

/// Why one prompt does not run, when it does not.
fn skip_reason(
    prompt: &Collected,
    config: &Config,
    taken: &[String],
    address: &str,
) -> Option<String> {
    if prompt.asserts.is_empty() {
        // `assert="none"` — an illustrative prompt, which the style
        // linter allows only off a scenario page. Running it would prove
        // nothing, because nothing was claimed.
        return Some("illustrative (assert=\"none\") — nothing was claimed to check".to_owned());
    }
    if let Some(reason) = config.skip_of(&prompt.page, &prompt.id) {
        return Some(format!("declared: {reason}"));
    }
    (!taken.iter().any(|a| a == address)).then(|| "outside this sample".to_owned())
}

/// Which prompts a sample takes.
///
/// The sample is random in the sense that matters — it is not the first
/// N of the corpus, so a run exercises pages nobody would have chosen —
/// and fixed in the sense that matters more: the same corpus and the same
/// number give the same prompts, so two runs a month apart are comparable
/// and a red one is reproducible. The order comes from a hash of the
/// address, which is the seed.
fn choose(collected: &[Collected], config: &Config, opts: &Options) -> Vec<String> {
    let limit = opts.sample.unwrap_or(config.sample);
    let mut addresses: Vec<String> = collected
        .iter()
        .map(|p| format!("{}#{}", p.page, p.id))
        .collect();
    if limit == 0 || limit >= addresses.len() {
        return addresses;
    }
    addresses.sort_by_key(|a| (seed(a), a.clone()));
    addresses.truncate(limit);
    addresses
}

/// FNV-1a over the address: a fixed seed, spelled out, so the sample does
/// not move when a standard library changes its hasher.
fn seed(address: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in address.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Run one prompt. Every failure is an OUTCOME, never an early return:
/// one broken fixture must not hide the nineteen prompts behind it.
fn one(
    package_dir: &Path,
    env: &RunnerEnv,
    config: &Config,
    runner: &str,
    prompt: &Collected,
    fixture: &str,
    built: &mut BTreeMap<String, Materialised>,
) -> Outcome {
    let mut outcome = Outcome {
        page: prompt.page.clone(),
        id: prompt.id.clone(),
        fixture: fixture.to_owned(),
        needs: prompt.needs.clone(),
        runner_code: None,
        tail: String::new(),
        asserts: Vec::new(),
        verdict: Verdict::Held,
    };
    match attempt(package_dir, env, config, runner, prompt, fixture, built) {
        Ok(done) => done,
        Err(e) => {
            outcome.verdict = Verdict::Failed {
                message: e.to_string(),
            };
            outcome
        }
    }
}

fn attempt(
    package_dir: &Path,
    env: &RunnerEnv,
    config: &Config,
    runner: &str,
    prompt: &Collected,
    fixture: &str,
    built: &mut BTreeMap<String, Materialised>,
) -> Result<Outcome> {
    let decl = fixture::read_fixture(package_dir, fixture)?;
    let base = sandbox::build_fixture(package_dir, env, fixture, built)?;
    let run_dir = env
        .sandbox_root
        .join("p")
        .join(slug(&prompt.page, &prompt.id));
    let live = sandbox::clone_for_example(&base, run_dir)?;

    let places = normalize::Placeholders {
        sandbox: Some(normalize::display_path(&live.root)),
        repo: env.repo_root.as_deref().map(normalize::display_path),
        home: env.user_home.as_deref().map(normalize::display_path),
    };
    let norm = Normalizer::compile(fixture, &decl.normalize, places)?;

    if let Some(skill) = &config.skill {
        project_skill(env, skill, &live)?;
    }

    let prompt_path = live.root.join(PROMPT_FILE);
    std::fs::write(&prompt_path, &prompt.text)
        .map_err(|e| DocError::io("writing", &prompt_path, e))?;

    let capture = invoke(runner, env, config, prompt, &prompt_path, &live)?;
    let mut outcome = Outcome {
        page: prompt.page.clone(),
        id: prompt.id.clone(),
        fixture: fixture.to_owned(),
        needs: prompt.needs.clone(),
        runner_code: Some(capture.code),
        tail: report::tail(&norm.apply(&join(&capture))),
        asserts: Vec::new(),
        verdict: Verdict::Held,
    };
    if capture.code != 0 {
        outcome.verdict = Verdict::Failed {
            message: format!("the runner exited {}", capture.code),
        };
        return Ok(outcome);
    }

    for line in &prompt.asserts {
        let parsed = assert::parse(line)?;
        let got = assert::run(
            &parsed,
            &env.binary,
            &live.cwd,
            &live.root,
            env.timeout_secs,
        );
        outcome.asserts.push(report::AssertOutcome {
            command: line.clone(),
            code: got.code,
            output: norm.apply(&join(&got)),
        });
    }
    if outcome.asserts.iter().any(|a| !a.passed()) {
        outcome.verdict = Verdict::Broke;
    }
    Ok(outcome)
}

/// Project the package's own skill into the sandbox project, when the
/// configuration names one.
///
/// Absent is the normal state and not a gap: a real agent's skill is part
/// of ITS environment, installed once on the machine that runs it, and
/// this check has no business writing to that machine. The setting exists
/// for the runner that is a script.
fn project_skill(env: &RunnerEnv, skill: &str, live: &Materialised) -> Result<()> {
    let line = format!("vibe skill install --skill {skill} --quiet");
    let cmd = command::parse(&line)?;
    let got = sandbox::run(env, &cmd, &live.cwd, &live.root)?;
    if got.code != 0 {
        return Err(DocError::Prompt {
            message: format!(
                "the skill `{skill}` did not project into the sandbox: {}",
                got.stderr.trim()
            ),
        });
    }
    Ok(())
}

/// Hand one prompt to the agent.
fn invoke(
    runner: &str,
    env: &RunnerEnv,
    config: &Config,
    prompt: &Collected,
    prompt_path: &Path,
    live: &Materialised,
) -> Result<Capture> {
    let words = command::split(runner)?;
    let (program, args) = words.split_first().ok_or_else(|| DocError::Prompt {
        message: "the runner command is empty".to_owned(),
    })?;
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(&live.cwd);
    for key in CLEARED {
        cmd.env_remove(key);
    }
    cmd.env(
        "VIBE_SETTINGS",
        normalize::display_path(&live.root.join(HOME_DIR)),
    )
    .env("NO_COLOR", "1")
    .env(ENV_PROMPT_FILE, normalize::display_path(prompt_path))
    .env(ENV_DIR, normalize::display_path(&live.cwd))
    .env(ENV_BINARY, normalize::display_path(&env.binary))
    .env(ENV_NEEDS, prompt.needs.clone().unwrap_or_default());
    spawn(cmd, Some(prompt.text.clone()), config.timeout)
}

/// Run a prepared command, feed it `stdin`, and collect both streams
/// under a deadline.
///
/// Both pipes are drained on their own threads: a child that fills one
/// while this process waits on the other deadlocks, and an agent is
/// exactly the kind of child that writes a great deal.
fn spawn(mut cmd: Command, stdin: Option<String>, timeout_secs: u64) -> Result<Capture> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let program = format!("{:?}", cmd.get_program());
    let fail = |message: String| DocError::Prompt {
        message: format!("{program}: {message}"),
    };
    let mut child = cmd
        .spawn()
        .map_err(|e| fail(format!("cannot start it: {e}")))?;
    if let Some(text) = stdin
        && let Some(mut pipe) = child.stdin.take()
    {
        let _ = pipe.write_all(text.as_bytes());
    }
    let out = child.stdout.take();
    let err = child.stderr.take();
    let out_thread = std::thread::spawn(move || drain(out));
    let err_thread = std::thread::spawn(move || drain(err));

    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(fail(format!(
                        "still running after {timeout_secs}s — killed"
                    )));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(fail(format!("waiting for it: {e}"))),
        }
    };
    Ok(Capture {
        code: status.code().unwrap_or(-1),
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
    })
}

fn drain<R: Read>(pipe: Option<R>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Both streams as a person reads them: an agent writes its narration to
/// one and its warnings to the other, and which is which is its business.
fn join(capture: &Capture) -> String {
    let mut out = capture.stdout.clone();
    if !capture.stderr.trim().is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&capture.stderr);
    }
    out
}

/// Lift every prompt off one page.
fn collect(page: &Page) -> Vec<Collected> {
    fn from_blocks(page: &Page, blocks: &[BlockNode], out: &mut Vec<Collected>) {
        for node in blocks {
            if let Block::Prompt {
                id,
                text,
                needs,
                asserts,
                ..
            } = &node.block
            {
                out.push(Collected {
                    page: page.rel.clone(),
                    id: id.clone(),
                    text: text.clone(),
                    needs: needs.clone(),
                    asserts: asserts.clone(),
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

/// A short, filesystem-safe name for one prompt's sandbox. Short on
/// purpose: the sandbox holds a materialised dependency tree, and Windows
/// counts every character of the path that leads to it.
fn slug(page: &str, id: &str) -> String {
    let stem = page
        .rsplit('/')
        .next()
        .unwrap_or(page)
        .trim_end_matches(".xml");
    format!("{stem}-{id}")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(40)
        .collect()
}

/// The sandbox root a run would use, for a caller that wants to name it.
pub fn sandbox_root(env: &RunnerEnv) -> PathBuf {
    env.sandbox_root.join("p")
}

#[cfg(test)]
mod tests;
