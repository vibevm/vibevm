macro_rules! safefs_transaction_transition_methods {
    () => {
    fn transition_matches(
        &self,
        quarantine: &Pinned,
        transition: &super::PathTransition,
        after: bool,
    ) -> Result<bool, TransactionError> {
        let project_root = self
            .project
            .root_dir()
            .map_err(fs_error("pinning project for observation"))?;
        let root = match transition.location {
            Location::Project => &project_root,
            Location::Quarantine => quarantine,
        };
        state_matches(
            &self.project,
            root,
            &transition.path,
            if after {
                &transition.after
            } else {
                &transition.before
            },
        )
    }

    fn apply_or_rollback_step(
        &self,
        quarantine: &Pinned,
        ownership_token: &str,
        step: &MutationStep,
        prepared_after: Option<&[u8]>,
        rollback: bool,
    ) -> Result<(), TransactionError> {
        let project = self
            .project
            .root_dir()
            .map_err(fs_error("pinning project for mutation"))?;
        match step.kind {
            MutationKind::CaptureBeforeImage => {
                let project_transition = one_at(step, Location::Project)?;
                let quarantine_transition = one_at(step, Location::Quarantine)?;
                let PathState::File(before) = &project_transition.before else {
                    return invalid_step(step, "capture source is not a file");
                };
                if rollback {
                    remove_file_exact(
                        &self.project,
                        quarantine,
                        &quarantine_transition.path,
                        before,
                    )
                } else {
                    let bytes = read_sealed_file_at(
                        &self.project,
                        &project,
                        &project_transition.path,
                        before,
                    )?;
                    require_absent(quarantine, &quarantine_transition.path)?;
                    durable_write(
                        &self.project,
                        quarantine,
                        &quarantine_transition.path,
                        &bytes,
                        before.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("apply:{}", step.id),
                            &quarantine_transition.path,
                        ),
                    )
                }
            }
            MutationKind::AtomicRewrite => {
                let transition = one_at(step, Location::Project)?;
                let (PathState::File(before), PathState::File(after)) =
                    (&transition.before, &transition.after)
                else {
                    return invalid_step(step, "rewrite transition is not file-to-file");
                };
                if rollback {
                    if !state_matches(&self.project, &project, &transition.path, &transition.after)?
                    {
                        return third_step(step, "rewrite target is not its sealed after state");
                    }
                    let backup_path = format!("before/{}", transition.path);
                    let bytes =
                        read_sealed_file_at(&self.project, quarantine, &backup_path, before)?;
                    durable_write(
                        &self.project,
                        &project,
                        &transition.path,
                        &bytes,
                        before.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("rollback:{}", step.id),
                            &transition.path,
                        ),
                    )
                } else {
                    if !state_matches(
                        &self.project,
                        &project,
                        &transition.path,
                        &transition.before,
                    )? {
                        return third_step(step, "rewrite source is not its sealed before state");
                    }
                    let bytes = prepared_after.ok_or_else(|| {
                        TransactionError::InvalidPrepared(format!(
                            "rewrite `{}` has no prepared-after payload",
                            step.id
                        ))
                    })?;
                    if digest_bytes(bytes) != after.sha256 || bytes.len() as u64 != after.bytes {
                        return invalid_step(step, "prepared-after payload differs from the plan");
                    }
                    durable_write(
                        &self.project,
                        &project,
                        &transition.path,
                        bytes,
                        after.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("apply:{}", step.id),
                            &transition.path,
                        ),
                    )
                }
            }
            MutationKind::Relocate => {
                let (source, destination) = move_pair(step)?;
                if rollback {
                    rename_exact(
                        &self.project,
                        &project,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        true,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        &project,
                        &destination.path,
                        &source.before,
                        false,
                    )
                }
            }
            MutationKind::ContractAncestorTreePark => {
                let source = step
                    .transitions
                    .iter()
                    .find(|transition| transition.location == Location::Project)
                    .ok_or_else(|| invalid_step_error(step, "missing project ancestor tree"))?;
                let destination = step
                    .transitions
                    .iter()
                    .find(|transition| transition.location == Location::Quarantine)
                    .ok_or_else(|| invalid_step_error(step, "missing parked ancestor tree"))?;
                if rollback {
                    rename_exact(
                        &self.project,
                        quarantine,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        false,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        quarantine,
                        &destination.path,
                        &source.before,
                        false,
                    )
                }
            }
            MutationKind::QuarantineFile | MutationKind::ContractDeleteLast => {
                let source = step
                    .transitions
                    .iter()
                    .find(|transition| {
                        transition.location == Location::Project
                            && matches!(transition.before, PathState::File(_))
                            && transition.after == PathState::Absent
                    })
                    .ok_or_else(|| invalid_step_error(step, "missing project file move"))?;
                let destination = step
                    .transitions
                    .iter()
                    .find(|transition| {
                        transition.location == Location::Quarantine
                            && transition.before == PathState::Absent
                            && transition.after == source.before
                    })
                    .ok_or_else(|| invalid_step_error(step, "missing quarantine file move"))?;
                let mut contract_ancestors = step
                    .transitions
                    .iter()
                    .filter(|transition| {
                        step.kind == MutationKind::ContractDeleteLast
                            && transition.location == Location::Project
                            && matches!(transition.before, PathState::EmptyDirectory { .. })
                    })
                    .collect::<Vec<_>>();
                if rollback {
                    // Recreate the sealed shallow-to-deep ancestor chain while
                    // every directory is still empty. Only then restore the
                    // contract without an implicit parent-creation shortcut.
                    contract_ancestors.sort_by_key(|transition| path_depth(&transition.path));
                    for transition in &contract_ancestors {
                        let PathState::EmptyDirectory { mode } = transition.before else {
                            unreachable!()
                        };
                        create_directory_exact(&self.project, &project, &transition.path, mode)?;
                    }
                    rename_exact(
                        &self.project,
                        quarantine,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        false,
                    )?;
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        quarantine,
                        &destination.path,
                        &source.before,
                        true,
                    )?;
                }
                if step.kind == MutationKind::ContractDeleteLast && !rollback {
                    contract_ancestors
                        .sort_by_key(|transition| std::cmp::Reverse(path_depth(&transition.path)));
                    for transition in contract_ancestors {
                        let PathState::EmptyDirectory { mode } = &transition.before else {
                            unreachable!()
                        };
                        remove_empty_directory_exact(
                            &self.project,
                            &project,
                            &transition.path,
                            *mode,
                        )?;
                    }
                }
                Ok(())
            }
            MutationKind::PruneEmptyDirectory => {
                let transition = one_at(step, Location::Project)?;
                let parked = parked_directory(step);
                if rollback {
                    rename_exact(
                        &self.project,
                        quarantine,
                        &parked,
                        &project,
                        &transition.path,
                        &transition.before,
                        true,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &transition.path,
                        quarantine,
                        &parked,
                        &transition.before,
                        true,
                    )
                }
            }
            MutationKind::CreateRelocationParent => {
                let transition = one_at(step, Location::Project)?;
                let PathState::EmptyDirectory { mode } = &transition.after else {
                    return invalid_step(step, "created relocation parent is not empty-directory");
                };
                if rollback {
                    remove_empty_directory_exact(&self.project, &project, &transition.path, *mode)
                } else {
                    create_directory_exact(&self.project, &project, &transition.path, *mode)
                }
            }
            MutationKind::ContractExternalPreserve => {
                if prepared_after.is_some() {
                    invalid_step(step, "external preservation received mutation bytes")
                } else {
                    Ok(())
                }
            }
        }
    }

    fn cleanup_parent(
        &self,
        journal: &Journal,
        name: &str,
    ) -> Result<(String, SafefsProject), TransactionError> {
        match &journal.execution {
            PreparedMode::Export(plan) => {
                if journal.candidate_name.as_deref() != Some(name) {
                    return Err(TransactionError::Store(
                        "cleanup name differs from the journaled export candidate".to_owned(),
                    ));
                }
                Ok((
                    Self::live_key(name, ".vibe-scrape-candidate-")?,
                    self.output_parent(plan)?,
                ))
            }
            PreparedMode::InPlace(plan) => {
                if journal.quarantine_name.as_deref() != Some(name) {
                    return Err(TransactionError::Store(
                        "cleanup name differs from the journaled quarantine".to_owned(),
                    ));
                }
                Ok((
                    Self::live_key(name, ".vibe-scrape-quarantine-")?,
                    self.quarantine_parent(plan)?,
                ))
            }
        }
    }

    fn release_cleanup_handles(
        &mut self,
        key: &str,
        name: &str,
        owner: &str,
        seal: &OwnedTreeSeal,
    ) -> Result<(), TransactionError> {
        let Some(live) = self.live.get_mut(key) else {
            // Root absence is meaningful only to the safefs active-intent
            // executor; it will reject every other shape below this seam.
            return Ok(());
        };
        if live.name != name || live.owner != owner {
            return Err(TransactionError::ThirdState(
                "live cleanup ownership differs from journal evidence".to_owned(),
            ));
        }
        if live.identity.as_str() != seal.directory_identity {
            return Err(TransactionError::ThirdState(
                "live cleanup root differs from the journaled cleanup identity".to_owned(),
            ));
        }
        // A restart after one or more cleanup syscalls necessarily holds a
        // reduced live manifest. The manifest-bound safefs prepare/execute
        // calls below compare it against the exact canonical completed prefix
        // (and active intent); repeating the original full digest here would
        // make legitimate restart recovery impossible.
        if matches!(live.state, LiveTreeState::Published(_)) {
            return Err(TransactionError::ThirdState(
                "published product cannot be cleanup payload".to_owned(),
            ));
        }
        if !matches!(live.state, LiveTreeState::PossiblyMoved(_)) {
            live.state = LiveTreeState::PossiblyMoved(
                "owned-tree cleanup is controlled by the durable entry WAL".to_owned(),
            );
        }
        Ok(())
    }
    };
}
