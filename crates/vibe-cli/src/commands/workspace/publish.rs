//! `vibe workspace publish` — the side-effecting publish orchestration
//! (PROP-007 §2.7–§2.9): selection + ordering via `vibe_workspace::publish`,
//! then the non-atomic stop-on-first-failure publish loop through
//! `vibe_publish::Publisher`. The `[origin]` provenance probe lives in
//! [`super::origin`].

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#surface");

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

use crate::cli::WorkspacePublishArgs;
use crate::output;

use super::origin::build_origin_info;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

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

pub(super) fn run_publish(ctx: &output::Context, args: WorkspacePublishArgs) -> Result<()> {
    // Discover the workspace enclosing the requested path. A standalone
    // node (no `[workspace]`) discovers as its own root with no members —
    // publishing it is just the root, if the root is a `[package]`.
    let start = args
        .path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", args.path.display()))?;
    let start = crate::commands::init::strip_unc_public(start);
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
#[specmark::spec(
    deviates = "spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "no-unwrap-in-domain: a PublishNode is built from the workspace's own \
              membership, so its rel_path always names a member; member_by_rel_path \
              cannot miss here, and threading a Result would carry a None the plan \
              construction excludes through every node_abs_dir caller"
)]
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
    let repo_name = config
        .naming
        .repo_name(Some(meta.kind), &meta.group, &meta.name)
        .with_context(|| format!("deriving the repo name for `{}/{}`", meta.group, meta.name))?;
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
