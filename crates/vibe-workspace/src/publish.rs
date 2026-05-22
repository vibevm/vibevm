//! Selective publish of a workspace's self-publishing members.
//!
//! Spec: [PROP-007 §2.7–§2.9](../../../spec/modules/vibe-workspace/PROP-007-workspace.md#selective-publish).
//!
//! `vibe workspace publish` walks the publishable nodes of a workspace in
//! dependency-first order and publishes each as its own repository, reusing
//! the per-package machinery in `vibe-publish`. This module owns the parts
//! that are pure and hermetically testable — *which* nodes publish, in *what*
//! order, and *what* a staged copy looks like — leaving the actual
//! repo-creation + push to the CLI (which threads `vibe-publish::Publisher`).
//!
//! Three concerns live here:
//!
//! - [`select_publishable_nodes`] — every node carrying `[package]` whose
//!   [`PublishPosture`] does not exclude it from the primary registry.
//! - [`topo_order`] — a stable dependency-first ordering of the selected
//!   nodes, derived from inter-member `path` dependencies. A cycle is a hard
//!   error per PROP-007 §2.7.
//! - [`stage_node`] — copy a node's directory into a fresh temp dir,
//!   excluding `.git/` and `.vibe/`, inject the `[origin]` provenance marker
//!   into the staged `vibe.toml`, prepend the "generated copy" README banner,
//!   write `.github/PULL_REQUEST_TEMPLATE.md`, and regenerate the boot
//!   artifacts for the published shape (PROP-009 §2.11) so they never
//!   dangle on the dev tree's workspace `vibedeps/` slots.
//!
//! Token discipline is not a concern of this module — no token, push URL, or
//! credential ever reaches it. That machinery is `vibe-publish`'s, reused
//! unchanged by the CLI.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use vibe_core::PackageKind;
use vibe_core::manifest::{Manifest, OriginSection};

use crate::{Workspace, WorkspaceError};

type Result<T> = std::result::Result<T, WorkspaceError>;

/// One node of a workspace selected for publishing — the root (when it
/// carries `[package]`) or a member.
///
/// `rel_path` is `"."` for the absolute root and the member's root-relative
/// forward-slashed path otherwise. The `kind`/`name` pair is the node's
/// package identity, lifted out of its `[package]` table so the topological
/// sort and reporting code do not have to re-unwrap it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishNode {
    /// `"."` for the absolute root, the member `rel_path` otherwise.
    pub rel_path: String,
    /// Package kind from the node's `[package].kind`.
    pub kind: PackageKind,
    /// Package name from the node's `[package].name`.
    pub name: String,
}

impl PublishNode {
    /// `<kind>:<name>` — the human-facing pkgref for progress lines / JSON.
    pub fn pkgref(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.name)
    }
}

/// A node skipped during selection, with the reason — surfaced to the
/// operator so a `publish = false` member never silently disappears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedNode {
    /// `"."` for the absolute root, the member `rel_path` otherwise.
    pub rel_path: String,
    /// Human-readable reason — e.g. `publish = false` or
    /// `publish posture excludes registry "vibespecs"`.
    pub reason: String,
}

/// Outcome of [`select_publishable_nodes`]: the publishable nodes plus the
/// `[package]`-carrying nodes deliberately skipped by their posture.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Nodes that will be published (before topological ordering).
    pub publishable: Vec<PublishNode>,
    /// `[package]` nodes excluded by their `publish` posture. Nodes with no
    /// `[package]` table at all are not packages and are not reported here.
    pub skipped: Vec<SkippedNode>,
}

/// Select the publishable nodes of `workspace`, honouring each node's
/// [`PublishPosture`] against `primary_registry`.
///
/// A node is publishable iff it carries `[package]` and its posture is not
/// `is_never()` and `includes(primary_registry)` (the latter narrows the
/// `publish = ["..."]` list form — PROP-007 §2.7).
///
/// `only_member` narrows the selection to a single node by `rel_path`
/// (`"."` selects the root). When it names a node that exists but whose
/// posture excludes it, that exclusion is still reported in `skipped` — the
/// operator asked for a node that will not publish and deserves to be told.
/// When it names no node at all, [`WorkspaceError::MemberNotFound`] is
/// raised so a typo'd `--member` fails loudly rather than silently
/// publishing nothing.
pub fn select_publishable_nodes(
    workspace: &Workspace,
    primary_registry: &str,
    only_member: Option<&str>,
) -> Result<Selection> {
    // If `--member` was given, confirm the node exists at all (root or a
    // member). A non-existent rel_path is an operator error.
    if let Some(target) = only_member {
        let exists = target == "." || workspace.members.iter().any(|m| m.rel_path == target);
        if !exists {
            return Err(WorkspaceError::MemberNotFound {
                pattern: target.to_string(),
                declared_in: "the --member argument".to_string(),
            });
        }
    }

    let mut selection = Selection::default();
    for (rel_path, manifest) in workspace.iter_nodes() {
        if let Some(target) = only_member
            && target != rel_path
        {
            continue;
        }
        let Some(meta) = &manifest.package else {
            // Not a publishable package — `[project]`-only or a virtual
            // workspace coordinator. Not an error, not reported.
            continue;
        };
        if meta.publish.is_never() {
            selection.skipped.push(SkippedNode {
                rel_path: rel_path.to_string(),
                reason: "publish = false (workspace-internal)".to_string(),
            });
            continue;
        }
        if !meta.publish.includes(primary_registry) {
            selection.skipped.push(SkippedNode {
                rel_path: rel_path.to_string(),
                reason: format!("publish posture excludes registry `{primary_registry}`"),
            });
            continue;
        }
        selection.publishable.push(PublishNode {
            rel_path: rel_path.to_string(),
            kind: meta.kind,
            name: meta.name.clone(),
        });
    }
    Ok(selection)
}

/// Order `nodes` dependency-first.
///
/// Node M depends on node N when M's `[requires].path_packages` contains a
/// `path` that resolves — relative to M's directory — to N's directory.
/// Dependencies are published before their dependents (N before M); nodes
/// with no inter-member path dependency keep a stable `rel_path`-sorted
/// order. A cycle among the selected nodes is [`WorkspaceError::NestingCycle`]
/// — distributed publishing has no way to break one (PROP-007 §2.7).
///
/// `workspace` supplies the directory layout used to resolve each `path`
/// dependency back to a node `rel_path`. A `path` dependency that resolves
/// outside the selected set (an external sibling, a non-publishing member)
/// is simply not an edge — it imposes no ordering constraint here.
pub fn topo_order(workspace: &Workspace, nodes: &[PublishNode]) -> Result<Vec<PublishNode>> {
    // Index the selected nodes by rel_path for O(1) membership tests, and
    // pre-sort by rel_path so the output is deterministic when the graph
    // imposes no constraint (Kahn's algorithm drains ready nodes in
    // insertion order).
    let mut sorted: Vec<&PublishNode> = nodes.iter().collect();
    sorted.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let selected: HashSet<&str> = sorted.iter().map(|n| n.rel_path.as_str()).collect();

    // Map an absolute directory back to a selected node's rel_path. Built
    // once; consulted per path-dependency.
    let mut dir_to_rel: HashMap<PathBuf, String> = HashMap::new();
    for node in &sorted {
        dir_to_rel.insert(
            node_abs_dir(workspace, &node.rel_path),
            node.rel_path.clone(),
        );
    }

    // Build the dependency edges: deps[m] = set of rel_paths m depends on.
    // Only edges between two selected nodes count.
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &sorted {
        deps.entry(node.rel_path.as_str()).or_default();
    }
    for node in &sorted {
        let manifest = node_manifest(workspace, &node.rel_path);
        let node_dir = node_abs_dir(workspace, &node.rel_path);
        for pd in &manifest.requires.path_packages {
            // Resolve the path-dep directory relative to the depending
            // node's directory, then normalise it (`..` / `.` segments).
            let dep_dir = normalise(&node_dir.join(rel_to_native(&pd.path)));
            let Some(dep_rel) = dir_to_rel.get(&dep_dir) else {
                continue; // not a selected node — imposes no ordering
            };
            if dep_rel.as_str() == node.rel_path.as_str() {
                continue; // self-edge — ignore
            }
            if !selected.contains(dep_rel.as_str()) {
                continue;
            }
            let m = node.rel_path.as_str();
            // De-dup edges — a node may path-dep the same member twice.
            let edge_list = deps.entry(m).or_default();
            if !edge_list.contains(&dep_rel.as_str()) {
                edge_list.push(dep_rel.as_str());
            }
        }
    }

    // Topological sort by repeated ready-node draining. A node is ready
    // once every dependency it has is already emitted; ties are broken by
    // rel_path order (`sorted` is rel_path-sorted, so the linear scan
    // emits the lexicographically-first ready node first — stable output
    // without a heap).
    let mut order: Vec<PublishNode> = Vec::with_capacity(sorted.len());
    let mut done: HashSet<&str> = HashSet::new();
    while order.len() < sorted.len() {
        let mut progressed = false;
        for node in &sorted {
            let rel = node.rel_path.as_str();
            if done.contains(rel) {
                continue;
            }
            // Ready iff every dependency is already emitted.
            let ready = deps
                .get(rel)
                .map(|ds| ds.iter().all(|d| done.contains(d)))
                .unwrap_or(true);
            if ready {
                order.push((*node).clone());
                done.insert(rel);
                progressed = true;
            }
        }
        if !progressed {
            // Remaining nodes form at least one cycle. Name one for the
            // error — the first node still not done.
            let stuck = sorted
                .iter()
                .find(|n| !done.contains(n.rel_path.as_str()))
                .map(|n| n.rel_path.clone())
                .unwrap_or_else(|| "<unknown>".to_string());
            return Err(WorkspaceError::NestingCycle { path: stuck });
        }
    }
    Ok(order)
}

/// Result of staging a node for publication — the temp staging directory
/// and the [`OriginSection`] written into its `vibe.toml`.
///
/// The [`tempfile::TempDir`] is owned by the caller; the staged content is
/// deleted when it drops. The CLI hands `staging.path()` to
/// `vibe-publish::Publisher` as the publish source directory.
pub struct StagedNode {
    /// The temp directory holding the staged copy. Drops → deleted.
    pub staging: tempfile::TempDir,
    /// The `[origin]` marker written into the staged `vibe.toml`.
    pub origin: OriginSection,
}

/// Inputs to [`stage_node`] describing the source-of-truth monorepo.
///
/// Kept as a struct rather than a long argument list because the CLI
/// computes these once (one `git remote get-url` + one `git rev-parse`)
/// and stages every node against the same values.
#[derive(Debug, Clone)]
pub struct OriginInfo {
    /// `[origin].upstream` — the workspace root's `origin` remote URL when
    /// the root is a git repo with that remote, else the root manifest's
    /// project/package name as a best-effort identity.
    pub upstream: String,
    /// `[origin].commit` — the root repo's `HEAD` commit, or `None` when
    /// the root is not a git repository.
    pub commit: Option<String>,
    /// `[origin].generated_by` — e.g. `vibe 0.1.0`.
    pub generated_by: String,
    /// `[origin].generated_at` — an ISO-8601 UTC timestamp.
    pub generated_at: String,
}

/// Stage a node's directory into a fresh temp dir, ready for publication.
///
/// Steps, in order:
///
/// 1. Copy the node's directory tree into a temp dir, **excluding** any
///    `.git/` and `.vibe/` subtree (the published copy is a fresh repo;
///    vibevm's per-project cache must not travel).
/// 2. Read the staged `vibe.toml`, attach an [`OriginSection`] computed
///    from `origin` + the node's root-relative `rel_path`, set the staged
///    `[package].description` to the "generated copy" string so the
///    publisher sends it verbatim to the host, then write it back.
/// 3. Prepend the "generated read-only copy" banner to the staged
///    `README.md` (created if absent).
/// 4. Write `.github/PULL_REQUEST_TEMPLATE.md` with a STOP notice.
/// 5. Regenerate the staged copy's boot artifacts for the published
///    shape (PROP-009 §2.11) — see [`regenerate_published_boot`].
///
/// `node_rel_path` is the node's path relative to the workspace root —
/// `"."` for the root, `"packages/flow-wal"` for a member. It is recorded
/// verbatim as `[origin].path` (a leading `./` is stripped so the marker
/// reads cleanly).
///
/// The returned [`StagedNode`] owns the temp dir; keep it alive until the
/// publish completes.
pub fn stage_node(
    source_dir: &Path,
    node_rel_path: &str,
    origin: &OriginInfo,
) -> Result<StagedNode> {
    let staging = tempfile::TempDir::new().map_err(|e| WorkspaceError::Io {
        path: std::env::temp_dir(),
        reason: format!("creating publish staging dir: {e}"),
    })?;
    let staging_path = staging.path();

    // Step 1 — copy the directory tree, skipping `.git/` and `.vibe/`.
    copy_tree_excluding(source_dir, staging_path)?;

    // Step 2 — inject `[origin]` + the generated-copy description.
    let manifest_path = staging_path.join(Manifest::FILENAME);
    let mut manifest =
        Manifest::read(&manifest_path).map_err(|source| WorkspaceError::Manifest {
            path: manifest_path.clone(),
            source: Box::new(source),
        })?;

    // `[origin].path` reads cleaner without a leading `./`; the root node
    // stages as `.` which we keep verbatim (it is the marker's honest value).
    let origin_path = node_rel_path.to_string();
    let origin_section = OriginSection {
        upstream: origin.upstream.clone(),
        path: origin_path,
        commit: origin.commit.clone(),
        generated_by: origin.generated_by.clone(),
        generated_at: origin.generated_at.clone(),
    };

    // The published copy must be unmistakably a generated read-only copy.
    // `vibe-publish::Publisher` derives the repo `description` from
    // `[package].description`; overwrite it here so the host-side
    // description reads "Generated copy of ... — contribute at ...". This
    // keeps `vibe-publish` API-stable (no new CreateOpts override needed).
    let pkgref = manifest
        .package
        .as_ref()
        .map(|p| format!("{}:{}", p.kind.as_str(), p.name))
        .unwrap_or_else(|| node_rel_path.to_string());
    if let Some(meta) = manifest.package.as_mut() {
        meta.description = Some(generated_copy_description(&pkgref, &origin.upstream));
    }
    manifest.origin = Some(origin_section.clone());
    manifest
        .write(&manifest_path)
        .map_err(|source| WorkspaceError::Manifest {
            path: manifest_path.clone(),
            source: Box::new(source),
        })?;

    // Step 3 — prepend the README banner (create README.md if absent).
    let readme_path = staging_path.join("README.md");
    let existing = std::fs::read_to_string(&readme_path).unwrap_or_default();
    let banner = generated_copy_readme_banner(&pkgref, &origin.upstream);
    let new_readme = if existing.trim().is_empty() {
        banner
    } else {
        format!("{banner}\n{existing}")
    };
    std::fs::write(&readme_path, new_readme).map_err(|e| WorkspaceError::Io {
        path: readme_path.clone(),
        reason: format!("writing README banner: {e}"),
    })?;

    // Step 4 — `.github/PULL_REQUEST_TEMPLATE.md` STOP notice.
    let gh_dir = staging_path.join(".github");
    std::fs::create_dir_all(&gh_dir).map_err(|e| WorkspaceError::Io {
        path: gh_dir.clone(),
        reason: format!("creating .github dir: {e}"),
    })?;
    let pr_template_path = gh_dir.join("PULL_REQUEST_TEMPLATE.md");
    std::fs::write(
        &pr_template_path,
        pull_request_template(&pkgref, &origin.upstream),
    )
    .map_err(|e| WorkspaceError::Io {
        path: pr_template_path.clone(),
        reason: format!("writing PULL_REQUEST_TEMPLATE.md: {e}"),
    })?;

    // Step 5 — regenerate the boot artifacts for the published shape
    // (PROP-009 §2.11). The dev tree's `INDEX.md` points at the
    // workspace `vibedeps/` slots, absent from a standalone published
    // copy; regenerate from the staged node's own authored boot.
    regenerate_published_boot(staging_path, &manifest)?;

    Ok(StagedNode {
        staging,
        origin: origin_section,
    })
}

/// Regenerate a staged copy's boot artifacts for the **published shape**
/// (PROP-009 §2.11).
///
/// In the development workspace a node's generated `INDEX.md` references
/// the dependency content materialised under the workspace-root
/// `vibedeps/` tree. A standalone published copy carries no such tree —
/// publishing the dev tree's artifacts verbatim would leave every
/// dependency entry dangling for an external consumer.
///
/// The published copy is regenerated as a standalone node: its own
/// authored boot only, with no inherited foundation and no materialised
/// dependencies. A consumer that installs the published package
/// re-materialises the dependency content into *its own* `vibedeps/` and
/// regenerates *its own* boot on `vibe install`; the published copy just
/// needs artifacts that name only the files it actually ships.
fn regenerate_published_boot(node_dir: &Path, manifest: &Manifest) -> Result<()> {
    let own = crate::install::node_own_boot(node_dir, ".")?;
    let effective = crate::boot::compute_effective_boot(crate::boot::NodeBootInputs {
        own_boot: &own,
        inherited_foundation: &[],
        dependencies: &[],
        default_link: manifest.boot.default_link,
    })?;
    crate::boot_artifacts::write_boot_artifacts(node_dir, node_dir, &effective)?;
    Ok(())
}

/// The repo `description` for a generated copy — surfaced in the host's
/// repo header. PROP-007 §2.8 layer 2.
pub fn generated_copy_description(pkgref: &str, upstream: &str) -> String {
    format!("Generated copy of `{pkgref}` — contribute at {upstream}")
}

/// The README banner block prepended to a generated copy's `README.md`.
/// PROP-007 §2.8 layer 1; tone follows `vibe-publish`'s redirect-stub
/// `build_redirect_readme`.
pub fn generated_copy_readme_banner(pkgref: &str, upstream: &str) -> String {
    format!(
        "<!-- vibevm:generated-copy -->\n\
         > # Generated copy — do not contribute here\n\
         >\n\
         > This repository is a **generated, read-only copy** of `{pkgref}`,\n\
         > published by `vibe workspace publish` from a vibevm workspace\n\
         > (PROP-007 §2.8). The development source of truth is the monorepo:\n\
         >\n\
         > > {upstream}\n\
         >\n\
         > **Pull requests opened here are not accepted.** Issues and changes\n\
         > belong upstream — open them against the monorepo above. This copy\n\
         > exists only so the package can be resolved as its own repository;\n\
         > it is overwritten wholesale on every re-publish.\n\
         <!-- /vibevm:generated-copy -->\n"
    )
}

/// The `.github/PULL_REQUEST_TEMPLATE.md` body written into a generated
/// copy — fires at PR-creation time. PROP-007 §2.8 layer 4.
pub fn pull_request_template(pkgref: &str, upstream: &str) -> String {
    format!(
        "<!-- vibevm:generated-copy -->\n\
         # STOP — this repository does not accept pull requests\n\n\
         This is a **generated, read-only copy** of `{pkgref}`, published from a\n\
         vibevm workspace by `vibe workspace publish`. Any change pushed here is\n\
         lost on the next re-publish.\n\n\
         Open your pull request against the development source of truth instead:\n\n\
         > {upstream}\n\n\
         Thank you — and sorry for the detour.\n"
    )
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// The manifest of a node by its `rel_path` (`"."` = the root).
fn node_manifest<'a>(workspace: &'a Workspace, rel_path: &str) -> &'a Manifest {
    if rel_path == "." {
        &workspace.root_manifest
    } else {
        workspace
            .member_by_rel_path(rel_path)
            .map(|m| &m.manifest)
            .expect("publish node rel_path always names a workspace node")
    }
}

/// The absolute directory of a node by its `rel_path` (`"."` = the root).
fn node_abs_dir(workspace: &Workspace, rel_path: &str) -> PathBuf {
    if rel_path == "." {
        workspace.root.clone()
    } else {
        let member = workspace
            .member_by_rel_path(rel_path)
            .expect("publish node rel_path always names a workspace node");
        workspace.member_abs_path(member)
    }
}

/// Turn a forward-slashed relative path into a native `PathBuf` (segment by
/// segment so Windows gets backslashes and `..` survives as a component).
fn rel_to_native(rel: &str) -> PathBuf {
    let mut p = PathBuf::new();
    for segment in rel.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        p.push(segment);
    }
    p
}

/// Lexically normalise a path — collapse `.` and resolve `..` against the
/// preceding component. No filesystem access (the directories may not all
/// exist yet under a `--dry-run`); a purely textual normalisation is enough
/// to compare a path-dep target against a node's directory.
fn normalise(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                // Pop a normal component; keep `..` if there is nothing to
                // pop (path escapes its base — rare, but do not corrupt it).
                if matches!(out.last().map(|s| s.as_os_str()), Some(s) if s != "..") {
                    out.pop();
                } else {
                    out.push(std::ffi::OsString::from(".."));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                out.push(comp.as_os_str().to_os_string());
            }
            Component::Normal(s) => out.push(s.to_os_string()),
        }
    }
    let mut p = PathBuf::new();
    for seg in out {
        p.push(seg);
    }
    p
}

/// Recursively copy `src` into `dst`, excluding any `.git/` or `.vibe/`
/// subtree at any depth. `dst` is created if absent.
fn copy_tree_excluding(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| WorkspaceError::Io {
        path: dst.to_path_buf(),
        reason: format!("create_dir_all: {e}"),
    })?;
    let mut stack: Vec<PathBuf> = vec![src.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| WorkspaceError::Io {
            path: dir.clone(),
            reason: format!("read_dir: {e}"),
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| WorkspaceError::Io {
                path: dir.clone(),
                reason: format!("read_dir entry: {e}"),
            })?;
            let path = entry.path();
            let rel = path.strip_prefix(src).expect("walk yields paths under src");
            // Skip `.git/` and `.vibe/` at any depth — the published copy
            // is a clean repo; the dev cache must not travel.
            if rel
                .components()
                .any(|c| matches!(c.as_os_str().to_str(), Some(".git") | Some(".vibe")))
            {
                continue;
            }
            let target = dst.join(rel);
            let file_type = entry.file_type().map_err(|e| WorkspaceError::Io {
                path: path.clone(),
                reason: format!("file_type: {e}"),
            })?;
            if file_type.is_dir() {
                std::fs::create_dir_all(&target).map_err(|e| WorkspaceError::Io {
                    path: target.clone(),
                    reason: format!("create_dir_all: {e}"),
                })?;
                stack.push(path);
            } else if file_type.is_file() {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| WorkspaceError::Io {
                        path: parent.to_path_buf(),
                        reason: format!("create_dir_all: {e}"),
                    })?;
                }
                std::fs::copy(&path, &target).map_err(|e| WorkspaceError::Io {
                    path: target.clone(),
                    reason: format!("copy: {e}"),
                })?;
            }
            // Symlinks and other node types are intentionally not copied —
            // a published package tree is plain files.
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn workspace_root(name: &str, members: &[&str]) -> String {
        let list = members
            .iter()
            .map(|m| format!("\"{m}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n\n[workspace]\nmembers = [{list}]\n"
        )
    }

    fn package(name: &str, kind: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n")
    }

    fn package_publish(name: &str, kind: &str, publish: &str) -> String {
        format!(
            "[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n\
             publish = {publish}\n"
        )
    }

    fn origin_info() -> OriginInfo {
        OriginInfo {
            upstream: "https://github.com/you/monorepo".to_string(),
            commit: Some("abc123def456".to_string()),
            generated_by: "vibe 0.1.0".to_string(),
            generated_at: "2026-05-21T00:00:00Z".to_string(),
        }
    }

    // ----- selection -----

    #[test]
    fn selection_includes_default_publish_and_skips_never() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        // a: default posture (publish = true). b: publish = false.
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &package_publish("b", "flow", "false"),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        assert_eq!(sel.publishable.len(), 1);
        assert_eq!(sel.publishable[0].rel_path, "packages/a");
        assert_eq!(sel.skipped.len(), 1);
        assert_eq!(sel.skipped[0].rel_path, "packages/b");
        assert!(sel.skipped[0].reason.contains("publish = false"));
    }

    #[test]
    fn selection_honours_registry_list_form() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        // a: publish only to "vibespecs". b: publish only to "corp".
        write(
            tmp.path(),
            "packages/a/vibe.toml",
            &package_publish("a", "flow", "[\"vibespecs\"]"),
        );
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &package_publish("b", "flow", "[\"corp\"]"),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        assert_eq!(sel.publishable.len(), 1);
        assert_eq!(sel.publishable[0].rel_path, "packages/a");
        // b is reported skipped — its list excludes "vibespecs".
        assert_eq!(sel.skipped.len(), 1);
        assert!(sel.skipped[0].reason.contains("excludes registry"));
    }

    #[test]
    fn selection_skips_non_package_nodes_without_reporting() {
        let tmp = TempDir::new().unwrap();
        // Root is a plain [project] — not a package; not reported.
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        assert_eq!(sel.publishable.len(), 1);
        // The [project] root is not in `skipped` — it is not a package.
        assert!(sel.skipped.is_empty());
    }

    #[test]
    fn selection_includes_root_when_it_is_a_package() {
        // cargo-style: root carries [package] + [workspace]. PROP-007 §2.9.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &format!(
                "{}\n[workspace]\nmembers = [\"packages/a\"]\n",
                package("umbrella", "stack")
            ),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        assert_eq!(sel.publishable.len(), 2);
        assert!(sel.publishable.iter().any(|n| n.rel_path == "."));
    }

    #[test]
    fn selection_member_filter_narrows_to_one() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/b/vibe.toml", &package("b", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", Some("packages/b")).unwrap();
        assert_eq!(sel.publishable.len(), 1);
        assert_eq!(sel.publishable[0].rel_path, "packages/b");
    }

    #[test]
    fn selection_member_filter_reports_excluded_target() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a"]),
        );
        write(
            tmp.path(),
            "packages/a/vibe.toml",
            &package_publish("a", "flow", "false"),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        // --member names a real node, but its posture excludes it.
        let sel = select_publishable_nodes(&ws, "vibespecs", Some("packages/a")).unwrap();
        assert!(sel.publishable.is_empty());
        assert_eq!(sel.skipped.len(), 1);
        assert!(sel.skipped[0].reason.contains("publish = false"));
    }

    #[test]
    fn selection_member_filter_rejects_unknown_node() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let err = select_publishable_nodes(&ws, "vibespecs", Some("packages/ghost")).unwrap_err();
        assert!(
            matches!(err, WorkspaceError::MemberNotFound { .. }),
            "{err}"
        );
    }

    // ----- topological order -----

    #[test]
    fn topo_order_is_dependency_first() {
        // b depends on a via a path dep — a must publish before b.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\"flow:a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
                package("b", "flow")
            ),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        let ordered = topo_order(&ws, &sel.publishable).unwrap();
        let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["packages/a", "packages/b"]);
    }

    #[test]
    fn topo_order_stable_without_edges() {
        // No inter-member deps — stable rel_path order.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/z", "packages/a", "packages/m"]),
        );
        write(tmp.path(), "packages/z/vibe.toml", &package("z", "flow"));
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/m/vibe.toml", &package("m", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        let ordered = topo_order(&ws, &sel.publishable).unwrap();
        let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["packages/a", "packages/m", "packages/z"]);
    }

    #[test]
    fn topo_order_chain_of_three() {
        // c → b → a. Publish order must be a, b, c.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b", "packages/c"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\"flow:a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
                package("b", "flow")
            ),
        );
        write(
            tmp.path(),
            "packages/c/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\"flow:b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
                package("c", "flow")
            ),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        let ordered = topo_order(&ws, &sel.publishable).unwrap();
        let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["packages/a", "packages/b", "packages/c"]);
    }

    #[test]
    fn topo_order_detects_cycle() {
        // a depends on b, b depends on a — a hard error.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        write(
            tmp.path(),
            "packages/a/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\"flow:b\" = {{ path = \"../b\", version = \"^0.1\" }}\n",
                package("a", "flow")
            ),
        );
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\"flow:a\" = {{ path = \"../a\", version = \"^0.1\" }}\n",
                package("b", "flow")
            ),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        let err = topo_order(&ws, &sel.publishable).unwrap_err();
        assert!(matches!(err, WorkspaceError::NestingCycle { .. }), "{err}");
    }

    #[test]
    fn topo_order_path_dep_outside_selection_imposes_no_edge() {
        // b path-deps an external dir that is not a selected node. That
        // imposes no ordering — both still publish, rel_path order.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/a", "packages/b"]),
        );
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(
            tmp.path(),
            "packages/b/vibe.toml",
            &format!(
                "{}\n[requires.packages]\n\
                 \"flow:ext\" = {{ path = \"../../external\", version = \"^0.1\" }}\n",
                package("b", "flow")
            ),
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let sel = select_publishable_nodes(&ws, "vibespecs", None).unwrap();
        let ordered = topo_order(&ws, &sel.publishable).unwrap();
        let rels: Vec<&str> = ordered.iter().map(|n| n.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["packages/a", "packages/b"]);
    }

    // ----- staging -----

    #[test]
    fn stage_node_writes_origin_section() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/a/spec/X.md", "spec content");
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
        let origin = manifest.origin.as_ref().expect("origin written");
        assert_eq!(origin.upstream, "https://github.com/you/monorepo");
        assert_eq!(origin.path, "packages/a");
        assert_eq!(origin.commit.as_deref(), Some("abc123def456"));
        assert_eq!(origin.generated_by, "vibe 0.1.0");
        assert_eq!(origin.generated_at, "2026-05-21T00:00:00Z");
        // Spec content travelled.
        assert!(staged.staging.path().join("spec/X.md").is_file());
    }

    #[test]
    fn stage_node_excludes_git_and_vibe_dirs() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/a/.git/HEAD", "ref: refs/heads/main");
        write(tmp.path(), "packages/a/.git/objects/x", "obj");
        write(tmp.path(), "packages/a/.vibe/cache.bin", "cache");
        write(tmp.path(), "packages/a/keep.md", "keep me");
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        assert!(!staged.staging.path().join(".git").exists());
        assert!(!staged.staging.path().join(".vibe").exists());
        assert!(staged.staging.path().join("keep.md").is_file());
    }

    #[test]
    fn stage_node_prepends_readme_banner() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/a/README.md", "# Original readme\n");
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        let readme = fs::read_to_string(staged.staging.path().join("README.md")).unwrap();
        assert!(readme.contains("Generated copy — do not contribute here"));
        assert!(readme.contains("https://github.com/you/monorepo"));
        // Original content preserved below the banner.
        assert!(readme.contains("# Original readme"));
        // Banner comes first.
        assert!(readme.starts_with("<!-- vibevm:generated-copy -->"));
    }

    #[test]
    fn stage_node_creates_readme_when_absent() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        let readme_path = staged.staging.path().join("README.md");
        assert!(readme_path.is_file());
        let readme = fs::read_to_string(&readme_path).unwrap();
        assert!(readme.contains("Generated copy — do not contribute here"));
    }

    #[test]
    fn stage_node_writes_pr_template() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        let pr_template = fs::read_to_string(
            staged
                .staging
                .path()
                .join(".github/PULL_REQUEST_TEMPLATE.md"),
        )
        .unwrap();
        assert!(pr_template.contains("does not accept pull requests"));
        assert!(pr_template.contains("https://github.com/you/monorepo"));
        assert!(pr_template.contains("flow:a"));
    }

    #[test]
    fn stage_node_sets_generated_copy_description() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();
        let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
        let desc = manifest
            .package
            .as_ref()
            .and_then(|p| p.description.clone())
            .expect("description set");
        assert!(desc.contains("Generated copy of `flow:a`"));
        assert!(desc.contains("https://github.com/you/monorepo"));
    }

    #[test]
    fn stage_node_omits_commit_when_none() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        let mut info = origin_info();
        info.commit = None;
        let staged = stage_node(&tmp.path().join("packages/a"), "packages/a", &info).unwrap();
        let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
        assert!(manifest.origin.as_ref().unwrap().commit.is_none());
    }

    #[test]
    fn stage_node_regenerates_boot_for_the_published_shape() {
        // PROP-009 §2.11: the dev tree's boot artifacts reference the
        // workspace `vibedeps/` slots, which do not exist in a standalone
        // published copy. `stage_node` regenerates them from the staged
        // node's own authored boot so nothing dangles.
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "packages/a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/a/spec/boot/00-core.md", "# core");
        // A stale dev-tree INDEX.md pointing at a workspace `vibedeps/`
        // slot — exactly what must not be published verbatim.
        write(
            tmp.path(),
            "packages/a/spec/boot/INDEX.md",
            "schema = 1\n\n[[entry]]\n\
             path = \"vibedeps/flow-dep/1.0.0/boot/dep.md\"\nkind = \"static\"\n",
        );
        // A stale INLINE.md left over from a dev-tree inline dependency.
        write(
            tmp.path(),
            "packages/a/spec/boot/INLINE.md",
            "stale inline lane",
        );
        write(tmp.path(), "packages/a/CLAUDE.md", "stale dev redirect");

        let staged =
            stage_node(&tmp.path().join("packages/a"), "packages/a", &origin_info()).unwrap();

        // The dangling `vibedeps/` reference is gone; the node's own
        // authored foundation boot is named instead.
        let index = fs::read_to_string(staged.staging.path().join("spec/boot/INDEX.md")).unwrap();
        assert!(
            !index.contains("vibedeps/"),
            "the published INDEX.md must not dangle on a workspace vibedeps/ slot:\n{index}"
        );
        assert!(
            index.contains("spec/boot/00-core.md"),
            "the published INDEX.md must name the node's own authored boot:\n{index}"
        );
        // No inline dependencies in the published shape — the stale
        // INLINE.md is removed.
        assert!(
            !staged.staging.path().join("spec/boot/INLINE.md").exists(),
            "a stale INLINE.md must be cleared in the published copy"
        );
        // The redirect is regenerated as a thin generated pointer.
        let claude = fs::read_to_string(staged.staging.path().join("CLAUDE.md")).unwrap();
        assert!(
            claude.contains("Generated by vibe") && claude.contains("spec/boot/INDEX.md"),
            "CLAUDE.md must be a regenerated redirect:\n{claude}"
        );
    }
}
