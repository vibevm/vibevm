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
//! After a successful branch push, fan-out refreshes the local
//! remote-tracking ref of every configured remote that points at the same URL
//! (e.g. `origin` → the GitVerse target). Pushing by raw URL — the manifest's
//! form — leaves `refs/remotes/<remote>/<branch>` untouched, so without this
//! the maintainer's `git status` reads "ahead of origin/main" right after a
//! green fan-out even though the host is level. The push was fast-forward-only
//! and succeeded, so the host now equals local `branch`; recording that needs
//! no extra network round-trip (a `git fetch` would do the same, redundantly).
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

/// One target's sync state relative to local mainline — the result both
/// `mirror --check` and `health --mirrors` read.
pub(crate) enum SyncState {
    InSync,
    Drift(String),
    Missing,
}

pub(crate) struct TargetStatus {
    pub name: String,
    pub state: SyncState,
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

fn rev_parse(root: &Path, rev: &str) -> Result<String> {
    let out = git(root, &["rev-parse", rev])?;
    if !out.status.success() {
        bail!(
            "git rev-parse {rev}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn local_main(root: &Path) -> Result<String> {
    rev_parse(root, MAINLINE)
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

/// The configured `(name, fetch-url)` remotes, parsed from `git remote -v`.
/// Tracking refs follow the *fetch* URL/refspec, so only fetch lines are
/// kept. A repo with no remotes yields an empty list (the maintainer who
/// pushes purely by URL); only a genuine git failure is an error.
fn named_remotes(root: &Path) -> Result<Vec<(String, String)>> {
    let out = git(root, &["remote", "-v"])?;
    if !out.status.success() {
        bail!(
            "git remote -v: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut remotes = Vec::new();
    for line in text.lines() {
        // "<name>\t<url> (fetch)" — the push lines are irrelevant here.
        if !line.ends_with("(fetch)") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
            continue;
        };
        remotes.push((name.to_string(), url.to_string()));
    }
    Ok(remotes)
}

/// Strip the gratuitous tail differences (`.git`, trailing `/`) so a
/// `mirrors.toml` URL and a configured remote URL for the same repo compare
/// equal. Scheme/host normalisation (ssh vs https) is deliberately out of
/// scope — those are genuinely different access paths, not the same string
/// dressed up.
fn normalize_url(u: &str) -> &str {
    u.trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/')
}

/// The names of every configured remote whose URL points at `target_url`.
fn remotes_matching<'a>(remotes: &'a [(String, String)], target_url: &str) -> Vec<&'a str> {
    let want = normalize_url(target_url);
    remotes
        .iter()
        .filter(|(_, url)| normalize_url(url) == want)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// After a successful fan-out push of `branch` to `target_url`, move the
/// local remote-tracking ref of every matching remote up to the just-pushed
/// commit (see the module header for why `git push <url>` leaves it stale).
/// Best-effort: the load-bearing act — the push — has already succeeded, so a
/// local `update-ref` hiccup warns but never fails the rollout, and
/// `git fetch <remote>` stays as the manual fallback.
fn refresh_tracking(root: &Path, remotes: &[(String, String)], target_url: &str, branch: &str) {
    let names = remotes_matching(remotes, target_url);
    if names.is_empty() {
        return;
    }
    // The push was fast-forward-only and succeeded, so every matching host's
    // `branch` now equals the local `branch` — record exactly that.
    let sha = match rev_parse(root, branch) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  warn   cannot resolve {branch} to refresh tracking refs: {e}");
            return;
        }
    };
    for name in names {
        let refname = format!("refs/remotes/{name}/{branch}");
        match git(root, &["update-ref", &refname, &sha]) {
            Ok(out) if out.status.success() => {
                println!("  track  {name}/{branch} -> {}", short(&sha))
            }
            Ok(out) => eprintln!(
                "  warn   could not refresh {name}/{branch}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => eprintln!("  warn   could not refresh {name}/{branch}: {e}"),
        }
    }
}

/// The argv for pushing one ref to a target URL — the single place the
/// fan-out's push command is built, so the load-bearing **never `--force`**,
/// fast-forward-only invariant (PROP-016 §6, the `CLAUDE.md` Rule 4 red
/// line) lives in one checkable spot. `tags` fans every tag (`--tags`); any
/// other ref pushes that branch with a bare `git push` — no `--force`, no
/// `+`-prefixed (force) refspec — so a non-fast-forward fails loud rather
/// than overwriting a diverged target. The `push_args_never_force` test
/// turns that guarantee from prose into runnable capital.
fn push_args<'a>(url: &'a str, git_ref: &'a str) -> Vec<&'a str> {
    if git_ref == "tags" {
        vec!["push", url, "--tags"]
    } else {
        vec!["push", url, git_ref]
    }
}

fn fan_out(root: &Path, targets: &[Target]) -> Result<()> {
    let head = local_main(root)?;
    let remotes = named_remotes(root).unwrap_or_else(|e| {
        eprintln!("mirror: could not read git remotes ({e}); tracking refs left as-is");
        Vec::new()
    });
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
                    let args = push_args(&t.url, r);
                    let out = git(root, &args)?;
                    if out.status.success() {
                        println!("  ok     {} {r}", t.name);
                        // Tags land in refs/tags/* directly and carry no
                        // per-remote tracking ref; only a branch push leaves
                        // refs/remotes/<remote>/<branch> stale.
                        if r != "tags" {
                            refresh_tracking(root, &remotes, &t.url, r);
                        }
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

/// Probe every target's `main` against local mainline. Shared by
/// `mirror --check` (which fails on drift) and `health --mirrors` (advisory).
fn probe(root: &Path, targets: &[Target]) -> Result<(String, Vec<TargetStatus>)> {
    let head = local_main(root)?;
    let mut statuses = Vec::with_capacity(targets.len());
    for t in targets {
        let state = match remote_main(root, &t.url)? {
            Some(sha) if sha == head => SyncState::InSync,
            Some(sha) => SyncState::Drift(sha),
            None => SyncState::Missing,
        };
        statuses.push(TargetStatus {
            name: t.name.clone(),
            state,
        });
    }
    Ok((head, statuses))
}

/// Load the manifest and probe every target — the entry `health --mirrors`
/// calls (it carries no loaded targets of its own).
pub(crate) fn sync_report(root: &Path) -> Result<(String, Vec<TargetStatus>)> {
    let targets = load_targets(root)?;
    probe(root, &targets)
}

fn verify(root: &Path, targets: &[Target]) -> Result<()> {
    let (head, statuses) = probe(root, targets)?;
    println!("mirror --check: local {MAINLINE} @ {}", short(&head));
    let mut drift = Vec::new();
    for s in &statuses {
        match &s.state {
            SyncState::InSync => println!("  sync   {}", s.name),
            SyncState::Drift(sha) => {
                println!("  DRIFT  {} at {}", s.name, short(sha));
                drift.push(s.name.as_str());
            }
            SyncState::Missing => {
                println!("  EMPTY  {} (no {MAINLINE})", s.name);
                drift.push(s.name.as_str());
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

#[cfg(test)]
mod tests {
    use super::{normalize_url, push_args, remotes_matching};

    fn remote(name: &str, url: &str) -> (String, String) {
        (name.to_string(), url.to_string())
    }

    #[test]
    fn push_args_never_force() {
        // The marquee invariant of the whole mirror system (PROP-016 §6,
        // CLAUDE.md Rule 4): the fan-out NEVER force-pushes. Guarding every
        // ref shape keeps a future edit from quietly slipping `--force` in.
        for git_ref in ["main", "tags", "release", "v1.0"] {
            let args = push_args("git@host:org/repo.git", git_ref);
            assert!(
                !args
                    .iter()
                    .any(|a| *a == "--force" || *a == "-f" || a.starts_with('+')),
                "fan-out push for `{git_ref}` must never force: {args:?}"
            );
            assert_eq!(args[0], "push", "first arg is always the push verb");
        }
    }

    #[test]
    fn push_args_shape_per_ref_kind() {
        // A branch ref pushes that branch by name; `tags` fans every tag.
        assert_eq!(push_args("URL", "main"), vec!["push", "URL", "main"]);
        assert_eq!(push_args("URL", "tags"), vec!["push", "URL", "--tags"]);
    }

    #[test]
    fn normalize_url_strips_git_suffix_and_trailing_slash() {
        assert_eq!(
            normalize_url("git@github.com:vibevm/vibevm.git"),
            "git@github.com:vibevm/vibevm"
        );
        assert_eq!(
            normalize_url("https://gitverse.ru/vibevm/vibevm.git/"),
            "https://gitverse.ru/vibevm/vibevm"
        );
        // Already bare — unchanged, and the leading `git@` is never touched
        // (only the tail is trimmed).
        assert_eq!(
            normalize_url("git@gitverse.ru:vibevm/vibevm"),
            "git@gitverse.ru:vibevm/vibevm"
        );
    }

    #[test]
    fn matching_remote_found_despite_git_suffix_difference() {
        // mirrors.toml carries the `.git` form; a remote may not, or vice
        // versa — normalisation makes the two compare equal.
        let remotes = vec![
            remote("origin", "git@gitverse.ru:vibevm/vibevm.git"),
            remote("github", "git@github.com:vibevm/vibevm"),
        ];
        assert_eq!(
            remotes_matching(&remotes, "git@gitverse.ru:vibevm/vibevm"),
            vec!["origin"]
        );
        assert_eq!(
            remotes_matching(&remotes, "git@github.com:vibevm/vibevm.git"),
            vec!["github"]
        );
    }

    #[test]
    fn no_matching_remote_when_url_is_unknown() {
        // A target with no configured remote (the push-by-URL-only case):
        // nothing to refresh, no spurious match.
        let remotes = vec![remote("origin", "git@gitverse.ru:vibevm/vibevm.git")];
        assert!(remotes_matching(&remotes, "git@example.com:someone/other.git").is_empty());
    }

    #[test]
    fn both_remotes_at_same_url_match_in_order() {
        // Two remotes pointing at one host: both tracking refs must move,
        // and the input order is preserved.
        let remotes = vec![
            remote("origin", "git@gitverse.ru:vibevm/vibevm.git"),
            remote("alias", "git@gitverse.ru:vibevm/vibevm.git"),
        ];
        assert_eq!(
            remotes_matching(&remotes, "git@gitverse.ru:vibevm/vibevm.git"),
            vec!["origin", "alias"]
        );
    }
}
