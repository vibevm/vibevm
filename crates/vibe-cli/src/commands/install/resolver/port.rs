//! The package-source construction port's CLI adapter.
//!
//! The shared application service consumes an opaque
//! [`vibe_orchestrator::ports::PackageSource`]; everything that makes one —
//! the neutral options projected from this surface's `InstallArgs`, the
//! `--git` source-flag grammar, and the PROP-008 §2.7 ambiguity exit code —
//! stays here at the CLI's composition root.

use anyhow::{Context, Result};
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_orchestrator::ports::{InstallManifestMutation, PackageSourceBuild, PackageSourceFactory};
use vibe_package_source::{InstallResolver, PackageQualifier, RegistryPackageSource};
use vibe_workspace::Workspace;

use crate::cli::InstallArgs;

use super::git_source_flag::{apply_git_source_flag, selected_node_manifest_mut};
use super::package_source_options;

/// The M1.15 `--git` source declaration, as this surface's own mutation.
///
/// The shared core knows only that a surface may rewrite its manifest at one
/// exact position — after the manifest and tree are consumed, before the global
/// registry config is loaded. WHICH flags exist, how they combine, what an
/// unknown `--git-auth` is called and which exit code a refusal carries are all
/// this surface's grammar, and none of it crosses.
pub(crate) struct CliGitSourceMutation<'a> {
    pub(crate) args: &'a InstallArgs,
}

impl InstallManifestMutation for CliGitSourceMutation<'_> {
    fn apply(
        &self,
        manifest: &mut Manifest,
        workspace: &mut Workspace,
        project_root: &std::path::Path,
    ) -> Result<()> {
        // M1.15: `vibe install <pkgref> --git <url> --tag/branch/rev <ref>`
        // adds a git-source declaration to `[requires.packages]` before
        // resolving. The added declaration is picked up by the resolver built
        // immediately after; subsequent installs of the same project reproduce
        // the install via the now-recorded git-source entry.
        if self.args.git.is_none() {
            return Ok(());
        }
        // Built once, applied to the STORED RAW snapshot, and persisted from
        // that same value — no second read of a file this command is rewriting.
        let dep = apply_git_source_flag(self.args, manifest, project_root)
            .context("recording --git declaration to vibe.toml")?;
        // Then the SAME delta is replayed onto the finalised node inside the
        // loaded tree — never an assignment, which would restore `var_packages`
        // and erase the concrete versions the loader resolved.
        // A tree that does not contain the selected node is a REFUSAL, never a
        // quiet skip. The `if let` this replaces wrote the declaration to disk
        // and then silently failed to replay it onto the loaded tree, so the
        // very resolution this flag exists to drive ran against a world without
        // it — and the next install, reading the file back, behaved differently
        // from the one the operator just watched.
        // Read before the mutable borrow, so the refusal can name the tree.
        let workspace_root = workspace.root.display().to_string();
        let selected = selected_node_manifest_mut(workspace, project_root).with_context(|| {
            format!(
                "recording --git declaration to vibe.toml: the loaded workspace rooted at                  `{workspace_root}` does not contain the selected node `{}`, so the                  declaration just written could not be replayed onto the tree this install                  resolves against",
                project_root.display(),
            )
        })?;
        vibe_install::record_git_source(selected, dep);
        Ok(())
    }
}

/// The CLI's PROP-008 §2.6 qualification, injected into the shared
/// [`RegistryPackageSource`].
///
/// No context is added: the refusal keeps its historical top-level wording
/// (the PROP-008 §2.7 collision message the operator has always seen), and
/// this adapter invents no second message beside it. The typed exit-7
/// identity — [`crate::exit_code::InstallError::AmbiguousPackage`], read by
/// the exit mapper through a chain-walking downcast — survives an ordinary
/// context wrapper; replacing or translating the typed error would destroy
/// it.
pub(crate) struct CliQualifier;

impl PackageQualifier for CliQualifier {
    fn qualify(
        &self,
        resolver: &InstallResolver,
        pkgref: &PackageRef,
        locked: &Lockfile,
    ) -> Result<PackageRef> {
        crate::commands::short_name::qualify(resolver, pkgref, locked)
    }
}

/// The CLI composition-root factory. It preserves the existing constructor and
/// its exact error identity; the port adds no context of its own.
///
/// The surface's own argument grammar is captured here rather than passed
/// through the port: registry, solver and source-preference flags have exactly
/// one consumer, and it is this factory.
pub(crate) struct CliPackageSourceFactory<'a> {
    pub(crate) args: &'a InstallArgs,
}

impl PackageSourceFactory for CliPackageSourceFactory<'_> {
    fn build(
        &self,
        input: PackageSourceBuild<'_>,
    ) -> Result<Box<dyn vibe_orchestrator::ports::PackageSource>> {
        let options = package_source_options(self.args);
        let resolver = vibe_package_source::build_install_resolver(
            &options,
            input.manifest,
            input.embedded_root,
            input.project_root,
            input.global,
            input.offline,
            input.locked,
        )?;
        Ok(Box::new(RegistryPackageSource::new(
            resolver,
            Box::new(CliQualifier),
        )))
    }
}
