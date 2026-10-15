//! Captured descriptor discovery. This operation cannot issue grants.
use super::{
    MAX_INPUT,
    captured_host::{CapturedHost, selected_manifest},
    hash,
};
use std::{collections::BTreeMap, path::Path};
use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
use vibe_extension_registry::MechanismProvider;
use vibe_wire::generated::{
    client_capability::e1::{error as e, request as q, result as r},
    machine::e1::operation_envelope as env,
};
use vibe_workspace::source_query::SourceQueryService;

pub const OPERATION: &str = "rust:vibe-orchestrator:client-capabilities";
pub const SOURCE: &str = "captured-client-capabilities/1";
pub struct ClientCapabilitiesHost {
    captured: CapturedHost,
    target: String,
    prepared: Result<(SourceQueryService, r::Result), String>,
}
impl ClientCapabilitiesHost {
    pub fn new(root: &Path, prepare: bool) -> anyhow::Result<Self> {
        let captured = CapturedHost::new(root)?;
        let target = hash(format!("{}:clients", captured.context).as_bytes());
        let prepared = if prepare {
            Self::prepare(&captured, &target).map_err(|e| e.to_string())
        } else {
            Err("explicit source preparation required".into())
        };
        Ok(Self {
            captured,
            target,
            prepared,
        })
    }
    fn prepare(
        captured: &CapturedHost,
        target: &str,
    ) -> anyhow::Result<(SourceQueryService, r::Result)> {
        let source = captured.prepare()?;
        let root = captured.host.root();
        let workspace = vibe_workspace::Workspace::discover(root)?;
        let loaded = crate::world::capture_admission_metadata(root, &workspace)?;
        let selected =
            vibe_extension_registry::client_requests::client_requests(&loaded.mechanisms)
                .map_err(anyhow::Error::msg)?;
        let mut rows = Vec::new();
        let mut witnesses = BTreeMap::new();
        for (id, row) in selected {
            let request = row
                .declaration()
                .client_request
                .as_ref()
                .expect("selected request");
            let (text, strength) = match row.provider() {
                MechanismProvider::Dependency(p) => (
                    selected_manifest(
                        &source,
                        p.id.group().as_str(),
                        p.id.name().as_str(),
                        &p.root,
                        &p.version,
                        false,
                    )?,
                    "lock-bound",
                ),
                MechanismProvider::Host(p) => {
                    let text = match &p.identity {
                        vibe_lifecycle::HostIdentity::Coordinate(id) => selected_manifest(
                            &source,
                            id.group().as_str(),
                            id.name().as_str(),
                            &p.root,
                            &p.version,
                            true,
                        )?,
                        vibe_lifecycle::HostIdentity::UngroupedProject(name) => {
                            anyhow::ensure!(p.root == root, "foreign host root");
                            let text = source
                                .captured_manifest_at(root)
                                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
                            let manifest = vibe_core::manifest::Manifest::parse_str(&text)?;
                            anyhow::ensure!(
                                manifest
                                    .project
                                    .as_ref()
                                    .is_some_and(|project| project.name == *name
                                        && project.group.is_none()
                                        && project.version == p.version),
                                "host identity mismatch"
                            );
                            text
                        }
                        _ => anyhow::bail!("virtual host cannot declare clients"),
                    };
                    (text, "observed")
                }
                MechanismProvider::Builtin => anyhow::bail!("no builtin client descriptor"),
            };
            let manifest = vibe_core::manifest::Manifest::parse_str(&text)?;
            let declaration = manifest
                .mechanism_decls
                .get(row.declaration_ordinal())
                .ok_or_else(|| anyhow::anyhow!("missing captured declaration"))?;
            anyhow::ensure!(
                declaration == row.declaration(),
                "descriptor changed between registry and capture"
            );
            let descriptor_digest = hash(&serde_json::to_vec(request)?);
            witnesses.insert(id, (hash(text.as_bytes()), strength));
            rows.push(ClientCapability {
                provider: row.pin().to_string(),
                descriptor_digest,
                request: request.clone(),
                disabled: row.is_disabled(),
            });
        }
        let map = ClientCapabilities::admit(rows).map_err(anyhow::Error::msg)?;
        let mut wire = BTreeMap::new();
        for (id, row) in map.rows() {
            let (manifest_digest, strength) = &witnesses[id];
            wire.insert(id, serde_json::json!({"provider":row.provider,"descriptor_digest":row.descriptor_digest,"manifest_digest":manifest_digest,"version":row.request.version,"source_strength":strength,"disabled":row.disabled,"authorization":"not-granted","implementation":"not-evaluated","availability":"not-probed","bindings":row.request.bindings}));
        }
        let capture_id = hash(&serde_json::to_vec(&(&captured.context, target, &wire))?);
        let result: r::Result = serde_json::from_value(
            serde_json::json!({"schema":1,"context_id":captured.context,"target_id":target,"capture_id":capture_id,"root":root.to_string_lossy(),"principal":"not-admitted","capabilities":wire}),
        )?;
        vibe_wire::behaviour::client_capability::result(&result).map_err(anyhow::Error::msg)?;
        source.revalidate().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok((source, result))
    }
    pub fn context_id(&self) -> &str {
        &self.captured.context
    }
    pub fn target_id(&self) -> &str {
        &self.target
    }
    pub fn describe(&self) -> env::OperationEnvelope {
        self.exchange(
            &serde_json::to_vec(&input(&q::Request {
                schema: 1,
                context_id: self.context_id().into(),
                target_id: self.target_id().into(),
                action: q::RequestAction::Describe,
            }))
            .expect("request"),
        )
    }
    pub fn exchange(&self, bytes: &[u8]) -> env::OperationEnvelope {
        let answer = (|| -> Result<r::Result, (&str, String)> {
            if bytes.len() > MAX_INPUT {
                return Err(("resource-limit", "input bound".into()));
            }
            let input: env::OperationEnvelope = serde_json::from_slice(bytes)
                .map_err(|_| ("invalid-request", "invalid envelope".into()))?;
            if input.schema != 1
                || input.operation_id.as_deref() != Some(OPERATION)
                || input.domain_owner.as_deref() != Some(super::OWNER)
                || input.source_id.as_deref() != Some(SOURCE)
                || input.evidence_id.is_some()
            {
                return Err(("invalid-request", "foreign envelope".into()));
            }
            let env::Body::Input(body) = input.body else {
                return Err(("invalid-request", "expected input".into()));
            };
            if body.format_id != "client-capability-request" {
                return Err(("invalid-request", "foreign format".into()));
            }
            let request: q::Request = serde_json::from_str(&body.payload_json)
                .map_err(|_| ("invalid-request", "invalid generated request".into()))?;
            if request.schema != 1 {
                return Err(("unsupported-epoch", "expected epoch1".into()));
            }
            if request.context_id != self.context_id() || request.target_id != self.target_id() {
                return Err(("wrong-context", "foreign binding".into()));
            }
            let (source, result) = self
                .prepared
                .as_ref()
                .map_err(|why| ("needs-preparation", why.clone()))?;
            source
                .revalidate()
                .map_err(|_| ("needs-preparation", "stale source".into()))?;
            let result = result.clone();
            source
                .revalidate()
                .map_err(|_| ("needs-preparation", "stale source".into()))?;
            if serde_json::to_vec(&result)
                .map_err(|_| ("invalid-request", "result encoding".into()))?
                .len()
                > super::MAX_OUTPUT
            {
                return Err(("resource-limit", "result bound".into()));
            }
            Ok(result)
        })();
        match answer {
            Ok(result) => envelope(env::Body::Result(Box::new(env::BodyResult {
                format_id: Some("client-capability-result".into()),
                payload_json: Some(serde_json::to_string(&result).expect("result")),
                provenance_id: Some(result.capture_id),
                effect_state: env::BodyResultEffectState::NoEffects,
                completeness: env::BodyResultCompleteness::Complete,
            }))),
            Err((code, message)) => {
                let message: String = message
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(4096)
                    .collect();
                let error: e::Error=serde_json::from_value(serde_json::json!({"schema":1,"code":code,"message":message,"context_id":self.context_id(),"target_id":self.target_id()})).expect("closed error");
                envelope(env::Body::Error(Box::new(env::BodyError {
                    format_id: Some("client-capability-error".into()),
                    payload_json: Some(serde_json::to_string(&error).expect("error")),
                    provenance_id: None,
                    effect_state: env::BodyErrorEffectState::NoEffects,
                    completeness: env::BodyErrorCompleteness::Unavailable,
                })))
            }
        }
    }
}
pub fn envelope(body: env::Body) -> env::OperationEnvelope {
    let value = env::OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION.into()),
        domain_owner: Some(super::OWNER.into()),
        source_id: Some(SOURCE.into()),
        evidence_id: None,
        body,
    };
    if matches!(value.body, env::Body::Result(_) | env::Body::Error(_)) {
        vibe_wire::behaviour::client_capability::publication(&value)
            .expect("owner publication invariant");
    }
    value
}
pub fn input(request: &q::Request) -> env::OperationEnvelope {
    envelope(env::Body::Input(Box::new(env::BodyInput {
        format_id: "client-capability-request".into(),
        payload_json: serde_json::to_string(request).expect("request"),
    })))
}
#[cfg(test)]
mod tests;
