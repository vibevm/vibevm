//! Steps 9 and 10 of the apply: rebuild `vibe.lock` from the fresh
//! resolution and write it, with the fetch provenance beside it. The last
//! fallible step before the install is durable, so the caller undoes the
//! slot changes when it fails.

use super::*;
use crate::record::locked_package_from_fetched;
use vibe_core::manifest::Lockfile;

pub(super) struct LockInputs<'a> {
    pub(super) fetched: &'a [Fetched],
    pub(super) resolution: &'a [ResolvedDep],
    pub(super) language_chain: &'a [String],
    pub(super) roots: &'a [PackageRef],
    pub(super) lockfile_path: &'a std::path::Path,
}

pub(super) fn record_lock(
    lockfile: &mut Lockfile,
    inputs: LockInputs<'_>,
    files: &mut MetadataFiles,
) -> Result<()> {
    let LockInputs {
        fetched,
        resolution,
        language_chain,
        roots,
        lockfile_path,
    } = inputs;
    // 9. Rebuild the lockfile from the fresh resolution — an install
    //    re-resolves the whole graph, so the recorded package set is
    //    replaced wholesale.
    let resolved_language: Option<String> = language_chain.first().cloned().filter(|l| l != "en");
    lockfile.packages.clear();
    lockfile.configurations.clear();
    for f in fetched {
        let resolved = resolution
            .iter()
            .find(|candidate| {
                candidate.group == f.cached.resolved.group
                    && candidate.name == f.cached.resolved.name
            })
            .ok_or_else(|| {
                Error::Solve(
                    DepProviderError::Other(format!(
                        "resolved package `{}/{}` has no matching visibility metadata \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-050#resolver; \
                          fix: report the mismatched Fetched/ResolvedDep sets in vibe-install)",
                        f.cached.resolved.group, f.cached.resolved.name
                    ))
                    .into(),
                )
            })?;
        if let Some(configuration) = &f.configuration {
            lockfile.configurations.insert(
                format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name),
                configuration.clone(),
            );
        }
        lockfile.packages.push(locked_package_from_fetched(
            f,
            resolved,
            resolved_language.as_deref(),
        ));
    }
    record_app_requirements(lockfile, fetched);
    lockfile.meta.generated_at = vibe_core::timestamp::now_utc();
    if !language_chain.is_empty() && language_chain != ["en"] {
        lockfile.meta.language_chain = language_chain.to_vec();
    }
    let mut active_features_global: BTreeSet<String> = BTreeSet::new();
    for f in fetched {
        let pkg_label = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
        for feat in &f.feature_expansion.active_features {
            active_features_global.insert(format!("{pkg_label}/{feat}"));
        }
    }
    lockfile.meta.active_features = active_features_global.into_iter().collect();

    // Both entry shapes record the complete post-edit compatibility snapshot.
    // Its unique rows are not the per-consumer constraints used by the solver.
    replace_root_dependencies(lockfile, roots);

    files.check_lock_path(lockfile_path)?;
    let lock_text = lockfile.render_with_comments(files.lock.previous_text()?)?;
    let provenance = crate::record::fetch_provenance(fetched)
        .render_with_comments(files.provenance.previous_text()?)?;
    files.lock.publish(Some(&lock_text))?;
    files.provenance.publish(provenance.as_deref())?;
    Ok(())
}
