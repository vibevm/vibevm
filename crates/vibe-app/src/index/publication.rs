//! Checked index publication under the actual application-home lock.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#proxy-index-upgrade");
use super::*;
use crate::MutationLock;
impl Index {
    /// Atomically save the inventory while taking this home's mutation lock.
    /// Callers already holding the lock use [`Self::save_locked`] instead.
    /// First index4 publication with existing managed readers requires the
    /// application operation API's compatible-image admission.
    pub fn save(
        &self,
        settings: &SettingsRoot,
        event: &crate::HomeEvent,
    ) -> Result<(), StateError> {
        let lock = settings.lock()?;
        self.save_locked(settings, event, &lock)
    }

    /// Save inside an existing atomic read-modify-write operation. The lock
    /// must belong to the actual selected home; aliases of that home agree.
    /// This raw writer creates no proxy copies and refuses an unsupported
    /// first3-to4 transition rather than stranding existing commands.
    ///
    /// ```
    /// let temp = tempfile::tempdir().unwrap();
    /// let home = vibe_app::SettingsRoot::new(temp.path());
    /// let lock = home.lock().unwrap();
    /// let index = vibe_app::Index::load(&home).unwrap();
    /// index.save_locked(&home, &vibe_app::HomeEvent::new(vibe_app::EventKind::Updated), &lock).unwrap();
    /// ```
    pub fn save_locked(
        &self,
        settings: &SettingsRoot,
        event: &crate::HomeEvent,
        lock: &MutationLock,
    ) -> Result<(), StateError> {
        self.publish_locked(settings, event, lock, None)
    }

    pub(crate) fn publish_locked(
        &self,
        settings: &SettingsRoot,
        event: &crate::HomeEvent,
        lock: &MutationLock,
        image: Option<&crate::proxies::CapturedImage>,
    ) -> Result<(), StateError> {
        lock.require_home(settings)?;
        let path = settings.index_path();
        for record in self.installations.values() {
            let bad = |reason| StateError::Index {
                path: path.display().to_string(),
                reason,
            };
            crate::configuration::record::validate_origins(&record.secret_origins).map_err(bad)?;
            if record.configuration.is_some() && !record.secret_origins.is_empty() {
                return Err(bad("two owners for secret parameter metadata".into()));
            }
            if let Some(configuration) = &record.configuration {
                crate::configuration::record::validate(configuration).map_err(bad)?;
            }
        }
        let mut bytes =
            serde_json::to_vec_pretty(&self.to_wire()).map_err(|error| StateError::Index {
                path: path.display().to_string(),
                reason: error.to_string(),
            })?;
        bytes.push(b'\n');
        crate::proxies::before_publication(settings, self, image)?;
        write_atomic(&path, &bytes)?;
        // Rewritten from these records on every publication, so an
        // uninstalled key stops being declared without a second mechanism
        // (`PROP-075#VERSION-FILE-FORMATS`).
        self.declared_version_files().save(settings)?;
        crate::catalog::bump_revision(settings, event).map(|_| ())
    }
}
