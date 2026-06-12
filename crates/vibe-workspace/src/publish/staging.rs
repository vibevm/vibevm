//! Staging a node for publication — the copy / origin-marker / banner
//! half of selective publish (PROP-007 §2.8–§2.9, PROP-009 §2.11).
//!
//! Split out of [`super`] (selection + ordering) along the
//! staging-vs-selection seam; every public item here is re-exported by
//! the parent, so the `vibe_workspace::publish::*` paths are unchanged.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish");

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{Manifest, OriginSection};

use crate::WorkspaceError;

use super::Result;

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
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#published-repos",
    r = 1
)]
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
        .map(|p| format!("{}/{}", p.group, p.name))
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
            let rel = path.strip_prefix(src).map_err(|_| WorkspaceError::Io {
                path: path.clone(),
                reason: format!("walked path escaped its copy root `{}`", src.display()),
            })?;
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
