/// A capability root selected explicitly by the caller, never inferred from
/// the project and never hardcoded below project `.vibe`.
///
/// ```
/// use vibe_safefs::ExternalStore;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let path = scope.path().join("external-state");
/// let store = ExternalStore::open_or_create(&path)?;
/// assert_eq!(store.path(), path);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ExternalStore {
    root: Pinned,
    ancestor_identities: Vec<FileIdentity>,
    bootstrap_syncs: Vec<DirectorySync>,
}

/// One retained directory below an [`ExternalStore`]. All operations remain
/// component-relative to this capability; `path()` is display-only.
///
/// ```
/// use vibe_safefs::ExternalStore;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let store = ExternalStore::open_or_create(&scope.path().join("external-state"))?;
/// let directory = store.root_directory()?;
/// assert_eq!(directory.path(), store.path());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ExternalDirectory {
    pinned: Pinned,
}

impl ExternalStore {
    /// Open or create an explicit absolute store path one no-follow component
    /// at a time.  A raced creator is reopened no-follow; a link, junction,
    /// file or special entry refuses.
    pub fn open_or_create(path: &Path) -> Result<Self> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("external store must not be a filesystem anchor");
        }
        let mut current = open_anchor(&anchor)?;
        let mut ancestors = vec![current.identity()?];
        for component in components {
            current = current.ensure_child(&component)?;
            ancestors.push(current.identity()?);
        }
        Ok(Self {
            root: current,
            ancestor_identities: ancestors,
            bootstrap_syncs: Vec::new(),
        })
    }

    /// Open/create an external store with the disjointness proof ordered
    /// before the first creation.  This is the transaction entry point:
    /// failure cannot leave even an empty store directory inside the project.
    pub fn open_or_create_disjoint(path: &Path, project: &Project) -> Result<Self> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("external store must not be a filesystem anchor");
        }
        let project_identity = project.root_identity()?;
        let mut current = open_anchor(&anchor)?;
        let mut ancestors = vec![current.identity()?];
        let mut missing_at = None;
        for (index, component) in components.iter().enumerate() {
            match current.open_child_checked(component) {
                Ok(Some(child)) => {
                    current = child;
                    ancestors.push(current.identity()?);
                }
                Ok(None) => {
                    missing_at = Some(index);
                    break;
                }
                Err(error) => {
                    return Err(error.context(format!(
                        "opening external-store component `{component}` no-follow"
                    )));
                }
            }
        }
        if let Some(index) = missing_at {
            if ancestors.contains(&project_identity) {
                bail!(
                    "external store `{}` would be created inside project `{}`; nothing was created",
                    path.display(),
                    project.root_path().display()
                );
            }
            let mut bootstrap_syncs = Vec::new();
            for component in &components[index..] {
                let parent_path = current.path().to_path_buf();
                let (child, created, durability) =
                    ensure_external_child_durable(&current, component)?;
                if created {
                    bootstrap_syncs.push(DirectorySync {
                        directory: parent_path,
                        durability,
                    });
                }
                current = child;
                ancestors.push(current.identity()?);
            }
            let store = Self {
                root: current,
                ancestor_identities: ancestors,
                bootstrap_syncs,
            };
            store.prove_disjoint_from(project)?;
            return Ok(store);
        }
        let store = Self {
            root: current,
            ancestor_identities: ancestors,
            bootstrap_syncs: Vec::new(),
        };
        store.prove_disjoint_from(project)?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.root.path()
    }

    #[must_use]
    pub fn bootstrap_durability(&self) -> &[DirectorySync] {
        &self.bootstrap_syncs
    }

    pub fn require_durable_bootstrap(&self) -> Result<()> {
        if let Some(sync) = self.bootstrap_syncs.iter().find(|sync| {
            let strong = matches!(sync.durability, DirectoryDurability::Synced);
            // Epoch-1 Windows has no persistent-media directory-fsync claim.
            // A capability-relative create is nevertheless safe to retry before
            // the initial journal: losing it can only strand liveness-only
            // external state, never a project mutation.
            let windows_liveness_only =
                cfg!(windows) && sync.durability == DirectoryDurability::JournalRecoverable;
            !strong && !windows_liveness_only
        }) {
            bail!(
                "external-store parent `{}` did not provide durable metadata sync: {:?}",
                sync.directory.display(),
                sync.durability
            );
        }
        Ok(())
    }

    /// Prove by capability ancestry, not textual prefix, that neither root is
    /// inside the other.  This also catches aliased spellings of the project.
    pub fn prove_disjoint_from(&self, project: &Project) -> Result<()> {
        let project_identity = project.root_identity()?;
        if self.ancestor_identities.contains(&project_identity) {
            bail!(
                "external store `{}` is inside project `{}`",
                self.path().display(),
                project.root_path().display()
            );
        }
        let store_identity = self.root.identity()?;
        if project.ancestor_identities.contains(&store_identity) {
            bail!(
                "project `{}` is inside external store `{}`; the roots are not disjoint",
                project.root_path().display(),
                self.path().display()
            );
        }
        Ok(())
    }

    /// Acquire an exclusive identity-rechecked OS lock below this explicit
    /// store.  The arbitrary project key is hashed into a portable filename;
    /// raw project paths and raw OS identity numbers never enter that name.
    pub fn open_and_lock_project(&self, project_key: &str) -> Result<ExternalProjectLock> {
        if project_key.is_empty() {
            bail!("project key must not be empty");
        }
        let (locks, _, _) = ensure_external_child_durable(&self.root, "locks")?;
        let mut digest = Sha256::new();
        digest.update(b"vibe-safefs-external-lock-e1\0");
        digest.update(project_key.as_bytes());
        let name = format!("{:x}.lock", digest.finalize());
        acquire_external_lock(&locks, &name)
    }

    /// Atomic durable file publication below the explicit store capability.
    pub fn write_durable(
        &self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<DurableWrite, crate::PublishError> {
        let view = project_view(&self.root)
            .map_err(|error| crate::PublishError::before(Vec::new(), error))?;
        view.write_atomic_durable(relative, bytes)
    }

    pub fn root(&self) -> Result<Pinned> {
        self.root.shallow_clone()
    }

    pub fn root_directory(&self) -> Result<ExternalDirectory> {
        Ok(ExternalDirectory {
            pinned: self.root.shallow_clone()?,
        })
    }

    /// Stable bounded read rooted at the retained store capability.
    pub fn read_stable_bounded(
        &self,
        relative: &str,
        cap: usize,
    ) -> Result<Option<crate::StableFileSnapshot>> {
        project_view(&self.root)?.read_file_snapshot_bounded(relative, cap)
    }

    /// Open a relative directory chain no-follow; absence is distinct from
    /// link/non-directory/I/O refusal.
    pub fn open_directory(&self, relative: &str) -> Result<Option<ExternalDirectory>> {
        let components = directory_components(relative)?;
        let mut current = self.root.shallow_clone()?;
        for component in components {
            let Some(child) = current.open_child_checked(&component)? else {
                return Ok(None);
            };
            current = child;
        }
        Ok(Some(ExternalDirectory { pinned: current }))
    }
}

impl ExternalDirectory {
    /// Display-only absolute spelling. Callers receive no ambient mutation
    /// authority from this value.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.pinned.path()
    }

    /// Canonical byte-sorted direct child names under a width fence.
    pub fn child_names_bounded(&self, max: usize) -> Result<Vec<String>> {
        let view = project_view(&self.pinned)?;
        let mut names = view.child_names_bounded(&self.pinned, max)?;
        names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        Ok(names)
    }

    /// Open one direct child no-follow. `None` is only absence.
    pub fn open_child(&self, name: &str) -> Result<Option<Self>> {
        Ok(self
            .pinned
            .open_child_checked(name)?
            .map(|pinned| Self { pinned }))
    }

    /// Ensure one direct child and report whether this call created it plus
    /// the exact parent-directory durability result for that creation.
    pub fn ensure_child(&self, name: &str) -> Result<(Self, bool, Option<DirectoryDurability>)> {
        let (pinned, created, checkpoint) = ensure_external_child_durable(&self.pinned, name)?;
        let durability = created.then_some(checkpoint);
        Ok((Self { pinned }, created, durability))
    }

    /// Stable bounded read below this retained directory.
    pub fn read_stable_bounded(
        &self,
        relative: &str,
        cap: usize,
    ) -> Result<Option<crate::StableFileSnapshot>> {
        project_view(&self.pinned)?.read_file_snapshot_bounded(relative, cap)
    }

    /// Observe one direct expected-state operand without following links.
    pub fn inspect_child_state(&self, name: &str) -> Result<Option<EntryState>> {
        self.pinned.inspect_child_state(name)
    }

    /// Strong exclusive owned subdirectory creation for a transaction id.
    pub fn create_owned_child_exclusive(
        &self,
        name: &str,
        ownership_token: &str,
    ) -> std::result::Result<OwnedDirectory, OwnedDirectoryCreateError> {
        self.pinned
            .create_owned_child_exclusive(name, ownership_token)
    }

    /// Remove one regular file through its expected state and a native held
    /// handle. The returned directory durability is part of the completion;
    /// absence or changed identity/content is a third state.
    pub fn remove_file_expected(
        &self,
        name: &str,
        expected: &EntryState,
    ) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
        if expected.kind != EntryStateKind::File {
            return Err(OwnedTreeCleanupError::Third {
                detail: "expected-state removal requires a regular file state".to_owned(),
            });
        }
        platform::remove_expected(&self.pinned, name, expected).map_err(|error| match error {
            platform::NativeRemoveError::Changed(detail) => OwnedTreeCleanupError::Third { detail },
            platform::NativeRemoveError::Io(error) => {
                OwnedTreeCleanupError::Io(anyhow::Error::new(error))
            }
            #[cfg(not(windows))]
            platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
        })
    }

    /// Prepare the next canonical manifest-bound retirement intent for an
    /// owned transaction directory. No removal occurs in this call.
    pub fn prepare_owned_child_retirement(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
    ) -> std::result::Result<CleanupPreparation, OwnedTreeCleanupError> {
        self.pinned.prepare_owned_tree_cleanup_next(
            name,
            ownership_token,
            expected_identity,
            expected_manifest,
            progress,
        )
    }

    /// Execute one already-durable retirement intent, including the
    /// syscall-before-completion recovery case.
    pub fn execute_owned_child_retirement(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
        intent: &CleanupIntent,
    ) -> std::result::Result<CleanupCompletion, OwnedTreeCleanupError> {
        self.pinned.execute_owned_tree_cleanup_intent(
            name,
            ownership_token,
            expected_identity,
            expected_manifest,
            progress,
            intent,
        )
    }
}

fn directory_components(relative: &str) -> Result<Vec<String>> {
    let (mut parents, name) = crate::split_relative(relative)?;
    parents.push(name);
    Ok(parents)
}

fn ensure_external_child_durable(
    parent: &Pinned,
    name: &str,
) -> Result<(Pinned, bool, DirectoryDurability)> {
    crate::ensure_safe_component(name)?;
    #[cfg(windows)]
    {
        match platform::create_directory(parent, name) {
            Ok((dir, durability)) => Ok((
                Pinned {
                    dir,
                    path: parent.join(name),
                },
                true,
                durability,
            )),
            Err(platform::NativeCreateError::NotCreated(error)) => {
                if let Some(existing) = parent.open_child_checked(name)? {
                    Ok((existing, false, DirectoryDurability::JournalRecoverable))
                } else {
                    Err(anyhow::Error::new(error).context(format!(
                        "creating external directory `{}` with a retained native handle",
                        parent.join(name).display()
                    )))
                }
            }
            Err(platform::NativeCreateError::CreatedButUnsealed(error)) => {
                Err(anyhow::Error::new(error).context(format!(
                    "external directory `{}` was created but could not be retained",
                    parent.join(name).display()
                )))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let (pinned, created) = parent.ensure_child_recording(name)?;
        let durability = if created {
            sync_directory(parent)
        } else {
            DirectoryDurability::Synced
        };
        Ok((pinned, created, durability))
    }
}

/// A live external project lock.  Drop (or process death) releases it.
///
/// ```
/// use vibe_safefs::ExternalStore;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let store = ExternalStore::open_or_create(&scope.path().join("external-state"))?;
/// let lock = store.open_and_lock_project("project-key")?;
/// lock.require_still_named()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ExternalProjectLock {
    file: std::fs::File,
    directory: Pinned,
    name: String,
}

impl ExternalProjectLock {
    /// Prove that the still-held lock handle remains the object named by the
    /// external lock capability. Windows additionally holds the file without
    /// `FILE_SHARE_DELETE`, so no rename/unlink can open a gap after this check.
    pub fn require_still_named(&self) -> Result<()> {
        let display = self.directory.join(&self.name);
        if lock_still_named(&self.directory, &self.name, &self.file, &display)? {
            Ok(())
        } else {
            bail!(
                "held external lock `{}` no longer occupies its capability-relative name",
                display.display()
            )
        }
    }
}

const LOCK_ATTEMPTS: u32 = 8;

fn acquire_external_lock(directory: &Pinned, name: &str) -> Result<ExternalProjectLock> {
    let display = directory.join(name);
    for _ in 0..LOCK_ATTEMPTS {
        let mut options = cap_options();
        #[cfg(windows)]
        options.share_mode(0x0000_0001 | 0x0000_0002);
        let file = directory
            .dir
            .open_with(name, options.read(true).write(true).create(true))
            .with_context(|| format!("opening external lock `{}`", display.display()))?
            .into_std();
        verify_regular_single_link(&file, &display)?;
        crate::race_hook::before_lock(directory, name);
        file.lock()
            .with_context(|| format!("locking `{}`", display.display()))?;
        if lock_still_named(directory, name, &file, &display)? {
            return Ok(ExternalProjectLock {
                file,
                directory: directory.shallow_clone()?,
                name: name.to_owned(),
            });
        }
        drop(file);
    }
    bail!(
        "external lock `{}` was replaced during every one of {LOCK_ATTEMPTS} attempts",
        display.display()
    )
}

fn lock_still_named(
    directory: &Pinned,
    name: &str,
    locked: &std::fs::File,
    display: &Path,
) -> Result<bool> {
    let mut options = cap_options();
    match directory.dir.open_with(name, options.read(true)) {
        Ok(current) => {
            let current = current.into_std();
            verify_regular_single_link(&current, display)?;
            Ok(crate::race_hook::lock_identity_matches(
                file_identity(locked, display)? == file_identity(&current, display)?,
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(anyhow::Error::new(error)
            .context(format!("rechecking external lock `{}`", display.display()))),
    }
}
