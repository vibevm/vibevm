//! Host-owned prepared adapter queries. Wire requests carry correlation only.
use crate::document_analysis::{
    AnalysisCapture, AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_agent_projection::pkgskill::PreparedProjectSkills;
use vibe_wire::{
    behaviour::adapter_plan as validate,
    generated::{
        adapter_plan::e1::{error as e, request as q, result as r},
        machine::e1::operation_envelope as env,
    },
};
use vibe_workspace::source_query::SourceQueryService;

pub const OPERATION: &str = "rust:vibe-orchestrator:adapter-plan";
pub const OWNER: &str = "rust:vibe-orchestrator";
pub const SOURCE: &str = "prepared-adapter-plan/1";
pub const MAX_INPUT: usize = 65_537;
const MAX_OUTPUT: usize = 8 * 1024 * 1024;
type Answer<T> = Result<T, e::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterScope {
    Project,
    User,
    Both,
}
#[derive(Debug, Clone)]
pub enum AdapterTarget {
    Skills {
        scope: AdapterScope,
        user_home: Option<PathBuf>,
    },
    Deploy {
        profile: String,
    },
}

pub fn missing() -> e::Error {
    fault(
        e::ErrorCode::NeedsPreparation,
        Some(e::ErrorReason::SourceCaptureMissing),
        "explicit source preparation is required",
    )
}
fn fault(code: e::ErrorCode, reason: Option<e::ErrorReason>, message: &str) -> e::Error {
    e::Error {
        schema: 1,
        code,
        reason,
        message: message.into(),
        context_id: None,
        target_id: None,
        capture_id: None,
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Preparation failures occur before the query channel. Effects may have
/// occurred in the incumbent source/cache owner; Err never proves NoEffects.
#[derive(Debug, thiserror::Error)]
#[error("adapter preparation failed (effects unknown): {detail}")]
pub struct AdapterPreparationFailure {
    pub detail: String,
}

/// Inputs admitted by the command owner; executable lookup occurs only while
/// constructing this explicit preparation request.
pub struct AdapterDeployPreparation<'a> {
    pub targets: &'a [vibe_core::manifest::DeployTarget],
    pub os: vibe_core::manifest::TargetOs,
    pub selection: &'a vibe_lifecycle::DeploySelection,
    pub state_home: &'a Path,
    pub settings_root: &'a Path,
    pub user_home: &'a Path,
    pub clients: &'a vibe_lifecycle::ClientExecutables,
    pub project: &'a str,
}

pub struct AdapterPlanHost {
    host: Arc<DocumentAnalysisHost>,
    capture: AnalysisCapture,
    target: AdapterTarget,
    context: String,
    target_id: String,
}
pub struct PreparedAdapterSession {
    context: String,
    target_id: String,
    id: String,
    source: SourceQueryService,
    skills: PreparedProjectSkills,
    deploy: Option<vibe_lifecycle::PreparedDeployQuery>,
    world: crate::world::LoadedRegistry,
}
pub struct AdapterPlanRoute {
    host: AdapterPlanHost,
    prepared: Option<PreparedAdapterSession>,
}

impl AdapterPlanHost {
    pub fn new(root: &Path, target: AdapterTarget) -> anyhow::Result<Self> {
        if let AdapterTarget::Skills { scope, user_home } = &target {
            anyhow::ensure!(
                *scope == AdapterScope::Project
                    || user_home.as_ref().is_some_and(|p| p.is_absolute()),
                "user scope requires a host-bound absolute home"
            );
        }
        let host = DocumentAnalysisHost::new(
            root,
            "vibe.toml",
            vec![ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            }],
            AnalysisLimits {
                resource_bytes: 1024 * 1024,
                capture_bytes: 1024 * 1024,
                open_buffers: 1,
            },
        )?;
        let capture = host.client()?.freeze()?;
        let context = format!("adapter-context:{}", capture.context());
        let target_id = hash(format!("{context}:{target:?}").as_bytes());
        Ok(Self {
            host,
            capture,
            target,
            context,
            target_id,
        })
    }

    pub fn prepare(
        &self,
        offline: bool,
    ) -> Result<PreparedAdapterSession, AdapterPreparationFailure> {
        if matches!(self.target, AdapterTarget::Deploy { .. }) {
            return Err(AdapterPreparationFailure {
                detail: "deploy requires explicit client preparation inputs".into(),
            });
        }
        self.prepare_source(offline)
    }
    pub fn prepare_deploy(
        &self,
        offline: bool,
        inputs: AdapterDeployPreparation<'_>,
    ) -> Result<PreparedAdapterSession, AdapterPreparationFailure> {
        self.prepare_deploy_with_client_state_owner(offline, inputs, None)
    }
    /// Trusted host injection, never supplied by an adapter query.
    pub fn prepare_deploy_with_client_state_owner(
        &self,
        offline: bool,
        inputs: AdapterDeployPreparation<'_>,
        owner: Option<Arc<dyn vibe_lifecycle::ClientStateOwner>>,
    ) -> Result<PreparedAdapterSession, AdapterPreparationFailure> {
        let AdapterTarget::Deploy { profile } = &self.target else {
            return Err(AdapterPreparationFailure {
                detail: "host is not a deploy target".into(),
            });
        };
        if inputs.selection.profile != *profile {
            return Err(AdapterPreparationFailure {
                detail: "foreign deploy preparation target".into(),
            });
        }
        let mut capture = self.prepare_source(offline)?;
        let validate_target = || -> anyhow::Result<()> {
            let manifest = vibe_core::manifest::Manifest::parse_str(
                &capture
                    .source
                    .captured_manifest_at(self.host.root())
                    .map_err(|error| anyhow::anyhow!("{error:?}"))?,
            )?;
            let deploy = manifest
                .deploy
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("captured manifest has no deploy section"))?;
            anyhow::ensure!(
                deploy.targets == inputs.targets,
                "deploy targets differ from captured declaration"
            );
            let selected = deploy.project_profile(profile, inputs.os)?;
            anyhow::ensure!(
                selected.active_targets() == inputs.selection.targets,
                "deploy selection differs from captured profile"
            );
            Ok(())
        };
        validate_target().map_err(|error| AdapterPreparationFailure {
            detail: error.to_string(),
        })?;
        let execution = vibe_lifecycle::DeployExecution {
            project_root: self.host.root(),
            targets: inputs.targets,
            selection: inputs.selection,
            registry: &capture.world.mechanisms,
            routes: &capture.world.mechanism_routes,
            state_home: inputs.state_home,
            settings_root: inputs.settings_root,
            user_home: inputs.user_home,
            clients: inputs.clients,
            project: inputs.project,
            package: None,
            created_at: "",
        };
        capture.deploy = Some(
            vibe_lifecycle::prepare_deploy_query_with_client_state_owner(&execution, owner)
                .map_err(|error| AdapterPreparationFailure {
                    detail: error.to_string(),
                })?,
        );
        capture.id = hash(
            format!(
                "{}:{}",
                capture.id,
                capture
                    .deploy
                    .as_ref()
                    .expect("prepared deploy")
                    .evidence_identity()
            )
            .as_bytes(),
        );
        Ok(capture)
    }
    fn prepare_source(
        &self,
        offline: bool,
    ) -> Result<PreparedAdapterSession, AdapterPreparationFailure> {
        let prepare = || -> anyhow::Result<PreparedAdapterSession> {
            let source = SourceQueryService::prepare_host(self.host.root())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            let manifest = source
                .captured_manifest_at(self.host.root())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            let resource = self.host.resource("vibe.toml").expect("host declaration");
            anyhow::ensure!(
                self.capture.resource(resource).and_then(|r| r.raw()) == Some(manifest.as_bytes()),
                "host manifest changed before preparation"
            );
            let mut loaded = crate::world::prepare_adapter_sources(self.host.root(), offline)?;
            let skills = loaded
                .adapter_sources
                .take()
                .expect("explicit source preparation");
            let bindings = skills.plan()?;
            for binding in &bindings {
                let (group, name, root, version, expected_self) = match &binding.skill.provider {
                    vibe_agent_projection::pkgskill::DeclaredSkillProvider::Authored {
                        group,
                        name,
                        root,
                        version,
                        ..
                    } => (group, name, root, version, true),
                    vibe_agent_projection::pkgskill::DeclaredSkillProvider::Installed {
                        group,
                        name,
                        root,
                        version,
                        ..
                    } => (group, name, root, version, false),
                };
                let selected = source
                    .selected_doc_instance(group.as_str(), name.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("skill provider is outside selected source world")
                    })?;
                anyhow::ensure!(
                    selected.root == *root
                        && (expected_self || selected.version == *version)
                        && selected.is_self == expected_self,
                    "selected provider mismatch"
                );
                source
                    .captured_manifest_at(root)
                    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            }
            source.revalidate().map_err(|e| anyhow::anyhow!("{e:?}"))?;
            let basis: Vec<_> = bindings
                .iter()
                .map(|b| (b.identity(), b.source_snapshot.clone()))
                .collect();
            let id = hash(&serde_json::to_vec(&(
                &self.context,
                &self.target_id,
                basis,
            ))?);
            Ok(PreparedAdapterSession {
                context: self.context.clone(),
                target_id: self.target_id.clone(),
                id,
                source,
                skills,
                deploy: None,
                world: loaded,
            })
        };
        prepare().map_err(|error| AdapterPreparationFailure {
            detail: error.to_string(),
        })
    }
}

impl AdapterPlanRoute {
    pub fn new(host: AdapterPlanHost, prepared: Option<PreparedAdapterSession>) -> Answer<Self> {
        if prepared
            .as_ref()
            .is_some_and(|p| p.context != host.context || p.target_id != host.target_id)
        {
            return Err(fault(
                e::ErrorCode::WrongContext,
                None,
                "foreign prepared session",
            ));
        }
        Ok(Self { host, prepared })
    }
    pub fn root(&self) -> &Path {
        self.host.host.root()
    }
    pub fn context_id(&self) -> &str {
        &self.host.context
    }
    pub fn target_id(&self) -> &str {
        &self.host.target_id
    }
    pub fn capture_id(&self) -> Option<&str> {
        self.prepared.as_ref().map(|p| p.id.as_str())
    }
    pub fn compare_context_root(&self, root: &Path) -> Answer<()> {
        if self.root() != root {
            return Err(self.bound(fault(
                e::ErrorCode::WrongContext,
                None,
                "host root mismatch",
            )));
        }
        Ok(())
    }
    fn bound(&self, mut error: e::Error) -> e::Error {
        error.context_id = Some(self.context_id().into());
        error.target_id = Some(self.target_id().into());
        error
    }
    pub fn request(&self, request: q::Request) -> Answer<r::Result> {
        let answer = self.request_inner(request.clone()).and_then(|result| {
            validate::result_for(&request, &result)
                .map_err(|_| fault(e::ErrorCode::Internal, None, "foreign result binding"))?;
            Ok(result)
        });
        answer.map_err(|error| self.bound(error))
    }
    fn request_inner(&self, request: q::Request) -> Answer<r::Result> {
        if request.schema != 1 {
            return Err(fault(
                e::ErrorCode::UnsupportedEpoch,
                None,
                "expected epoch 1",
            ));
        }
        validate::request(&request)
            .map_err(|_| fault(e::ErrorCode::InvalidRequest, None, "invalid request"))?;
        if request.context_id != self.context_id() {
            return Err(fault(e::ErrorCode::WrongContext, None, "foreign context"));
        }
        if request.target_id != self.target_id() {
            return Err(fault(e::ErrorCode::WrongTarget, None, "foreign target"));
        }
        let body = match request.command {
            q::Command::Describe(_) => {
                let (adapter_kind, scope, profile) = match &self.host.target {
                    AdapterTarget::Skills { scope, .. } => (
                        r::BodySessionAdapterKind::Skills,
                        match scope {
                            AdapterScope::Project => r::BodySessionScope::Project,
                            AdapterScope::User => r::BodySessionScope::User,
                            AdapterScope::Both => r::BodySessionScope::Both,
                        },
                        None,
                    ),
                    AdapterTarget::Deploy { profile } => (
                        r::BodySessionAdapterKind::Deploy,
                        r::BodySessionScope::Project,
                        Some(profile.clone()),
                    ),
                };
                r::Body::Session(Box::new(r::BodySession {
                    adapter_kind,
                    scope,
                    profile,
                    preparation: if self.prepared.is_some() {
                        r::BodySessionPreparation::Ready
                    } else {
                        r::BodySessionPreparation::Missing
                    },
                    source_evidence: self.capture_id().into_iter().map(str::to_owned).collect(),
                    client_evidence: self
                        .prepared
                        .as_ref()
                        .filter(|p| p.deploy.is_some())
                        .map(|p| vec![p.id.clone()])
                        .unwrap_or_default(),
                    complete: true,
                }))
            }
            q::Command::Plan(plan) => {
                let prepared = self.prepared.as_ref().ok_or_else(missing)?;
                let requested = plan.capture_id.as_deref().ok_or_else(missing)?;
                if requested != prepared.id {
                    return Err(fault(
                        e::ErrorCode::UnknownCapture,
                        None,
                        "capture not retained by this host",
                    ));
                }
                let stale = || {
                    fault(
                        e::ErrorCode::NeedsPreparation,
                        Some(e::ErrorReason::SourceCaptureStale),
                        "selected source observation changed",
                    )
                };
                prepared.source.revalidate().map_err(|_| stale())?;
                let bindings = prepared.skills.plan().map_err(|_| stale())?;
                if let AdapterTarget::Deploy { .. } = &self.host.target {
                    let captured = prepared.deploy.as_ref().ok_or_else(|| {
                        fault(
                            e::ErrorCode::NeedsPreparation,
                            Some(e::ErrorReason::ClientCaptureMissing),
                            "client capture missing",
                        )
                    })?;
                    let plans = captured.plan().map_err(|error| {
                        let reason = match error {
                            vibe_lifecycle::DeployError::Provider(vibe_lifecycle::MechanismError::Deploy(vibe_lifecycle::DeployProviderError::NeedsPreparation { reason, .. })) => reason,
                            _ => e::ErrorReason::ClientCaptureStale,
                        };
                        fault(
                            e::ErrorCode::NeedsPreparation,
                            Some(reason),
                            "deploy currentness unavailable or changed; explicit preparation evidence remains historical",
                        )
                    })?;
                    let rows = plans
                        .into_iter()
                        .map(|plan| {
                            r::Row::Deploy(Box::new(r::RowDeploy {
                                target: plan.report.target,
                                provider: plan.report.provider,
                                config_digest: plan.config_digest,
                                fingerprint: plan.fingerprint,
                                resources: plan
                                    .report
                                    .resources
                                    .into_iter()
                                    .map(|resource| r::Resource {
                                        resource: resource.resource,
                                        desired_digest: resource.desired_digest,
                                        recorded_digest: resource.recorded_digest,
                                    })
                                    .collect(),
                                client_evidence: vec![prepared.id.clone()],
                                reversible: plan.reversible,
                            }))
                        })
                        .collect();
                    return self.publish(r::Body::Plan(Box::new(r::BodyPlan {
                        rows,
                        complete: true,
                    })));
                }
                let AdapterTarget::Skills { scope, user_home } = &self.host.target else {
                    unreachable!()
                };
                let mut rows = Vec::new();
                for binding in bindings {
                    for target in &binding.targets {
                        for user in [false, true] {
                            if (user && *scope == AdapterScope::Project)
                                || (!user && *scope == AdapterScope::User)
                            {
                                continue;
                            }
                            let destination = if user {
                                target
                                    .agent
                                    .user_skills_root_from_home(
                                        user_home.as_deref().ok_or_else(missing)?,
                                    )
                                    .ok_or_else(missing)?
                                    .join(&binding.skill.decl.name)
                            } else {
                                target.path.clone()
                            };
                            let files = binding.selected_files.as_ref().ok_or_else(missing)?;
                            rows.push(r::Row::Skill(Box::new(r::RowSkill {
                                provider: binding.skill.provider.identity(),
                                declaration: binding.skill.decl.name.clone(),
                                agent: target.agent.as_str().into(),
                                scope: if user {
                                    r::RowSkillScope::User
                                } else {
                                    r::RowSkillScope::Project
                                },
                                target: destination
                                    .strip_prefix(if user {
                                        user_home.as_deref().ok_or_else(missing)?
                                    } else {
                                        self.root()
                                    })
                                    .map_err(|_| stale())?
                                    .to_string_lossy()
                                    .replace('\\', "/"),
                                ownership: r::RowSkillOwnership::Unavailable,
                                source_digest: binding.source_snapshot.clone(),
                                proof: r::RowSkillProof::Observed,
                                files: files
                                    .iter()
                                    .map(|(path, bytes)| r::File {
                                        path: path.clone(),
                                        sha256: format!("{:x}", Sha256::digest(bytes)),
                                        bytes: bytes.len().to_string(),
                                    })
                                    .collect(),
                            })));
                        }
                    }
                }
                r::Body::Plan(Box::new(r::BodyPlan {
                    rows,
                    complete: true,
                }))
            }
        };
        self.publish(body)
    }
    fn publish(&self, body: r::Body) -> Answer<r::Result> {
        let result = r::Result {
            schema: 1,
            context_id: self.context_id().into(),
            target_id: self.target_id().into(),
            capture_id: self.capture_id().map(str::to_owned),
            result_id: hash(
                serde_json::to_string(&body)
                    .expect("generated body")
                    .as_bytes(),
            ),
            body,
        };
        validate::result(&result)
            .map_err(|_| fault(e::ErrorCode::Internal, None, "invalid owner result"))?;
        Ok(result)
    }
    pub fn exchange(&self, bytes: &[u8]) -> env::OperationEnvelope {
        let answer = (|| {
            if bytes.len() > MAX_INPUT {
                return Err(fault(
                    e::ErrorCode::ResourceLimit,
                    None,
                    "input bound exceeded",
                ));
            }
            let input: env::OperationEnvelope = serde_json::from_slice(bytes)
                .map_err(|_| fault(e::ErrorCode::InvalidRequest, None, "invalid envelope"))?;
            if input.schema != 1
                || input.operation_id.as_deref() != Some(OPERATION)
                || input.domain_owner.as_deref() != Some(OWNER)
                || input.source_id.as_deref() != Some(SOURCE)
                || input.evidence_id.is_some()
            {
                return Err(fault(
                    e::ErrorCode::InvalidRequest,
                    None,
                    "foreign envelope binding",
                ));
            }
            let env::Body::Input(body) = input.body else {
                return Err(fault(e::ErrorCode::InvalidRequest, None, "expected input"));
            };
            if body.format_id != "adapter-plan-request" {
                return Err(fault(e::ErrorCode::InvalidRequest, None, "foreign format"));
            }
            let request = validate::read_request(body.payload_json.as_bytes()).map_err(|_| {
                fault(
                    e::ErrorCode::InvalidRequest,
                    None,
                    "invalid generated request",
                )
            })?;
            self.request(request)
        })();
        match answer {
            Ok(result) => {
                let json = serde_json::to_string(&result).expect("generated result");
                if json.len() > MAX_OUTPUT {
                    return refusal(self.bound(fault(
                        e::ErrorCode::ResourceLimit,
                        None,
                        "result bound exceeded",
                    )));
                }
                envelope(env::Body::Result(Box::new(env::BodyResult {
                    format_id: Some("adapter-plan-result".into()),
                    payload_json: Some(json),
                    effect_state: env::BodyResultEffectState::NoEffects,
                    completeness: env::BodyResultCompleteness::Complete,
                    provenance_id: result.capture_id,
                })))
            }
            Err(error) => refusal(self.bound(error)),
        }
    }
}
pub fn envelope(body: env::Body) -> env::OperationEnvelope {
    let value = env::OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION.into()),
        domain_owner: Some(OWNER.into()),
        source_id: Some(SOURCE.into()),
        evidence_id: None,
        body,
    };
    if matches!(value.body, env::Body::Result(_) | env::Body::Error(_)) {
        validate::publication(&value).expect("owner publication invariant");
    }
    value
}
pub fn input(request: &q::Request) -> env::OperationEnvelope {
    envelope(env::Body::Input(Box::new(env::BodyInput {
        format_id: "adapter-plan-request".into(),
        payload_json: serde_json::to_string(request).expect("generated request"),
    })))
}
pub fn refusal(error: e::Error) -> env::OperationEnvelope {
    validate::error(&error).expect("owner error invariant");
    envelope(env::Body::Error(Box::new(env::BodyError {
        format_id: Some("adapter-plan-error".into()),
        payload_json: Some(serde_json::to_string(&error).expect("generated error")),
        effect_state: env::BodyErrorEffectState::NoEffects,
        completeness: env::BodyErrorCompleteness::Unavailable,
        provenance_id: error.capture_id,
    })))
}

#[cfg(test)]
mod tests;
