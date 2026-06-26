//! The apply phase — runs only after the caller confirmed the plan:
//! merge the requested roots into `vibe.toml`, materialise the
//! resolution into `vibedeps/`, and rebuild `vibe.lock` wholesale.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;

use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::SlotIntegrity;
use vibe_workspace::Workspace;
use vibe_workspace::hooks::{HookPolicy, HookReport};
use vibe_workspace::install::{InstallOutcome, apply_resolution, run_post_install_hooks};

use crate::error::{Error, Result};
use crate::plan::PlannedInstall;
use crate::record::{
    finalize_pkgref_for_manifest, locked_package_from_fetched, merge_manifest_requires,
    merge_root_dependencies,
};

/// What [`apply`] did — the caller renders it.
#[derive(Debug)]
pub struct ApplyReport {
    /// Materialised / skipped / pruned slots, the regenerated nodes, and
    /// the `pre-install` hook reports — straight from the workspace
    /// orchestrator.
    pub outcome: InstallOutcome,
    /// `post-install` hook reports (PROP-020 §2.1), gathered after the
    /// lockfile was written. Empty when no materialised package declares a
    /// `post-install` hook; the CLI renders them with the pre-install
    /// reports carried on `outcome.hook_reports`.
    pub post_install_reports: Vec<HookReport>,
}

/// Apply a confirmed plan. `slot_integrity` selects the PROP-011 §2.3
/// materialise-diff strategy (the caller reads it from the user
/// config, so a malformed config fails before resolution, not here).
pub fn apply(
    planned: PlannedInstall,
    slot_integrity: SlotIntegrity,
    hooks: &HookPolicy,
) -> Result<ApplyReport> {
    let PlannedInstall {
        project_root,
        request,
        mut manifest,
        mut lockfile,
        language_chain,
        roots,
        fetched,
        resolution,
    } = planned;

    // 6. Update `vibe.toml` `[requires].packages` with the requested
    //    roots — caret by default, `exact` pins `=<resolved>`, an
    //    explicit constraint is preserved verbatim. De-dup by
    //    `(group, name)`; a no-op in install-from-manifest mode.
    //
    //    This MUST run before the boot regeneration below:
    //    `apply_resolution` composes each node's boot from its
    //    `[requires]`, so a package installed by pkgref has to be
    //    declared first or its boot snippet is dropped from the
    //    generated `INDEX.md`.
    //
    //    A requested root absent from the fetched set means the
    //    install source returned an incomplete resolution — through
    //    the seam that is checkable input, not a construction
    //    invariant, so it surfaces as an error.
    let finalized_roots: Vec<PackageRef> = request
        .roots
        .iter()
        .map(|cli_pkgref| {
            let resolved = fetched
                .iter()
                .find(|f| {
                    Some(&f.cached.resolved.group) == cli_pkgref.group.as_ref()
                        && f.cached.resolved.name == cli_pkgref.name
                })
                .map(|f| &f.cached.resolved.version)
                .ok_or_else(|| Error::RootNotFetched {
                    pkgref: cli_pkgref.to_string(),
                })?;
            Ok(finalize_pkgref_for_manifest(
                cli_pkgref,
                resolved,
                request.exact,
            ))
        })
        .collect::<Result<_>>()?;
    let manifest_changed = if finalized_roots.is_empty() {
        false
    } else {
        merge_manifest_requires(&mut manifest, &finalized_roots)
    };
    if manifest_changed {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }

    // 7. Re-discover the workspace so the boot computation reads the
    //    just-updated `[requires]` from disk.
    let workspace = Workspace::discover(&project_root)?;

    // 8. Apply: materialise each package into vibedeps/, run each freshly
    //    populated slot's pre-install hook (PROP-020 §2.1), and regenerate
    //    every node's boot artifacts.
    let outcome = apply_resolution(&workspace, &resolution, slot_integrity, Some(hooks))?;

    // 9. Rebuild the lockfile from the fresh resolution — an install
    //    re-resolves the whole graph, so the recorded package set is
    //    replaced wholesale.
    let resolved_language: Option<String> = language_chain.first().cloned().filter(|l| l != "en");
    lockfile.packages.clear();
    for f in &fetched {
        lockfile
            .packages
            .push(locked_package_from_fetched(f, resolved_language.as_deref()));
    }
    lockfile.meta.generated_at = vibe_core::timestamp::now_utc();
    if !language_chain.is_empty() && language_chain != ["en"] {
        lockfile.meta.language_chain = language_chain.clone();
    }
    let mut active_features_global: BTreeSet<String> = BTreeSet::new();
    for f in &fetched {
        let pkg_label = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
        for feat in &f.feature_expansion.active_features {
            active_features_global.insert(format!("{pkg_label}/{feat}"));
        }
    }
    lockfile.meta.active_features = active_features_global.into_iter().collect();

    // 10. Mirror the declared roots into `meta.root_dependencies` so
    //     the lockfile stays a self-contained snapshot (PROP-002 §2.7).
    //     Install-from-manifest records the workspace-derived roots;
    //     an explicit install records the finalized request roots.
    let lock_roots: &[PackageRef] = if finalized_roots.is_empty() {
        &roots
    } else {
        &finalized_roots
    };
    merge_root_dependencies(&mut lockfile, lock_roots);

    lockfile.write(workspace.lockfile_path())?;

    // 11. PROP-020 §2.1 — post-install hooks run once the package is durable
    //     (lockfile written, boot regenerated). A non-zero exit is surfaced
    //     as a flagged report, not fatal; a missing interpreter is a hard
    //     error. Only the freshly-materialised slots run their hook.
    let post_install_reports =
        run_post_install_hooks(&workspace.root, &resolution, &outcome.materialised, hooks)?;

    Ok(ApplyReport {
        outcome,
        post_install_reports,
    })
}
