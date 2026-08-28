//! Input normalisation at the command boundary: the effective spec format,
//! the generator stamp, and the canonical project root.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_workspace::Workspace;

/// Effective PROP-045 setting: a project pin is reproducible and wins over
/// the operator default; absence at both layers preserves legacy `mixed`.
///
/// Takes the operator default as a VALUE rather than the whole user
/// configuration: the surface owns that value, and it also carries global
/// provider, model and credential settings this crate must never see.
///
/// ```
/// use vibe_orchestrator::resolve_spec_format;
/// fn effective(m: &vibe_core::manifest::Manifest) -> vibe_core::manifest::SpecFormat {
///     resolve_spec_format(m, None)
/// }
/// ```
#[must_use]
pub fn resolve_spec_format(manifest: &Manifest, default: Option<SpecFormat>) -> SpecFormat {
    manifest
        .consumer_node()
        .and_then(|node| node.spec_format)
        .or(default)
        .unwrap_or_default()
}

/// The lockfile provenance stamp the product writes.
///
/// It is the PRODUCT version, not this crate's identity: every workspace crate
/// carries `version.workspace = true`, so the extraction cannot move the stamp.
///
/// ```
/// assert!(vibe_orchestrator::generated_by().starts_with("vibe "));
/// ```
#[must_use]
pub fn generated_by() -> String {
    format!("vibe {}", env!("CARGO_PKG_VERSION"))
}

/// The selected node's manifest inside a tree, by borrow.
///
/// The read-only twin of the workspace crate's own mutable finder. A caller
/// holding a tree the command itself produced — the post-apply workspace, say — needs the
/// manifest THAT tree carries, not the pre-apply copy it was handed earlier.
///
/// ```
/// use vibe_orchestrator::selected_node_manifest;
/// fn find<'a>(w: &'a vibe_workspace::Workspace, root: &std::path::Path)
///     -> Option<&'a vibe_core::manifest::Manifest>
/// {
///     selected_node_manifest(w, root)
/// }
/// ```
#[must_use]
pub fn selected_node_manifest<'a>(
    workspace: &'a Workspace,
    project_root: &Path,
) -> Option<&'a Manifest> {
    if workspace.root == project_root {
        return Some(&workspace.root_manifest);
    }
    workspace
        .members
        .iter()
        .find(|member| workspace.member_abs_path(member) == project_root)
        .map(|member| &member.manifest)
}

/// Canonicalise one selected project root and prove it carries a manifest.
///
/// ```
/// let refused = vibe_orchestrator::resolve_project_root(std::path::Path::new("/definitely/absent"));
/// assert!(refused.is_err());
/// ```
pub fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = vibe_workspace::strip_unc_prefix(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

#[cfg(test)]
#[path = "inputs/tests.rs"]
mod tests;
