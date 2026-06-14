//! `cargo xtask mirror` — fan the local mainline out to every target in
//! `mirrors.toml` (the benevolent-dictator / hub-and-spoke model, no primary).
//!
//! Mainline is the maintainer's integrated local `main` — single-writer, so
//! there is no multi-master conflict: every target is a downstream read-replica
//! that is canonical for *reading* in its region, and nobody writes a target
//! directly (contributions arrive as PRs / branches / email patches on any
//! host, are integrated into local mainline, then fanned out from here). This
//! command:
//!
//! - `cargo xtask mirror` — push mainline to every `push` target,
//!   fast-forward-only, **never `--force`**; a non-fast-forward means a target
//!   diverged (someone wrote it directly) → fail loud, reconcile by hand. A
//!   `self-pull` target (one that mirrors itself from elsewhere) is not pushed,
//!   only checked for keeping up.
//! - `--check` — verify every target equals local mainline; push nothing.
//!   Read-only, suitable for a health probe.
//! - `--from <name>` — fast-forward local mainline to a host's accepted-PR
//!   merge (`git fetch` + `git merge --ff-only`) before fanning out: the bridge
//!   for "I accepted/merged a PR via that host's web UI".
//!
//! Auth is the maintainer's per-host SSH keys in the agent; `mirrors.toml`
//! carries only URLs, no secrets.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::repo_root;

const MANIFEST: &str = "mirrors.toml";
const MAINLINE: &str = "main";

enum Mode {
    Push,
    SelfPull,
}

struct Target {
    name: String,
    url: String,
    mode: Mode,
    refs: Vec<String>,
}

pub(crate) fn run_mirror(check: bool, from: Option<&str>) -> Result<()> {
    let root = repo_root()?;
    let targets = load_targets(&root)?;
    if targets.is_empty() {
        bail!("{MANIFEST} lists no targets");
    }
    if let Some(name) = from {
        pull_from(&root, &targets, name)?;
    }
    if check {
        verify(&root, &targets)
    } else {
        fan_out(&root, &targets)
    }
}

fn load_targets(root: &Path) -> Result<Vec<Target>> {
    let path = root.join(MANIFEST);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let doc: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let mut out = Vec::new();
    for t in doc
        .get("target")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let name = str_field(t, "name")?;
        let url = str_field(t, "url")?;
        let mode = match t.get("mode").and_then(|v| v.as_str()).unwrap_or("push") {
            "push" => Mode::Push,
            "self-pull" => Mode::SelfPull,
            other => bail!("target `{name}`: unknown mode `{other}` (want `push` or `self-pull`)"),
        };
        let refs = match t.get("refs").and_then(|v| v.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            None => vec![MAINLINE.to_string()],
        };
        out.push(Target {
            name,
            url,
            mode,
            refs,
        });
    }
    Ok(out)
}

fn str_field(t: &toml::Value, key: &str) -> Result<String> {
    t.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .with_context(|| format!("a [[target]] in {MANIFEST} is missing the string field `{key}`"))
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("running `git {}`", args.join(" ")))
}

fn short(sha: &str) -> &str {
    &sha[..7.min(sha.len())]
}

fn local_main(root: &Path) -> Result<String> {
    let out = git(root, &["rev-parse", MAINLINE])?;
    if !out.status.success() {
        bail!(
            "git rev-parse {MAINLINE}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn remote_main(root: &Path, url: &str) -> Result<Option<String>> {
    let out = git(root, &["ls-remote", url, &format!("refs/heads/{MAINLINE}")])?;
    if !out.status.success() {
        bail!(
            "git ls-remote {url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .map(String::from))
}

fn fan_out(root: &Path, targets: &[Target]) -> Result<()> {
    let head = local_main(root)?;
    println!(
        "mirror: fanning {MAINLINE} @ {} out to {} target(s)",
        short(&head),
        targets.len()
    );
    let mut failures = Vec::new();
    for t in targets {
        match t.mode {
            Mode::Push => {
                for r in &t.refs {
                    let args: Vec<&str> = if r == "tags" {
                        vec!["push", t.url.as_str(), "--tags"]
                    } else {
                        vec!["push", t.url.as_str(), r.as_str()]
                    };
                    let out = git(root, &args)?;
                    if out.status.success() {
                        println!("  ok     {} {r}", t.name);
                    } else {
                        eprintln!(
                            "  FAIL   {} {r} -- {}",
                            t.name,
                            String::from_utf8_lossy(&out.stderr).trim()
                        );
                        failures.push(format!("{}:{r}", t.name));
                    }
                }
            }
            Mode::SelfPull => match remote_main(root, &t.url)? {
                Some(sha) if sha == head => println!("  sync   {} (self-pull)", t.name),
                Some(sha) => println!("  BEHIND {} (self-pull, at {})", t.name, short(&sha)),
                None => println!("  EMPTY  {} (self-pull, no {MAINLINE})", t.name),
            },
        }
    }
    if !failures.is_empty() {
        bail!(
            "mirror: {} push(es) failed -- a non-fast-forward means a target diverged \
             (someone wrote it directly); reconcile by hand, never --force: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    println!("mirror: all push targets synced.");
    Ok(())
}

fn verify(root: &Path, targets: &[Target]) -> Result<()> {
    let head = local_main(root)?;
    println!("mirror --check: local {MAINLINE} @ {}", short(&head));
    let mut drift = Vec::new();
    for t in targets {
        match remote_main(root, &t.url)? {
            Some(sha) if sha == head => println!("  sync   {}", t.name),
            Some(sha) => {
                println!("  DRIFT  {} at {}", t.name, short(&sha));
                drift.push(t.name.clone());
            }
            None => {
                println!("  EMPTY  {} (no {MAINLINE})", t.name);
                drift.push(t.name.clone());
            }
        }
    }
    if !drift.is_empty() {
        bail!(
            "mirror --check: {} target(s) drifted from mainline: {}",
            drift.len(),
            drift.join(", ")
        );
    }
    println!("mirror --check: all targets in sync.");
    Ok(())
}

fn pull_from(root: &Path, targets: &[Target], name: &str) -> Result<()> {
    let target = targets
        .iter()
        .find(|t| t.name == name)
        .with_context(|| format!("--from: no target named `{name}` in {MANIFEST}"))?;
    if !git(root, &["status", "--porcelain"])?.stdout.is_empty() {
        bail!("--from: the working tree is dirty -- commit or stash before pulling a host merge");
    }
    let branch_out = git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = String::from_utf8_lossy(&branch_out.stdout)
        .trim()
        .to_string();
    if branch != MAINLINE {
        bail!("--from: check out `{MAINLINE}` first (currently on `{branch}`)");
    }
    println!(
        "mirror --from {name}: fetching {MAINLINE} from {}",
        target.url
    );
    let fetch = git(root, &["fetch", target.url.as_str(), MAINLINE])?;
    if !fetch.status.success() {
        bail!(
            "--from: fetch failed: {}",
            String::from_utf8_lossy(&fetch.stderr).trim()
        );
    }
    let merge = git(root, &["merge", "--ff-only", "FETCH_HEAD"])?;
    if !merge.status.success() {
        bail!(
            "--from: local {MAINLINE} cannot fast-forward to {name}'s {MAINLINE} -- histories \
             diverged; reconcile by hand, never --force: {}",
            String::from_utf8_lossy(&merge.stderr).trim()
        );
    }
    println!("mirror --from {name}: local {MAINLINE} fast-forwarded; fanning out...");
    Ok(())
}
