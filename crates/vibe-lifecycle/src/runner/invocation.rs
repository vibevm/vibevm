//! Invocation admission below every phase/slot handler path.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#DECLARED-SOURCE-EFFECTS");
use crate::HandlerExecution;
use specmark::spec;
use vibe_core::manifest::DeclaredEffects;

#[derive(Debug, thiserror::Error)]
pub enum InvocationRefusal {
    #[error("unsupported-effect-classification: `{key}` at `{point}` has no declared effects")]
    UnknownEffects { key: String, point: String },
    #[error("creation-required: `{key}` has no comparable completed production basis ({reason})")]
    CreationRequired { key: String, reason: &'static str },
    #[error("preparation-required: `{key}` has no comparable completed build basis ({reason})")]
    BuildRequired { key: String, reason: &'static str },
    #[error("unsupported-invocation: `{requested}` grants no known invocation role")]
    UnknownInvocation { requested: String },
}

/// This checks declaration availability, not freshness, authority or completion.
pub(crate) fn known_effects(
    execution: &HandlerExecution,
) -> Result<&DeclaredEffects, InvocationRefusal> {
    execution
        .row()
        .declaration()
        .effects
        .as_ref()
        .ok_or_else(|| InvocationRefusal::UnknownEffects {
            key: execution.key(),
            point: execution.row().declaration().point.to_string(),
        })
}

/// Compare actual completed input/output basis; neither header nor existence
/// alone establishes production. This reads no other state's selected body.
pub(super) fn comparable_completion(
    record: &crate::state::current::ExecutionRecord,
    prepared: &crate::state::PreparedFingerprint,
    selected: &str,
    key: &str,
    phase: &str,
    root: &str,
) -> Result<(), &'static str> {
    use crate::state::current::ExecutionRecordStatus;
    if !matches!(
        record.status,
        ExecutionRecordStatus::Ok | ExecutionRecordStatus::Fresh
    ) {
        return Err("completion-status-unavailable");
    }
    let basis = record
        .completion_basis
        .as_ref()
        .ok_or("original-basis-unavailable")?;
    if basis.selected != selected {
        return Err("original-selected-differs");
    }
    if prepared.completion_digest.as_deref() != Some(basis.digest.as_str()) {
        return Err("completion-subject-changed");
    }
    let prior = record
        .input_measurement
        .as_ref()
        .ok_or("original-inputs-unavailable")?;
    let current = prepared
        .input_manifest
        .as_ref()
        .ok_or("current-inputs-unavailable")?;
    let witnessed = current.measured().ok_or("current-inputs-unstable")?;
    if prior.execution != key
        || prior.phase != phase
        || prior.declaration_fingerprint != prepared.declaration_fingerprint
        || prior.patterns != current.patterns
        || serde_json::to_value(&prior.witness).ok() != serde_json::to_value(witnessed).ok()
    {
        return Err("original-inputs-changed");
    }
    if record.artifacts.is_empty() {
        return Err("original-outputs-unavailable");
    }
    let observer = crate::artifacts::observe::ArtifactObserver::new(root);
    for artifact in &record.artifacts {
        let baseline = artifact
            .witness
            .as_ref()
            .ok_or("original-output-unwitnessed")?;
        if artifact.measured_run_id.as_deref() != Some(basis.run_id.as_str()) {
            return Err("original-output-attribution-differs");
        }
        let crate::artifacts::observe::WitnessOutcome::Measured(observed) =
            observer.observe(&artifact.id, &artifact.path)
        else {
            return Err("current-output-unavailable-or-unstable");
        };
        if serde_json::to_value(baseline).ok() != serde_json::to_value(observed).ok() {
            return Err("original-output-changed");
        }
    }
    Ok(())
}

/// Shared complete-prefix admission before a run header or handler effect.
/// Caller supplies the actual retained selected registry, not a page/filter.
pub fn validate_invocation_prefix(
    lease: &crate::LifecycleLease,
    project: &vibe_wire::generated::lifecycle::e1::context::Project,
    world: &vibe_wire::generated::lifecycle::e1::context::World,
    metadata: &crate::RunMetadata,
    executions: &[(String, HandlerExecution)],
    agent: &dyn crate::AgentBackend,
) -> Result<(), super::LifecycleRunError> {
    use crate::{ExecutionSession, InvocationIntent};
    use vibe_core::manifest::DeclaredEffect;
    let intent = InvocationIntent::from_requested(&metadata.requested).ok_or_else(|| {
        InvocationRefusal::UnknownInvocation {
            requested: metadata.requested.clone(),
        }
    })?;
    // Check the complete classification inventory first: no early useful
    // observer may run before a later hidden producer/unknown is discovered.
    for (_, execution) in executions {
        known_effects(execution)?;
    }
    let state = crate::LifecycleStateStore::peek_with_lease(lease)?;
    let mut replay = ExecutionSession::new(project.clone(), world.clone(), metadata.clone());
    for (phase, execution) in executions {
        let effects = known_effects(execution)?;
        let production =
            effects.contains(DeclaredEffect::ProductSource) && !intent.admits_product_source();
        let build =
            effects.contains(DeclaredEffect::BuildArtifact) && !intent.admits_build_artifact();
        let key = execution.key();
        let prior = state.as_ref().and_then(|state| state.execution.get(&key));
        if production || build {
            let envelope = replay
                .envelope_for_execution(phase, execution)
                .map_err(|source| super::LifecycleRunError::Envelope {
                    key: key.clone(),
                    source,
                })?;
            let prepared_agent =
                crate::agent::prepare(agent, execution.row(), &envelope).map_err(|source| {
                    super::LifecycleRunError::AgentPreparation {
                        key: key.clone(),
                        source: Box::new(source),
                    }
                })?;
            let prepared = crate::state::prepare_handler_execution_with(
                execution,
                &envelope,
                prepared_agent.as_ref(),
            )
            .map_err(|source| super::LifecycleRunError::Fingerprint {
                key: key.clone(),
                source,
            })?;
            let result = prior
                .ok_or("completed-record-unavailable")
                .and_then(|record| {
                    comparable_completion(
                        record,
                        &prepared,
                        &metadata.selected,
                        &key,
                        phase,
                        &project.root,
                    )
                });
            if let Err(reason) = result {
                return Err(if production {
                    InvocationRefusal::CreationRequired { key, reason }
                } else {
                    InvocationRefusal::BuildRequired { key, reason }
                }
                .into());
            }
        }
        // Exact retained rows establish only the replay's predecessor input
        // coordinates; no baseline is made current or strengthened here.
        if let Some(record) = prior {
            replay.hydrate_artifacts(phase, &record.artifacts);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub enum ExecutionReuse {
    FreshnessAware,
    Always,
}

impl super::LifecycleRun {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reusable_completion(
        &self,
        execution: &HandlerExecution,
        phase: &str,
        reuse: ExecutionReuse,
        envelope: &vibe_wire::generated::lifecycle::e1::context::Context,
        inputs: &crate::state::PreparedFingerprint,
        agent: Option<&crate::agent::PreparedAgent>,
        runtime: &crate::handlers::HandlerRuntime<'_>,
    ) -> Result<Option<crate::state::current::ExecutionRecord>, super::LifecycleRunError> {
        use crate::InvocationIntent;
        use vibe_core::manifest::DeclaredEffect;
        let effects = known_effects(execution)?;
        let intent =
            InvocationIntent::from_requested(&envelope.run.requested).ok_or_else(|| {
                InvocationRefusal::UnknownInvocation {
                    requested: envelope.run.requested.clone(),
                }
            })?;
        let production =
            effects.contains(DeclaredEffect::ProductSource) && !intent.admits_product_source();
        let build =
            effects.contains(DeclaredEffect::BuildArtifact) && !intent.admits_build_artifact();
        let key = execution.key();
        if production || build {
            let result = self.state.as_ref().and_then(|state| state.prior(&key));
            let cause = match result {
                Some(record) => comparable_completion(
                    record,
                    inputs,
                    self.session
                        .as_ref()
                        .ok_or(super::LifecycleRunError::Unbound)?
                        .metadata()
                        .selected
                        .as_str(),
                    &key,
                    phase,
                    &envelope.project.root,
                )
                .err(),
                None => Some("completed-record-unavailable"),
            };
            if let Some(reason) = cause {
                return Err(if production {
                    InvocationRefusal::CreationRequired { key, reason }
                } else {
                    InvocationRefusal::BuildRequired { key, reason }
                }
                .into());
            }
            return Ok(result.cloned());
        }
        // A verification observer is an actual re-observation/retest, never
        // fresh source production and never hidden behind a cached flag.
        if intent == InvocationIntent::Verify && effects.contains(DeclaredEffect::EvidenceObserver)
        {
            return Ok(None);
        }
        let hosted = agent.is_some()
            && envelope.run.agent_mode
                == vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Agent;
        let prior = ((reuse == ExecutionReuse::FreshnessAware || hosted) && !self.force)
            .then(|| {
                self.state
                    .as_ref()
                    .and_then(|state| state.reusable_record(&key, &inputs.fingerprint))
                    .cloned()
            })
            .flatten();
        Ok(prior.filter(|record| {
            let legacy: Vec<vibe_wire::generated::lifecycle_state::StateArtifact> =
                match serde_json::to_value(&record.artifacts)
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                {
                    Some(rows) => rows,
                    None => return false,
                };
            if let Some(agent) = agent {
                return crate::agent::probe_outputs(
                    std::path::Path::new(&envelope.project.root),
                    agent.contract(),
                    &legacy,
                );
            }
            let vibe_core::manifest::ExtensionHandler::Builtin { name } =
                &execution.row().declaration().handler
            else {
                return true;
            };
            !crate::BuiltinRegistry::is_package_binding(name, execution.row())
                || runtime
                    .package_binding
                    .probe(&key, &legacy)
                    .unwrap_or(false)
        }))
    }
}

#[cfg(test)]
mod m_03_a_1_cases {
    use crate::state::{current, io};
    use std::path::Path;
    const CURRENT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/lifecycle-state/e2/classified-success.json"
    ));
    fn bytes(raw: &str) -> Vec<u8> {
        let value: current::LifecycleState = serde_json::from_str(raw).unwrap();
        toml::to_string(&value).unwrap().into_bytes()
    }
    #[test]
    fn current_codec_keeps_original_subject_and_rejects_legacy_basis_smuggling() {
        let data = bytes(CURRENT);
        let state = io::decode(&data, Path::new("owned/lifecycle.toml")).unwrap();
        let basis = state.execution["org.demo/consumer#produce"]
            .completion_basis
            .as_ref()
            .unwrap();
        assert_eq!(basis.selected, ".");
        assert_eq!(basis.run_id, "00112233445566778899aabbccddeeff");
        assert_eq!(state.schema, 2);
        let mut legacy = state.clone();
        legacy.schema = 1;
        let error = io::decode(
            &toml::to_string(&legacy).unwrap().into_bytes(),
            Path::new("owned/lifecycle.toml"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("completion_basis"), "{error}");
    }
    #[test]
    fn frozen_legacy_history_projects_read_only_without_completion_relabelling() {
        for raw in [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../formats/corpora/lifecycle/e1/state.json"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../formats/corpora/lifecycle/e1/state_measured.json"
            )),
        ] {
            let old: vibe_wire::generated::lifecycle_state::LifecycleState =
                serde_json::from_str(raw).unwrap();
            let original = serde_json::to_value(&old).unwrap();
            let bytes = toml::to_string(&old).unwrap().into_bytes();
            if old.execution.values().any(|row| {
                row.status
                    == vibe_wire::generated::lifecycle_state::ExecutionRecordStatus::Delegated
                    && row.scope.is_none()
            }) {
                // This frozen wire corpus is structurally valid but violates
                // the incumbent engine's typed delegated-scope invariant.
                let error = io::decode(&bytes, Path::new("owned/lifecycle.toml")).unwrap_err();
                assert!(
                    error
                        .to_string()
                        .contains("delegated but carries no typed scope"),
                    "{error}"
                );
                assert_eq!(serde_json::to_value(&old).unwrap(), original);
                continue;
            }
            let current = io::decode(&bytes, Path::new("owned/lifecycle.toml")).unwrap();
            assert_eq!(current.schema, 1);
            assert!(
                current
                    .execution
                    .values()
                    .all(|row| row.completion_basis.is_none())
            );
            assert_eq!(serde_json::to_value(&current).unwrap(), original);
        }
    }

    #[test]
    fn registered_current_corpus_keeps_closed_fields_epochs_and_legacy_boundary() {
        let unknown = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/lifecycle-state/e2/unknown-field.json"
        ));
        let error = serde_json::from_str::<current::LifecycleState>(unknown).unwrap_err();
        assert!(
            error.to_string().contains("unknown field `foreign`"),
            "{error}"
        );
        let unknown = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/lifecycle-state/e2/unknown-epoch.json"
        ));
        let error = io::decode(&bytes(unknown), Path::new("owned/lifecycle.toml")).unwrap_err();
        assert!(matches!(
            error,
            crate::state::LifecycleStateError::Unsupported { schema: 99, .. }
        ));
        let smuggled = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/lifecycle-state/e2/legacy-smuggled-basis.json"
        ));
        let error = io::decode(&bytes(smuggled), Path::new("owned/lifecycle.toml")).unwrap_err();
        assert!(error.to_string().contains("completion_basis"), "{error}");
    }

    #[test]
    fn failed_or_escaping_original_completion_never_decodes_as_current_proof() {
        for (raw, cause) in [
            (
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../formats/corpora/lifecycle-state/e2/failed-with-basis.json"
                )),
                "inconsistent original completion basis",
            ),
            (
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../formats/corpora/lifecycle-state/e2/escaping-original-selected.json"
                )),
                "inconsistent original completion basis",
            ),
        ] {
            let error = io::decode(&bytes(raw), Path::new("owned/lifecycle.toml")).unwrap_err();
            assert!(error.to_string().contains(cause), "{error}");
        }
    }
}

#[cfg(test)]
#[path = "invocation/m_03_a_1_execution.rs"]
mod m_03_a_1_execution;
