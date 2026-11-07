//! Dependency-slot placement classification and validation.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use vibe_core::ContentHash;
use vibe_core::manifest::{Manifest, Materialization};

use super::ResolvedDep;
use crate::{WorkspaceError, vibedeps};

/// Source-owned retained bytes, consumed by the workspace's existing undo owner.
/// Implementations validate their own context; workspace checks exact identity
/// and destination before calling them. No registry dependency crosses this seam.
///
/// A source keeps its staged bytes and drift check alive while the workspace
/// borrows the placement. Validation never publishes the destination.
/// ```
/// use std::path::{Path, PathBuf};
/// use vibe_core::Group;
/// use vibe_workspace::install::PreparedSlotPlacement;
///
/// struct HeldSource {
///     group: Group,
///     version: semver::Version,
///     destination: PathBuf,
///     staged: PathBuf,
///     expected: Vec<u8>,
/// }
/// impl PreparedSlotPlacement for HeldSource {
///     fn coordinate(&self) -> (&Group, &str, &semver::Version) {
///         (&self.group, "demo", &self.version)
///     }
///     fn destination(&self) -> &Path { &self.destination }
///     fn staged_path(&self) -> &Path { &self.staged }
///     fn changed(&self) -> bool { true }
///     fn validate(&self) -> Result<(), String> {
///         let bytes = std::fs::read(self.staged.join("payload"))
///             .map_err(|error| error.to_string())?;
///         if bytes == self.expected { Ok(()) }
///         else { Err("staged source changed".into()) }
///     }
/// }
/// let temporary = tempfile::tempdir()?;
/// let staged = temporary.path().join("staged");
/// std::fs::create_dir(&staged)?;
/// std::fs::write(staged.join("payload"), b"selected bytes")?;
/// let held = HeldSource {
///     group: Group::parse("org.example")?,
///     version: semver::Version::new(1, 0, 0),
///     destination: temporary.path().join("destination"),
///     staged,
///     expected: b"selected bytes".to_vec(),
/// };
/// let placements: [&dyn PreparedSlotPlacement; 1] = [&held];
/// assert!(placements[0].validate().is_ok());
/// assert_eq!(placements[0].coordinate().1, "demo");
/// assert!(!placements[0].destination().exists());
/// std::fs::write(held.staged_path().join("payload"), b"later bytes")?;
/// assert!(placements[0].validate().is_err());
/// assert!(!placements[0].destination().exists());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
pub trait PreparedSlotPlacement {
    fn coordinate(&self) -> (&vibe_core::Group, &str, &semver::Version);
    fn destination(&self) -> &std::path::Path;
    fn staged_path(&self) -> &std::path::Path;
    fn validate(&self) -> Result<(), String>;
    fn changed(&self) -> bool;
}

pub(super) fn required_source_hash(dep: &ResolvedDep) -> Result<&ContentHash, WorkspaceError> {
    dep.source_hash
        .as_ref()
        .ok_or_else(|| WorkspaceError::SpecMaterialization {
            path: dep.content_dir.clone(),
            reason: format!(
                "materialisation of `{}/{}@{}` requires the fetched source_hash",
                dep.group, dep.name, dep.version
            ),
        })
}

/// The copy placement mode for a resolved **copy / hardlink** package
/// (PROP-022 §2.1). `hardlink` shares bytes with the cache by link; `copy`
/// (the default) is a full copy. An `in-place` package never reaches here.
pub(super) fn copy_mode_for(manifest: &Manifest) -> vibedeps::CopyMode {
    match manifest.package.as_ref().map(|p| p.materialization) {
        Some(Materialization::Hardlink) => vibedeps::CopyMode::Hardlink,
        _ => vibedeps::CopyMode::Copy,
    }
}

/// Whether this dependency uses the git-native, unversioned `in-place` slot.
pub(super) fn is_in_place(dep: &ResolvedDep) -> bool {
    dep.manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place())
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
pub(super) fn validate_prepared(
    workspace_root: &std::path::Path,
    resolution: &[ResolvedDep],
    prepared: &[&dyn PreparedSlotPlacement],
) -> Result<(), WorkspaceError> {
    for (i, placement) in prepared.iter().enumerate() {
        let (group, name, version) = placement.coordinate();
        let matches =
            |dep: &&ResolvedDep| &dep.group == group && dep.name == name && &dep.version == version;
        let mut matched = resolution.iter().filter(matches);
        let dep = matched
            .next()
            .ok_or_else(|| WorkspaceError::SpecMaterialization {
                path: placement.destination().into(),
                reason: "prepared slot has no matching resolution identity".into(),
            })?;
        if matched.next().is_some()
            || !is_in_place(dep)
            || placement.destination()
                != vibedeps::in_place_slot_abs_path(workspace_root, group, name)
            || prepared[..i]
                .iter()
                .any(|other| other.coordinate() == placement.coordinate())
        {
            return Err(WorkspaceError::SpecMaterialization {
                path: placement.destination().into(),
                reason: "prepared slot identity or destination is ambiguous".into(),
            });
        }
        placement
            .validate()
            .map_err(|reason| WorkspaceError::SpecMaterialization {
                path: placement.destination().into(),
                reason,
            })?;
    }
    Ok(())
}
