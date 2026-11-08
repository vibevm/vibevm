//! Preserve project-recorded history without publishing developer-local routes.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#provenance");
use super::*;
use crate::error::Result;

fn same_selection(a: &LockedPackage, b: &LockedPackage) -> bool {
    a.group == b.group
        && a.name == b.name
        && a.version == b.version
        && a.registry == b.registry
        && a.source_url == b.source_url
        && a.source_ref == b.source_ref
        && a.resolved_commit == b.resolved_commit
        && a.content_hash == b.content_hash
        && a.source_kind == b.source_kind
        && a.overridden == b.overridden
        && a.via_redirect == b.via_redirect
}
fn validate_observations(cached: &[&vibe_registry::CachedPackage], after: &Lockfile) -> Result<()> {
    for cached in cached {
        let invalid = || vibe_core::Error::InvalidLockfile {
            reason: format!(
                "cached observation for {}/{} does not match the planned lock identity",
                cached.resolved.group, cached.resolved.name
            ),
        };
        let selected = after
            .find(&cached.resolved.group, &cached.resolved.name)
            .ok_or_else(invalid)?;
        let held = cached.locked_selection.as_ref();
        let source = held.map(|selection| selection.source_row());
        let commit = held
            .and_then(|selection| selection.replayed_row())
            .map_or(cached.resolved_commit.as_deref(), |row| {
                row.resolved_commit.as_deref()
            });
        if selected.version != cached.resolved.version
            || selected.content_hash.as_str() != cached.content_hash
            || selected.source_url.as_str()
                != source.map_or(cached.source_uri.as_str(), |row| row.source_url.as_str())
            || selected.source_ref.as_deref()
                != source.map_or(cached.source_ref.as_deref(), |row| {
                    row.source_ref.as_deref()
                })
            || selected.registry.as_deref()
                != source.map_or(cached.registry_name.as_deref(), |row| {
                    row.registry.as_deref()
                })
            || selected.resolved_commit.as_deref() != commit
            || selected.source_kind != cached.source_kind()
            || selected.overridden != source.map_or(cached.overridden, |row| row.overridden)
            || selected.via_redirect.as_deref()
                != source.map_or(cached.via_redirect.as_deref(), |row| {
                    row.via_redirect.as_deref()
                })
            || !cached.manifest.package.as_ref().is_some_and(|meta| {
                meta.group == selected.group
                    && meta.name == selected.name
                    && meta.version == selected.version
                    && meta.kind == selected.kind
            })
        {
            return Err(invalid().into());
        }
    }
    Ok(())
}
pub(crate) fn merge_fetch_provenance(
    fetched: &[Fetched],
    before: &Lockfile,
    after: &Lockfile,
    previous: Option<&str>,
) -> Result<Option<String>> {
    validate_observations(
        &fetched.iter().map(|row| &row.cached).collect::<Vec<_>>(),
        after,
    )?;
    merge_records(fetch_provenance(fetched), before, after, previous)
}
pub(crate) fn merge_cached_provenance(
    cached: &[&vibe_registry::CachedPackage],
    before: &Lockfile,
    after: &Lockfile,
    previous: Option<&str>,
) -> Result<Option<String>> {
    validate_observations(cached, after)?;
    let observations = LockProvenance {
        packages: cached
            .iter()
            .filter(|cached| cached.route_publication == vibe_core::RoutePublication::Project)
            .filter_map(|cached| {
                Some(FetchedFrom {
                    package: format!("{}/{}", cached.resolved.group, cached.resolved.name),
                    version: cached.resolved.version.to_string(),
                    fetched_from: vibe_core::progress::transfer::safe_remote(
                        cached.fetched_from.as_deref()?,
                    ),
                    row: None,
                })
            })
            .collect(),
    };
    merge_records(observations, before, after, previous)
}
pub(crate) fn same_selection_set(a: &Lockfile, b: &Lockfile) -> bool {
    a.packages.len() == b.packages.len()
        && a.packages.iter().all(|row| {
            b.find(&row.group, row.name.as_str())
                .is_some_and(|other| same_selection(row, other))
        })
}
fn merge_records(
    observations: LockProvenance,
    before: &Lockfile,
    after: &Lockfile,
    previous: Option<&str>,
) -> Result<Option<String>> {
    let old = previous
        .map(LockProvenance::parse_str)
        .transpose()?
        .unwrap_or_default();
    let mut merged = LockProvenance {
        packages: old
            .packages
            .iter()
            .filter(|record| {
                after.packages.iter().any(|current| {
                    format!("{}/{}", current.group, current.name) == record.package
                        && current.version.to_string() == record.version
                        && before
                            .find(&current.group, current.name.as_str())
                            .is_some_and(|old| same_selection(old, current))
                })
            })
            .cloned()
            .collect(),
    };
    for observation in observations.packages {
        if merged.packages.iter().any(|old| {
            old.package == observation.package
                && old.version == observation.version
                && old.fetched_from == observation.fetched_from
        }) {
            continue;
        }
        merged
            .packages
            .retain(|old| old.package != observation.package);
        merged.packages.push(observation);
    }
    // Nothing left to record removes the sidecar, even when the stale file
    // already parsed as empty (e.g. comments only); never republish it.
    if merged.is_empty() {
        return Ok(None);
    }
    if merged == old {
        return Ok(previous.map(str::to_owned));
    }
    Ok(merged.render_with_comments(previous)?)
}
#[cfg(test)]
mod tests;
