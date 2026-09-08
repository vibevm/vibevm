impl Project {
    /// Stable opaque project identity suitable as input to project-key
    /// hashing.  Only a domain-separated SHA-256 leaves this crate.
    pub fn identity_token(&self) -> Result<String> {
        Ok(identity_token(
            b"vibe-safefs-project-identity-e1\0",
            self.root_identity()?,
        ))
    }

    /// Atomically publish a file and report the host's parent-directory flush
    /// capability instead of silently discarding it.
    pub fn write_atomic_durable(
        &self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<DurableWrite, crate::PublishError> {
        let root = self
            .root_dir()
            .map_err(|error| crate::PublishError::before(Vec::new(), error))?;
        self.write_atomic_durable_in(&root, relative, bytes)
    }
}

impl Pinned {
    /// Compare only filesystem/volume identity.  Object identity remains
    /// opaque and is not exposed as a platform number.
    pub fn same_filesystem(&self, other: &Pinned) -> Result<bool> {
        Ok(volume_hook::check(
            self.identity()?.same_filesystem(other.identity()?),
        ))
    }

    /// Ask this exact held directory capability for its strongest available
    /// metadata flush and report the result explicitly.
    #[must_use]
    pub fn sync_directory(&self) -> DirectoryDurability {
        sync_directory(self)
    }

    /// Observe a direct child as the complete expected state needed by a
    /// later guarded rename. Links, hard links and special files refuse.
    pub fn inspect_child_state(&self, name: &str) -> Result<Option<EntryState>> {
        tree::inspect_child_state(self, name)
    }

    /// Inspect only the exact reserved deterministic transaction-stage
    /// grammar. Ordinary component APIs continue to reject this namespace.
    pub fn inspect_transaction_stage_state(&self, name: &str) -> Result<Option<EntryState>> {
        tree::inspect_transaction_stage_state(self, name)
    }

    /// Remove one exact direct child through its held native handle and
    /// return the host's truthful namespace-persistence evidence.
    pub fn remove_child_expected(
        &self,
        name: &str,
        expected: &EntryState,
    ) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
        platform::remove_expected(self, name, expected).map_err(|error| match error {
            platform::NativeRemoveError::Changed(detail) => OwnedTreeCleanupError::Third { detail },
            platform::NativeRemoveError::Io(error) => {
                OwnedTreeCleanupError::Io(anyhow::Error::new(error))
            }
            #[cfg(not(windows))]
            platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
        })
    }

    /// Exclusively create one direct child for a journaled namespace intent.
    /// The returned capability is the object created by the syscall, never a
    /// name-based reopen; persistence evidence remains journal-recoverable.
    pub fn create_child_exclusive_journaled(
        &self,
        name: &str,
    ) -> std::result::Result<(Pinned, DirectoryDurability), OwnedDirectoryCreateError> {
        crate::ensure_safe_component(name).map_err(OwnedDirectoryCreateError::NotCreated)?;
        let path = self.join(name);
        let (dir, durability) =
            platform::create_directory(self, name).map_err(|error| match error {
                platform::NativeCreateError::NotCreated(error) => {
                    OwnedDirectoryCreateError::NotCreated(anyhow::Error::new(error))
                }
                platform::NativeCreateError::CreatedButUnsealed(error) => {
                    OwnedDirectoryCreateError::CreatedButUnsealed {
                        path: path.clone(),
                        source: anyhow::Error::new(error),
                    }
                }
                #[cfg(not(windows))]
                platform::NativeCreateError::Unsupported => OwnedDirectoryCreateError::Unsupported,
            })?;
        Ok((Pinned { dir, path }, durability))
    }

    /// Capability-relative, source-state-guarded, atomic no-replace move for
    /// either a file or a directory. For a directory this proves only the root
    /// entry; it never claims manifest-bound tree publication. Scrape export
    /// must use [`OwnedDirectory::publish_noreplace_to`].
    pub fn rename_child_to(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<(), RenameError> {
        self.rename_child_noreplace_to(destination, source_name, destination_name, expected)
    }

    /// The explicit publication spelling of [`Self::rename_child_to`].  On
    /// Epoch-1 execution is Windows-only and uses a source handle plus native
    /// no-replace rename. Every other platform returns `Unsupported`; there is
    /// no partial runtime path that weakens the retained-handle contract.
    pub fn rename_child_noreplace_to(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<(), RenameError> {
        self.rename_child_noreplace_to_durable(destination, source_name, destination_name, expected)
            .map(|_| ())
    }

    /// The same handle-relative no-replace rename, retaining truthful
    /// namespace-persistence evidence for a surrounding WAL transaction.
    pub fn rename_child_noreplace_to_durable(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<DirectoryDurability, RenameError> {
        crate::ensure_safe_component(source_name).map_err(RenameError::Failed)?;
        crate::ensure_safe_component(destination_name).map_err(RenameError::Failed)?;
        if !self
            .same_filesystem(destination)
            .map_err(RenameError::Failed)?
        {
            return Err(RenameError::CrossFilesystem);
        }
        require_expected_source(self, source_name, expected)?;
        rename_hook::before(self, destination, source_name, destination_name);
        // Recheck after the deterministic race seam.  The syscall itself is
        // still the no-replace authority for the destination name.
        require_expected_source(self, source_name, expected)?;
        final_rename_hook::after(self, destination, source_name, destination_name);
        let durability =
            platform::rename_noreplace(self, destination, source_name, destination_name, expected)
                .map_err(|error| match error {
                    platform::NoReplaceError::Occupied => RenameError::Occupied {
                        path: destination.join(destination_name),
                    },
                    platform::NoReplaceError::SourceChanged => RenameError::SourceChanged {
                        path: self.join(source_name),
                        detail: "native source handle did not match the expected state".to_owned(),
                    },
                    platform::NoReplaceError::SourceReappeared => RenameError::PossiblyMoved {
                        source: self.join(source_name),
                        destination: destination.join(destination_name),
                        detail: "source name was concurrently recreated after rename".to_owned(),
                    },
                    platform::NoReplaceError::CrossFilesystem => RenameError::CrossFilesystem,
                    platform::NoReplaceError::Unsupported => RenameError::Unsupported,
                    platform::NoReplaceError::Io(error) => {
                        RenameError::Failed(anyhow::Error::new(error).context(format!(
                            "renaming `{}` to `{}` without replacement",
                            self.join(source_name).display(),
                            destination.join(destination_name).display()
                        )))
                    }
                })?;
        match destination.inspect_child_state(destination_name) {
            Ok(Some(actual)) if &actual == expected => Ok(durability),
            Ok(Some(_)) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: "destination identity/content differs from the expected source".to_owned(),
            }),
            Ok(None) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: "destination is absent after the rename primitive reported success"
                    .to_owned(),
            }),
            Err(error) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: format!("destination cannot be re-observed: {error:#}"),
            }),
        }
    }
}

fn require_expected_source(
    source: &Pinned,
    name: &str,
    expected: &EntryState,
) -> Result<(), RenameError> {
    match source.inspect_child_state(name) {
        Ok(Some(actual)) if &actual == expected => Ok(()),
        Ok(Some(_)) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: "the name holds a different object or state".to_owned(),
        }),
        Ok(None) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: "the expected entry is absent".to_owned(),
        }),
        Err(error) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: format!("{error:#}"),
        }),
    }
}

pub(crate) fn identity_token(domain: &[u8], identity: FileIdentity) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(identity.identity_bytes());
    format!("sha256:{:x}", hash.finalize())
}

pub(crate) fn project_view(root: &Pinned) -> Result<Project> {
    Ok(Project {
        root: root
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}`", root.path().display()))?,
        root_path: root.path().to_path_buf(),
        ancestor_identities: vec![root.identity()?],
    })
}

pub(crate) fn sync_directory(directory: &Pinned) -> DirectoryDurability {
    match directory
        .dir
        .try_clone()
        .and_then(|handle| handle.into_std_file().sync_all())
    {
        Ok(()) => DirectoryDurability::Synced,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            DirectoryDurability::Unsupported(error.kind())
        }
        Err(error) => DirectoryDurability::Failed(error.kind()),
    }
}

/// Classify an otherwise unsupported Windows directory flush after an atomic
/// capability-relative namespace operation. This is not a durability upgrade:
/// callers may accept it only when a prior WAL makes before/after replay exact,
/// or before the initial journal while failure can strand liveness-only
/// external state but cannot accompany a product mutation.
pub(crate) fn journal_recoverable_checkpoint(
    durability: DirectoryDurability,
) -> DirectoryDurability {
    #[cfg(windows)]
    if matches!(
        durability,
        DirectoryDurability::Unsupported(
            std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied
        )
    ) {
        return DirectoryDurability::JournalRecoverable;
    }
    durability
}
