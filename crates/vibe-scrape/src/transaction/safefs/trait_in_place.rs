macro_rules! safefs_transaction_filesystem_in_place_methods {
    () => {
    fn create_quarantine(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        if self.live.contains_key(&key) {
            return Err(TransactionError::ThirdState(
                "transaction id already owns a live safefs tree".to_owned(),
            ));
        }
        let parent = self.quarantine_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning quarantine parent"))?;
        if !root
            .same_filesystem(
                &self
                    .project
                    .root_dir()
                    .map_err(fs_error("pinning project"))?,
            )
            .map_err(fs_error("proving quarantine volume"))?
        {
            return Err(TransactionError::Filesystem(
                "quarantine and project are on different filesystems".to_owned(),
            ));
        }
        match root.create_owned_child_exclusive(quarantine_name, ownership_token) {
            Ok(directory) => {
                let durable = directory.parent_durability();
                let quarantine_root = directory
                    .directory()
                    .map_err(fs_error("retaining quarantine for topology setup"))?;
                for path in quarantine_topology(plan) {
                    create_directory_exact(&self.project, &quarantine_root, &path, None)?;
                }
                let lease = directory
                    .lease_existing_entries()
                    .map_err(fs_error("leasing created quarantine"))?;
                self.live.insert(
                    key,
                    live_owned(
                        quarantine_name,
                        ownership_token,
                        parent.root_path(),
                        directory,
                        lease,
                    ),
                );
                if namespace_checkpoint_satisfied(durable) {
                    Ok(ExclusiveTreeCreation::Owned)
                } else {
                    Ok(ExclusiveTreeCreation::CreatedNotReopened {
                        detail: format!("quarantine parent did not durably sync: {durable:?}"),
                    })
                }
            }
            Err(error) => Ok(map_create(error)),
        }
    }

    fn observe_step(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine = self.quarantine_directory(&key, quarantine_name, ownership_token)?;
        if step.kind == MutationKind::ContractDeleteLast {
            let project = self.project.root_dir().map_err(fs_error(
                "pinning project for contract-boundary observation",
            ))?;
            let project_tree = model_tree_at(&self.project, &project)?;
            let quarantine_before = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, false)
                        .unwrap_or(false)
                });
            let quarantine_after = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, true)
                        .unwrap_or(false)
                });
            return Ok(
                match (
                    project_tree == plan.pre_contract_tree && quarantine_before,
                    project_tree == plan.post_contract_tree && quarantine_after,
                ) {
                    (true, false) => SealedObservation::Before,
                    (false, true) => SealedObservation::After,
                    _ => SealedObservation::Third {
                        detail: format!(
                            "contract step `{}` matches neither complete pre-contract nor post-contract tree",
                            step.id
                        ),
                    },
                },
            );
        }
        if step.kind == MutationKind::ContractAncestorTreePark {
            let project = self
                .project
                .root_dir()
                .map_err(fs_error("pinning project for contract-cleanup observation"))?;
            let project_tree = model_tree_at(&self.project, &project)?;
            let quarantine_before = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, false)
                        .unwrap_or(false)
                });
            let quarantine_after = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, true)
                        .unwrap_or(false)
                });
            return Ok(
                match (
                    project_tree == plan.post_contract_tree && quarantine_before,
                    project_tree == plan.after_tree && quarantine_after,
                ) {
                    (true, false) => SealedObservation::Before,
                    (false, true) => SealedObservation::After,
                    _ => SealedObservation::Third {
                        detail: format!(
                            "contract cleanup `{}` matches neither post-contract nor final tree",
                            step.id
                        ),
                    },
                },
            );
        }
        let before = step.transitions.iter().all(|transition| {
            self.transition_matches(&quarantine, transition, false)
                .unwrap_or(false)
        });
        let after = step.transitions.iter().all(|transition| {
            self.transition_matches(&quarantine, transition, true)
                .unwrap_or(false)
        });
        let supplemental = supplemental_observation(&self.project, &quarantine, step)?;
        match (before, after, supplemental) {
            (true, false, Supplemental::Before | Supplemental::Either) => {
                Ok(SealedObservation::Before)
            }
            (false, true, Supplemental::After | Supplemental::Either) => {
                Ok(SealedObservation::After)
            }
            (true, true, Supplemental::After) => Ok(SealedObservation::After),
            (true, true, _) if step.transitions.is_empty() => Ok(SealedObservation::After),
            _ => Ok(SealedObservation::Third {
                detail: format!("step `{}` matches neither complete sealed side", step.id),
            }),
        }
    }

    fn observe_quarantine_root(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedRootObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let parent = self.quarantine_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning quarantine parent"))?;
        let Some(live) = self.live.get_mut(&key) else {
            return match root.open_child_checked(quarantine_name) {
                Ok(None) => Ok(OwnedRootObservation::Absent),
                Ok(Some(_)) => Ok(OwnedRootObservation::Third {
                    detail: "quarantine exists without a live ownership seal".to_owned(),
                }),
                Err(error) => Ok(OwnedRootObservation::Third {
                    detail: format!("quarantine root cannot be opened no-follow: {error:#}"),
                }),
            };
        };
        if live.name != quarantine_name || live.owner != ownership_token {
            return Ok(OwnedRootObservation::Third {
                detail: "quarantine ownership evidence changed".to_owned(),
            });
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        match &live.state {
            LiveTreeState::Owned { lease, .. } => {
                match root
                    .observe_owned_tree(
                        quarantine_name,
                        ownership_token,
                        &live.identity,
                        &live.manifest,
                        lease,
                    )
                    .map_err(fs_error("observing quarantine root"))?
                {
                    SafefsTreeObservation::Absent => Ok(OwnedRootObservation::Absent),
                    SafefsTreeObservation::MatchesAtObservation(_) => {
                        Ok(OwnedRootObservation::ExactOwned)
                    }
                    SafefsTreeObservation::Third { detail } => {
                        Ok(OwnedRootObservation::Third { detail })
                    }
                }
            }
            LiveTreeState::PossiblyMoved(detail) => Ok(OwnedRootObservation::Third {
                detail: detail.clone(),
            }),
            LiveTreeState::Published(_) => Ok(OwnedRootObservation::Third {
                detail: "quarantine unexpectedly entered published state".to_owned(),
            }),
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        }
    }

    fn apply_step(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine =
            self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
        self.apply_or_rollback_step(&quarantine, ownership_token, step, prepared_after, false)?;
        let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn rollback_step(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine =
            self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
        self.apply_or_rollback_step(&quarantine, ownership_token, step, None, true)?;
        let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn cleanup_unpublished_step_stage(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        match step.kind {
            MutationKind::AtomicRewrite => {
                let transition = one_at(step, Location::Project)?;
                let PathState::File(after) = &transition.after else {
                    return invalid_step(step, "rewrite after state is not a file");
                };
                let root = self
                    .project
                    .root_dir()
                    .map_err(fs_error("pinning project for stage cleanup"))?;
                remove_transaction_stage(
                    &root,
                    &transition.path,
                    &transaction_stage_name(
                        ownership_token,
                        &format!("apply:{}", step.id),
                        &transition.path,
                    ),
                    after,
                )
            }
            MutationKind::CaptureBeforeImage => {
                let transition = one_at(step, Location::Quarantine)?;
                let PathState::File(after) = &transition.after else {
                    return invalid_step(step, "capture after state is not a file");
                };
                let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
                let root =
                    self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
                let result = remove_transaction_stage(
                    &root,
                    &transition.path,
                    &transaction_stage_name(
                        ownership_token,
                        &format!("apply:{}", step.id),
                        &transition.path,
                    ),
                    after,
                );
                let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
                Self::refresh_owned(live)?;
                result
            }
            _ => Ok(()),
        }
    }
    };
}
