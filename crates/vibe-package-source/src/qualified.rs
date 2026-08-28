//! The qualification split — PROP-008 §2.6 short-name resolution stays a
//! SURFACE capability (the CLI's own input boundary, with its exit-7
//! ambiguity refusal), injected here as a [`PackageQualifier`] and composed
//! into [`RegistryPackageSource`], the one value that is simultaneously the
//! install substrate's [`InstallSource`] and the orchestrator's
//! [`PackageSource`](vibe_orchestrator::ports::PackageSource).
//!
//! Why the split exists: `vibe install wal` resolves the bare name against
//! the lockfile and the registries, and a collision is the CLI's typed
//! `InstallError::AmbiguousPackage` (exit 7, read by downcast in the exit
//! mapper). Moving that logic down here would drag a surface's exit codes
//! below the boundary; leaving the impl up would orphan the trait once
//! `InstallResolver` left the CLI. So the resolver is shared and the
//! QUALIFIER is injected, and the CLI's own qualifier adds no context on
//! purpose: the refusal keeps its historical top-level wording, and no
//! second surface message is invented beside it. The typed exit-7 identity
//! itself survives an ordinary context wrapper (the exit mapper downcasts
//! through the chain) — what would destroy it is replacing or translating
//! the typed error into some other error.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use vibe_core::PackageRef;
use vibe_core::manifest::Lockfile;
use vibe_install::InstallSource;
use vibe_registry::{CachedPackage, RegistryError};

use crate::source::InstallResolver;

/// Surface-injected short-name qualification over the shared resolver.
///
/// The CLI implements this by calling its own PROP-008 §2.6 boundary with NO
/// added context, so the ambiguity refusal surfaces exactly as it always
/// has. A hosted surface injects [`RefusesQualification`] — the accepted
/// hosted grammar admits no package inputs at all, so
/// `InstallInputs::packages` is empty and the qualifier is provably
/// unreachable.
///
/// ```
/// use vibe_core::PackageRef;
/// use vibe_core::manifest::Lockfile;
/// use vibe_package_source::{InstallResolver, PackageQualifier};
///
/// struct SurfaceQualifier;
///
/// impl PackageQualifier for SurfaceQualifier {
///     fn qualify(
///         &self,
///         resolver: &InstallResolver,
///         pkgref: &PackageRef,
///         locked: &Lockfile,
///     ) -> anyhow::Result<PackageRef> {
///         // A qualified ref passes through untouched — the minimal honest
///         // implementation.
///         if pkgref.is_qualified() {
///             return Ok(pkgref.clone());
///         }
///         let candidates = resolver.candidate_groups(&pkgref.name)?;
///         match candidates.as_slice() {
///             [only] => Ok(PackageRef { group: Some(only.clone()), ..pkgref.clone() }),
///             _ => anyhow::bail!("the surface refuses an ambiguous short name"),
///         }
///     }
/// }
/// ```
pub trait PackageQualifier: Send + Sync {
    /// Qualify one surface-supplied pkgref against the resolver and the
    /// lockfile. An already-qualified reference should pass through
    /// untouched; an ambiguous short name is the surface's own typed
    /// refusal, returned unchanged so its historical wording and
    /// presentation survive — an ordinary context wrapper would not hide
    /// the typed error from a chain-walking downcast, but replacing or
    /// translating it would.
    fn qualify(
        &self,
        resolver: &InstallResolver,
        pkgref: &PackageRef,
        locked: &Lockfile,
    ) -> Result<PackageRef>;
}

/// The named hosted canary: a surface whose grammar admits no package
/// inputs injects this, so ANY qualification call — a bare short name or an
/// already-qualified ref — is a loud, typed refusal rather than a silent
/// network walk. It refuses every call; it never passes a qualified ref
/// through.
///
/// Unreachable by construction in the hosted posture — the accepted hosted
/// grammar has no package inputs at all, so `InstallInputs::packages` is
/// empty and the orchestrator's one qualification call site never fires —
/// which is exactly why it is named: if that construction ever changes,
/// this errors instead of guessing.
///
/// ```
/// use vibe_package_source::RefusesQualification;
/// fn takes(_: &RefusesQualification) {}
/// ```
pub struct RefusesQualification;

impl PackageQualifier for RefusesQualification {
    fn qualify(
        &self,
        _resolver: &InstallResolver,
        pkgref: &PackageRef,
        _locked: &Lockfile,
    ) -> Result<PackageRef> {
        bail!(
            "internal: the hosted surface never qualifies a package reference — its \
             grammar admits no package inputs (`InstallInputs::packages` is \
             empty), yet `{}` reached qualification",
            pkgref.name,
        )
    }
}

/// The composed package source one install run owns: the shared
/// [`InstallResolver`] delegating every [`InstallSource`] method, plus the
/// surface-injected [`PackageQualifier`] behind the orchestrator's
/// `ports::PackageSource` capability.
///
/// Building one is the surface's composition step: resolve via
/// [`build_install_resolver`](crate::build_install_resolver), then wrap with
/// the surface's qualifier.
///
/// ```
/// use vibe_package_source::{PackageQualifier, RegistryPackageSource};
/// fn takes(qualifier: Box<dyn PackageQualifier>) -> Box<RegistryPackageSource> {
///     // The resolver half arrives from `build_install_resolver`; the
///     // qualifier half is this surface's own.
///     # let _ = &qualifier;
///     # unimplemented!()
/// }
/// # let _ = takes;
/// ```
pub struct RegistryPackageSource {
    resolver: InstallResolver,
    qualifier: Box<dyn PackageQualifier>,
}

impl RegistryPackageSource {
    /// Compose the shared resolver with the surface's qualifier.
    pub fn new(resolver: InstallResolver, qualifier: Box<dyn PackageQualifier>) -> Self {
        Self {
            resolver,
            qualifier,
        }
    }

    /// The shared resolver, for surface code that talks to the
    /// [`InstallSource`] seam directly (pre-warm, re-fetch, update).
    pub fn resolver(&self) -> &InstallResolver {
        &self.resolver
    }
}

impl InstallSource for RegistryPackageSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        self.resolver
            .resolve_and_fetch(pkgref, store_root, expected_hash)
    }

    fn solve(
        &self,
        roots: &[PackageRef],
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        self.resolver.solve(roots)
    }

    fn manifest_of(
        &self,
        pkg: &PackageRef,
    ) -> Result<vibe_core::manifest::Manifest, vibe_resolver::SolveError> {
        self.resolver.manifest_of(pkg)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<vibe_resolver::ResolvedGraph, vibe_resolver::SolveError> {
        self.resolver.solve_masked(roots, blocked)
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        self.resolver.materialise_in_place(pkgref, slot)
    }
}

impl vibe_orchestrator::ports::PackageSource for RegistryPackageSource {
    fn qualify(&self, pkgref: &PackageRef, locked: &Lockfile) -> Result<PackageRef> {
        // No context added: the refusal keeps its historical top-level
        // wording, and no second message is invented here. The typed exit-7
        // identity would survive an ordinary context wrapper (the surface's
        // exit mapper downcasts through the chain); replacing or translating
        // the typed error would destroy it.
        self.qualifier.qualify(&self.resolver, pkgref, locked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::Group;

    /// A minimal local registry serving one package under one group, so the
    /// delegation reds run against a resolver that really resolves.
    fn local_registry_dir(name: &str, group: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join(group).join(name).join("v0.1.0");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"tool\"\nversion = \"0.1.0\"\n"
            ),
        )
        .unwrap();
        dir
    }

    struct Recording(Group);

    impl PackageQualifier for Recording {
        fn qualify(
            &self,
            _resolver: &InstallResolver,
            pkgref: &PackageRef,
            _locked: &Lockfile,
        ) -> Result<PackageRef> {
            Ok(PackageRef {
                group: Some(self.0.clone()),
                ..pkgref.clone()
            })
        }
    }

    /// The orchestrator capability delegates to the INJECTED qualifier over
    /// the SAME resolver the substrate methods delegate to — neither half
    /// consults anything else.
    #[test]
    fn qualify_delegates_to_the_injected_qualifier() {
        let dir = local_registry_dir("wal", "org.vibevm");
        let resolver = crate::cells::local_registry(dir.path().to_path_buf()).unwrap();
        let resolver = InstallResolver::Local(resolver, None);
        let source = RegistryPackageSource::new(
            resolver,
            Box::new(Recording(Group::parse("org.vibevm").unwrap())),
        );
        let lockfile = Lockfile::empty("test", "2026-01-01");
        let bare = PackageRef::parse("wal").unwrap();
        let qualified =
            vibe_orchestrator::ports::PackageSource::qualify(&source, &bare, &lockfile).unwrap();
        assert_eq!(qualified.qualified_name(), "org.vibevm/wal");
    }

    /// The canary refuses, loudly and typed — never guesses a group.
    #[test]
    fn the_refuses_qualification_canary_errors() {
        let dir = local_registry_dir("wal", "org.vibevm");
        let resolver = crate::cells::local_registry(dir.path().to_path_buf()).unwrap();
        let source = RegistryPackageSource::new(
            InstallResolver::Local(resolver, None),
            Box::new(RefusesQualification),
        );
        let lockfile = Lockfile::empty("test", "2026-01-01");
        let bare = PackageRef::parse("wal").unwrap();
        let err = vibe_orchestrator::ports::PackageSource::qualify(&source, &bare, &lockfile)
            .unwrap_err();
        assert!(
            err.to_string().contains("never qualifies"),
            "the canary must name its own unreachability: {err}"
        );
    }

    /// The substrate seam delegates: a solve over the composed source is the
    /// resolver's own solve (empty roots → the empty graph, through the
    /// selected cells).
    #[test]
    fn the_install_source_seam_delegates_to_the_resolver() {
        let dir = local_registry_dir("wal", "org.vibevm");
        let resolver = crate::cells::local_registry(dir.path().to_path_buf()).unwrap();
        let source = RegistryPackageSource::new(
            InstallResolver::Local(resolver, None),
            Box::new(RefusesQualification),
        );
        let graph = source.solve(&[]).unwrap();
        assert!(graph.iter().next().is_none());
        let missing = PackageRef::parse("tool:nope@0.1.0").unwrap();
        assert!(
            source.manifest_of(&missing).is_err(),
            "manifest_of routes through the resolver's provider"
        );
    }

    /// `resolver()` hands back the shared half for a surface that talks to
    /// the seam directly.
    #[test]
    fn the_resolver_accessor_returns_the_shared_half() {
        let dir = local_registry_dir("wal", "org.vibevm");
        let resolver = crate::cells::local_registry(dir.path().to_path_buf()).unwrap();
        let source = RegistryPackageSource::new(
            InstallResolver::Local(resolver, None),
            Box::new(RefusesQualification),
        );
        let groups = source.resolver().candidate_groups("wal").unwrap();
        assert_eq!(groups.len(), 1);
    }
}
