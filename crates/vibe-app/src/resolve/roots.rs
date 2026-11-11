//! Which **directory-shaped** registries the configured chain reads, and
//! where — the places a template is discovered (PROP-075
//! `#TEMPLATE-DISCOVERY`, `#VERSIONS-CHANNEL-DISCOVERY`).
//!
//! A root is not always a path someone typed. A `monorepo` registry's
//! root is a working copy its source **captures**, and that capture lives
//! exactly as long as the source does — so this module's answer is the
//! roots **together with the lease that keeps them on disk**, and that is
//! why it is a type and not a `Vec<PathBuf>`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-CHANNEL-DISCOVERY");

use std::path::{Path, PathBuf};

use crate::error::ResolveError;

use super::global_resolver;

/// The roots of the **directory-shaped** registries the configured chain
/// reads, with the chain that opened them retained.
///
/// A `monorepo` registry's root is a capture its source **owns**: the
/// working copy lives exactly as long as the registry that captured it,
/// and a caller handed the bare path would be handed a path that stops
/// existing when this call returns. The lease therefore travels with the
/// roots, the way `LayoutRoot` retains its own source (PROP-073
/// `#NATIVE-PREPARATION`), and a reader holds the whole value for as long
/// as it reads the trees.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_app::DirectoryRoots;
///
/// // A root named outright — `--registry DIR` — was captured by nothing
/// // and so carries no lease.
/// let given = DirectoryRoots::given(vec![PathBuf::from("/srv/drop")]);
/// assert_eq!(given.roots(), [PathBuf::from("/srv/drop")]);
///
/// // An embedded registry outranks whatever the configured chain
/// // contributed, and putting it ahead keeps the lease.
/// let ordered = given.ahead_of([PathBuf::from("/opt/vibe/embedded")]);
/// assert_eq!(
///     ordered.roots(),
///     [
///         PathBuf::from("/opt/vibe/embedded"),
///         PathBuf::from("/srv/drop"),
///     ]
/// );
/// ```
pub struct DirectoryRoots {
    roots: Vec<PathBuf>,
    /// The resolver whose sources captured the working copies above —
    /// held, never read. `None` when every root was named outright.
    _retained: Option<vibe_package_source::InstallResolver>,
}

impl DirectoryRoots {
    /// Roots named outright — `--registry DIR`, an embedded registry —
    /// which nothing captured and so need no lease.
    pub fn given(roots: Vec<PathBuf>) -> Self {
        DirectoryRoots {
            roots,
            _retained: None,
        }
    }

    /// Put `roots` ahead of the ones the configured chain contributed,
    /// keeping the lease.
    pub fn ahead_of(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut ahead: Vec<PathBuf> = roots.into_iter().collect();
        ahead.append(&mut self.roots);
        ahead.dedup();
        self.roots = ahead;
        self
    }

    /// The roots, in precedence order.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }
}

/// The directory roots of the configured global registries, in precedence
/// order — the places a template is discovered.
///
/// PROP-075 `#TEMPLATE-DISCOVERY` reads a template off the manifests of a
/// package directory, which is what a local-directory registry, a
/// `monorepo` working copy and a mirrored layout registry all are. A
/// per-package **git** registry has no such root and contributes none.
///
/// An enumeration that skipped the monorepo left a home whose only
/// registry is a mirrored monorepo — the corporate shape — with no
/// directory root at all, so every listing answered the version union
/// alone, with no channel table, no origins and no template provenance
/// (PROP-075 `#VERSIONS-CHANNEL-DISCOVERY`).
pub fn directory_roots_from_global_with_operation(
    settings_root: &Path,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<DirectoryRoots, ResolveError> {
    let resolver = global_resolver(
        settings_root,
        "(the configured registries)",
        offline,
        operation,
    )?;
    let mut roots: Vec<PathBuf> = Vec::new();
    let declared_roots = |multi: &vibe_registry::MultiRegistryResolver,
                          roots: &mut Vec<PathBuf>| {
        if let Ok(sources) = multi.sources() {
            // Which sources have a directory root, and where, is the
            // registry layer's own decision, taken beside the variants so
            // a new kind of registry cannot be silently left out here.
            roots.extend(
                sources
                    .iter()
                    .filter_map(|source| source.directory_root(offline)),
            );
        }
    };
    match &resolver {
        vibe_package_source::InstallResolver::Local(local, _) => {
            roots.push(local.root().to_path_buf());
        }
        vibe_package_source::InstallResolver::LockedLocal { local, held, .. } => {
            roots.push(local.root().to_path_buf());
            declared_roots(held, &mut roots);
        }
        vibe_package_source::InstallResolver::Multi(multi, _) => {
            declared_roots(multi, &mut roots);
        }
        vibe_package_source::InstallResolver::Embedded {
            locals, declared, ..
        } => {
            for local in locals {
                roots.push(local.root().to_path_buf());
            }
            if let Some(multi) = declared {
                declared_roots(multi, &mut roots);
            }
        }
    }
    roots.dedup();
    Ok(DirectoryRoots {
        roots,
        _retained: Some(resolver),
    })
}
