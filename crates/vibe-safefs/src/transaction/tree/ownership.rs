specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

/// A just-created sibling directory whose handle and namespace identity have
/// both been pinned.
///
/// ```
/// use vibe_safefs::{OwnedDirectoryCreateError, Project};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let parent = project.dir(&["scratch"], true)?;
/// match parent.create_owned_child_exclusive("owned", "journal-token") {
///     Ok(owned) => assert!(owned.path().ends_with("owned")),
///     Err(OwnedDirectoryCreateError::Unsupported) => {}
///     Err(error) => return Err(Box::new(error)),
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct OwnedDirectory {
    parent: Pinned,
    name: String,
    directory: Pinned,
    identity: OwnedDirectoryIdentity,
    parent_durability: DirectoryDurability,
}

/// An identity-rebound owned directory paired with its existing-entry lease.
///
/// ```
/// use vibe_safefs::{OwnedDirectoryCreateError, Project};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let parent = project.dir(&["scratch"], true)?;
/// let owned = match parent.create_owned_child_exclusive("owned", "journal-token") {
///     Ok(owned) => owned,
///     Err(OwnedDirectoryCreateError::Unsupported) => return Ok(()),
///     Err(error) => return Err(Box::new(error)),
/// };
/// let identity = owned.identity().clone();
/// drop(owned);
/// let reopened = parent.reopen_owned_child_by_identity("owned", "journal-token", &identity)?;
/// assert_eq!(reopened.identity(), &identity);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ReopenedOwnedDirectory {
    owned: OwnedDirectory,
    pub entry_lease: ExistingTreeEntryLease,
}

impl ReopenedOwnedDirectory {
    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        self.owned.identity()
    }

    #[must_use]
    pub fn manifest(&self) -> &TreeManifest {
        self.entry_lease.manifest()
    }

    pub fn directory(&self) -> Result<Pinned> {
        self.owned.directory()
    }

    #[must_use]
    pub fn into_parts(self) -> (OwnedDirectory, ExistingTreeEntryLease) {
        (self.owned, self.entry_lease)
    }
}

/// Held identities and bytes for every entry present in one owned tree.
///
/// ```
/// use vibe_safefs::{OwnedDirectoryCreateError, Project};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let parent = project.dir(&["scratch"], true)?;
/// let owned = match parent.create_owned_child_exclusive("owned", "journal-token") {
///     Ok(owned) => owned,
///     Err(OwnedDirectoryCreateError::Unsupported) => return Ok(()),
///     Err(error) => return Err(Box::new(error)),
/// };
/// let lease = owned.lease_existing_entries()?;
/// assert_eq!(lease.identity(), owned.identity());
/// assert!(lease.manifest().entries.is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ExistingTreeEntryLease {
    manifest: TreeManifest,
    identity: OwnedDirectoryIdentity,
    root_state: EntryState,
    _handles: Vec<std::fs::File>,
}

impl ExistingTreeEntryLease {
    #[must_use]
    pub fn manifest(&self) -> &TreeManifest {
        &self.manifest
    }

    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        &self.identity
    }
}

/// A moved owned tree that still requires immediate final re-observation.
///
/// ```
/// use vibe_safefs::{OwnedDirectoryCreateError, OwnedTreeObservation, Project};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let source = project.dir(&["source"], true)?;
/// let destination = project.dir(&["destination"], true)?;
/// let owned = match source.create_owned_child_exclusive("candidate", "journal-token") {
///     Ok(owned) => owned,
///     Err(OwnedDirectoryCreateError::Unsupported) => return Ok(()),
///     Err(error) => return Err(Box::new(error)),
/// };
/// let directory = owned.directory()?;
/// project.write_atomic_in(&directory, "payload.bin", b"payload")?;
/// drop(directory);
/// let identity = owned.identity().clone();
/// let manifest = owned.manifest()?;
/// let lease = owned.lease_existing_entries()?;
/// let pending = owned.publish_noreplace_to(
///     &destination, "published", "journal-token", &manifest, lease,
/// )?;
/// assert!(matches!(
///     pending.reobserve_published(&identity, &manifest)?,
///     OwnedTreeObservation::MatchesAtObservation(_)
/// ));
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PublishedPendingVerification {
    /// Holds existing entry identities/bytes stable. It does not seal
    /// directory membership; callers must still perform final re-observation.
    pub entry_lease: ExistingTreeEntryLease,
    destination_parent_capability: Pinned,
    destination_name: String,
    pub source_parent: DirectoryDurability,
    pub destination_parent: DirectoryDurability,
}

impl PublishedPendingVerification {
    /// Re-enumerate the complete destination after health and immediately
    /// before the transaction's durable `Verified` transition. This
    /// point-in-time observation—not the entry lease—is the membership
    /// evidence. A transaction that cannot place its transition immediately
    /// after this call must re-observe again.
    pub fn reobserve_published(
        &self,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
    ) -> Result<OwnedTreeObservation> {
        if self.entry_lease.identity() != expected_identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "pending publication is bound to a different root identity".to_owned(),
            });
        }
        if self.entry_lease.manifest() != expected_manifest {
            return Ok(OwnedTreeObservation::Third {
                detail:
                    "caller manifest differs from the manifest sealed into the pending publication"
                        .to_owned(),
            });
        }
        let directory = match self
            .destination_parent_capability
            .open_child_checked(&self.destination_name)
        {
            Ok(Some(directory)) => directory,
            Ok(None) => return Ok(OwnedTreeObservation::Absent),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("published root cannot be reopened no-follow: {error:#}"),
                });
            }
        };
        if directory_state(&directory)?.identity != self.entry_lease.root_state.identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "published root identity changed".to_owned(),
            });
        }
        let actual = match manifest(&directory) {
            Ok(actual) => actual,
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("published tree cannot be fully re-observed: {error:#}"),
                });
            }
        };
        if actual == *expected_manifest {
            Ok(OwnedTreeObservation::MatchesAtObservation(actual))
        } else {
            Ok(OwnedTreeObservation::Third {
                detail: manifest_difference(expected_manifest, &actual),
            })
        }
    }
}

/// Typed publication refusal that distinguishes pre-move and uncertain states.
///
/// ```
/// use vibe_safefs::OwnedTreePublishError;
///
/// let error = OwnedTreePublishError::Unsupported;
/// assert!(error.to_string().contains("unsupported"));
/// ```
#[derive(Debug)]
pub enum OwnedTreePublishError {
    BeforeMove {
        detail: String,
    },
    Occupied {
        path: std::path::PathBuf,
    },
    Unsupported,
    PossiblyMoved {
        source_identity: OwnedDirectoryIdentity,
        destination_identity: Option<OwnedDirectoryIdentity>,
        detail: String,
        destination_entry_lease: Option<Box<ExistingTreeEntryLease>>,
    },
}

impl std::fmt::Display for OwnedTreePublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeMove { detail } => write!(f, "owned tree was not published: {detail}"),
            Self::Occupied { path } => {
                write!(f, "publication target `{}` is occupied", path.display())
            }
            Self::Unsupported => f.write_str("owned-tree publication is unsupported"),
            Self::PossiblyMoved { detail, .. } => {
                write!(
                    f,
                    "owned tree may have moved and requires compensation: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for OwnedTreePublishError {}

impl OwnedDirectory {
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        self.directory.path()
    }

    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn parent_durability(&self) -> DirectoryDurability {
        self.parent_durability
    }

    pub fn directory(&self) -> Result<Pinned> {
        self.directory.shallow_clone()
    }

    pub fn manifest(&self) -> Result<TreeManifest> {
        manifest(&self.directory)
    }

    /// Hold identities/bytes of every currently existing entry stable on
    /// Windows, then repeat the complete manifest observation. Directory
    /// membership is deliberately not claimed immutable: Windows directory
    /// handles cannot prevent creation of a new child.
    pub fn lease_existing_entries(&self) -> Result<ExistingTreeEntryLease> {
        self.lease_existing_entries_mode(false)
    }

    /// Hold a complete owned-tree manifest that contains exactly the one
    /// deterministic transaction stage authorized by the caller's durable
    /// journal.  No other reserved stage spelling is admitted.
    pub fn lease_existing_entries_with_transaction_stage(
        &self,
        authorized_stage_path: &str,
    ) -> Result<ExistingTreeEntryLease> {
        validate_authorized_transaction_stage_path(authorized_stage_path)?;
        let lease = self.lease_existing_entries_mode(true)?;
        let mut found = false;
        for entry in &lease.manifest.entries {
            let name = entry
                .path
                .rsplit_once('/')
                .map_or(entry.path.as_str(), |(_, name)| name);
            if is_transaction_stage_name(name) {
                if found || entry.path != authorized_stage_path {
                    bail!(
                        "owned tree contains a transaction stage other than the journal-authorized `{authorized_stage_path}`"
                    );
                }
                found = true;
            }
        }
        if !found {
            bail!(
                "journal-authorized transaction stage `{authorized_stage_path}` is absent from the owned tree"
            );
        }
        Ok(lease)
    }

    fn lease_existing_entries_mode(
        &self,
        allow_transaction_stage: bool,
    ) -> Result<ExistingTreeEntryLease> {
        let first = manifest_mode(&self.directory, allow_transaction_stage)?;
        let root_state = directory_state(&self.directory)?;
        let mut handles = vec![super::platform::lease_entry(
            &self.parent,
            &self.name,
            &root_state,
        )?];
        let view = project_view(&self.directory)?;
        for entry in &first.entries {
            let (parent, name) = holder(&view, &entry.path)?;
            handles.push(super::platform::lease_entry(&parent, &name, &entry.state)?);
        }
        lease_hook::during(&self.directory);
        let second = manifest_mode(&self.directory, allow_transaction_stage)?;
        if first != second {
            bail!("tree changed while acquiring its manifest lease");
        }
        Ok(ExistingTreeEntryLease {
            manifest: second,
            identity: self.identity.clone(),
            root_state,
            _handles: handles,
        })
    }

    /// Move this owned directory into its publication slot, returning only a
    /// pending-verification state. The returned entry lease stabilizes entries
    /// that existed at the post-move observation point, not membership.
    pub fn publish_noreplace_to(
        self,
        destination: &Pinned,
        destination_name: &str,
        ownership_token: &str,
        expected_manifest: &TreeManifest,
        source_lease: ExistingTreeEntryLease,
    ) -> std::result::Result<PublishedPendingVerification, OwnedTreePublishError> {
        if source_lease.identity != self.identity || source_lease.manifest != *expected_manifest {
            return Err(OwnedTreePublishError::BeforeMove {
                detail: "source lease is not bound to the expected identity/manifest".to_owned(),
            });
        }
        let expected_owned = owned_identity(
            ownership_token,
            self.directory
                .identity()
                .map_err(|error| OwnedTreePublishError::BeforeMove {
                    detail: format!("source identity cannot be rechecked: {error:#}"),
                })?,
        );
        if expected_owned != self.identity {
            return Err(OwnedTreePublishError::BeforeMove {
                detail: "ownership token does not bind the source directory".to_owned(),
            });
        }
        let source_identity = self.identity.clone();
        let source_parent = self.parent;
        let source_name = self.name;
        let root_state = source_lease.root_state.clone();
        drop(source_lease);
        drop(self.directory);
        publish_hook::before_move(&source_parent, &source_name);
        let rename_durability = match source_parent.rename_child_noreplace_to_durable(
            destination,
            &source_name,
            destination_name,
            &root_state,
        ) {
            Ok(durability) => durability,
            Err(crate::RenameError::Occupied { path }) => {
                return Err(OwnedTreePublishError::Occupied { path });
            }
            Err(crate::RenameError::Unsupported) => {
                return Err(OwnedTreePublishError::Unsupported);
            }
            Err(crate::RenameError::PossiblyMoved { detail, .. }) => {
                return Err(OwnedTreePublishError::PossiblyMoved {
                    source_identity,
                    destination_identity: None,
                    detail,
                    destination_entry_lease: None,
                });
            }
            Err(error) => {
                return Err(OwnedTreePublishError::BeforeMove {
                    detail: error.to_string(),
                });
            }
        };
        publish_hook::after_move(destination, destination_name);
        let published_dir = match destination.open_child(destination_name) {
            Ok(directory) => directory,
            Err(error) => {
                return Err(OwnedTreePublishError::PossiblyMoved {
                    source_identity,
                    destination_identity: None,
                    detail: format!("destination cannot be pinned after rename: {error:#}"),
                    destination_entry_lease: None,
                });
            }
        };
        let destination_identity = owned_identity(
            ownership_token,
            published_dir
                .identity()
                .map_err(|error| OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: None,
                    detail: format!("destination identity cannot be read: {error:#}"),
                    destination_entry_lease: None,
                })?,
        );
        if destination_identity != source_identity {
            return Err(OwnedTreePublishError::PossiblyMoved {
                source_identity,
                destination_identity: Some(destination_identity),
                detail: "destination name does not hold the moved owned root".to_owned(),
                destination_entry_lease: None,
            });
        }
        let published = OwnedDirectory {
            parent: destination.shallow_clone().map_err(|error| {
                OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: Some(destination_identity.clone()),
                    detail: format!("destination parent cannot be retained: {error:#}"),
                    destination_entry_lease: None,
                }
            })?,
            name: destination_name.to_owned(),
            directory: published_dir,
            identity: destination_identity.clone(),
            parent_durability: rename_durability,
        };
        let lease = published.lease_existing_entries().map_err(|error| {
            OwnedTreePublishError::PossiblyMoved {
                source_identity: source_identity.clone(),
                destination_identity: Some(destination_identity.clone()),
                detail: format!("destination could not be sealed: {error:#}"),
                destination_entry_lease: None,
            }
        })?;
        if lease.manifest != *expected_manifest {
            return Err(OwnedTreePublishError::PossiblyMoved {
                source_identity,
                destination_identity: Some(destination_identity),
                detail: manifest_difference(expected_manifest, &lease.manifest),
                destination_entry_lease: Some(Box::new(lease)),
            });
        }
        Ok(PublishedPendingVerification {
            entry_lease: lease,
            destination_parent_capability: destination.shallow_clone().map_err(|error| {
                OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: Some(destination_identity.clone()),
                    detail: format!("destination parent cannot be retained: {error:#}"),
                    destination_entry_lease: None,
                }
            })?,
            destination_name: destination_name.to_owned(),
            source_parent: rename_durability,
            destination_parent: rename_durability,
        })
    }
}
