//! Exact held ref clarification and source-owner invalidation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
impl MultiRegistryResolver {
    pub(super) fn updates_explicit_ref(row: &LockedPackage) -> bool {
        matches!(
            row.source_kind,
            Some(SourceKind::Git | SourceKind::Override)
        ) || row.overridden
            || row.via_redirect.is_some()
    }
    /// Capture project-authored override clarification independently of the catalog.
    /// ```
    /// # fn clarify(r: vibe_registry::MultiRegistryResolver) {
    /// let _r=r.with_project_overrides(Vec::new());
    /// # }
    /// ```
    pub fn with_project_overrides(mut self, declarations: Vec<OverrideSection>) -> Self {
        self.candidate_owner = Arc::new(());
        self.project_overrides = declarations
            .into_iter()
            .map(|row| (row.pkgref.clone(), row))
            .collect();
        self
    }
    pub(super) fn held_update_reference(
        &self,
        row: &LockedPackage,
    ) -> Result<String, RegistryError> {
        let raw = row.source_ref.as_deref().ok_or_else(|| {
            refusal("direct update has no recorded ref; explicitly select a ref at the held URL")
        })?;
        let qualified = |value: &str| {
            value.starts_with("refs/heads/")
                || value.starts_with("refs/tags/")
                || (matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit()))
        };
        if qualified(raw) {
            return Ok(raw.into());
        }
        let key = format!("{}/{}", row.group, row.name);
        if let Some(dep) = self.git_packages.get(&key)
            && dep.url == row.source_url.as_str()
            && dep.ref_kind.as_str() == raw
        {
            use vibe_core::manifest::GitRefKind;
            return match &dep.ref_kind {
                GitRefKind::Branch(name) => Ok(format!("refs/heads/{name}")),
                GitRefKind::Tag(name) => Ok(format!("refs/tags/{name}")),
                GitRefKind::Rev(revision) if qualified(revision) => Ok(revision.clone()),
                _ => Err(refusal(
                    "revision clarification must use a qualified ref or exact object ID",
                )),
            };
        }
        if let Some(declaration) = self.project_overrides.get(&key)
            && declaration.source_url == row.source_url.as_str()
            && let Some(reference) = declaration.r#ref.as_deref()
            && qualified(reference)
            && (reference.strip_prefix("refs/heads/") == Some(raw)
                || reference.strip_prefix("refs/tags/") == Some(raw))
        {
            return Ok(reference.into());
        }
        Err(refusal(
            "legacy direct ref has no retained namespace: declare this same held URL and branch/tag in project [requires.packages], or its qualified ref in project [[override]], then run vibe update; ordinary replay remains pinned",
        ))
    }
}
