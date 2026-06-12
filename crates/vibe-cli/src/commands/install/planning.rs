//! Plan-side helpers for `vibe install` — project / lockfile loading,
//! pinned-pkgref construction, the PROP-003 language chain and feature
//! request, and the conditional-dep activation context.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{PackageRef, VersionSpec};
use vibe_registry::CachedPackage;
use vibe_resolver::{
    ActivationContext, CapabilityTag, FeatureExpansion, FeatureRequest, ResolvedNode,
};

use crate::cli::InstallArgs;

/// Per-node install metadata threaded from the solver into the lockfile
/// register call.
pub(super) struct NodeInstallMeta {
    pub(super) dependencies: Vec<PackageRef>,
    pub(super) is_root: bool,
}

/// One resolved + fetched package, with the feature expansion and the
/// per-node metadata gathered alongside it during resolution.
pub(super) struct Fetched {
    pub(super) cached: CachedPackage,
    pub(super) feature_expansion: FeatureExpansion,
    pub(super) meta: NodeInstallMeta,
}

/// Build a `<group>/<name>@=<exact-version>` pkgref for fetching the
/// version the solver chose, regardless of how the user originally
/// constrained the package.
pub(crate) fn exact_pinned_pkgref(node: &ResolvedNode) -> PackageRef {
    let req = semver::VersionReq::parse(&format!("={}", node.version))
        .expect("exact version always parses as VersionReq");
    PackageRef {
        kind: None,
        group: Some(node.group.clone()),
        name: node.name.clone(),
        version: VersionSpec::Req(req),
    }
}

pub(super) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

pub(super) fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

pub(super) fn load_or_empty_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        ))
    }
}

/// Build the resolved language chain from a CLI flag override + the
/// project's `[i18n]` declaration. The CLI flag is the head of the
/// chain; project-level preference and fallback come next; canonical /
/// registry-default `en` close the chain.
pub(super) fn build_language_chain(cli_language: Option<&str>, manifest: &Manifest) -> Vec<String> {
    let mut effective = manifest.i18n.clone();
    if let Some(lang) = cli_language {
        effective.preferred = Some(lang.to_string());
    }
    if effective.is_default() && cli_language.is_none() {
        Vec::new()
    } else {
        effective.project_preference_chain()
    }
}

/// Build the feature request to apply to root packages from the CLI
/// flags. `--all-features` wins over `--features` if both are set.
pub(super) fn build_feature_request(args: &InstallArgs) -> FeatureRequest {
    FeatureRequest {
        explicit: args.features.clone(),
        no_defaults: args.no_default_features,
        all: args.all_features,
    }
}

/// Per-root-package tailoring: trim `explicit` features down to those
/// the package actually declares. A multi-root `vibe install A B
/// --features X` should not fail just because X belongs to A and not B
/// — silently filter X out of B's request and rely on the post-phase-1
/// visibility check to surface a warning if X matched no root at all.
pub(super) fn tailor_feature_request(
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
/// for `if_files` glob matching and `language_chain` for `if_language`.
pub(super) fn build_activation_context<'a, I>(
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
                .with_context(|| format!("package tag for `{}`", c.resolved.name))?;
        ctx.add_present(kind_tag);
        for cap in &c.manifest.provides.capabilities {
            let qualified = CapabilityTag::parse(cap.qualified())
                .with_context(|| format!("capability tag of `{}`", c.resolved.name))?;
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
