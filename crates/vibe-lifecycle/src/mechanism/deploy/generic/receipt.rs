//! Retained closed semantics; no executable or config payload in typed records.
use super::super::state::{DeployState, DeploymentHome};
use super::*;
use vibe_wire::behaviour::deploy_generic_state::{RETENTION_LIMIT, semantics_digest, validate};
use vibe_wire::generated::deploy_generic_state as wire;

pub(super) fn stage(
    host: &GenericHost,
    state: &DeployState,
    home: &DeploymentHome,
    plan: &GenericPlan,
    plan_hash: String,
) -> Result<(), GenericError> {
    let mut sidecar =
        state
            .read_generic(home)
            .map_err(refuse)?
            .unwrap_or(wire::DeployGenericState {
                schema: 1,
                committed: None,
                pending: None,
            });
    if sidecar.pending.is_some() {
        return Err(refuse(
            "pending generic state requires fresh recovery grant",
        ));
    }
    let prior = state.read_receipt(home).map_err(refuse)?;
    if let Some(c) = &sidecar.committed {
        let r = prior
            .as_ref()
            .ok_or_else(|| refuse("missing joined receipt"))?;
        if c.receipt_digest.as_deref() != Some(digest(&encoded(r)?).as_str())
            || c.generation != r.generation
        {
            return Err(refuse("changed receipt join"));
        }
        if c.revoked {
            return Err(refuse("receipt revoked"));
        }
    }
    let generation = prior
        .as_ref()
        .map(|r| {
            r.generation
                .checked_add(1)
                .ok_or_else(|| refuse("generation exhausted"))
        })
        .transpose()?
        .unwrap_or(0);
    // Bound aggregate allocations before the first retained write.
    let mut total = 0u64;
    let mut retained = Vec::new();
    for r in plan.resources.iter().chain(&plan.removed) {
        let d = plan
            .documents
            .iter()
            .find(|d| d.scope == r.scope && d.destination == r.destination)
            .ok_or_else(|| refuse("missing planned document"))?;
        let before = plan::observe_value(d.before.as_ref().map(|b| b.bytes.as_slice()), r)?;
        let desired = plan::observe_value(d.after.as_deref(), r)?;
        total = total
            .checked_add(before.as_ref().map_or(0, |b| b.len() as u64))
            .and_then(|n| n.checked_add(desired.as_ref().map_or(0, |b| b.len() as u64)))
            .ok_or_else(|| refuse("retention overflow"))?;
        if total > RETENTION_LIMIT / 2 {
            return Err(refuse("aggregate private retention budget"));
        }
        let mode = d.before.as_ref().and_then(|b| b.unix_mode).unwrap_or(0o600);
        if mode > 0o777 {
            return Err(refuse("special mode unsupported by retained semantics"));
        }
        retained.push((r, before, desired, mode));
    }
    let mut resources = Vec::new();
    for (r, before, desired, mode) in retained {
        let before = if plan.migration {
            wire::Before::Unavailable(Box::new(wire::BeforeUnavailable {
                reason: wire::BeforeUnavailableReason::LegacyNoBeforeState,
            }))
        } else {
            match before {
                None => wire::Before::Absent(Box::new(wire::BeforeAbsent {})),
                Some(bytes) => wire::Before::Retained(Box::new(wire::BeforeRetained {
                    blob: state
                        .retain_generic_blob(home, &bytes, mode)
                        .map_err(refuse)?,
                })),
            }
        };
        resources.push(wire::Resource {
            scope: match r.scope {
                ClientScope::Project => wire::ResourceScope::Project,
                ClientScope::User => wire::ResourceScope::User,
            },
            destination: r.destination.clone(),
            key: r.key.clone(),
            operation: match r.operation {
                vibe_core::manifest::ClientOperation::ReconcileJsonMember => {
                    wire::ResourceOperation::JsonMember
                }
                vibe_core::manifest::ClientOperation::ReconcileTomlMember => {
                    wire::ResourceOperation::TomlMember
                }
                _ => wire::ResourceOperation::File,
            },
            before,
            desired: match desired {
                Some(bytes) => wire::Value::Present(Box::new(wire::ValuePresent {
                    blob: state
                        .retain_generic_blob(home, &bytes, mode)
                        .map_err(refuse)?,
                })),
                None => wire::Value::Absent(Box::new(wire::ValueAbsent {})),
            },
            progress: wire::ResourceProgress::NotStarted,
        });
    }
    let pins = serde_json::from_slice(&encoded(&plan.admitted.capture.observation.identity)?)
        .map_err(refuse)?;
    let mut binding = wire::Binding {
        deployment: home.id().into(),
        coordination_identity: host.arena_identity()?,
        generation,
        plan_hash,
        semantics: wire::BindingSemantics::ClosedFileConfig,
        semantics_digest: String::new(),
        grant_subject: plan.identity(),
        operation: if plan.migration {
            wire::BindingOperation::Adoption
        } else if prior.is_some() {
            wire::BindingOperation::Upgrade
        } else {
            wire::BindingOperation::Apply
        },
        pins,
        resources: resources.clone(),
        revoked: false,
        phase: wire::BindingPhase::Prepared,
        receipt_digest: None,
        transfer: plan
            .transfer
            .as_ref()
            .map(|t| {
                Ok::<_, GenericError>(wire::Transfer {
                    family: t.family.clone(),
                    donor_id: t.donor_id.clone(),
                    donor_root_identity: t.donor_root_identity.clone(),
                    receipt_digest: digest(&t.receipt),
                    hold_id: plan.identity(),
                    phase: wire::TransferPhase::Held,
                    receipt_blob: state
                        .retain_generic_blob(home, &t.receipt, 0o600)
                        .map_err(refuse)?,
                    resources: resources.clone(),
                })
            })
            .transpose()?
            .or_else(|| sidecar.committed.as_ref().and_then(|b| b.transfer.clone())),
    };
    binding.semantics_digest = semantics_digest(&binding).map_err(refuse)?;
    sidecar.pending = Some(binding);
    validate(&sidecar).map_err(refuse)?;
    state.write_generic(home, &sidecar).map_err(refuse)?;
    let _ = host;
    Ok(())
}

pub(super) fn progress(
    host: &GenericHost,
    target: &str,
    destination: &str,
    scope: ClientScope,
    completed: bool,
) -> Result<(), GenericError> {
    progress_member(host, target, destination, scope, None, completed)
}

pub(super) fn progress_member(
    host: &GenericHost,
    target: &str,
    destination: &str,
    scope: ClientScope,
    key: Option<&[String]>,
    completed: bool,
) -> Result<(), GenericError> {
    let state = DeployState::observe_bound(&host.settings)
        .map_err(refuse)?
        .ok_or_else(|| refuse("missing generic state home"))?;
    let home = host.home(target);
    let mut record = state
        .read_generic(&home)
        .map_err(refuse)?
        .ok_or_else(|| refuse("missing generic state"))?;
    let pending = record
        .pending
        .as_mut()
        .ok_or_else(|| refuse("missing pending semantics"))?;
    pending.phase = if completed {
        wire::BindingPhase::Checkpoint
    } else {
        wire::BindingPhase::Publication
    };
    for r in &mut pending.resources {
        if r.destination == destination
            && key.is_none_or(|key| r.key == key)
            && matches!(
                (&r.scope, scope),
                (wire::ResourceScope::Project, ClientScope::Project)
                    | (wire::ResourceScope::User, ClientScope::User)
            )
        {
            r.progress = if completed {
                wire::ResourceProgress::Completed
            } else {
                wire::ResourceProgress::Uncertain
            };
        }
    }
    state.write_generic(&home, &record).map_err(refuse)
}
