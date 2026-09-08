#[derive(Default)]
struct MemoryFs {
    candidate: Option<Vec<TreeEntry>>,
    output: Option<Vec<TreeEntry>>,
    output_occupied: bool,
    publish_unsupported: bool,
    output_third: bool,
    source_mutations: usize,
    quarantine: bool,
    quarantine_third: bool,
    steps: BTreeMap<String, StepWorld>,
    prepared_payloads: BTreeMap<String, Vec<u8>>,
    cleanup_failures: usize,
    candidate_creation: Option<ExclusiveTreeCreation>,
    quarantine_creation: Option<ExclusiveTreeCreation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum StepWorld {
    #[default]
    Before,
    After,
    Third,
}

impl TransactionFilesystem for MemoryFs {
    fn rebind_owned_tree(&mut self, _journal: &Journal) -> Result<(), TransactionError> {
        Ok(())
    }

    fn owned_tree_seal(
        &mut self,
        name: &str,
        _ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError> {
        let entries = if name.contains("candidate") {
            self.candidate
                .as_ref()
                .or(self.output.as_ref())
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(OwnedTreeSeal {
            directory_identity: format!("owned/{name}"),
            manifest_digest: partial_manifest(entries.clone()).digest.0,
            entries: entries
                .into_iter()
                .map(|entry| OwnedEntrySeal {
                    identity: format!("identity/{}", entry.path),
                    path: entry.path,
                    kind: entry.kind,
                    sha256: entry.sha256,
                    bytes: entry.bytes,
                    mode: entry.mode,
                })
                .collect(),
        })
    }
    fn create_export_candidate(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        if let Some(outcome) = self.candidate_creation.take() {
            if !matches!(outcome, ExclusiveTreeCreation::NotCreated { .. }) {
                self.candidate = Some(Vec::new());
            }
            return Ok(outcome);
        }
        if self.candidate.is_some() {
            return Ok(ExclusiveTreeCreation::NotCreated {
                detail: "candidate occupied".into(),
            });
        }
        self.candidate = Some(Vec::new());
        Ok(ExclusiveTreeCreation::Owned)
    }
    fn apply_export_entry(
        &mut self,
        plan: &ExportPlan,
        _: &str,
        _: &str,
        entry: &ExportEntry,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        let sealed = plan
            .final_manifest
            .entries
            .iter()
            .find(|sealed| sealed.path == entry.target_path)
            .expect("validated manifest")
            .clone();
        self.candidate.as_mut().unwrap().push(sealed);
        if let Some(bytes) = prepared_after {
            self.prepared_payloads
                .insert(entry.target_path.clone(), bytes.to_vec());
        }
        Ok(())
    }
    fn observe_export_tree(
        &mut self,
        plan: &ExportPlan,
        slot: ExportTreeSlot,
        _: &str,
        _: &str,
    ) -> Result<OwnedTreeObservation, TransactionError> {
        if slot == ExportTreeSlot::Output && self.output_third {
            return Ok(OwnedTreeObservation::Third {
                detail: "extra descendant concurrent.txt".into(),
            });
        }
        let tree = match slot {
            ExportTreeSlot::Candidate => self.candidate.as_ref(),
            ExportTreeSlot::Output => self.output.as_ref(),
        };
        Ok(match tree {
            None => OwnedTreeObservation::Absent,
            Some(entries) if entries == &plan.final_manifest.entries => {
                OwnedTreeObservation::Exact(plan.final_manifest.clone())
            }
            Some(entries) => OwnedTreeObservation::Exact(partial_manifest(entries.clone())),
        })
    }
    fn publish_export_noreplace(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<(), TransactionError> {
        if self.publish_unsupported {
            return Err(TransactionError::AtomicNoReplaceUnsupported);
        }
        if self.output_occupied {
            return Err(TransactionError::OutputRace("raced occupant".into()));
        }
        self.output = self.candidate.take();
        Ok(())
    }
    fn unpublish_export(
        &mut self,
        _: &ExportPlan,
        _: &str,
        _: &str,
    ) -> Result<(), TransactionError> {
        if self.candidate.is_some() {
            return Err(TransactionError::ThirdState("candidate also exists".into()));
        }
        self.candidate = self.output.take();
        Ok(())
    }
    fn prepare_owned_tree_cleanup(
        &mut self,
        _: &Journal,
        name: &str,
        _: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError> {
        let order = memory_cleanup_order(seal);
        if completed != &order[..completed.len().min(order.len())] {
            return Err(TransactionError::ThirdState(
                "cleanup progress is not canonical".into(),
            ));
        }
        if completed.len() == order.len() {
            return Ok(OwnedTreeCleanupPreparation::Complete);
        }
        let key = order[completed.len()].clone();
        let root = key == "root";
        let expected = if root {
            OwnedEntrySeal {
                path: name.to_owned(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
                identity: format!("root/{name}"),
            }
        } else {
            seal.entries
                .iter()
                .find(|entry| memory_cleanup_key(entry) == key)
                .expect("cleanup order derives from seal")
                .clone()
        };
        Ok(OwnedTreeCleanupPreparation::Intent(
            OwnedTreeCleanupIntent {
                intent_token: format!("intent/{key}"),
                progress_key: key,
                path: expected.path.clone(),
                expected,
                root,
            },
        ))
    }

    fn execute_owned_tree_cleanup(
        &mut self,
        _: &Journal,
        name: &str,
        _: &str,
        _: &OwnedTreeSeal,
        _: &[String],
        intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError> {
        if self.cleanup_failures != 0 {
            self.cleanup_failures -= 1;
            return Err(TransactionError::Filesystem(
                "injected cleanup refusal".into(),
            ));
        }
        let recovered = if name.contains("candidate") {
            if intent.root {
                self.candidate.replace(Vec::new()).is_none()
            } else {
                let Some(entries) = self.candidate.as_mut() else {
                    return Err(TransactionError::ThirdState(
                        "candidate disappeared before cleanup".into(),
                    ));
                };
                let before = entries.len();
                entries.retain(|entry| entry.path != intent.path);
                before == entries.len()
            }
        } else if intent.root {
            let recovered = !self.quarantine;
            self.quarantine = false;
            recovered
        } else {
            false
        };
        if intent.root && name.contains("candidate") {
            self.candidate = None;
        }
        Ok(OwnedTreeCleanupCompletion {
            progress_key: intent.progress_key.clone(),
            recovered_after_syscall: recovered,
        })
    }
    fn create_quarantine(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        if let Some(outcome) = self.quarantine_creation.take() {
            if !matches!(outcome, ExclusiveTreeCreation::NotCreated { .. }) {
                self.quarantine = true;
            }
            return Ok(outcome);
        }
        if self.quarantine {
            return Ok(ExclusiveTreeCreation::NotCreated {
                detail: "quarantine occupied".into(),
            });
        }
        self.quarantine = true;
        Ok(ExclusiveTreeCreation::Owned)
    }
    fn observe_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError> {
        Ok(
            match self.steps.get(&step.id).copied().unwrap_or_default() {
                StepWorld::Before => SealedObservation::Before,
                StepWorld::After => SealedObservation::After,
                StepWorld::Third => SealedObservation::Third {
                    detail: "concurrent bytes".into(),
                },
            },
        )
    }
    fn apply_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
        _: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.source_mutations += usize::from(step.kind != MutationKind::ContractExternalPreserve);
        self.steps.insert(step.id.clone(), StepWorld::After);
        Ok(())
    }
    fn rollback_step(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.steps.insert(step.id.clone(), StepWorld::Before);
        Ok(())
    }
    fn cleanup_unpublished_step_stage(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
        _: &MutationStep,
    ) -> Result<(), TransactionError> {
        Ok(())
    }
    fn observe_quarantine_root(
        &mut self,
        _: &InPlacePlan,
        _: &str,
        _: &str,
    ) -> Result<OwnedRootObservation, TransactionError> {
        Ok(if self.quarantine_third {
            OwnedRootObservation::Third {
                detail: "quarantine identity changed".into(),
            }
        } else if self.quarantine {
            OwnedRootObservation::ExactOwned
        } else {
            OwnedRootObservation::Absent
        })
    }
}

fn memory_cleanup_key(entry: &OwnedEntrySeal) -> String {
    format!(
        "{}:{}",
        match entry.kind {
            TreeEntryKind::File => "file",
            TreeEntryKind::Directory => "directory",
        },
        entry.path
    )
}

fn memory_cleanup_order(seal: &OwnedTreeSeal) -> Vec<String> {
    let mut files = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::File)
        .map(memory_cleanup_key)
        .collect::<Vec<_>>();
    files.sort();
    let mut directories = seal
        .entries
        .iter()
        .filter(|entry| entry.kind == TreeEntryKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .path
            .matches('/')
            .count()
            .cmp(&left.path.matches('/').count())
            .then_with(|| right.path.cmp(&left.path))
    });
    files.extend(directories.into_iter().map(memory_cleanup_key));
    files.push("root".into());
    files
}

#[derive(Default)]
struct AcceptingVerifier {
    fail: Option<VerificationPhase>,
    error: Option<VerificationPhase>,
    drift_view: Option<VerificationPhase>,
    drift_real: Option<VerificationPhase>,
    execute_calls: usize,
    calls: Vec<(
        VerificationPhase,
        VerificationRootKind,
        String,
        Digest,
        bool,
        Option<String>,
    )>,
}

impl TransactionVerifier for AcceptingVerifier {
    fn observe_phase_view(
        &mut self,
        _: &Journal,
        context: &VerificationContext<'_>,
    ) -> Result<TreeManifest, TransactionError> {
        if self.drift_view == Some(context.phase) {
            Ok(manifest("drift", vec![file("concurrent", "bytes")]))
        } else {
            Ok(context.expected_tree.clone())
        }
    }

    fn execute_verification(
        &mut self,
        _: &Journal,
        context: VerificationContext<'_>,
    ) -> Result<VerificationEvidence, TransactionError> {
        self.execute_calls += 1;
        if self.error == Some(context.phase) {
            return Err(TransactionError::Verification(format!(
                "injected {:?} execution/environment/protocol error",
                context.phase
            )));
        }
        self.calls.push((
            context.phase,
            context.root_kind,
            context.root_display.to_owned(),
            context.expected_tree.digest.clone(),
            context.same_display_path_required,
            context.contract_exemption.map(str::to_owned),
        ));
        let canonical_evidence = match context.phase {
            VerificationPhase::Before | VerificationPhase::AfterHealth => serde_json::to_vec(
                &serde_json::json!({
                    "phase": if context.phase == VerificationPhase::Before { "before" } else { "after" },
                    "plan_id": hash("health").0,
                    "checks": [],
                    "assurance_reduced": false,
                }),
            )
            .unwrap(),
            _ => format!("evidence/{:?}", context.phase).into_bytes(),
        };
        Ok(VerificationEvidence {
            accepted: self.fail != Some(context.phase),
            assurance: Assurance::Full,
            summary: format!("{:?}", context.phase),
            canonical_evidence,
        })
    }

    fn reprove_real_tree(
        &mut self,
        journal: &Journal,
        root_kind: VerificationRootKind,
        _root_display: &str,
    ) -> Result<TreeManifest, TransactionError> {
        let phase = match root_kind {
            VerificationRootKind::ExportFinal => VerificationPhase::FinalTree,
            VerificationRootKind::Source if journal.mode == TransactionMode::Export => {
                VerificationPhase::SourceUnchanged
            }
            VerificationRootKind::Source | VerificationRootKind::InPlaceView => {
                VerificationPhase::FinalTree
            }
        };
        if self.drift_real == Some(phase) {
            return Ok(manifest("real-drift", vec![file("foreign", "bytes")]));
        }
        Ok(match (&journal.execution, root_kind) {
            (PreparedMode::Export(plan), VerificationRootKind::ExportFinal) => {
                plan.final_manifest.clone()
            }
            (PreparedMode::Export(plan), VerificationRootKind::Source) => plan.source_tree.clone(),
            (PreparedMode::InPlace(plan), VerificationRootKind::InPlaceView) => {
                plan.after_tree.clone()
            }
            (PreparedMode::InPlace(plan), VerificationRootKind::Source) => plan.before_tree.clone(),
            _ => {
                return Err(TransactionError::Verification(
                    "invalid real-tree root".into(),
                ));
            }
        })
    }
}

#[cfg(windows)]
#[derive(Default)]
struct RealTreeVerifier;

#[cfg(windows)]
impl TransactionVerifier for RealTreeVerifier {
    fn observe_phase_view(
        &mut self,
        _journal: &Journal,
        context: &VerificationContext<'_>,
    ) -> Result<TreeManifest, TransactionError> {
        observe_real_tree(std::path::Path::new(context.root_display))
    }

    fn execute_verification(
        &mut self,
        _journal: &Journal,
        context: VerificationContext<'_>,
    ) -> Result<VerificationEvidence, TransactionError> {
        let canonical_evidence = match context.phase {
            VerificationPhase::Before | VerificationPhase::AfterHealth => serde_json::to_vec(
                &serde_json::json!({
                    "phase": if context.phase == VerificationPhase::Before { "before" } else { "after" },
                    "plan_id": hash("health").0,
                    "checks": [],
                    "assurance_reduced": context.phase == VerificationPhase::AfterHealth,
                }),
            )
            .unwrap(),
            _ => format!("real-tree/e1/{:?}", context.phase).into_bytes(),
        };
        Ok(VerificationEvidence {
            accepted: true,
            assurance: if context.phase == VerificationPhase::AfterHealth {
                Assurance::Reduced
            } else {
                Assurance::Full
            },
            summary: format!("real {:?} accepted", context.phase),
            canonical_evidence,
        })
    }

    fn reprove_real_tree(
        &mut self,
        _journal: &Journal,
        _root_kind: VerificationRootKind,
        root_display: &str,
    ) -> Result<TreeManifest, TransactionError> {
        observe_real_tree(std::path::Path::new(root_display))
    }
}

#[cfg(windows)]
fn observe_real_tree(root: &std::path::Path) -> Result<TreeManifest, TransactionError> {
    let observed = crate::health::tree::observe(root)
        .map_err(|error| TransactionError::Verification(error.to_string()))?;
    Ok(TreeManifest {
        digest: Digest(observed.tree_digest),
        entries: observed
            .entries
            .into_iter()
            .map(|entry| TreeEntry {
                path: entry.path,
                kind: match entry.kind {
                    crate::health::tree::TreeEntryKind::File => TreeEntryKind::File,
                    crate::health::tree::TreeEntryKind::Directory => TreeEntryKind::Directory,
                },
                sha256: entry.sha256.map(Digest),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    })
}
