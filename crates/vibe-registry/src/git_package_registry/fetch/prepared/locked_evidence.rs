//! Locked evidence is projected without retargeting the retained source objects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
impl PreparedInPlace {
    pub(crate) fn with_registry_evidence(
        mut self,
        resolution: &crate::MultiResolution,
    ) -> Result<Self, RegistryError> {
        let path = self.0.slot.clone();
        let owned = Arc::get_mut(&mut self.0)
            .ok_or_else(|| refusal(&path, "shared preparation cannot change registry evidence"))?;
        owned.canonical = resolution.source_url.clone();
        owned.registry_name = resolution.registry_name.clone().unwrap_or_default();
        owned.requested = resolution
            .source_ref
            .clone()
            .ok_or_else(|| refusal(&path, "retained registry has no ref"))?;
        owned.via_redirect = resolution.via_redirect.clone();
        Ok(self)
    }

    /// Report publication scope is independent of source and credential authority.
    pub fn publication_scope(&self) -> vibe_core::RoutePublication {
        self.0.route_publication
    }
    pub(crate) fn with_publication(
        mut self,
        scope: vibe_core::RoutePublication,
        selected: &str,
    ) -> Result<Self, RegistryError> {
        let path = self.0.slot.clone();
        let owned = Arc::get_mut(&mut self.0)
            .ok_or_else(|| refusal(&path, "shared preparation cannot change report scope"))?;
        let actual = owned.serving_endpoint.as_deref().unwrap_or(&owned.origin);
        owned.route_publication = scope.observed(selected, actual);
        Ok(self)
    }

    pub(crate) fn with_direct_evidence(
        mut self,
        canonical: &str,
        reference: &str,
        kind: vibe_core::manifest::SourceKind,
    ) -> Result<Self, RegistryError> {
        let path = self.0.slot.clone();
        let owned = Arc::get_mut(&mut self.0)
            .ok_or_else(|| refusal(&path, "shared preparation cannot change source evidence"))?;
        if owned.locked_selection.is_some() {
            return Err(refusal(
                &path,
                "held source cannot become a new direct source",
            ));
        }
        owned.canonical = canonical.into();
        owned.requested = reference.into();
        owned.registry_name.clear();
        owned.direct_kind = Some(kind);
        Ok(self)
    }

    pub(crate) fn with_locked_evidence(
        mut self,
        selection: &crate::multi_registry_resolver::LockedSelection,
    ) -> Result<Self, RegistryError> {
        let row = selection.source_row();
        let path = self.0.slot.clone();
        let owned = Arc::get_mut(&mut self.0)
            .ok_or_else(|| refusal(&path, "shared candidate cannot change evidence"))?;
        owned.registry_name = row.registry.clone().unwrap_or_default();
        owned.canonical = row.source_url.to_string();
        owned.requested = row
            .source_ref
            .clone()
            .unwrap_or_else(|| owned.revision.commit.clone());
        owned.via_redirect = row.via_redirect.clone();
        owned.locked_selection = Some(selection.clone());
        Ok(self)
    }
    /// Held source kind, when this candidate replays an authoritative lock row.
    /// ```
    /// # fn inspect(p: &vibe_registry::PreparedInPlace) {
    /// let _: Option<vibe_core::manifest::SourceKind> = p.recorded_source_kind();
    /// # }
    /// ```
    pub fn recorded_source_kind(&self) -> Option<vibe_core::manifest::SourceKind> {
        match &self.0.locked_selection {
            Some(held) => held.source_row().source_kind,
            None => self.0.direct_kind,
        }
    }
    /// Retained held authority, independent of this preparation's transport.
    /// ```
    /// # fn inspect(p: &vibe_registry::PreparedInPlace) {
    /// let _held = p.locked_selection();
    /// # }
    /// ```
    pub fn locked_selection(&self) -> Option<&crate::multi_registry_resolver::LockedSelection> {
        self.0.locked_selection.as_ref()
    }
}
