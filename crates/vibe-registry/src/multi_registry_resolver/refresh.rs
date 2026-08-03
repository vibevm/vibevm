//! Lockfile-driven clone refresh — `vibe registry sync`'s walk over
//! locked entries, refreshing registry-served clones via the named
//! `[[registry]]` and override clones via the `__overrides__`
//! subtree, with per-entry skip reporting.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
    /// Walk every entry in `lockfile` and refresh its on-disk clone
    /// (registry-served entries via the appropriate `[[registry]]`,
    /// override-resolved entries via the `__overrides__` subtree).
    /// Called by `vibe registry sync`.
    ///
    /// Entries with `registry: None` and `overridden: false` (legacy
    /// content fetched through pre-PROP-002 paths, or `LocalRegistry`
    /// installs) are reported as skipped — there is nothing per-package
    /// to refresh for them.
    ///
    /// Errors short-circuit: a partial refresh can still leave
    /// already-refreshed clones up-to-date, but we surface the first
    /// failure rather than silently swallowing.
    pub fn refresh_lockfile_clones(
        &self,
        lockfile: &Lockfile,
    ) -> Result<RefreshReport, RegistryError> {
        let mut report = RefreshReport::default();
        for entry in &lockfile.packages {
            if entry.overridden {
                self.refresh_override_entry(entry, &mut report)?;
            } else if let Some(registry_name) = entry.registry.as_deref() {
                self.refresh_registry_entry(entry, registry_name, &mut report)?;
            } else {
                report.skipped.push(SkippedEntry {
                    group: entry.group.clone(),
                    name: entry.name.to_string(),
                    reason: "lockfile entry has neither `registry` nor `overridden = true` \
                             (likely installed via `--registry <path>` or a legacy v1 path)"
                        .to_string(),
                });
            }
        }
        Ok(report)
    }

    fn refresh_registry_entry(
        &self,
        entry: &vibe_core::manifest::LockedPackage,
        registry_name: &str,
        report: &mut RefreshReport,
    ) -> Result<(), RegistryError> {
        let Some(reg) = self.registries.iter().find(|r| r.name() == registry_name) else {
            report.skipped.push(SkippedEntry {
                group: entry.group.clone(),
                name: entry.name.to_string(),
                reason: format!(
                    "lockfile names registry `{registry_name}` but no `[[registry]]` with that \
                     name exists in `vibe.toml` — drop the lockfile entry or restore the registry"
                ),
            });
            return Ok(());
        };
        // Use the recorded source_ref if present (typically `v<version>`);
        // fall back to the registry's own ref otherwise.
        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| format!("v{}", entry.version));
        reg.refresh_package(&entry.group, entry.name.as_str(), &refname)?;
        report.refreshed.push(RefreshedEntry {
            group: entry.group.clone(),
            name: entry.name.to_string(),
            via: RefreshedVia::Registry(registry_name.to_string()),
            refname,
        });
        Ok(())
    }

    fn refresh_override_entry(
        &self,
        entry: &vibe_core::manifest::LockedPackage,
        report: &mut RefreshReport,
    ) -> Result<(), RegistryError> {
        let url = entry.source_url.clone();
        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let clone_dir = self.override_clone_dir(&entry.group, entry.name.as_str());
        ensure_clone_at(self.backend.as_ref(), &url, &refname, &clone_dir)?;
        report.refreshed.push(RefreshedEntry {
            group: entry.group.clone(),
            name: entry.name.to_string(),
            via: RefreshedVia::Override,
            refname,
        });
        Ok(())
    }
}

/// Per-entry outcome of [`MultiRegistryResolver::refresh_lockfile_clones`].
#[derive(Debug, Clone, Default)]
pub struct RefreshReport {
    pub refreshed: Vec<RefreshedEntry>,
    pub skipped: Vec<SkippedEntry>,
}

#[derive(Debug, Clone)]
pub struct RefreshedEntry {
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity of
    /// the refreshed lockfile entry (PROP-008).
    pub group: Group,
    pub name: String,
    pub via: RefreshedVia,
    pub refname: String,
}

#[derive(Debug, Clone)]
pub enum RefreshedVia {
    Registry(String),
    Override,
}

#[derive(Debug, Clone)]
pub struct SkippedEntry {
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity of
    /// the skipped lockfile entry (PROP-008).
    pub group: Group,
    pub name: String,
    pub reason: String,
}
