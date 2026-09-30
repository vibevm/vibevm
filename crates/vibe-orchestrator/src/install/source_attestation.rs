//! Internal one-package source wrapper for post-fetch byte attestation.
//!
//! The global cache may already have changed when `resolve_and_fetch`
//! returns. A mismatch is therefore an uncertain partial effect, never a
//! no-effect preflight refusal. No production submission route uses this yet.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_install::InstallSource;
use vibe_lifecycle::process::{
    PackageTreeWitness, attest_package_tree_copy, capture_selected_manifest_lock,
    recheck_package_tree_witness,
};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{DepProviderError, ResolvedGraph, SolveError};

use crate::ports::{PackageSource, PackageSourceBuild, PackageSourceFactory};

/// Borrows the composition owner's factory but returns an owned wrapper for
/// every source it builds. It does not assert that factory construction is
/// side-effect-free or local-only; that is a separate host admission.
pub(crate) struct AttestingFactory<'a> {
    inner: &'a dyn PackageSourceFactory,
    expected: PackageTreeWitness,
    manifest: Manifest,
}

impl<'a> AttestingFactory<'a> {
    pub(crate) fn new(
        inner: &'a dyn PackageSourceFactory,
        expected: &PackageTreeWitness,
    ) -> Result<Self> {
        expected.validate()?;
        recheck_package_tree_witness(expected)?;
        let observed = capture_selected_manifest_lock(&expected.root)?;
        ensure!(
            observed.lock.is_none(),
            "attested local package has an unexpected lock"
        );
        let row = expected
            .files
            .iter()
            .find(|file| file.relative_path == "vibe.toml")
            .context("source witness lacks package manifest")?;
        ensure!(
            row.bytes == observed.manifest.size && row.sha256 == observed.manifest.sha256,
            "attested source manifest differs from tree witness"
        );
        let manifest = Manifest::parse_str(std::str::from_utf8(&observed.manifest.bytes)?)?;
        ensure!(
            manifest.require_package()?.as_package_ref()? == expected.package_pin,
            "attested source manifest differs from exact pin"
        );
        Ok(Self {
            inner,
            expected: expected.clone(),
            manifest,
        })
    }
}

impl PackageSourceFactory for AttestingFactory<'_> {
    fn build(&self, input: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        ensure!(input.offline, "attested one-package source must be offline");
        recheck_package_tree_witness(&self.expected)?;
        Ok(Box::new(AttestingSource {
            inner: self.inner.build(input)?,
            expected: self.expected.clone(),
            manifest: self.manifest.clone(),
        }))
    }
}

struct AttestingSource {
    inner: Box<dyn PackageSource>,
    expected: PackageTreeWitness,
    manifest: Manifest,
}

impl AttestingSource {
    fn pin_matches(&self, pkgref: &PackageRef) -> bool {
        pkgref.group == self.expected.package_pin.group
            && pkgref.name == self.expected.package_pin.name
            && pkgref.version == self.expected.package_pin.version
            && pkgref
                .kind
                .is_none_or(|kind| Some(kind) == self.expected.package_pin.kind)
    }

    fn graph_matches(&self, roots: &[PackageRef], graph: &ResolvedGraph) -> bool {
        roots.len() == 1
            && self.pin_matches(&roots[0])
            && graph.packages.len() == 1
            && graph.packages[0].group == self.expected.package_pin.group.as_ref().unwrap().clone()
            && graph.packages[0].name == self.expected.package_pin.name.as_str()
            && self
                .expected
                .package_pin
                .version
                .matches(&graph.packages[0].version)
            && graph.packages[0].is_root
            && graph.packages[0].dependencies.is_empty()
    }

    fn attest_cached(&self, cached: &CachedPackage, store_root: &Path) -> Result<()> {
        let package = self.manifest.require_package()?;
        ensure!(
            cached.resolved.group == package.group
                && cached.resolved.name == package.name
                && cached.resolved.version == package.version
                && cached.manifest == self.manifest
                && cached.resolved.source_dir == self.expected.root
                && !cached.is_git_source
                && !cached.is_path_source
                && !cached.is_embedded
                && !cached.overridden,
            "fetched package identity, source or manifest differs from selected tree"
        );
        let expected_cache = vibe_registry::store::entry_dir(
            store_root,
            &package.group,
            &package.name,
            &package.version,
        );
        ensure!(
            cached.cache_dir == expected_cache,
            "fetched package did not land in the selected cache entry"
        );
        attest_package_tree_copy(&self.expected, &cached.cache_dir)?;
        // `cached.content_hash` is producer metadata. Its registry recipe
        // needs a separate bounded no-follow owner before a production lock
        // may use it as an attested identity. This wrapper is internal only.
        recheck_package_tree_witness(&self.expected)?;
        Ok(())
    }

    fn registry_failure(&self, reason: impl std::fmt::Display) -> RegistryError {
        RegistryError::Io {
            path: self.expected.root.clone(),
            source: std::io::Error::other(format!(
                "source attestation failed after possible global-cache effect; recovery required: {reason}"
            )),
        }
    }
}

impl PackageSource for AttestingSource {
    fn qualify(&self, pkgref: &PackageRef, _locked: &Lockfile) -> Result<PackageRef> {
        ensure!(
            self.pin_matches(pkgref),
            "attested source refuses a different package pin"
        );
        Ok(pkgref.clone())
    }
}

impl InstallSource for AttestingSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        if !self.pin_matches(pkgref) {
            return Err(self.registry_failure("fetch requested another package"));
        }
        let cached = self
            .inner
            .resolve_and_fetch(pkgref, store_root, expected_hash)?;
        self.attest_cached(&cached, store_root)
            .map_err(|error| self.registry_failure(error))?;
        Ok(cached)
    }

    fn resolve_and_fetch_with_progress(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        progress: &vibe_core::progress::Progress,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        if !self.pin_matches(pkgref) {
            return Err(self.registry_failure("fetch requested another package"));
        }
        let cached = self.inner.resolve_and_fetch_with_progress(
            pkgref,
            store_root,
            expected_hash,
            progress,
        )?;
        self.attest_cached(&cached, store_root)
            .map_err(|error| self.registry_failure(error))?;
        Ok(cached)
    }

    fn solve(&self, roots: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        if roots.len() != 1 || !self.pin_matches(&roots[0]) {
            return Err(SolveError::Provider(DepProviderError::Other(
                "attested source requires one exact package root".into(),
            )));
        }
        let graph = self.inner.solve(roots)?;
        if !self.graph_matches(roots, &graph) {
            return Err(SolveError::Provider(DepProviderError::Other(
                "attested source produced a different or transitive graph".into(),
            )));
        }
        Ok(graph)
    }

    fn manifest_of(&self, pkg: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        if !self.pin_matches(pkg) {
            return Err(SolveError::Provider(DepProviderError::Other(
                "attested source requested foreign package metadata".into(),
            )));
        }
        let manifest = self.inner.manifest_of(pkg)?;
        if manifest != self.manifest {
            return Err(SolveError::Provider(DepProviderError::Other(
                "attested source metadata differs from witnessed manifest".into(),
            )));
        }
        Ok(manifest)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        if !blocked.is_empty() {
            return Err(SolveError::Provider(DepProviderError::Other(
                "attested source refuses masked dependency edges".into(),
            )));
        }
        self.solve(roots)
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        Err(self.registry_failure("in-place source is outside one-package attestation"))
    }
}
