macro_rules! safefs_transaction_filesystem_export_methods {
    () => {
    fn rebind_owned_tree(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.rebind_from_journal(journal)
    }

    fn owned_tree_seal(
        &mut self,
        name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError> {
        self.require_project()?;
        let live = self
            .live
            .values()
            .find(|live| live.name == name && live.owner == ownership_token)
            .ok_or_else(|| {
                TransactionError::ThirdState(format!(
                    "no live safefs ownership handle exists for `{name}`"
                ))
            })?;
        Ok(owned_tree_seal(&live.identity, &live.manifest))
    }

    fn create_export_candidate(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        if self.live.contains_key(&key) {
            return Err(TransactionError::ThirdState(
                "transaction id already owns a live safefs tree".to_owned(),
            ));
        }
        let output = Path::new(&plan.output_display_path);
        let absent = SafefsProject::pin_absent_path(output)
            .map_err(|error| TransactionError::OutputRace(format!("{error:#}")))?;
        if absent.identity_token() != plan.output_identity {
            return Err(TransactionError::OutputRace(
                "output slot identity changed since planning".to_owned(),
            ));
        }
        let parent = self.output_parent(plan)?;
        let parent_root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        match parent_root.create_owned_child_exclusive(candidate_name, ownership_token) {
            Ok(directory) => {
                if !namespace_checkpoint_satisfied(directory.parent_durability()) {
                    let detail = format!(
                        "candidate parent did not durably sync: {:?}",
                        directory.parent_durability()
                    );
                    let lease = directory
                        .lease_existing_entries()
                        .map_err(fs_error("leasing non-durable candidate"))?;
                    self.live.insert(
                        key,
                        live_owned(
                            candidate_name,
                            ownership_token,
                            parent.root_path(),
                            directory,
                            lease,
                        ),
                    );
                    return Ok(ExclusiveTreeCreation::CreatedNotReopened { detail });
                }
                let lease = directory
                    .lease_existing_entries()
                    .map_err(fs_error("leasing created candidate"))?;
                self.live.insert(
                    key,
                    live_owned(
                        candidate_name,
                        ownership_token,
                        parent.root_path(),
                        directory,
                        lease,
                    ),
                );
                Ok(ExclusiveTreeCreation::Owned)
            }
            Err(error) => Ok(map_create(error)),
        }
    }

    fn apply_export_entry(
        &mut self,
        _plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
        entry: &ExportEntry,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let root = {
            let live = self.owned_mut(&key, candidate_name, ownership_token)?;
            if live.namespace_name != candidate_name {
                return Err(TransactionError::ThirdState(
                    "export entry cannot be applied after candidate publication".to_owned(),
                ));
            }
            Self::begin_owned_mutation(live)?
        };

        match entry.kind {
            TreeEntryKind::Directory => {
                if entry.payload.is_some() {
                    return Err(TransactionError::InvalidPrepared(
                        "export directory carries a payload".to_owned(),
                    ));
                }
                create_directory_exact(&self.project, &root, &entry.target_path, entry.mode)?;
            }
            TreeEntryKind::File => {
                let bytes = match (&entry.payload, prepared_after) {
                    (
                        Some(ExportPayload::Source {
                            source_path,
                            before,
                        }),
                        None,
                    ) => read_sealed_file(&self.project, source_path, before)?,
                    (Some(ExportPayload::PreparedAfter { .. }), Some(bytes)) => bytes.to_vec(),
                    _ => {
                        return Err(TransactionError::InvalidPrepared(format!(
                            "export payload for `{}` does not match its prepared bytes",
                            entry.target_path
                        )));
                    }
                };
                let desired = PathState::File(FileState {
                    sha256: digest_bytes(&bytes),
                    bytes: bytes.len() as u64,
                    mode: entry.mode,
                });
                if state_matches(&self.project, &root, &entry.target_path, &desired)? {
                    let live = self.owned_mut(&key, candidate_name, ownership_token)?;
                    return Self::refresh_owned(live);
                }
                require_absent(&root, &entry.target_path)?;
                let write = self
                    .project
                    .write_atomic_transactional_in_with_mode(
                        &root,
                        &entry.target_path,
                        &bytes,
                        entry.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("export:{}", entry.target_path),
                            &entry.target_path,
                        ),
                    )
                    .map_err(|error| TransactionError::Filesystem(format!("{error:#}")))?;
                require_durable_write(&write, &entry.target_path)?;
            }
        }
        let live = self.owned_mut(&key, candidate_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn observe_export_tree(
        &mut self,
        plan: &ExportPlan,
        slot: ExportTreeSlot,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let requested_name = match slot {
            ExportTreeSlot::Candidate => candidate_name,
            ExportTreeSlot::Output => &plan.output_name,
        };

        let Some(live) = self.live.get_mut(&key) else {
            return observe_unowned_name(&root, requested_name);
        };
        if live.name != candidate_name || live.owner != ownership_token {
            return Ok(OwnedTreeObservation::Third {
                detail: "owned export evidence changed".to_owned(),
            });
        }
        if live.namespace_name != requested_name {
            return observe_expected_absence(&root, requested_name);
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        match &mut live.state {
            LiveTreeState::Published(pending) => map_safefs_observation(
                pending
                    .reobserve_published(&live.identity, &live.manifest)
                    .map_err(fs_error("reobserving published output"))?,
            ),
            LiveTreeState::Owned { directory, .. } => {
                let recovery_stage_path = live.recovery_stage_path.clone();
                let lease = if let Some(stage_path) = recovery_stage_path.as_deref() {
                    directory.lease_existing_entries_with_transaction_stage(stage_path)
                } else {
                    directory.lease_existing_entries()
                }
                .map_err(fs_error("leasing export tree for observation"))?;
                let expected = lease.manifest().clone();
                let identity = lease.identity().clone();
                let observed = root
                    .observe_owned_tree(
                        requested_name,
                        ownership_token,
                        &identity,
                        &expected,
                        &lease,
                    )
                    .map_err(fs_error("observing export tree"))?;
                live.identity = identity;
                live.manifest = expected;
                if let LiveTreeState::Owned {
                    lease: held_lease, ..
                } = &mut live.state
                {
                    *held_lease = lease;
                }
                strip_owned_observation_stage(
                    map_safefs_observation(observed)?,
                    recovery_stage_path.as_deref(),
                )
            }
            LiveTreeState::PossiblyMoved(detail) => Ok(OwnedTreeObservation::Third {
                detail: detail.clone(),
            }),
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        }
    }

    fn publish_export_noreplace(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let destination = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let live = self.live.remove(&key).ok_or_else(|| {
            TransactionError::ThirdState("candidate has no live ownership handle".to_owned())
        })?;
        if live.name != candidate_name || live.owner != ownership_token {
            self.live.insert(key, live);
            return Err(TransactionError::ThirdState(
                "candidate ownership evidence changed".to_owned(),
            ));
        }
        let LiveOwnedTree {
            name,
            namespace_name,
            owner,
            parent_path,
            identity: prior_identity,
            manifest: prior_manifest,
            recovery_stage_path,
            state,
        } = live;
        let (directory, original_lease) = match state {
            LiveTreeState::Owned { directory, lease } => (directory, lease),
            state => {
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        identity: prior_identity,
                        manifest: prior_manifest,
                        recovery_stage_path,
                        state,
                    },
                );
                return Err(TransactionError::ThirdState(
                    "candidate is not in publishable owned state".to_owned(),
                ));
            }
        };
        drop(original_lease);
        let publish_lease = directory
            .lease_existing_entries()
            .map_err(fs_error("sealing candidate for publication"))?;
        let manifest = publish_lease.manifest().clone();
        let identity = publish_lease.identity().clone();
        if model_manifest(&manifest) != plan.final_manifest {
            self.live.insert(
                key,
                LiveOwnedTree {
                    name,
                    namespace_name,
                    owner,
                    parent_path,
                    state: LiveTreeState::Owned {
                        directory,
                        lease: publish_lease,
                    },
                    manifest,
                    identity,
                    recovery_stage_path: None,
                },
            );
            return Err(TransactionError::ThirdState(
                "candidate differs from the sealed final manifest".to_owned(),
            ));
        }
        match directory.publish_noreplace_to(
            &destination,
            &plan.output_name,
            ownership_token,
            &manifest,
            publish_lease,
        ) {
            Ok(pending) => {
                let source_parent = pending.source_parent;
                let destination_parent = pending.destination_parent;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name: plan.output_name.clone(),
                        owner,
                        parent_path,
                        state: LiveTreeState::Published(pending),
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                require_namespace_checkpoint(source_parent, "candidate parent after publish")?;
                require_namespace_checkpoint(destination_parent, "output parent after publish")?;
                Ok(())
            }
            Err(OwnedTreePublishError::Occupied { path }) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::OutputRace(format!(
                    "`{}` is occupied",
                    path.display()
                )))
            }
            Err(OwnedTreePublishError::Unsupported) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::AtomicNoReplaceUnsupported)
            }
            Err(OwnedTreePublishError::BeforeMove { detail }) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::ThirdState(detail))
            }
            Err(OwnedTreePublishError::PossiblyMoved { detail, .. }) => {
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state: LiveTreeState::PossiblyMoved(detail.clone()),
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::ThirdState(detail))
            }
        }
    }

    fn unpublish_export(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let live = self.owned_mut(&key, candidate_name, ownership_token)?;
        if live.namespace_name != plan.output_name {
            return Err(TransactionError::ThirdState(
                "export output is not at its published namespace".to_owned(),
            ));
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        let observed = match &live.state {
            LiveTreeState::Published(pending) => pending
                .reobserve_published(&live.identity, &live.manifest)
                .map_err(fs_error("reobserving output before rollback"))?,
            LiveTreeState::Owned { lease, .. } => root
                .observe_owned_tree(
                    &plan.output_name,
                    ownership_token,
                    &live.identity,
                    &live.manifest,
                    lease,
                )
                .map_err(fs_error("reobserving rebound output before rollback"))?,
            LiveTreeState::PossiblyMoved(detail) => {
                return Err(TransactionError::ThirdState(detail.clone()));
            }
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        };
        match observed {
            SafefsTreeObservation::MatchesAtObservation(_) => {}
            SafefsTreeObservation::Absent => {
                return Err(TransactionError::ThirdState(
                    "published output disappeared before rollback".to_owned(),
                ));
            }
            SafefsTreeObservation::Third { detail } => {
                return Err(TransactionError::ThirdState(detail));
            }
        }
        let expected = root
            .inspect_child_state(&plan.output_name)
            .map_err(fs_error("sealing output root for rollback"))?
            .ok_or_else(|| TransactionError::ThirdState("published output is absent".to_owned()))?;
        let identity = live.identity.clone();
        let manifest = live.manifest.clone();
        let prior = std::mem::replace(
            &mut live.state,
            LiveTreeState::PossiblyMoved("unpublish state transition incomplete".to_owned()),
        );
        if matches!(
            &prior,
            LiveTreeState::OwnedMutable { .. } | LiveTreeState::PossiblyMoved(_)
        ) {
            unreachable!("unpublish state was checked and resealed");
        }
        // Close every root/descendant lease before asking Windows for DELETE
        // access to the exact source root.
        drop(prior);
        let durability = root
            .rename_child_noreplace_to_durable(&root, &plan.output_name, candidate_name, &expected)
            .map_err(map_rename)?;
        require_namespace_checkpoint(durability, "output parent after unpublish")?;
        live.namespace_name = candidate_name.to_owned();
        live.state =
            reopen_owned_state(&root, candidate_name, ownership_token, &identity, &manifest)?;
        Ok(())
    }

    fn prepare_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError> {
        self.require_project()?;
        let (key, parent) = self.cleanup_parent(journal, name)?;
        self.release_cleanup_handles(&key, name, ownership_token, seal)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning owned-tree cleanup parent"))?;
        let authorized_stage = authorized_owned_stage_in_seal(journal, ownership_token, seal)?;
        let (identity, manifest) = safefs_seal(seal, authorized_stage.as_deref())?;
        let progress = OwnedTreeCleanupProgress::from_completed(completed.to_vec())
            .map_err(fs_error("validating cleanup progress"))?;
        match root
            .prepare_owned_tree_cleanup_next(name, ownership_token, &identity, &manifest, &progress)
            .map_err(map_cleanup)?
        {
            SafefsCleanupPreparation::Complete => {
                self.live.remove(&key);
                Ok(OwnedTreeCleanupPreparation::Complete)
            }
            SafefsCleanupPreparation::Intent(intent) => Ok(OwnedTreeCleanupPreparation::Intent(
                model_cleanup_intent(&intent),
            )),
        }
    }

    fn execute_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
        intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError> {
        self.require_project()?;
        let (key, parent) = self.cleanup_parent(journal, name)?;
        self.release_cleanup_handles(&key, name, ownership_token, seal)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning owned-tree cleanup parent"))?;
        let authorized_stage = authorized_owned_stage_in_seal(journal, ownership_token, seal)?;
        let (identity, manifest) = safefs_seal(seal, authorized_stage.as_deref())?;
        let progress = OwnedTreeCleanupProgress::from_completed(completed.to_vec())
            .map_err(fs_error("validating cleanup progress"))?;
        let safefs_intent = safefs_cleanup_intent(intent)?;
        let completion = root
            .execute_owned_tree_cleanup_intent(
                name,
                ownership_token,
                &identity,
                &manifest,
                &progress,
                &safefs_intent,
            )
            .map_err(map_cleanup)?;
        require_namespace_checkpoint(
            completion.durability(),
            &format!("cleanup parent for `{}`", completion.path),
        )?;
        if intent.root {
            self.live.remove(&key);
        }
        Ok(OwnedTreeCleanupCompletion {
            progress_key: completion.progress_key,
            recovered_after_syscall: completion.recovered_after_syscall,
        })
    }
    };
}
