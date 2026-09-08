impl Pinned {
    /// Rebind an owned root after restart using only the identity that was
    /// durable before the interrupted mutation. The returned lease contains a
    /// fresh complete manifest; callers must accept it only as one of the
    /// finite before/after states authorized by their durable intent.
    pub fn reopen_owned_child_by_identity(
        &self,
        name: &str,
        ownership_token: &str,
        persisted_identity: &OwnedDirectoryIdentity,
    ) -> std::result::Result<ReopenedOwnedDirectory, ReopenOwnedDirectoryError> {
        #[cfg(not(windows))]
        {
            let _ = (name, ownership_token, persisted_identity);
            return Err(ReopenOwnedDirectoryError::Unsupported);
        }
        #[cfg(windows)]
        {
            crate::ensure_safe_component(name)
                .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
            if ownership_token.is_empty() {
                return Err(ReopenOwnedDirectoryError::InvalidPersisted(
                    anyhow::anyhow!("ownership token is empty"),
                ));
            }
            validate_identity_token(persisted_identity.as_str())
                .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
            let metadata = match self.dir.symlink_metadata(name) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(ReopenOwnedDirectoryError::Third {
                        detail: "journaled owned directory is absent".to_owned(),
                    });
                }
                Err(error) => {
                    return Err(ReopenOwnedDirectoryError::Io(anyhow::Error::new(error)));
                }
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ReopenOwnedDirectoryError::Third {
                    detail: "journaled owned name is now a link, file, or special entry".to_owned(),
                });
            }
            let parent = self
                .shallow_clone()
                .map_err(ReopenOwnedDirectoryError::Io)?;
            let directory = self
                .open_child(name)
                .map_err(ReopenOwnedDirectoryError::Io)?;
            let actual_identity = owned_identity(
                ownership_token,
                directory
                    .identity()
                    .map_err(ReopenOwnedDirectoryError::Io)?,
            );
            if actual_identity != *persisted_identity {
                return Err(ReopenOwnedDirectoryError::Third {
                    detail: "current root identity differs from the journaled owned root"
                        .to_owned(),
                });
            }
            let owned = OwnedDirectory {
                parent,
                name: name.to_owned(),
                directory,
                identity: actual_identity,
                parent_durability: sync_directory(self),
            };
            let entry_lease = owned.lease_existing_entries_mode(true).map_err(|error| {
                if error
                    .chain()
                    .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
                {
                    ReopenOwnedDirectoryError::Io(error)
                } else {
                    ReopenOwnedDirectoryError::Third {
                        detail: format!(
                            "current owned tree cannot be completely sealed: {error:#}"
                        ),
                    }
                }
            })?;
            Ok(ReopenedOwnedDirectory { owned, entry_lease })
        }
    }

    /// Rebind a journaled candidate/quarantine after process restart without
    /// adopting whatever merely occupies its old spelling. The current root
    /// identity and complete descendant observation must equal the persisted
    /// opaque evidence before handles are returned.
    pub fn reopen_owned_child(
        &self,
        name: &str,
        ownership_token: &str,
        persisted_identity: &OwnedDirectoryIdentity,
        persisted_manifest: &TreeManifest,
    ) -> std::result::Result<ReopenedOwnedDirectory, ReopenOwnedDirectoryError> {
        persisted_manifest
            .validate_persisted()
            .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
        let reopened =
            self.reopen_owned_child_by_identity(name, ownership_token, persisted_identity)?;
        if reopened.manifest() != persisted_manifest {
            return Err(ReopenOwnedDirectoryError::Third {
                detail: manifest_difference(persisted_manifest, reopened.manifest()),
            });
        }
        Ok(reopened)
    }

    /// Exclusively create one direct child and seal its identity with the
    /// caller's already-durable ownership token.
    pub fn create_owned_child_exclusive(
        &self,
        name: &str,
        ownership_token: &str,
    ) -> std::result::Result<OwnedDirectory, OwnedDirectoryCreateError> {
        if ownership_token.is_empty() {
            return Err(OwnedDirectoryCreateError::NotCreated(anyhow::anyhow!(
                "ownership token must be durable and non-empty before directory creation"
            )));
        }
        crate::ensure_safe_component(name).map_err(OwnedDirectoryCreateError::NotCreated)?;
        let parent = self
            .shallow_clone()
            .map_err(OwnedDirectoryCreateError::NotCreated)?;
        let path = self.join(name);
        let (dir, parent_durability) =
            super::platform::create_directory(self, name).map_err(|error| match error {
                super::platform::NativeCreateError::NotCreated(error) => {
                    OwnedDirectoryCreateError::NotCreated(anyhow::Error::new(error))
                }
                super::platform::NativeCreateError::CreatedButUnsealed(error) => {
                    OwnedDirectoryCreateError::CreatedButUnsealed {
                        path: path.clone(),
                        source: anyhow::Error::new(error),
                    }
                }
                #[cfg(not(windows))]
                super::platform::NativeCreateError::Unsupported => {
                    OwnedDirectoryCreateError::Unsupported
                }
            })?;
        let directory = Pinned { dir, path };
        let raw = directory.identity().map_err(|source| {
            OwnedDirectoryCreateError::CreatedButUnsealed {
                path: directory.path().to_path_buf(),
                source,
            }
        })?;
        let identity = owned_identity(ownership_token, raw);
        Ok(OwnedDirectory {
            parent,
            name: name.to_owned(),
            directory,
            identity,
            parent_durability,
        })
    }

    /// Re-observe an owned child by its current namespace name. A matching
    /// result is explicitly point-in-time membership evidence; the supplied
    /// lease stabilizes only entries already present. Link/special/hardlink/
    /// walk errors are never collapsed to absence.
    pub fn observe_owned_tree(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        lease: &ExistingTreeEntryLease,
    ) -> Result<OwnedTreeObservation> {
        crate::ensure_safe_component(name)?;
        if lease.identity() != expected_identity || lease.manifest() != expected_manifest {
            return Ok(OwnedTreeObservation::Third {
                detail: "manifest lease is not bound to the expected owned tree".to_owned(),
            });
        }
        tree_hook::before(self, name);
        let directory = match self.open_child_checked(name) {
            Ok(Some(directory)) => directory,
            Ok(None) => return Ok(OwnedTreeObservation::Absent),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("owned root no longer opens no-follow: {error:#}"),
                });
            }
        };
        let actual_identity = match directory.identity() {
            Ok(raw) => owned_identity(ownership_token, raw),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("owned root identity cannot be rechecked: {error:#}"),
                });
            }
        };
        if &actual_identity != expected_identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "owned root name now denotes a different directory".to_owned(),
            });
        }
        let actual = match manifest_mode(
            &directory,
            manifest_has_transaction_stage(expected_manifest),
        ) {
            Ok(actual) => actual,
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!(
                        "owned descendant set is not a complete ordinary tree: {error:#}"
                    ),
                });
            }
        };
        if &actual == expected_manifest {
            Ok(OwnedTreeObservation::MatchesAtObservation(actual))
        } else {
            Ok(OwnedTreeObservation::Third {
                detail: manifest_difference(expected_manifest, &actual),
            })
        }
    }

    /// Advance exact owned-tree cleanup by at most one by-handle removal.
    ///
    /// The caller durably records the returned `progress_key` before calling
    /// again. Absence is accepted only for that canonical completed prefix;
    /// a crash after deletion but before the record is therefore an explicit
    /// third state rather than guessed success. Every successful mutation
    /// carries its parent-directory durability result.
    pub fn prepare_owned_tree_cleanup_next(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
    ) -> std::result::Result<CleanupPreparation, OwnedTreeCleanupError> {
        crate::ensure_safe_component(name).map_err(OwnedTreeCleanupError::Io)?;
        tree_hook::before(self, name);
        let order = cleanup_order(expected_manifest);
        if progress.completed.len() > order.len()
            || progress
                .completed
                .iter()
                .zip(&order)
                .any(|(completed, expected)| completed != expected)
        {
            return Err(third_error(
                "durable cleanup progress is not the canonical prefix".to_owned(),
            ));
        }
        if progress.completed.len() == order.len() {
            return match self.dir.symlink_metadata(name) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(CleanupPreparation::Complete)
                }
                Ok(_) => Err(third_error("completed owned root reappeared".to_owned())),
                Err(error) => Err(OwnedTreeCleanupError::Io(anyhow::Error::new(error))),
            };
        }

        let root_metadata = match self.dir.symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(third_error(
                    "owned root is absent before its durable step".into(),
                ));
            }
            Err(error) => return Err(OwnedTreeCleanupError::Io(anyhow::Error::new(error))),
        };
        if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
            return Err(third_error(
                "owned root name is now a link, file, or special entry".to_owned(),
            ));
        }
        let root = self.open_child(name).map_err(OwnedTreeCleanupError::Io)?;
        let root_raw = root.identity().map_err(OwnedTreeCleanupError::Io)?;
        if owned_identity(ownership_token, root_raw) != *expected_identity {
            return Err(third_error("owned root identity changed".to_owned()));
        }
        let actual = manifest_mode(&root, manifest_has_transaction_stage(expected_manifest))
            .map_err(classify_manifest_cleanup_error)?;
        let completed = progress
            .completed
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let remaining = expected_manifest
            .entries
            .iter()
            .filter(|entry| !completed.contains(&entry_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        if actual.entries != remaining {
            return Err(third_error(manifest_entries_difference(
                &remaining,
                &actual.entries,
            )));
        }

        let next_key = &order[progress.completed.len()];
        if next_key == "root" {
            let root_state = EntryState {
                kind: EntryStateKind::Directory,
                sha256: None,
                bytes: None,
                unix_mode: root.unix_mode().map_err(OwnedTreeCleanupError::Io)?,
                identity: entry_identity(root_raw),
            };
            return Ok(CleanupPreparation::Intent(CleanupIntent {
                intent_token: cleanup_intent_token(expected_identity, expected_manifest, next_key),
                progress_key: next_key.clone(),
                path: name.to_owned(),
                expected: root_state,
                root: true,
            }));
        }

        let entry = expected_manifest
            .entries
            .iter()
            .find(|entry| entry_key(entry) == *next_key)
            .ok_or_else(|| {
                third_error("cleanup order names an absent manifest entry".to_owned())
            })?;
        Ok(CleanupPreparation::Intent(CleanupIntent {
            intent_token: cleanup_intent_token(expected_identity, expected_manifest, next_key),
            progress_key: next_key.clone(),
            path: entry.path.clone(),
            expected: entry.state.clone(),
            root: false,
        }))
    }

    /// Execute exactly one intent that the caller durably journaled after
    /// `prepare_owned_tree_cleanup_next`. A missing target is recoverable only
    /// when this exact manifest-bound in-flight intent is supplied.
    pub fn execute_owned_tree_cleanup_intent(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
        intent: &CleanupIntent,
    ) -> std::result::Result<CleanupCompletion, OwnedTreeCleanupError> {
        let order = cleanup_order(expected_manifest);
        if progress.completed.len() >= order.len()
            || progress
                .completed
                .iter()
                .zip(&order)
                .any(|(completed, expected)| completed != expected)
            || order[progress.completed.len()] != intent.progress_key
            || intent.intent_token
                != cleanup_intent_token(expected_identity, expected_manifest, &intent.progress_key)
        {
            return Err(third_error(
                "cleanup intent is not the exact canonical in-flight step".to_owned(),
            ));
        }
        let expected_entry = if intent.root {
            if intent.progress_key != "root" || intent.path != name {
                return Err(third_error("root cleanup intent shape changed".to_owned()));
            }
            None
        } else {
            let entry = expected_manifest
                .entries
                .iter()
                .find(|entry| entry_key(entry) == intent.progress_key)
                .ok_or_else(|| {
                    third_error("intent target is absent from the manifest".to_owned())
                })?;
            if entry.path != intent.path || entry.state != intent.expected {
                return Err(third_error("intent expected state changed".to_owned()));
            }
            Some(entry)
        };

        let root = match self.open_child_checked(name) {
            Ok(Some(root)) => root,
            Ok(None) if intent.root => {
                return Ok(CleanupCompletion {
                    progress_key: intent.progress_key.clone(),
                    path: intent.path.clone(),
                    parent: sync_directory(self),
                    recovered_after_syscall: true,
                });
            }
            Ok(None) => {
                return Err(third_error(
                    "owned root disappeared during cleanup".to_owned(),
                ));
            }
            Err(error) => return Err(OwnedTreeCleanupError::Io(error)),
        };
        if owned_identity(
            ownership_token,
            root.identity().map_err(OwnedTreeCleanupError::Io)?,
        ) != *expected_identity
        {
            return Err(third_error("owned root identity changed".to_owned()));
        }
        let actual = manifest_mode(&root, manifest_has_transaction_stage(expected_manifest))
            .map_err(classify_manifest_cleanup_error)?;
        let completed = progress
            .completed
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let remaining = expected_manifest
            .entries
            .iter()
            .filter(|entry| !completed.contains(&entry_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        let after = remaining
            .iter()
            .filter(|entry| {
                Some(entry.path.as_str()) != expected_entry.map(|entry| entry.path.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        let recovered = if actual.entries == remaining {
            false
        } else if !intent.root && actual.entries == after {
            true
        } else {
            return Err(third_error(manifest_entries_difference(
                &remaining,
                &actual.entries,
            )));
        };

        let parent = if intent.root {
            drop(root);
            if recovered {
                super::DirectoryDurability::JournalRecoverable
            } else {
                remove_native(self, name, &intent.expected)?
            }
        } else {
            let view = project_view(&root).map_err(OwnedTreeCleanupError::Io)?;
            let (parent, child) = holder(&view, &intent.path).map_err(OwnedTreeCleanupError::Io)?;
            let durability = if recovered {
                super::DirectoryDurability::JournalRecoverable
            } else {
                remove_native(&parent, &child, &intent.expected)?
            };
            return Ok(CleanupCompletion {
                progress_key: intent.progress_key.clone(),
                path: intent.path.clone(),
                parent: durability,
                recovered_after_syscall: recovered,
            });
        };
        Ok(CleanupCompletion {
            progress_key: intent.progress_key.clone(),
            path: intent.path.clone(),
            parent,
            recovered_after_syscall: recovered,
        })
    }
}
