impl Pinned {
    /// Opaque filesystem identity of this held directory capability.
    pub fn identity(&self) -> Result<crate::file::identity::FileIdentity> {
        let handle = self
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}` for identity", self.path.display()))?
            .into_std_file();
        crate::file::identity::file_identity(&handle, &self.path)
    }

    /// Exact Unix permission bits for this held directory; absent elsewhere.
    pub fn unix_mode(&self) -> Result<Option<u32>> {
        let metadata = self
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}` for mode", self.path.display()))?
            .into_std_file()
            .metadata()
            .with_context(|| format!("inspecting directory `{}`", self.path.display()))?;
        Ok(crate::file::unix_mode(&metadata))
    }

    /// Open one direct child directory, refusing reparse points.
    pub fn open_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("opening no-follow directory `{}`", child.display()))?;
        Ok(Self { path: child, dir })
    }

    /// Open one direct child directory without following links; `Ok(None)`
    /// when it does not exist. Every other failure — including a link, a
    /// reparse point and a non-directory occupant — propagates.
    pub fn open_child_checked(&self, name: &str) -> Result<Option<Self>> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Some(Self { path: child, dir })),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(anyhow::Error::new(error)
                .context(format!("opening no-follow directory `{}`", child.display()))),
        }
    }

    /// [`ensure_child`](Self::ensure_child), reporting whether **this call**
    /// is the one that created the directory.
    ///
    /// The answer comes from the `create_dir` syscall, not from the earlier
    /// probe: between "it is absent" and "create it" another process can win,
    /// and a loser that reported `created = true` would make the caller offer
    /// to remove a directory somebody else owns. The loser reopens no-follow,
    /// so losing the race still proves the winner did not plant a link.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn ensure_child_recording(&self, name: &str) -> Result<(Self, bool)> {
        if let Some(existing) = self.open_child_checked(name)? {
            return Ok((existing, false));
        }
        ensure_safe_component(name)?;
        let child = self.join(name);
        crate::race_hook::before_create_dir(self, name);
        let created = match self.dir.create_dir(name) {
            Ok(()) => true,
            // Lost the race. The directory the caller asked for exists, but
            // this invocation did not make it, so it does not own it.
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(error) => {
                return Err(
                    anyhow::Error::new(error).context(format!("creating `{}`", child.display()))
                );
            }
        };
        let dir = self
            .dir
            .open_dir_nofollow(name)
            .with_context(|| format!("reopening `{}` after creation", child.display()))?;
        Ok((Self { path: child, dir }, created))
    }

    /// Open one direct child directory, creating it when absent; an existing
    /// link/reparse child refuses.
    pub fn ensure_child(&self, name: &str) -> Result<Self> {
        ensure_safe_component(name)?;
        let child = self.join(name);
        crate::race_hook::before_create_dir(self, name);
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Self { path: child, dir }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Same benign create race as `Project::dir`: the loser still
                // gets the directory it asked for, and still proves through
                // the no-follow reopen that it is a directory, not a link.
                match self.dir.create_dir(name) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(anyhow::Error::new(error)
                            .context(format!("creating `{}`", child.display())));
                    }
                }
                let dir = self
                    .dir
                    .open_dir_nofollow(name)
                    .with_context(|| format!("reopening created `{}`", child.display()))?;
                Ok(Self { path: child, dir })
            }
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "ensuring no-follow directory `{}`",
                child.display()
            ))),
        }
    }

    /// Create one direct child directory exclusively; an existing entry —
    /// including a crash leftover or attacker-planted spelling — refuses
    /// rather than being reused.
    ///
    /// The failure is **discriminated**, because the two ways this can fail
    /// differ in what this call did, and a caller that cannot tell them apart
    /// must guess. `NotCreated` means nothing at that name came from here;
    /// `CreatedNotReopened` means this call's `create_dir` succeeded and the
    /// entry then could not be reopened no-follow — the one case a bare `?`
    /// turns into an entry nobody ever collects.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn create_child_exclusive(&self, name: &str) -> Result<Self, ExclusiveChildError> {
        let child = self.join(name);
        ensure_safe_component(name).map_err(ExclusiveChildError::NotCreated)?;
        self.dir
            .create_dir(name)
            .with_context(|| {
                format!(
                    "exclusively creating `{}` (existing entry refuses)",
                    child.display()
                )
            })
            .map_err(ExclusiveChildError::NotCreated)?;
        // From here this call has created an entry at that name. What is there
        // *now* is a separate question — the reopen below is what would answer
        // it — so every path out says only the first fact.
        if let Some(injected) = crate::race_hook::after_create_dir(self, name) {
            return Err(ExclusiveChildError::CreatedNotReopened {
                path: child,
                source: anyhow::Error::new(injected),
            });
        }
        match self.dir.open_dir_nofollow(name) {
            Ok(dir) => Ok(Self { path: child, dir }),
            Err(error) => Err(ExclusiveChildError::CreatedNotReopened {
                source: anyhow::Error::new(error)
                    .context(format!("reopening created `{}`", child.display())),
                path: child,
            }),
        }
    }

    pub(crate) fn shallow_clone(&self) -> Result<Self> {
        Ok(Self {
            path: self.path.clone(),
            dir: self.dir.try_clone().with_context(|| {
                format!("retaining the pinned capability `{}`", self.path.display())
            })?,
        })
    }

    /// The absolute display path of a child, for diagnostics only — never a
    /// path this crate then opens with ambient authority.
    #[must_use]
    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// The absolute display path of this pinned directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Walk `components` below `base` without following links. `Ok(None)` when an
/// intermediate directory does not exist.
pub(crate) fn descend(base: &Pinned, components: &[&str]) -> std::io::Result<Option<Pinned>> {
    let mut current = Pinned {
        path: base.path.clone(),
        dir: base
            .dir
            .try_clone()
            .map_err(|error| std::io::Error::other(format!("retaining capability: {error}")))?,
    };
    for component in components {
        match current.dir.open_dir_nofollow(component) {
            Ok(dir) => {
                current = Pinned {
                    path: current.join(component),
                    dir,
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(Some(current))
}
