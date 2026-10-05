//! A named offline, exact-local composition over the existing source cells.
//!
//! Root identity and package bytes are rechecked before building. The
//! orchestrator's attesting wrapper still owns post-fetch cache-byte proof;
//! this factory alone does not authorize a production machine submission.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use vibe_core::PackageRef;
use vibe_core::manifest::Lockfile;
use vibe_lifecycle::process::{PackageTreeWitness, recheck_package_tree_witness};
use vibe_orchestrator::ports::{
    BoundOnePackageSource, PackageSource, PackageSourceBuild, PackageSourceFactory,
};
use vibe_safefs::{FileIdentity, Project};

use crate::{
    InstallResolver, PackageQualifier, PackageSourceOptions, RegistryPackageSource,
    build_install_resolver_with_progress,
};

/// Host-inspectable local binding. Physical identities are local comparison
/// values, never portable serialized authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedLocalServingRoot {
    pub registry_root: PathBuf,
    pub registry_identity: FileIdentity,
    pub source_root: PathBuf,
    pub source_identity: FileIdentity,
    pub package_pin: PackageRef,
    pub tree_sha256: String,
}

/// The only source this factory can construct is one exact package from one
/// explicit local registry. It cannot select global, project-local, embedded,
/// git or network fallback sources.
pub struct PinnedLocalFactory {
    binding: CheckedLocalServingRoot,
    witness: PackageTreeWitness,
}

impl PinnedLocalFactory {
    /// The host supplies a previously captured package witness. The source
    /// root must spell the exact local-registry slot under `registry_root`.
    pub fn new(
        registry_root: &Path,
        package_pin: &PackageRef,
        witness: &PackageTreeWitness,
    ) -> Result<Self> {
        ensure!(
            registry_root.is_absolute(),
            "local registry root must be absolute"
        );
        ensure!(
            package_pin == &witness.package_pin,
            "local package pin differs from witness"
        );
        witness.validate()?;
        recheck_package_tree_witness(witness)?;
        let group = package_pin
            .group
            .as_ref()
            .context("local pin has no group")?;
        let version = package_pin
            .version
            .to_string()
            .strip_prefix('=')
            .context("local pin does not carry an exact version")?
            .to_owned();
        let expected_source = registry_root
            .join(group.as_str())
            .join(package_pin.name.as_str())
            .join(format!("v{version}"));
        ensure!(
            witness.root == expected_source,
            "witnessed source root differs from exact local registry slot"
        );
        let registry = Project::open(registry_root).with_context(|| {
            format!(
                "opening local registry no-follow `{}`",
                registry_root.display()
            )
        })?;
        let registry_identity = registry.root_identity()?;
        // The legacy builder canonicalizes its explicit registry path. Check
        // that operation cannot move this request to a different object.
        let canonical = registry_root.canonicalize()?;
        let canonical_root = Project::open(&canonical)?;
        ensure!(
            canonical_root.root_identity()? == registry_identity,
            "local registry canonicalization changed physical root"
        );
        let source = Project::open(&witness.root)?;
        ensure!(
            source.root_identity()? == witness.root_identity,
            "local package source root changed since witness capture"
        );
        Ok(Self {
            binding: CheckedLocalServingRoot {
                registry_root: registry_root.to_path_buf(),
                registry_identity,
                source_root: witness.root.clone(),
                source_identity: witness.root_identity,
                package_pin: package_pin.clone(),
                tree_sha256: witness.tree_sha256.clone(),
            },
            witness: witness.clone(),
        })
    }

    /// Read-only current-root check for the host composition. This does not
    /// grant an effect and does not replace later cache-byte attestation.
    pub fn checked_serving_root(&self) -> Result<CheckedLocalServingRoot> {
        let registry = Project::open(&self.binding.registry_root)?;
        ensure!(
            registry.root_identity()? == self.binding.registry_identity,
            "local registry root identity changed"
        );
        recheck_package_tree_witness(&self.witness)?;
        Ok(self.binding.clone())
    }
}

impl PackageSourceFactory for PinnedLocalFactory {
    fn build(&self, input: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        ensure!(
            input.offline,
            "pinned local factory requires offline posture"
        );
        ensure!(
            input.locked.is_empty(),
            "pinned local factory refuses prior lock members"
        );
        ensure!(
            input.embedded_root.is_none(),
            "pinned local factory refuses embedded source fallback"
        );
        self.checked_serving_root()?;
        let options = PackageSourceOptions {
            registry: Some(self.binding.registry_root.clone()),
            no_default_registry: true,
            no_prefer_local: true,
            ..Default::default()
        };
        // This exact options shape enters builder.rs's explicit Local branch
        // and its existing cells.rs LocalRegistry/solver constructors.
        let resolver = build_install_resolver_with_progress(
            &options,
            input.manifest,
            None,
            input.project_root,
            &vibe_core::GlobalRegistryConfig::default(),
            true,
            &[],
            input.progress,
        )?;
        let InstallResolver::Local(local, _) = &resolver else {
            bail!("pinned local source builder selected a nonlocal resolver")
        };
        let built_root = Project::open(local.root())?;
        ensure!(
            built_root.root_identity()? == self.binding.registry_identity,
            "built local resolver root differs from pinned registry"
        );
        self.checked_serving_root()?;
        Ok(Box::new(RegistryPackageSource::new(
            resolver,
            Box::new(ExactLocalQualifier(self.binding.package_pin.clone())),
        )))
    }
}

impl BoundOnePackageSource for PinnedLocalFactory {
    fn source_root(&self) -> &Path {
        &self.binding.source_root
    }

    fn recheck_source_binding(&self, witness: &PackageTreeWitness) -> Result<()> {
        ensure!(
            witness == &self.witness,
            "local source witness differs from pinned factory witness"
        );
        self.checked_serving_root()?;
        Ok(())
    }
}

struct ExactLocalQualifier(PackageRef);
impl PackageQualifier for ExactLocalQualifier {
    fn qualify(
        &self,
        _resolver: &InstallResolver,
        pkgref: &PackageRef,
        _locked: &Lockfile,
    ) -> Result<PackageRef> {
        ensure!(
            pkgref == &self.0,
            "pinned local source refuses a foreign package pin"
        );
        Ok(pkgref.clone())
    }
}

#[cfg(test)]
mod tests;
