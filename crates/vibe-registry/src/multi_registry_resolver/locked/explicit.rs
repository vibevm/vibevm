//! Explicit install roots moving a held version at their locked source.
//! Split from `locked.rs` for the file-length budget; same owner.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW");
use super::*;

/// Whether an explicit version move can stay on its held source.
/// ```
/// let a = vibe_registry::LockedUpdateAvailability { source: "registry `r`".into(), available: false };
/// assert!(!a.available);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedUpdateAvailability {
    /// Display of the locked source (registry name and redacted URL).
    pub source: String,
    /// The locked source serves at least one version the request admits.
    pub available: bool,
}
impl MultiRegistryResolver {
    /// Explicit install roots (`vibe install <pkg>@<req>`) whose held version
    /// may move at the locked source; every other held coordinate keeps
    /// strict replay and no source is ever reselected (PROP-073
    /// `#LOCKED-SOURCE-LAW`).
    /// ```
    /// # fn install(r: vibe_registry::MultiRegistryResolver) {
    /// let _r = r.with_locked_updates_for(["org.example/demo".to_string()]);
    /// # }
    /// ```
    pub fn with_locked_updates_for(mut self, keys: impl IntoIterator<Item = String>) -> Self {
        self.candidate_owner = Arc::new(());
        self.locked_update_keys.extend(keys);
        self
    }
    pub(in crate::multi_registry_resolver) fn updates_locked(&self, key: &str) -> bool {
        self.locked_updates || self.locked_update_keys.contains(key)
    }
    /// For a held coordinate this operation may move at its locked source:
    /// the source it stays on and whether that source serves a version the
    /// request admits. `None` for replayed or unheld coordinates.
    /// ```
    /// # fn inspect(r: &vibe_registry::MultiRegistryResolver, p: &vibe_core::PackageRef) {
    /// let _ = r.locked_update_availability(p);
    /// # }
    /// ```
    pub fn locked_update_availability(
        &self,
        pkg: &PackageRef,
    ) -> Result<Option<LockedUpdateAvailability>, RegistryError> {
        let key = pkg.qualified_name();
        let Some(row) = self.locked.get(&key).filter(|_| self.updates_locked(&key)) else {
            return Ok(None);
        };
        let available = self
            .locked_versions(row)?
            .iter()
            .any(|version| version_matches(&pkg.version, version));
        let source = vibe_core::progress::transfer::safe_remote(row.source_url.as_str());
        Ok(Some(LockedUpdateAvailability {
            source: match &row.registry {
                Some(registry) => format!("registry `{registry}` ({source})"),
                None => source,
            },
            available,
        }))
    }
}
