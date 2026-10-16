//! Narrow registered client route; no incoming value constructs a host grant.
use crate::machine_submission::{SubmissionOutcome, client_deploy::Journal, submit};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
    client_deploy::{Approval, Cancellation, Host, Prepared},
};
use vibe_wire::{
    behaviour::client_deploy as wire,
    generated::{
        client_deploy::e1::{
            reply::{Reply, ReplyAction, ReplyStatus},
            request::{Request, RequestAction},
        },
        machine::e1::operation_envelope as e,
    },
};
pub const OPERATION_ID: &str = "rust:vibe-lifecycle:client-deploy";
pub const DOMAIN_OWNER: &str = "rust:vibe-lifecycle";
pub struct ClientDeployRoute {
    host: Arc<Host>,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    plans: Mutex<BTreeMap<String, (Prepared, Option<Approval>)>>,
    active: Mutex<Option<(String, String, Cancellation)>>,
    clock: String,
}
impl ClientDeployRoute {
    pub fn new(
        host: Arc<Host>,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        clock: String,
    ) -> Result<Self, String> {
        let lease = LifecycleLease::acquire(host.root()).map_err(|e| e.to_string())?;
        SubmissionReservations::validate_namespace(&lease, &namespace, &scope)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            host,
            namespace,
            scope,
            plans: Mutex::new(BTreeMap::new()),
            active: Mutex::new(None),
            clock,
        })
    }
    pub fn register(&self, plan: Prepared, approval: Option<Approval>) -> Result<String, String> {
        let id = plan.identity().to_owned();
        self.plans
            .lock()
            .map_err(|_| "plan lock")?
            .insert(id.clone(), (plan, approval));
        Ok(id)
    }
    pub fn compare_context_root(&self, root: &Path) -> Result<(), String> {
        let found = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
        let expected = vibe_safefs::Project::open(self.host.root()).map_err(|e| e.to_string())?;
        if found.root_correlation_token().map_err(|e| e.to_string())?
            != expected
                .root_correlation_token()
                .map_err(|e| e.to_string())?
        {
            return Err("foreign context root".into());
        }
        Ok(())
    }
    pub fn root(&self) -> &Path {
        self.host.root()
    }
    pub fn dispatch(&self, q: &Request) -> Result<Reply, String> {
        wire::read_request(&serde_json::to_vec(q).map_err(|e| e.to_string())?)
            .map_err(str::to_owned)?;
        if q.action == RequestAction::Cancel {
            let active = self.active.lock().map_err(|_| "control lock")?;
            if let Some((key, plan, cancel)) = active
                .as_ref()
                .filter(|(key, plan, _)| key == &q.key && plan == &q.plan_id)
            {
                let run = self.host.proposed_run(key);
                if q.run_id.as_deref().is_some_and(|id| id != run) {
                    return Err("foreign cancellation run".into());
                }
                cancel.request();
                let (mut r, _) = self.host.inspect(&run)?.ok_or("pending initialization")?;
                r.cancel_pending = true;
                r.status = ReplyStatus::Unknown;
                r.message =
                    "cancellation pending; opaque call has not reached a safe boundary".into();
                let _ = plan;
                return Ok(r);
            }
        }
        let lease = Arc::new(LifecycleLease::acquire(self.host.root()).map_err(|e| e.to_string())?);
        SubmissionReservations::validate_namespace(&lease, &self.namespace, &self.scope)
            .map_err(|e| e.to_string())?;
        if matches!(
            q.action,
            RequestAction::Inspect
                | RequestAction::Verify
                | RequestAction::Cancel
                | RequestAction::Recover
        ) {
            let pointer = SubmissionReservations::inspect_existing_key(
                &lease,
                &self.namespace,
                &self.scope,
                &q.key,
            )
            .map_err(|e| e.to_string())?
            .ok_or("unknown key")?;
            if q.run_id
                .as_deref()
                .is_some_and(|v| v != pointer.domain_run_id)
            {
                return Err("foreign run identity".into());
            }
            let (mut r, _) = self
                .host
                .inspect(&pointer.domain_run_id)?
                .ok_or("missing retained run")?;
            if q.plan_id != r.plan_id {
                return Err("foreign plan identity".into());
            }
            if q.action == RequestAction::Verify {
                return self.host.verify(&pointer.domain_run_id);
            }
            if q.action == RequestAction::Cancel {
                return self.host.cancel_at_boundary(&pointer.domain_run_id);
            }
            if q.action == RequestAction::Inspect {
                return Ok(r);
            }
            let plans = self.plans.lock().map_err(|_| "plan lock")?;
            let Some((p, g)) = plans.get(&q.plan_id) else {
                r.status = ReplyStatus::NeedsAuthority;
                return Ok(r);
            };
            // Explicit continuation of the reserved run; never call submit again.
            let cancel = Cancellation::default();
            *self.active.lock().map_err(|_| "control lock")? =
                Some((q.key.clone(), q.plan_id.clone(), cancel.clone()));
            let result = self.host.execute(
                p,
                g.as_ref(),
                &pointer.domain_run_id,
                true,
                &cancel,
                &self.clock,
            );
            *self.active.lock().map_err(|_| "control lock")? = None;
            return result;
        }
        let plans = self.plans.lock().map_err(|_| "plan lock")?;
        let Some((p, g)) = plans.get(&q.plan_id) else {
            if q.action == RequestAction::Plan {
                return Ok(Reply {
                    schema: 1,
                    action: ReplyAction::Plan,
                    plan_id: q.plan_id.clone(),
                    run_id: String::new(),
                    status: ReplyStatus::NeedsPreparation,
                    authority:
                        vibe_wire::generated::client_deploy::e1::reply::ReplyAuthority::Required,
                    resources: Vec::new(),
                    steps: Vec::new(),
                    cancel_pending: false,
                    message: "explicit trusted host preparation required".into(),
                });
            }
            return Err("NeedsPreparation: no trusted held plan".into());
        };
        if q.action == RequestAction::Plan {
            let mut reply = self.host.plan(p);
            reply.action = ReplyAction::Plan;
            return Ok(reply);
        }
        let planned = self.host.plan(p);
        let action = match q.action {
            RequestAction::Apply => ReplyAction::Apply,
            RequestAction::Update => ReplyAction::Update,
            RequestAction::Remove => ReplyAction::Remove,
            _ => return Err("action".into()),
        };
        if action != planned.action {
            return Err("action differs from approved footprint".into());
        }
        // Missing or stale approval is typed preflight, before reservation.
        let existing = SubmissionReservations::inspect_existing_key(
            &lease,
            &self.namespace,
            &self.scope,
            &q.key,
        )
        .map_err(|e| e.to_string())?;
        if existing.is_none()
            && let Err(reason) = self.host.validate_approval(p, g.as_ref())
        {
            let mut refusal = planned;
            refusal.status = ReplyStatus::NeedsAuthority;
            refusal.authority =
                vibe_wire::generated::client_deploy::e1::reply::ReplyAuthority::Required;
            refusal.message = reason;
            return Ok(refusal);
        }
        let cancel = Cancellation::default();
        *self.active.lock().map_err(|_| "control lock")? =
            Some((q.key.clone(), q.plan_id.clone(), cancel.clone()));
        let mut journal = Journal {
            host: &self.host,
            plan: p,
            grant: g.as_ref(),
            key: &q.key,
            cancel: &cancel,
            clock: &self.clock,
        };
        let outcome = submit(
            lease,
            &self.namespace,
            &self.scope,
            &q.key,
            &serde_json::to_vec(q).map_err(|e| e.to_string())?,
            p.identity().as_bytes(),
            &mut journal,
        );
        *self.active.lock().map_err(|_| "control lock")? = None;
        match outcome {
            Err(error) => {
                if matches!(
                    error,
                    crate::machine_submission::SubmissionFailure::Reservation(_)
                ) {
                    return Err(error.to_string());
                }
                if let Some((mut retained, _)) =
                    self.host.inspect(&self.host.proposed_run(&q.key))?
                {
                    retained.status = ReplyStatus::Unknown;
                    retained.message = error.to_string();
                    return Ok(retained);
                }
                return Err(error.to_string());
            }
            Ok(
                SubmissionOutcome::Invoked { outcome, .. }
                | SubmissionOutcome::Replayed { outcome, .. },
            ) => Ok(outcome),
            Ok(SubmissionOutcome::RecoveryRequired { run_id }) => {
                let (mut r, _) = self
                    .host
                    .inspect(&run_id)?
                    .ok_or("recovery journal missing")?;
                r.status = ReplyStatus::Unknown;
                r.message = "explicit recovery required".into();
                Ok(r)
            }
            Ok(SubmissionOutcome::Retired { .. }) => Err("retired namespace".into()),
        }
    }
    pub fn exchange_local(
        route: Option<&Self>,
        bytes: &[u8],
        root: &Path,
    ) -> Result<e::OperationEnvelope, String> {
        let mut selected = None;
        let mut uncertain = false;
        let mut known_run = None;
        let mut result = (|| {
            if bytes.len() > 1024 * 1024 {
                return Err("input cap".to_owned());
            }
            let env: e::OperationEnvelope =
                serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if env.schema != 1
                || env.operation_id.as_deref() != Some(OPERATION_ID)
                || env.domain_owner.as_deref() != Some(DOMAIN_OWNER)
                || env.source_id.is_some()
                || env.evidence_id.is_some()
            {
                return Err("foreign envelope identity".into());
            }
            let e::Body::Input(input) = env.body else {
                return Err("expected Input".into());
            };
            if input.format_id != "client-deploy-request" {
                return Err("input format".into());
            }
            let q = wire::read_request(input.payload_json.as_bytes()).map_err(str::to_owned)?;
            let route = route.ok_or("client deployment host unavailable")?;
            route.compare_context_root(root)?;
            selected = Some(q.clone());
            route.dispatch(&q)
        })();
        if let (Err(message), Some(q), Some(route)) = (&result, &selected, route) {
            let original = message.clone();
            let retained = (|| -> Result<Option<Reply>, String> {
                let lease =
                    LifecycleLease::acquire(route.host.root()).map_err(|e| e.to_string())?;
                let Some(pointer) = SubmissionReservations::inspect_existing_key(
                    &lease,
                    &route.namespace,
                    &route.scope,
                    &q.key,
                )
                .map_err(|e| e.to_string())?
                else {
                    return Ok(None);
                };
                known_run = Some(pointer.domain_run_id.clone());
                let (r, _) = route
                    .host
                    .inspect(&pointer.domain_run_id)?
                    .ok_or("retained owner result unavailable")?;
                if r.plan_id != q.plan_id || q.run_id.as_deref().is_some_and(|v| v != r.run_id) {
                    return Ok(None);
                }
                Ok(Some(r))
            })();
            match retained {
                Ok(Some(mut r)) => {
                    r.status = ReplyStatus::Unknown;
                    r.message = original;
                    result = Ok(r)
                }
                Err(_) => uncertain = true,
                Ok(None) => {}
            }
        }
        let body = match result {
            Ok(reply) => {
                wire::validate_reply(&reply).map_err(str::to_owned)?;
                let complete = reply.status == ReplyStatus::Completed;
                let touched = reply.steps.iter().any(|s| {
                    s.status != vibe_wire::generated::client_deploy::e1::reply::StepStatus::Pending
                });
                e::Body::Result(Box::new(e::BodyResult {
                    format_id: Some("client-deploy-reply".into()),
                    payload_json: Some(serde_json::to_string(&reply).map_err(|e| e.to_string())?),
                    provenance_id: (!reply.run_id.is_empty()).then_some(reply.run_id),
                    effect_state: if complete {
                        e::BodyResultEffectState::Completed
                    } else if touched {
                        e::BodyResultEffectState::Uncertain
                    } else {
                        e::BodyResultEffectState::NoEffects
                    },
                    completeness: if complete || reply.status == ReplyStatus::Planned {
                        e::BodyResultCompleteness::Complete
                    } else {
                        e::BodyResultCompleteness::Partial
                    },
                }))
            }
            Err(message) => e::Body::Error(Box::new(e::BodyError {
                format_id: Some("client-deploy-error".into()),
                payload_json: Some(
                    serde_json::json!({"schema":1,"code":"unavailable","message":message,"effect_state":if uncertain{"uncertain"}else{"no_effects"},"run_id":known_run})
                        .to_string(),
                ),
                provenance_id: known_run,
                effect_state: if uncertain{e::BodyErrorEffectState::Uncertain}else{e::BodyErrorEffectState::NoEffects},
                completeness: e::BodyErrorCompleteness::Unavailable,
            })),
        };
        Ok(e::OperationEnvelope {
            schema: 1,
            operation_id: Some(OPERATION_ID.into()),
            domain_owner: Some(DOMAIN_OWNER.into()),
            source_id: None,
            evidence_id: None,
            body,
        })
    }
}
#[cfg(test)]
mod m_08_d_2_cases;
