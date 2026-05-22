//! `vibe workspace …` — multi-package workspace operations.
//!
//! Spec: [PROP-007 §2.7–§2.9](../../../spec/modules/vibe-workspace/PROP-007-workspace.md#selective-publish).
//!
//! Today the one subcommand is `publish`. `vibe workspace publish` discovers
//! the workspace enclosing the current directory, selects every node that
//! carries `[package]` and is not `publish = false`, orders the selection
//! dependency-first via inter-member `path` dependencies, and publishes each
//! node as its own repository in the workspace's primary `[[registry]]` org.
//!
//! The selection / ordering / staging logic lives in `vibe-workspace`
//! (`vibe_workspace::publish`) — it is pure and hermetically tested there.
//! This file owns the side-effecting half: discovering the workspace,
//! resolving the publish token, building the `[origin]` provenance from the
//! root git repo, and threading each staged node through
//! `vibe_publish::Publisher`. The per-package publish machinery — repo
//! creation, push, tag, token discipline — is reused unchanged from
//! `vibe-publish`; nothing about tokens or push URLs is reimplemented here.
//!
//! Publishing is **not atomic** (PROP-007 §2.7). Distributed publishing
//! across N independent host repos has no transaction, so on the first
//! node's failure the command stops and reports which nodes were already
//! published and which remain — a clear partial-progress report beats a
//! rollback that would be a lie.
//!
//! Deferred (PROP-007 §2.8, noted here so the boundary is explicit):
//! `--archive` (the GitHub `archived = true` lockdown + its
//! unarchive→push→archive re-publish cycle), `has_issues = false` at repo
//! creation, the `published_repos = "read-only" | "open"` workspace
//! setting, and multi-registry fan-out (one node → several registries).
//! `vibe workspace publish` targets the workspace's primary registry only;
//! the `[origin]` marker + README banner + PR-template already make a
//! published copy unmistakably a generated read-only copy.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::Manifest;
use vibe_publish::{
    PublishConfig, Publisher, creator_for_url, extract_host_segment, extract_org_segment,
    load_token_for_host,
};
use vibe_workspace::Workspace;
use vibe_workspace::publish::{
    OriginInfo, PublishNode, select_publishable_nodes, stage_node, topo_order,
};

use crate::cli::{WorkspaceArgs, WorkspacePublishArgs, WorkspaceSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: WorkspaceArgs) -> Result<()> {
    match args.command {
        WorkspaceSubcommand::Publish(sub) => run_publish(ctx, sub),
    }
}

// ---------------------------------------------------------------------------
// JSON envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct PublishReport {
    ok: bool,
    command: &'static str,
    dry_run: bool,
    /// Nodes published successfully, in publish order.
    published: Vec<PublishedEntry>,
    /// `[package]` nodes excluded by their `publish` posture.
    skipped: Vec<SkippedEntry>,
    /// Nodes that were selected and ordered but not reached — either the
    /// publish stopped on an earlier node's failure, or (rare) the run
    /// ended before reaching them. Empty on a fully successful run.
    remaining: Vec<RemainingEntry>,
}

#[derive(Debug, Serialize)]
struct PublishedEntry {
    /// `<kind>:<name>` of the node.
    pkgref: String,
    /// Root-relative path of the node (`.` for the workspace root).
    rel_path: String,
    /// Repository name on the host.
    repo_name: String,
    /// Public clone URL of the published repository (credential-free).
    repo_url: String,
    /// The tag pushed — `v<version>`.
    tag: String,
    /// `true` if the publish created the repository, `false` if it
    /// reused an existing one. Always `false` under `--dry-run`'s
    /// reuse path, `true` under `--dry-run`'s would-create path.
    created_repo: bool,
}

#[derive(Debug, Serialize)]
struct SkippedEntry {
    rel_path: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct RemainingEntry {
    pkgref: String,
    rel_path: String,
}

// ---------------------------------------------------------------------------
// vibe workspace publish
// ---------------------------------------------------------------------------

fn run_publish(ctx: &output::Context, args: WorkspacePublishArgs) -> Result<()> {
    // Discover the workspace enclosing the requested path. A standalone
    // node (no `[workspace]`) discovers as its own root with no members —
    // publishing it is just the root, if the root is a `[package]`.
    let start = args
        .path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", args.path.display()))?;
    let start = super::init::strip_unc_public(start);
    let workspace = Workspace::discover(&start)
        .with_context(|| format!("discovering workspace from `{}`", start.display()))?;

    // The primary registry drives both the publish target and the
    // `publish = ["..."]` list-form filter. A workspace with no
    // `[[registry]]` cannot publish — there is nowhere to publish to.
    let primary = workspace
        .root_manifest
        .primary_registry()
        .ok_or_else(|| {
            anyhow!(
                "no `[[registry]]` in the workspace root `{}` — `vibe workspace publish` \
                 needs a target registry; add one with `vibe registry add`",
                workspace.root.join(Manifest::FILENAME).display()
            )
        })?
        .clone();

    // Select the publishable nodes, honouring every `PublishPosture` shape
    // against the primary registry's name. `--member` narrows to one node.
    let selection = select_publishable_nodes(&workspace, &primary.name, args.member.as_deref())
        .map_err(|e| anyhow!("{e}"))?;

    // Order dependency-first. A cycle among the selected nodes is a hard
    // error (PROP-007 §2.7) — distributed publishing cannot break one.
    let ordered = topo_order(&workspace, &selection.publishable).map_err(|e| {
        anyhow!(
            "{e} — `vibe workspace publish` cannot publish a dependency cycle \
             (PROP-007 §2.7); break the inter-member `path` dependency cycle first"
        )
    })?;

    let skipped: Vec<SkippedEntry> = selection
        .skipped
        .iter()
        .map(|s| SkippedEntry {
            rel_path: s.rel_path.clone(),
            reason: s.reason.clone(),
        })
        .collect();

    ctx.heading(&format!(
        "Publishing workspace `{}` → registry `{}` (`{}`){}",
        workspace.root.display(),
        primary.name,
        primary.url,
        if args.dry_run { " [dry-run]" } else { "" },
    ));

    if ordered.is_empty() {
        // Nothing to publish. Still a success — an entirely-local
        // workspace (every member `publish = false`) is a first-class
        // PROP-007 §2.7 extreme, not an error.
        for s in &skipped {
            ctx.skipped(&s.rel_path, &s.reason);
        }
        if ctx.is_json() {
            ctx.emit_json(&PublishReport {
                ok: true,
                command: "workspace:publish",
                dry_run: args.dry_run,
                published: Vec::new(),
                skipped,
                remaining: Vec::new(),
            })?;
            return Ok(());
        }
        ctx.summary(&format!(
            "\nvibe workspace publish: no self-publishing nodes ({} skipped). \
             Nothing left the machine.",
            skipped.len()
        ));
        return Ok(());
    }

    // Build the `[origin]` provenance once — every staged node references
    // the same monorepo. `upstream` is the root repo's `origin` remote
    // when the root is a git repo, else the root manifest's name.
    let origin_base = build_origin_info(&workspace);

    // Report the planned order up front so a `--dry-run` reader (and a
    // human watching a real run) sees the dependency-first sequence.
    let order_line = ordered
        .iter()
        .map(PublishNode::pkgref)
        .collect::<Vec<_>>()
        .join(" → ");
    ctx.step(&format!("Publish order (dependency-first): {order_line}"));

    let host = extract_host_segment(&primary.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", primary.url))?;
    let org_segment = extract_org_segment(&primary.url)
        .map_err(|e| anyhow!("registry URL `{}`: {e}", primary.url))?;

    // GitVerse publishing is a stub (PROP-002 §2.10 — no org-scoped repo
    // creation API). Refuse early, before any token load, with the same
    // shape `vibe registry publish` uses. `--dry-run` is allowed through
    // so an operator can still inspect the plan against a GitVerse
    // registry without a usable publish path.
    let host_lower = host.to_ascii_lowercase();
    let is_gitverse = host_lower == "gitverse.ru" || host_lower.ends_with(".gitverse.ru");
    if is_gitverse && !args.dry_run {
        bail!(
            "GitVerse publishing is not implemented yet — the GitVerse public API does not \
             expose org-scoped repository creation, so `vibe workspace publish` cannot drive \
             the create-repo + push-tag flow. Publish to a GitHub `[[registry]]` instead. \
             `--dry-run` still works against a GitVerse registry to inspect the plan."
        );
    }

    // Token is loaded once and reused for every node's publish. Under
    // `--dry-run` no repo is created and nothing is pushed, so the token
    // is not needed at all — skip the load so a dry-run plan works even
    // without a token configured.
    let creator = if args.dry_run {
        None
    } else {
        let token = load_token_for_host(&host).context("loading publish token")?;
        // Surface the *source* of the token, never the value. Token::Display
        // redacts to `***` defensively.
        ctx.step(&format!(
            "Loaded publish token from {} (value redacted)",
            match token.source() {
                vibe_publish::TokenSource::Explicit => "explicit argument".to_string(),
                vibe_publish::TokenSource::EnvVar(name) => format!("$ {name}"),
                vibe_publish::TokenSource::File(p) => p.display().to_string(),
            }
        ));
        Some(
            creator_for_url(&primary.url, org_segment.clone(), token)
                .map_err(|e| anyhow!("{e}"))?,
        )
    };

    // Publish loop. Non-atomic, stop on first failure (PROP-007 §2.7).
    // The loop body is `publish_loop` — extracted so it can be exercised
    // hermetically with a mock `RepoCreator` (see the test module).
    let inputs: Vec<PublishInput> = ordered
        .iter()
        .map(|n| PublishInput {
            node: n.clone(),
            source_dir: node_abs_dir(&workspace, n),
        })
        .collect();
    let plan = PublishPlan {
        org_url: primary.url.clone(),
        naming: primary.naming,
        dry_run: args.dry_run,
        origin: origin_base,
    };

    match publish_loop(creator.as_deref(), &inputs, &plan, &mut |entry, dry_run| {
        if dry_run {
            ctx.step(&format!(
                "Would publish {} → repo `{}` (tag `{}`)",
                entry.pkgref, entry.repo_name, entry.tag
            ));
        } else {
            ctx.step(&format!(
                "Published {} → `{}` (tag `{}`)",
                entry.pkgref, entry.repo_url, entry.tag
            ));
        }
    }) {
        Ok(published) => {
            // Every node published. `remaining` is empty.
            for s in &skipped {
                ctx.skipped(&s.rel_path, &s.reason);
            }
            if ctx.is_json() {
                ctx.emit_json(&PublishReport {
                    ok: true,
                    command: "workspace:publish",
                    dry_run: args.dry_run,
                    published,
                    skipped,
                    remaining: Vec::new(),
                })?;
                return Ok(());
            }
            if args.dry_run {
                ctx.summary(&format!(
                    "\nvibe workspace publish [dry-run]: {} node(s) would publish, {} skipped. \
                     Re-run without `--dry-run` to apply.",
                    published.len(),
                    skipped.len()
                ));
            } else {
                ctx.summary(&format!(
                    "\nvibe workspace publish: {} node(s) published, {} skipped.",
                    published.len(),
                    skipped.len()
                ));
            }
            Ok(())
        }
        Err(failure) => finish_failure(
            ctx,
            args.dry_run,
            failure.published,
            skipped,
            &ordered,
            failure.failed_idx,
            failure.error,
        ),
    }
}

/// One node fed into [`publish_loop`] — the node identity plus its
/// on-disk source directory.
struct PublishInput {
    node: PublishNode,
    source_dir: std::path::PathBuf,
}

/// Shared inputs for [`publish_loop`] — the registry URL, naming
/// convention, dry-run flag, and the `[origin]` provenance every staged
/// node shares.
struct PublishPlan {
    org_url: String,
    naming: vibe_core::manifest::NamingConvention,
    dry_run: bool,
    origin: OriginInfo,
}

/// A publish loop that stopped on a node's failure — what landed before,
/// the index of the failed node, and the error.
#[derive(Debug)]
struct PublishFailure {
    /// Nodes published before the failure, in order.
    published: Vec<PublishedEntry>,
    /// Index in the ordered node list of the node that failed.
    failed_idx: usize,
    /// The failure.
    error: anyhow::Error,
}

/// Publish every node in `inputs`, in order, stopping on the first failure.
///
/// Each node is staged (copy + `[origin]` + README banner + PR template)
/// then handed to `vibe_publish::Publisher` against `creator`. On success
/// the per-node `PublishedEntry` is collected and `on_progress` is called
/// so the caller can render a progress line. On a node's failure the loop
/// stops and returns [`PublishFailure`] carrying the nodes already
/// published, the failed index, and the error — the non-atomic
/// partial-progress contract of PROP-007 §2.7.
///
/// `creator` is `None` only on the `--dry-run` path: a dry-run touches no
/// network, so instead of constructing a host adapter the outcome is
/// synthesised from the staged manifest via [`dry_run_outcome`].
///
/// Extracted as a free function taking `creator: Option<&dyn RepoCreator>`
/// precisely so the loop — including the stop-on-first-failure behaviour —
/// can be unit-tested with a mock `RepoCreator`.
fn publish_loop(
    creator: Option<&dyn vibe_publish::RepoCreator>,
    inputs: &[PublishInput],
    plan: &PublishPlan,
    on_progress: &mut dyn FnMut(&PublishedEntry, bool),
) -> std::result::Result<Vec<PublishedEntry>, PublishFailure> {
    let mut published: Vec<PublishedEntry> = Vec::new();
    for (idx, input) in inputs.iter().enumerate() {
        let node = &input.node;

        // Stage the node — copy its directory excluding `.git/` / `.vibe/`,
        // inject the `[origin]` marker, prepend the README banner, write
        // the PR template. `[origin].path` is the node's rel_path.
        let staged = match stage_node(&input.source_dir, &node.rel_path, &plan.origin) {
            Ok(s) => s,
            Err(e) => {
                return Err(PublishFailure {
                    published,
                    failed_idx: idx,
                    error: anyhow!("staging `{}`: {e}", node.pkgref()),
                });
            }
        };

        // Publish the staged dir via the per-package machinery. The staged
        // manifest already carries the generated-copy `[package].description`
        // so `Publisher` sends the right host-side description.
        let config = PublishConfig {
            source_dir: staged.staging.path().to_path_buf(),
            org_url: plan.org_url.clone(),
            naming: plan.naming,
            tag_prefix: "v".to_string(),
            dry_run: plan.dry_run,
        };

        let outcome = if let Some(creator) = creator {
            match Publisher::new(creator).publish(&config) {
                Ok(o) => o,
                Err(e) => {
                    return Err(PublishFailure {
                        published,
                        failed_idx: idx,
                        error: anyhow!("publishing `{}`: {e}", node.pkgref()),
                    });
                }
            }
        } else {
            // `--dry-run` with no creator — synthesise the plan outcome.
            match dry_run_outcome(&config, &plan.org_url) {
                Ok(o) => o,
                Err(e) => {
                    return Err(PublishFailure {
                        published,
                        failed_idx: idx,
                        error: anyhow!("planning publish of `{}`: {e}", node.pkgref()),
                    });
                }
            }
        };

        let entry = PublishedEntry {
            pkgref: node.pkgref(),
            rel_path: node.rel_path.clone(),
            repo_name: outcome.repo_name,
            repo_url: outcome.repo_url,
            tag: outcome.tag,
            created_repo: outcome.created_repo,
        };
        on_progress(&entry, plan.dry_run);
        published.push(entry);
    }
    Ok(published)
}

/// Stop the publish loop on a node's failure and report partial progress.
///
/// `failed_idx` is the index in `ordered` of the node that failed; nodes at
/// `failed_idx` and beyond are the `remaining` set (the failed node itself
/// counts as remaining — it did not publish). PROP-007 §2.7: non-atomic
/// publishing reports what landed and what did not, never pretends a
/// rollback happened.
#[allow(clippy::too_many_arguments)]
fn finish_failure(
    ctx: &output::Context,
    dry_run: bool,
    published: Vec<PublishedEntry>,
    skipped: Vec<SkippedEntry>,
    ordered: &[PublishNode],
    failed_idx: usize,
    err: anyhow::Error,
) -> Result<()> {
    let remaining: Vec<RemainingEntry> = ordered[failed_idx..]
        .iter()
        .map(|n| RemainingEntry {
            pkgref: n.pkgref(),
            rel_path: n.rel_path.clone(),
        })
        .collect();

    if ctx.is_json() {
        // The error itself flows through the normal `ctx.error` path the
        // caller invokes on the returned `Err`. The structured
        // partial-progress envelope goes to stdout here so a JSON consumer
        // sees exactly what landed before the failure.
        ctx.emit_json(&PublishReport {
            ok: false,
            command: "workspace:publish",
            dry_run,
            published,
            skipped,
            remaining,
        })?;
    } else {
        // Human mode: a clear partial-progress block before the error line.
        if published.is_empty() {
            ctx.summary("\nvibe workspace publish: stopped — no nodes were published.");
        } else {
            let done = published
                .iter()
                .map(|p| p.pkgref.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            ctx.summary(&format!(
                "\nvibe workspace publish: stopped after a failure. \
                 Already published: {done}."
            ));
        }
        if !remaining.is_empty() {
            let rest = remaining
                .iter()
                .map(|r| r.pkgref.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            ctx.summary(&format!(
                "Not published (re-run `vibe workspace publish` to resume): {rest}."
            ));
        }
        ctx.summary(
            "Publishing is non-atomic — already-published nodes stay published \
             (PROP-007 §2.7).",
        );
    }
    Err(err)
}

/// The absolute on-disk directory of a publish node.
fn node_abs_dir(workspace: &Workspace, node: &PublishNode) -> std::path::PathBuf {
    if node.rel_path == "." {
        workspace.root.clone()
    } else {
        let member = workspace
            .member_by_rel_path(&node.rel_path)
            .expect("publish node rel_path always names a workspace node");
        workspace.member_abs_path(member)
    }
}

/// Build the `[origin]` provenance for every staged node.
///
/// `upstream` is the workspace root's `origin` remote URL when the root is a
/// git repository carrying that remote; otherwise it falls back to the root
/// manifest's project/package `name` (a best-effort identity — an external
/// reader at least learns which monorepo this came from by name). `commit`
/// is the root repo's `HEAD` when it is a git repository, else `None`.
fn build_origin_info(workspace: &Workspace) -> OriginInfo {
    let upstream = git_remote_origin_url(&workspace.root)
        .unwrap_or_else(|| root_identity_name(&workspace.root_manifest));
    let commit = git_head_commit(&workspace.root);
    OriginInfo {
        upstream,
        commit,
        generated_by: format!("vibe {}", env!("CARGO_PKG_VERSION")),
        generated_at: vibe_core::timestamp::now_utc(),
    }
}

/// The best-effort identity name of the workspace root — its project or
/// package `name`, or a literal `unknown` if the root carries neither (a
/// virtual `[workspace]`-only coordinator).
fn root_identity_name(root: &Manifest) -> String {
    if let Some(p) = &root.project {
        return p.name.clone();
    }
    if let Some(p) = &root.package {
        return p.name.clone();
    }
    "unknown".to_string()
}

/// Run `git remote get-url origin` in `dir`. Returns `None` when `dir` is
/// not a git repo, has no `origin` remote, or `git` is unavailable. The URL
/// is used only as the `[origin].upstream` marker value — it is a public
/// remote URL, not a credentialed push URL, and never carries a token.
fn git_remote_origin_url(dir: &Path) -> Option<String> {
    let out = git_in(dir, &["remote", "get-url", "origin"])?;
    let url = out.trim();
    if url.is_empty() {
        return None;
    }
    Some(url.to_string())
}

/// Run `git rev-parse HEAD` in `dir`. Returns `None` when `dir` is not a
/// git repo or `git` is unavailable.
fn git_head_commit(dir: &Path) -> Option<String> {
    let out = git_in(dir, &["rev-parse", "HEAD"])?;
    let sha = out.trim();
    if sha.is_empty() {
        return None;
    }
    Some(sha.to_string())
}

/// Run `git <args>` in `dir`, returning trimmed stdout on success or `None`
/// on any failure (non-zero exit, `git` missing, I/O error). A best-effort
/// probe — a missing git repo is not an error for `vibe workspace publish`,
/// it just means the `[origin]` marker falls back to the root name.
fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Synthesise a [`vibe_publish::PublishOutcome`] for a `--dry-run` node
/// without constructing a network `RepoCreator`.
///
/// A dry-run makes no API calls and pushes nothing, so all that is needed
/// is the plan: the package identity (read from the staged manifest), the
/// repo name (from the naming convention), the tag, and the would-be repo
/// URL (derived from the org URL the same way `Publisher::publish` does on
/// its dry-run path). `created_repo` is reported `true` — a dry-run cannot
/// probe repo presence without the host API, and "would create" is the
/// honest default expectation for a node that has never been published.
fn dry_run_outcome(config: &PublishConfig, org_url: &str) -> Result<vibe_publish::PublishOutcome> {
    let manifest_path = config.source_dir.join(Manifest::FILENAME);
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading staged manifest `{}`", manifest_path.display()))?;
    let meta = manifest
        .require_package()
        .map_err(|e| anyhow!("staged manifest is not a package: {e}"))?;
    let repo_name = config.naming.repo_name(meta.kind, &meta.name);
    let tag = format!("{}{}", config.tag_prefix, meta.version);
    let repo_url = format!("{}/{}.git", org_url.trim_end_matches('/'), repo_name);
    Ok(vibe_publish::PublishOutcome {
        kind: meta.kind,
        name: meta.name.clone(),
        version: meta.version.clone(),
        repo_name,
        repo_url,
        tag,
        created_repo: true,
        host: extract_host_segment(org_url).unwrap_or_else(|_| "git".to_string()),
        dry_run: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use vibe_core::manifest::NamingConvention;
    use vibe_publish::{CreateOpts, PublishError, RepoCreator, RepoInfo};

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn package(name: &str, kind: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n")
    }

    fn origin_info() -> OriginInfo {
        OriginInfo {
            upstream: "https://github.com/you/monorepo".to_string(),
            commit: Some("abc123".to_string()),
            generated_by: "vibe test".to_string(),
            generated_at: "2026-05-21T00:00:00Z".to_string(),
        }
    }

    /// A hermetic mock [`RepoCreator`]. Each `create_repo` provisions a
    /// real bare git repo under `bare_root` so `Publisher::publish`'s
    /// `push_release` has a working `file://` push target — no network.
    /// `fail_on` makes `create_repo` for that exact repo name return a
    /// `PublishError`, so the stop-on-first-failure path can be exercised
    /// deterministically.
    struct MockCreator {
        bare_root: PathBuf,
        /// Repo name whose `create_repo` should fail. `None` = never fail.
        fail_on: Option<String>,
        /// Names passed to `create_repo`, in call order.
        created: RefCell<Vec<String>>,
    }

    impl MockCreator {
        fn new(bare_root: PathBuf) -> Self {
            MockCreator {
                bare_root,
                fail_on: None,
                created: RefCell::new(Vec::new()),
            }
        }

        fn failing_on(bare_root: PathBuf, repo: &str) -> Self {
            MockCreator {
                bare_root,
                fail_on: Some(repo.to_string()),
                created: RefCell::new(Vec::new()),
            }
        }

        fn bare_path(&self, name: &str) -> PathBuf {
            self.bare_root.join(format!("{name}.git"))
        }
    }

    impl RepoCreator for MockCreator {
        fn host_name(&self) -> &str {
            "mock-host"
        }

        fn repo_exists(&self, _org: &str, _name: &str) -> Result<bool, PublishError> {
            // Fresh workspace publish — nothing exists yet.
            Ok(false)
        }

        fn create_repo(
            &self,
            _org: &str,
            name: &str,
            _opts: &CreateOpts,
        ) -> Result<RepoInfo, PublishError> {
            self.created.borrow_mut().push(name.to_string());
            if self.fail_on.as_deref() == Some(name) {
                return Err(PublishError::Git(format!(
                    "mock: create_repo deliberately failed for `{name}`"
                )));
            }
            // Provision a real bare repo so the subsequent push lands.
            let bare = self.bare_path(name);
            let init = Command::new("git")
                .args(["init", "--bare", bare.to_str().unwrap()])
                .env("LC_ALL", "C")
                .status()
                .map_err(|e| PublishError::Git(format!("git init --bare: {e}")))?;
            if !init.success() {
                return Err(PublishError::Git("git init --bare failed".into()));
            }
            // The created bare repo defaults HEAD to whatever git's
            // `init.defaultBranch` is; force `main` so the push matches.
            let _ = Command::new("git")
                .args([
                    "-C",
                    bare.to_str().unwrap(),
                    "symbolic-ref",
                    "HEAD",
                    "refs/heads/main",
                ])
                .env("LC_ALL", "C")
                .status();
            let url = format!("file://{}", bare.to_string_lossy().replace('\\', "/"));
            Ok(RepoInfo {
                html_url: url.clone(),
                clone_url: url,
            })
        }

        fn push_url(&self, _org: &str, name: &str) -> String {
            format!(
                "file://{}",
                self.bare_path(name).to_string_lossy().replace('\\', "/")
            )
        }
    }

    fn plan(bare_root: &Path, dry_run: bool) -> PublishPlan {
        PublishPlan {
            // Org URL only matters for the dry-run synth path; the mock
            // creator overrides the push URL on the real path.
            org_url: format!("file://{}", bare_root.to_string_lossy().replace('\\', "/")),
            naming: NamingConvention::KindName,
            dry_run,
            origin: origin_info(),
        }
    }

    fn input(src_root: &Path, rel: &str, kind: &str, name: &str) -> PublishInput {
        PublishInput {
            node: PublishNode {
                rel_path: rel.to_string(),
                kind: match kind {
                    "flow" => vibe_core::PackageKind::Flow,
                    "feat" => vibe_core::PackageKind::Feat,
                    "stack" => vibe_core::PackageKind::Stack,
                    _ => vibe_core::PackageKind::Tool,
                },
                name: name.to_string(),
            },
            source_dir: src_root.join(rel),
        }
    }

    #[test]
    fn publish_loop_publishes_every_node_in_order() {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return;
        }
        let src = tempfile::tempdir().unwrap();
        write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(src.path(), "packages/b/vibe.toml", &package("b", "feat"));
        let bare_root = tempfile::tempdir().unwrap();
        let creator = MockCreator::new(bare_root.path().to_path_buf());
        let inputs = vec![
            input(src.path(), "packages/a", "flow", "a"),
            input(src.path(), "packages/b", "feat", "b"),
        ];
        let plan = plan(bare_root.path(), false);
        let mut seen: Vec<String> = Vec::new();
        let published = publish_loop(Some(&creator), &inputs, &plan, &mut |e, _| {
            seen.push(e.pkgref.clone());
        })
        .expect("publish loop should succeed");
        assert_eq!(published.len(), 2);
        assert_eq!(published[0].pkgref, "flow:a");
        assert_eq!(published[1].pkgref, "feat:b");
        // Progress callback fired once per node, in order.
        assert_eq!(seen, vec!["flow:a", "feat:b"]);
        // Repos created in order: flow-a then feat-b (kind-name naming).
        assert_eq!(*creator.created.borrow(), vec!["flow-a", "feat-b"]);
    }

    #[test]
    fn publish_loop_stops_on_first_failure_and_reports_partial_progress() {
        if !git_available() {
            eprintln!("skipping: git not on PATH");
            return;
        }
        let src = tempfile::tempdir().unwrap();
        write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(src.path(), "packages/b/vibe.toml", &package("b", "feat"));
        write(src.path(), "packages/c/vibe.toml", &package("c", "tool"));
        let bare_root = tempfile::tempdir().unwrap();
        // The middle node (feat-b) fails. a publishes, b fails, c is
        // never reached.
        let creator = MockCreator::failing_on(bare_root.path().to_path_buf(), "feat-b");
        let inputs = vec![
            input(src.path(), "packages/a", "flow", "a"),
            input(src.path(), "packages/b", "feat", "b"),
            input(src.path(), "packages/c", "tool", "c"),
        ];
        let plan = plan(bare_root.path(), false);
        let failure = publish_loop(Some(&creator), &inputs, &plan, &mut |_, _| {})
            .expect_err("publish loop should fail on the middle node");
        // Only `a` published before the failure.
        assert_eq!(failure.published.len(), 1);
        assert_eq!(failure.published[0].pkgref, "flow:a");
        // The failed node is index 1 (`b`) — `b` and `c` are the
        // `remaining` set when finish_failure slices `ordered[1..]`.
        assert_eq!(failure.failed_idx, 1);
        let msg = format!("{}", failure.error);
        assert!(msg.contains("feat:b"), "error names the failed node: {msg}");
        // `c` was never reached — create_repo never called for it.
        assert!(!creator.created.borrow().iter().any(|n| n == "tool-c"));
    }

    #[test]
    fn publish_loop_dry_run_makes_no_network_calls() {
        // No creator at all — the dry-run path must synthesise the
        // outcome from the staged manifest without touching the network.
        let src = tempfile::tempdir().unwrap();
        write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(src.path(), "packages/b/vibe.toml", &package("b", "stack"));
        let bare_root = tempfile::tempdir().unwrap();
        let inputs = vec![
            input(src.path(), "packages/a", "flow", "a"),
            input(src.path(), "packages/b", "stack", "b"),
        ];
        let plan = plan(bare_root.path(), true);
        let published = publish_loop(None, &inputs, &plan, &mut |_, _| {})
            .expect("dry-run publish loop should succeed");
        assert_eq!(published.len(), 2);
        assert_eq!(published[0].pkgref, "flow:a");
        assert_eq!(published[0].repo_name, "flow-a");
        assert_eq!(published[0].tag, "v0.1.0");
        assert_eq!(published[1].repo_name, "stack-b");
        // No bare repos were provisioned — dry-run wrote nothing.
        assert!(
            fs::read_dir(bare_root.path()).unwrap().next().is_none(),
            "dry-run must not create any repo on disk"
        );
    }

    #[test]
    fn publish_loop_staged_copy_carries_origin_and_banner() {
        // Exercise the staging side of the loop through dry-run and
        // confirm the staged content is correct by staging directly.
        let src = tempfile::tempdir().unwrap();
        write(src.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(src.path(), "packages/a/README.md", "# upstream readme\n");
        let staged =
            stage_node(&src.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        // [origin] present and correct.
        let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
        let origin = manifest.origin.as_ref().expect("[origin] present");
        assert_eq!(origin.path, "packages/a");
        assert_eq!(origin.upstream, "https://github.com/you/monorepo");
        // README banner prepended.
        let readme = fs::read_to_string(staged.staging.path().join("README.md")).unwrap();
        assert!(readme.starts_with("<!-- vibevm:generated-copy -->"));
        assert!(readme.contains("# upstream readme"));
        // PR template written.
        assert!(
            staged
                .staging
                .path()
                .join(".github/PULL_REQUEST_TEMPLATE.md")
                .is_file()
        );
    }

    #[test]
    fn dry_run_outcome_reads_staged_manifest() {
        let staged = tempfile::tempdir().unwrap();
        write(staged.path(), "vibe.toml", &package("wal", "flow"));
        let config = PublishConfig {
            source_dir: staged.path().to_path_buf(),
            org_url: "https://github.com/vibespecs".to_string(),
            naming: NamingConvention::KindName,
            tag_prefix: "v".to_string(),
            dry_run: true,
        };
        let outcome = dry_run_outcome(&config, "https://github.com/vibespecs").unwrap();
        assert_eq!(outcome.repo_name, "flow-wal");
        assert_eq!(outcome.tag, "v0.1.0");
        assert_eq!(
            outcome.repo_url,
            "https://github.com/vibespecs/flow-wal.git"
        );
        assert!(outcome.dry_run);
    }

    #[test]
    fn root_identity_name_prefers_project_then_package() {
        let proj =
            Manifest::parse_str("[project]\nname = \"mono\"\nversion = \"0.0.1\"\n").unwrap();
        assert_eq!(root_identity_name(&proj), "mono");
        let pkg = Manifest::parse_str(&package("umbrella", "stack")).unwrap();
        assert_eq!(root_identity_name(&pkg), "umbrella");
        let virt = Manifest::parse_str("[workspace]\nmembers = []\n").unwrap();
        assert_eq!(root_identity_name(&virt), "unknown");
    }
}
