//! Fetch-side support: the per-node carrier the plan accumulates, the
//! PROP-003 language chain and per-root feature tailoring, and the
//! activation context the conditional-dependency fixpoint evaluates
//! against.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use vibe_core::PackageRef;
use vibe_core::manifest::{LockedEmbeddedSource, Lockfile, Manifest};
use vibe_registry::CachedPackage;
use vibe_resolver::{ActivationContext, CapabilityTag, FeatureExpansion, FeatureRequest};

use crate::error::{Error, Result};

/// Per-node install metadata threaded from the solver into the
/// lockfile register call.
#[derive(Debug, Clone)]
pub struct NodeInstallMeta {
    /// The node's resolved dependency pkgrefs, exactly as the solver
    /// pinned them.
    pub dependencies: Vec<PackageRef>,
    /// Whether the node was a root of the resolution.
    pub is_root: bool,
}

/// One resolved + fetched package, with the feature expansion and the
/// per-node metadata gathered alongside it during resolution.
#[derive(Debug)]
pub struct Fetched {
    pub cached: CachedPackage,
    pub feature_expansion: FeatureExpansion,
    pub meta: NodeInstallMeta,
    /// External source pins authenticated during the read-mostly fetch phase.
    /// Apply records these exact inputs without making a second network
    /// decision after dependency-slot mutation begins.
    pub embedded_sources: Vec<LockedEmbeddedSource>,
    /// Exact prospective source retained until apply. None means the package
    /// was fetched normally; Some binds deferred checkout to admitted bytes.
    pub in_place: Option<vibe_registry::PreparedInPlace>,
    pub configuration: Option<vibe_core::manifest::PackageConfiguration>,
}

/// Load the workspace lockfile, or an empty one stamped with the
/// caller's identity when none exists yet.
///
/// Existing unsupported schemas refuse instead of silently losing their
/// source constraints. Only an absent lock begins an unlocked selection.
pub(crate) fn load_or_empty_lockfile(root: &Path, generated_by: &str) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        return Ok(Lockfile::read(&path)?);
    }
    Ok(Lockfile::empty(
        generated_by.to_string(),
        vibe_core::timestamp::now_utc(),
    ))
}

/// Build the resolved language chain from a caller override + the
/// project's `[i18n]` declaration. The override is the head of the
/// chain; project-level preference and fallback come next; canonical /
/// registry-default `en` close the chain (PROP-003 §2.7).
pub(crate) fn build_language_chain(language: Option<&str>, manifest: &Manifest) -> Vec<String> {
    let mut effective = manifest.i18n.clone();
    if let Some(lang) = language {
        effective.preferred = Some(lang.to_string());
    }
    if effective.is_default() && language.is_none() {
        Vec::new()
    } else {
        effective.project_preference_chain()
    }
}

/// Per-root-package tailoring: trim `explicit` features down to those
/// the package actually declares. A multi-root `vibe install A B
/// --features X` should not fail just because X belongs to A and not B
/// — silently filter X out of B's request and rely on the post-fetch
/// visibility check to surface a warning if X matched no root at all.
pub(crate) fn tailor_feature_request(
    request: &FeatureRequest,
    table: &vibe_core::manifest::FeaturesTable,
) -> FeatureRequest {
    FeatureRequest {
        explicit: request
            .explicit
            .iter()
            .filter(|f| table.features.contains_key(f.as_str()))
            .cloned()
            .collect(),
        no_defaults: request.no_defaults,
        all: request.all,
    }
}

/// The same conditional context before payload acquisition. Only admitted
/// package manifests supply capabilities; no cache directory is required.
pub(crate) fn build_activation_context_from_manifests<'a>(
    manifests: impl IntoIterator<Item = &'a Manifest>,
    project_root: &Path,
    language_chain: &[String],
) -> Result<ActivationContext> {
    let mut ctx = ActivationContext {
        project_root: Some(project_root.to_path_buf()),
        language_chain: language_chain.to_vec(),
        ..Default::default()
    };
    for manifest in manifests {
        let package = manifest.require_package()?;
        // The conditional-dep `context(<key>)` predicate matches an
        // opaque present-set token; for a package the token is the
        // `<kind>:<name>` tag (PROP-003 §2.6.1), consistent with the
        // `<type>:<name>` shape of capability / interface tags. This is
        // not a package label — identity remains `(group, name)`.
        // Both shapes are `:`-qualified by construction, so the parse
        // can only fail on a malformed manifest that slipped past
        // validation — which deserves the loud exit, not a silent skip.
        let kind_tag = CapabilityTag::parse(format!("{}:{}", package.kind, package.name)).map_err(
            |source| Error::CapabilityTag {
                package: package.name.to_string(),
                source,
            },
        )?;
        ctx.add_present(kind_tag);
        for cap in &manifest.provides.capabilities {
            let qualified =
                CapabilityTag::parse(cap.qualified()).map_err(|source| Error::CapabilityTag {
                    package: package.name.to_string(),
                    source,
                })?;
            let is_interface = qualified.as_str().starts_with("interface:");
            ctx.add_present(qualified.clone());
            if is_interface {
                ctx.add_provides(qualified);
            }
        }
        if let Some(purl) = &package.describes {
            ctx.describes_types.insert(purl.purl_type.clone());
        }
    }
    Ok(ctx)
}

#[cfg(test)]
mod lock_regeneration_tests {
    use super::load_or_empty_lockfile;

    /// An unsupported-schema lock must read as EMPTY on the install path —
    /// install is the regeneration verb and may never refuse to run because
    /// the artifact it is about to rewrite is outdated (PROP-050 W2 landing).
    #[test]
    fn unsupported_schema_lock_preserves_source_constraints_by_refusing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("vibe.lock"),
            "[meta]\ngenerated_by = \"vibe old\"\ngenerated_at = \"2026-01-01T00:00:00Z\"\nschema_version = 5\n",
        )
        .expect("write v5 lock");
        let before = std::fs::read(dir.path().join("vibe.lock")).expect("original lock");
        assert!(load_or_empty_lockfile(dir.path(), "vibe test").is_err());
        assert_eq!(
            before,
            std::fs::read(dir.path().join("vibe.lock")).expect("retained lock")
        );
    }
}
