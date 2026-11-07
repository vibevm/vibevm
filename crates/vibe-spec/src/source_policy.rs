//! One caller-selected manifest observation before legacy semantic readers.
//! This is neither a source world nor an authentication or write capability.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use std::path::{Path, PathBuf};
use vibe_core::manifest::{Manifest, PackageMeta, SourceDescriptor};
use vibe_safefs::{FileIdentity, Pinned, Project};

/// Distinguish absence of package context from verified legacy and explicit policy.
/// ```
/// use vibe_spec::source_policy::{observe_package_policy, PackageSourceMode};
/// let root = tempfile::tempdir().unwrap();
/// assert_eq!(observe_package_policy(root.path()).unwrap().mode(), PackageSourceMode::MissingManifest);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSourceMode {
    MissingManifest,
    Legacy,
    ExplicitEmpty,
    Explicit,
}

/// An observation failure never authorizes a legacy fallback.
/// ```
/// use vibe_spec::source_policy::{observe_package_policy, SourcePolicyErrorKind};
/// let root = tempfile::tempdir().unwrap();
/// std::fs::write(root.path().join("vibe.toml"), "[broken").unwrap();
/// assert_eq!(observe_package_policy(root.path()).unwrap_err().kind, SourcePolicyErrorKind::InvalidManifest);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePolicyErrorKind {
    /// No native descriptor selects this address. A promised file that is
    /// absent remains `Unavailable`, not this legitimate address miss.
    MissingNativeAddress,
    Unavailable,
    InvalidManifest,
    Changed,
    Unsupported,
}

/// Stable bounded diagnosis without embedding manifest contents.
/// ```
/// use vibe_spec::source_policy::observe_package_policy;
/// let root = tempfile::tempdir().unwrap();
/// std::fs::write(root.path().join("vibe.toml"), "[broken").unwrap();
/// let error = observe_package_policy(root.path()).unwrap_err();
/// assert!(error.to_string().contains("PROP-072#SOURCE-ADMISSION-BOUNDARY"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY"
)]
#[error(
    "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY: {reason} at {root:?}; fix surface: selected root and its authored source policy"
)]
pub struct SourcePolicyError {
    pub kind: SourcePolicyErrorKind,
    pub root: PathBuf,
    pub reason: String,
}

/// Manifest bytes and the held root used for this finite observation.
/// ```
/// use vibe_spec::source_policy::observe_package_policy;
/// let root = tempfile::tempdir().unwrap();
/// let observed = observe_package_policy(root.path()).unwrap();
/// assert!(observed.manifest_bytes().is_none());
/// observed.recheck_unchanged().unwrap();
/// ```
#[derive(Debug)]
pub struct PackagePolicyObservation {
    state: PolicyState,
    project: Project,
}

/// Guard-only checkpoint retaining no open root or source-read capability.
/// Rechecks compare finite named-root identity and manifest observations; unlike
/// a held observation, dropping the root handle cannot exclude inode-reuse ABA.
/// This is neither source authentication nor read/write authority.
/// ```
/// use vibe_spec::source_policy::{observe_package_policy, PackageSourceMode};
/// let root = tempfile::tempdir().unwrap();
/// let checkpoint = observe_package_policy(root.path()).unwrap().into_checkpoint();
/// assert_eq!(checkpoint.mode(), PackageSourceMode::MissingManifest);
/// checkpoint.recheck_unchanged().unwrap();
/// ```
#[derive(Debug)]
pub struct PackagePolicyCheckpoint {
    state: PolicyState,
}

#[derive(Debug)]
struct PolicyState {
    logical_root: PathBuf,
    caller_root_alias: bool,
    identity: FileIdentity,
    bytes: Option<Vec<u8>>,
    manifest: Option<Manifest>,
}

fn failure(
    root: &Path,
    kind: SourcePolicyErrorKind,
    reason: impl Into<String>,
) -> SourcePolicyError {
    SourcePolicyError {
        kind,
        root: root.into(),
        reason: reason.into(),
    }
}
fn admit(root: &Path) -> Result<Project, SourcePolicyError> {
    // The explicit root may itself be an alias. Descendants are never
    // canonicalized to grant a redirect; their reads use the held capability.
    let physical = root.canonicalize().map_err(|_| {
        failure(
            root,
            SourcePolicyErrorKind::Unavailable,
            "selected root cannot be admitted",
        )
    })?;
    Project::open(&physical).map_err(|_| {
        failure(
            root,
            SourcePolicyErrorKind::Unavailable,
            "selected root cannot be held",
        )
    })
}

/// Observe only the supplied root's manifest. No ancestor search or source walk.
/// ```
/// use vibe_spec::source_policy::{observe_package_policy, PackageSourceMode};
/// let root = tempfile::tempdir().unwrap();
/// std::fs::write(root.path().join("vibe.toml"), "[project]\nname='demo'\nversion='1.0.0'\n").unwrap();
/// assert_eq!(observe_package_policy(root.path()).unwrap().mode(), PackageSourceMode::Legacy);
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY"
)]
pub fn observe_package_policy(root: &Path) -> Result<PackagePolicyObservation, SourcePolicyError> {
    let logical_root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| {
                failure(
                    root,
                    SourcePolicyErrorKind::Unavailable,
                    "relative root cannot be bound",
                )
            })?
            .join(root)
    };
    let project = admit(&logical_root)?;
    let identity = project.root_identity().map_err(|_| {
        failure(
            root,
            SourcePolicyErrorKind::Unavailable,
            "root identity unavailable",
        )
    })?;
    let bytes = project.read_source_bytes(Manifest::FILENAME).map_err(|_| {
        failure(
            root,
            SourcePolicyErrorKind::Unavailable,
            "manifest source observation refused",
        )
    })?;
    let manifest = bytes
        .as_deref()
        .map(|bytes| {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                failure(
                    root,
                    SourcePolicyErrorKind::InvalidManifest,
                    "manifest is not UTF-8",
                )
            })?;
            Manifest::parse_str(text).map_err(|_| {
                failure(
                    root,
                    SourcePolicyErrorKind::InvalidManifest,
                    "manifest or source policy is invalid",
                )
            })
        })
        .transpose()?;
    let result = PackagePolicyObservation {
        state: PolicyState {
            logical_root,
            caller_root_alias: true,
            identity,
            bytes,
            manifest,
        },
        project,
    };
    result.recheck_unchanged()?;
    Ok(result)
}

/// Observe a derived package root through an already admitted no-follow capability.
/// The caller retains Project/Pinned for actual body reads. Recheck never
/// canonicalizes this descendant into a new grant; the checkpoint itself is not
/// a source-read or write capability.
/// ```
/// use vibe_spec::source_policy::{observe_package_policy_in, PackageSourceMode};
/// let dir = tempfile::tempdir().unwrap();
/// let project = vibe_safefs::Project::open(dir.path()).unwrap();
/// let root = project.root_dir().unwrap();
/// let policy = observe_package_policy_in(&project, &root).unwrap();
/// assert_eq!(policy.mode(), PackageSourceMode::MissingManifest);
/// policy.recheck_unchanged().unwrap();
/// ```
pub fn observe_package_policy_in(
    project: &Project,
    root: &Pinned,
) -> Result<PackagePolicyCheckpoint, SourcePolicyError> {
    let logical_root = root.path().to_path_buf();
    let identity = root.identity().map_err(|_| {
        failure(
            &logical_root,
            SourcePolicyErrorKind::Unavailable,
            "held root identity unavailable",
        )
    })?;
    let bytes = project
        .read_source_bytes_in(root, Manifest::FILENAME)
        .map_err(|_| {
            failure(
                &logical_root,
                SourcePolicyErrorKind::Unavailable,
                "held manifest source observation refused",
            )
        })?;
    let manifest = bytes
        .as_deref()
        .map(|bytes| {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                failure(
                    &logical_root,
                    SourcePolicyErrorKind::InvalidManifest,
                    "manifest is not UTF-8",
                )
            })?;
            Manifest::parse_str(text).map_err(|_| {
                failure(
                    &logical_root,
                    SourcePolicyErrorKind::InvalidManifest,
                    "manifest or source policy is invalid",
                )
            })
        })
        .transpose()?;
    let result = PackagePolicyCheckpoint {
        state: PolicyState {
            logical_root,
            caller_root_alias: false,
            identity,
            bytes,
            manifest,
        },
    };
    result.recheck_unchanged()?;
    Ok(result)
}

impl PolicyState {
    /// Bind this finite checkpoint to the supplied held descendant, then
    /// recheck its logical name and the same manifest bytes. This grants no
    /// authority to an unrelated Project/Pinned pair.
    fn recheck_in(&self, project: &Project, root: &Pinned) -> Result<(), SourcePolicyError> {
        let refuse = || {
            failure(
                &self.logical_root,
                SourcePolicyErrorKind::Changed,
                "supplied held root does not match captured package policy",
            )
        };
        let current = self.named_root()?;
        if root.path() != current.root_path() || root.identity().ok() != Some(self.identity) {
            return Err(refuse());
        }
        let relative = root
            .path()
            .strip_prefix(project.root_path())
            .map_err(|_| refuse())?;
        let components: Vec<_> = relative
            .iter()
            .map(|p| p.to_str().ok_or_else(&refuse))
            .collect::<Result<_, _>>()?;
        let linked = project.dir(&components, false).map_err(|_| refuse())?;
        if linked.identity().ok() != Some(self.identity) {
            return Err(refuse());
        }
        let bytes = project
            .read_source_bytes_in(root, Manifest::FILENAME)
            .map_err(|_| refuse())?;
        if bytes != self.bytes {
            return Err(refuse());
        }
        self.recheck_unchanged()
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-POLICY",
        r = 1
    )]
    fn mode(&self) -> PackageSourceMode {
        match self.manifest.as_ref() {
            None => PackageSourceMode::MissingManifest,
            Some(manifest) => match manifest.package.as_ref() {
                Some(package) if package.has_explicit_source_policy() => {
                    if package.sources.as_deref().unwrap_or(&[]).is_empty() {
                        PackageSourceMode::ExplicitEmpty
                    } else {
                        PackageSourceMode::Explicit
                    }
                }
                _ => match manifest
                    .project
                    .as_ref()
                    .and_then(|project| project.sources.as_ref())
                {
                    Some(rows) if rows.is_empty() => PackageSourceMode::ExplicitEmpty,
                    Some(_) => PackageSourceMode::Explicit,
                    None => PackageSourceMode::Legacy,
                },
            },
        }
    }
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }
    pub fn package(&self) -> Option<&PackageMeta> {
        self.manifest.as_ref()?.package.as_ref()
    }
    pub fn package_sources(&self) -> Option<&[SourceDescriptor]> {
        let package = self.package()?;
        package
            .has_explicit_source_policy()
            .then(|| package.sources.as_deref().unwrap_or(&[]))
    }
    pub fn source_rows(&self) -> Option<&[SourceDescriptor]> {
        self.package_sources()
            .or_else(|| self.manifest.as_ref()?.project.as_ref()?.sources.as_deref())
    }
    pub fn manifest_bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
    pub fn unsupported(&self, consumer: &'static str) -> SourcePolicyError {
        failure(
            &self.logical_root,
            SourcePolicyErrorKind::Unsupported,
            format!("{consumer} does not support this explicitly classified source operation"),
        )
    }
    fn named_root(&self) -> Result<Project, SourcePolicyError> {
        let admitted = if self.caller_root_alias {
            admit(&self.logical_root)
        } else {
            Project::open(&self.logical_root).map_err(|_| {
                failure(
                    &self.logical_root,
                    SourcePolicyErrorKind::Changed,
                    "derived root no longer names a no-follow directory",
                )
            })
        };
        let current = admitted.map_err(|_| {
            failure(
                &self.logical_root,
                SourcePolicyErrorKind::Changed,
                "selected logical root is no longer available",
            )
        })?;
        if current.root_identity().ok() != Some(self.identity) {
            return Err(failure(
                &self.logical_root,
                SourcePolicyErrorKind::Changed,
                "selected logical root changed identity",
            ));
        }
        Ok(current)
    }
    /// Re-admit the logical root; a detached old handle alone cannot pass.
    /// Absence is compared too, so a missing manifest becoming present refuses.
    pub fn recheck_unchanged(&self) -> Result<(), SourcePolicyError> {
        let current = self.named_root()?;
        let now = current.read_source_bytes(Manifest::FILENAME).map_err(|_| {
            failure(
                &self.logical_root,
                SourcePolicyErrorKind::Changed,
                "manifest reobservation refused",
            )
        })?;
        if now != self.bytes {
            return Err(failure(
                &self.logical_root,
                SourcePolicyErrorKind::Changed,
                "selected manifest observation changed",
            ));
        }
        Ok(())
    }
}

impl PackagePolicyObservation {
    /// Check that the supplied capability is this observation's held root.
    pub fn recheck_in(&self, project: &Project, root: &Pinned) -> Result<(), SourcePolicyError> {
        self.state.recheck_in(project, root)
    }
    pub fn manifest(&self) -> Option<&Manifest> {
        self.state.manifest()
    }

    pub fn mode(&self) -> PackageSourceMode {
        self.state.mode()
    }
    pub fn package(&self) -> Option<&PackageMeta> {
        self.state.package()
    }
    pub fn package_sources(&self) -> Option<&[SourceDescriptor]> {
        self.state.package_sources()
    }
    pub fn source_rows(&self) -> Option<&[SourceDescriptor]> {
        self.state.source_rows()
    }
    pub fn manifest_bytes(&self) -> Option<&[u8]> {
        self.state.manifest_bytes()
    }
    pub fn unsupported(&self, consumer: &'static str) -> SourcePolicyError {
        self.state.unsupported(consumer)
    }
    pub fn recheck_unchanged(&self) -> Result<(), SourcePolicyError> {
        self.state.recheck_unchanged()
    }
    /// Consume the held root, retaining only guard metadata for large scans.
    pub fn into_checkpoint(self) -> PackagePolicyCheckpoint {
        PackagePolicyCheckpoint { state: self.state }
    }
    /// Read through the originally held root, checking its selected logical name.
    pub fn read_source_observation(
        &self,
        relative: &str,
    ) -> Result<Option<vibe_safefs::SourceObservation>, SourcePolicyError> {
        self.state.named_root()?;
        self.project.read_source_observation(relative).map_err(|_| {
            failure(
                &self.state.logical_root,
                SourcePolicyErrorKind::Unavailable,
                "held source observation refused",
            )
        })
    }

    /// Read through the originally held root, checking its selected logical name.
    pub fn read_source_bytes(&self, relative: &str) -> Result<Option<Vec<u8>>, SourcePolicyError> {
        self.state.named_root()?;
        self.project.read_source_bytes(relative).map_err(|_| {
            failure(
                &self.state.logical_root,
                SourcePolicyErrorKind::Unavailable,
                "held source observation refused",
            )
        })
    }
}
impl PackagePolicyCheckpoint {
    /// Bind this checkpoint to the same supplied root before reading through it.
    pub fn recheck_in(&self, project: &Project, root: &Pinned) -> Result<(), SourcePolicyError> {
        self.state.recheck_in(project, root)
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.state.manifest()
    }

    pub fn mode(&self) -> PackageSourceMode {
        self.state.mode()
    }
    pub fn package(&self) -> Option<&PackageMeta> {
        self.state.package()
    }
    pub fn package_sources(&self) -> Option<&[SourceDescriptor]> {
        self.state.package_sources()
    }
    pub fn source_rows(&self) -> Option<&[SourceDescriptor]> {
        self.state.source_rows()
    }
    pub fn manifest_bytes(&self) -> Option<&[u8]> {
        self.state.manifest_bytes()
    }
    pub fn unsupported(&self, consumer: &'static str) -> SourcePolicyError {
        self.state.unsupported(consumer)
    }
    pub fn recheck_unchanged(&self) -> Result<(), SourcePolicyError> {
        self.state.recheck_unchanged()
    }
}

#[cfg(test)]
#[path = "source_policy/tests.rs"]
mod tests;

mod host;
pub use host::CitedNativeSource;

mod inventory;
pub use inventory::{CapturedHostInventory, CapturedHostResource};

mod authored_boot;
pub use authored_boot::{CapturedAuthoredBoot, CapturedBootResource};
