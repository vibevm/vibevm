//! `[[override]]` resolution mechanics — resolving a pkgref pinned to
//! a specific URL/ref by the manifest, plus the override manifest
//! reader. Split from `walk.rs` along the override seam when the
//! store-backed resolution work pushed the combined file past the
//! 600-line budget; `walk.rs` keeps the decision TREE and calls in
//! here for the override leg.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
    /// Resolve a pkgref through its `[[override]]` entry: read the
    /// manifest at the pinned URL/ref, verify the identity matches the
    /// override's `(group, name)`, and hand back the resolution with
    /// `overridden = true`.
    pub(super) fn resolve_override(
        &self,
        pkgref: &PackageRef,
        ovr: &OverrideSection,
    ) -> Result<MultiResolution, RegistryError> {
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        let refname = ovr
            .r#ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let manifest = self.read_override_manifest(&ovr.source_url, &refname)?;
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:vibe.toml", ovr.source_url, refname)),
                reason: e.to_string(),
            })?;
        // Sanity: the override is supposed to point at *this* package. If
        // the manifest at the pinned ref names a different `(group, name)`
        // identity, installing it would silently misroute on disk. Refuse
        // loudly. `kind` is metadata (PROP-008 §2.3) — not compared here.
        if &meta.group != group || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:vibe.toml", ovr.source_url, refname)),
                reason: format!(
                    "override for `{}/{}` points at a manifest declaring `{}/{}` — refusing to install",
                    group, pkgref.name, meta.group, meta.name
                ),
            });
        }
        let resolved = ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.to_string(),
            version: meta.version.clone(),
            source_dir: self.override_clone_dir(group, pkgref.name.as_str()),
        };
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            source_url: ovr.source_url.clone(),
            source_ref: Some(refname),
            overridden: true,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
            from_store: false,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Read `vibe.toml` at the override's pinned URL/ref through the
    /// git backend's archive fetch.
    pub(super) fn read_override_manifest(
        &self,
        url: &str,
        refname: &str,
    ) -> Result<Manifest, RegistryError> {
        let bytes = self.backend.fetch_file_at_ref(
            strip_git_plus_prefix(url),
            refname,
            Manifest::FILENAME,
        )?;
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{refname}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{refname}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
    }
}
