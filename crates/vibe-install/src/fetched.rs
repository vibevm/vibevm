//! Fetch-side support: the per-node carrier the plan accumulates, the
//! PROP-003 language chain and per-root feature tailoring, and the
//! activation context the conditional-dependency fixpoint evaluates
//! against.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
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
    /// `true` iff this is a re-resolve of an already-present `in-place`
    /// package (PROP-022 §2.4) that the plan deferred rather than re-cloned:
    /// `cached` was built from the existing slot (manifest read locally,
    /// network-free), and [`apply`](crate::apply) performs the incremental
    /// `git fetch` against the live `.git` post-confirmation. `false` for a
    /// normally-fetched package — including a *fresh* in-place install, which
    /// still clones once and moves the clone into the slot.
    pub in_place_incremental: bool,
}

/// Load the workspace lockfile, or an empty one stamped with the
/// caller's identity when none exists yet.
pub(crate) fn load_or_empty_lockfile(root: &Path, generated_by: &str) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            generated_by.to_string(),
            vibe_core::timestamp::now_utc(),
        ))
    }
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

/// Build the [`ActivationContext`] from the set of fetched packages
/// plus project state. Walks every package's manifest once to populate
/// `present`, `provides`, and `describes_types`. Sets `project_root`
/// for `if_files` glob matching and `language_chain` for
/// `if_language`.
pub(crate) fn build_activation_context<'a, I>(
    cached: I,
    project_root: &Path,
    language_chain: &[String],
) -> Result<ActivationContext>
where
    I: IntoIterator<Item = &'a CachedPackage>,
{
    let mut ctx = ActivationContext {
        project_root: Some(project_root.to_path_buf()),
        language_chain: language_chain.to_vec(),
        ..Default::default()
    };
    for c in cached {
        // The conditional-dep `context(<key>)` predicate matches an
        // opaque present-set token; for a package the token is the
        // `<kind>:<name>` tag (PROP-003 §2.6.1), consistent with the
        // `<type>:<name>` shape of capability / interface tags. This is
        // not a package label — identity remains `(group, name)`.
        // Both shapes are `:`-qualified by construction, so the parse
        // can only fail on a malformed manifest that slipped past
        // validation — which deserves the loud exit, not a silent skip.
        let kind_tag =
            CapabilityTag::parse(format!("{}:{}", c.package_meta().kind, c.resolved.name))
                .map_err(|source| Error::CapabilityTag {
                    package: c.resolved.name.clone(),
                    source,
                })?;
        ctx.add_present(kind_tag);
        for cap in &c.manifest.provides.capabilities {
            let qualified =
                CapabilityTag::parse(cap.qualified()).map_err(|source| Error::CapabilityTag {
                    package: c.resolved.name.clone(),
                    source,
                })?;
            let is_interface = qualified.as_str().starts_with("interface:");
            ctx.add_present(qualified.clone());
            if is_interface {
                ctx.add_provides(qualified);
            }
        }
        if let Some(purl) = &c.package_meta().describes {
            ctx.describes_types.insert(purl.purl_type.clone());
        }
    }
    Ok(ctx)
}
