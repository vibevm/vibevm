//! In-memory query evidence. This is never apply authority.
use super::*;
use crate::process::{ProcessError, ProcessOutput, ProcessSpec};
use std::path::PathBuf;

#[derive(Clone)]
pub(super) struct Capture {
    binding: String,
    fingerprint_binding: String,
    executable: ExecutableObservation,
    state: Option<StateObservation>,
    pub(super) plan: DeployPlan,
    pub(super) fingerprint: DeployFingerprint,
    // Keep exact arguments, environment, exit status and bounded output, not
    // merely the parsed version. These observations live only in this host.
    observations: Vec<(ProcessSpec, ProcessOutput)>,
}
struct Recorder<'a> {
    runner: &'a dyn ProcessRunner,
    observations: Mutex<Vec<(ProcessSpec, ProcessOutput)>>,
}
impl ProcessRunner for Recorder<'_> {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        let output = self.runner.run(spec)?;
        self.observations
            .lock()
            .expect("capture lock")
            .push((spec.clone(), output.clone()));
        Ok(output)
    }
}
fn binding(request: &DeployTargetRequest<'_>) -> String {
    // All borrowed values are engine-owned observations, including exact prior
    // receipt and intent, selected artifact, profile and injected client paths.
    framed_hash(
        "prepared-plugin-request/1",
        &[("request", &format!("{request:?}"))],
    )
}
fn fingerprint_binding(request: &DeployTargetRequest<'_>) -> String {
    // Staging and recovery intent belong to the effect transaction, not the
    // client version/probe fingerprint. The full plan binding retains both.
    binding(&DeployTargetRequest {
        staging: None,
        recovery_intent: None,
        ..*request
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutableObservation {
    launcher: PathBuf,
    resolved: PathBuf,
    state: vibe_safefs::StableFileState,
    identity: String,
}
#[derive(Clone)]
struct StateObservation {
    owner: Arc<dyn client_state::ClientStateOwner>,
    binding: client_state::ClientStateBinding,
    witness: client_state::ClientStateWitness,
}
fn unavailable(request: &DeployTargetRequest<'_>, state: bool) -> MechanismError {
    DeployProviderError::NeedsPreparation {
        target: request.target.id.clone(),
        reason: if state {
            vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientStateUnavailable
        } else {
            vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientExecutableUnavailable
        },
    }
    .into()
}
fn read_executable(
    path: &std::path::Path,
    request: &DeployTargetRequest<'_>,
    initial: bool,
) -> Result<ExecutableObservation, MechanismError> {
    let absent = || need(request, !initial);
    let failed = || unavailable(request, false);
    if !path.is_absolute() {
        return Err(failed());
    }
    let resolve = || {
        std::fs::canonicalize(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                absent()
            } else {
                failed()
            }
        })
    };
    // Resolve only the admitted launcher, never search PATH. Parent and leaf
    // links participate. The original launcher still goes to ProcessSpec.
    let resolved = resolve()?;
    let parent = resolved.parent().ok_or_else(failed)?;
    let name = resolved
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(failed)?;
    let project = vibe_safefs::Project::open(parent).map_err(|_| failed())?;
    let (state, identity) = project
        .stable_file_state_with_identity(name)
        .map_err(|_| failed())?
        .ok_or_else(absent)?;
    if resolve()? != resolved {
        return Err(need(request, true));
    }
    Ok(ExecutableObservation {
        launcher: path.into(),
        resolved,
        state,
        identity: identity.observation_correlation_token(),
    })
}
impl Capture {
    fn executable_current(&self, request: &DeployTargetRequest<'_>) -> Result<(), MechanismError> {
        if read_executable(&self.executable.launcher, request, false)? != self.executable {
            return Err(need(request, true));
        }
        Ok(())
    }
    fn state_current(
        &self,
        client: PluginClient,
        request: &DeployTargetRequest<'_>,
    ) -> Result<(), MechanismError> {
        if client == PluginClient::OpenCode {
            return Ok(());
        }
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| unavailable(request, true))?;
        let now = state
            .owner
            .observe(&state.binding)
            .map_err(|_| unavailable(request, true))?;
        if now != state.witness {
            return Err(need(request, true));
        }
        Ok(())
    }
}
pub(super) fn need(request: &DeployTargetRequest<'_>, stale: bool) -> MechanismError {
    DeployProviderError::NeedsPreparation {
        target: request.target.id.clone(),
        reason: if stale {
            vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientCaptureStale
        } else {
            vibe_wire::generated::adapter_plan::e1::error::ErrorReason::ClientCaptureMissing
        },
    }
    .into()
}
impl ClientPluginProvider {
    pub(super) fn prepare_capture(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<(), MechanismError> {
        *self.prepared.lock().expect("capture lock") = None;
        let executable = self
            .client
            .executable(request.clients)
            .resolved_path()
            .ok_or_else(|| need(request, false))?
            .to_owned();
        let executable = read_executable(&executable, request, true)?;
        let recorder = Recorder {
            runner: self.runner.as_ref(),
            observations: Mutex::new(Vec::new()),
        };
        let admitted = self.admit(request)?;
        let state = if self.client == PluginClient::OpenCode {
            None
        } else {
            let identity = self.cli_identity(request, &admitted)?;
            let binding = client_state::ClientStateBinding {
                client: self.client.as_str().into(),
                coordinate: identity.coordinate,
                scope: "user".into(),
                user_home: request.user_home.into(),
            };
            self.state_owner
                .lock()
                .expect("state owner lock")
                .clone()
                .map(|owner| {
                    let witness = owner
                        .observe(&binding)
                        .map_err(|_| unavailable(request, true))?;
                    if !witness.matches_binding(&binding) {
                        return Err(unavailable(request, true));
                    }
                    Ok(StateObservation {
                        owner,
                        binding,
                        witness,
                    })
                })
                .transpose()?
        };
        let version = wire::probe_version(&recorder, self.client, request)?;
        let plan = if self.client == PluginClient::OpenCode {
            opencode::plan(
                request,
                &admitted.projection,
                self.config_digest(&admitted.config.name),
            )?
        } else {
            let identity = self.cli_identity(request, &admitted)?;
            let installed = wire::list(
                &recorder,
                self.client,
                request,
                &admitted.config.name,
                &identity.marketplace,
                &identity.coordinate,
            )?;
            self.cli_plan(request, &admitted, identity, installed.as_ref())?
        };
        if read_executable(&executable.launcher, request, false)? != executable {
            return Err(need(request, true));
        }
        if let Some(state) = &state {
            let after = state
                .owner
                .observe(&state.binding)
                .map_err(|_| unavailable(request, true))?;
            if after != state.witness {
                return Err(need(request, true));
            }
        }
        let mut fingerprint = self.version_fingerprint(&version);
        let observations = recorder.observations.into_inner().expect("capture lock");
        fingerprint.digest = framed_hash(
            "prepared-plugin-evidence/1",
            &[
                ("version", &fingerprint.digest),
                ("binding", &binding(request)),
                ("probes", &format!("{observations:?}")),
                ("launcher_mapping", &format!("{:?}", executable)),
                (
                    "client_state",
                    &format!("{:?}", state.as_ref().map(|s| &s.witness)),
                ),
                ("executable_sha256", &executable.state.sha256),
                ("executable_identity", &executable.identity),
                (
                    "executable_mode",
                    &format!("{:?}", executable.state.unix_mode),
                ),
            ],
        );
        *self.prepared.lock().expect("capture lock") = Some(Capture {
            binding: binding(request),
            fingerprint_binding: fingerprint_binding(request),
            executable,
            state,
            plan,
            fingerprint,
            observations,
        });
        Ok(())
    }
    pub(super) fn preparation_result(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<(DeployPlan, DeployFingerprint), MechanismError> {
        let capture = self
            .prepared
            .lock()
            .expect("capture lock")
            .clone()
            .ok_or_else(|| need(request, false))?;
        if capture.fingerprint_binding != fingerprint_binding(request) {
            return Err(need(request, true));
        }
        capture.executable_current(request)?;
        self.admit(request)?;
        Ok((capture.plan, capture.fingerprint))
    }
    pub(super) fn captured_fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        let capture = self
            .prepared
            .lock()
            .expect("capture lock")
            .clone()
            .ok_or_else(|| need(request, false))?;
        if capture.fingerprint_binding != fingerprint_binding(request) || capture.plan != *plan {
            return Err(need(request, true));
        }
        capture.executable_current(request)?;
        capture.state_current(self.client, request)?;
        self.admit(request)?;
        // Fingerprint obeys exactly the plan currentness rule, including OpenCode.
        self.captured(request)?;
        Ok(capture.fingerprint)
    }
    pub(super) fn captured(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<Capture, MechanismError> {
        let capture = self
            .prepared
            .lock()
            .expect("capture lock")
            .clone()
            .ok_or_else(|| need(request, false))?;
        if capture.binding != binding(request) || capture.observations.is_empty() {
            return Err(need(request, true));
        }
        capture.executable_current(request)?;
        capture.state_current(self.client, request)?;
        // Recheck the artifact using its incumbent admission boundary; no
        // executable lookup or process observation is available here.
        let admitted = self.admit(request)?;
        if self.client == PluginClient::OpenCode
            && opencode::plan(
                request,
                &admitted.projection,
                self.config_digest(&admitted.config.name),
            )? != capture.plan
        {
            return Err(need(request, true));
        }
        Ok(capture)
    }
}

/// A query has no process port and no effect methods.
struct ClientObservation {
    client: PluginClient,
    capture: Capture,
}
impl super::super::protocol::PreparedDeployObservation for ClientObservation {
    fn current(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<(DeployPlan, DeployFingerprint), MechanismError> {
        if self.capture.binding != binding(request) {
            return Err(need(request, true));
        }
        self.capture.executable_current(request)?;
        self.capture.state_current(self.client, request)?;
        let artifact = request.artifact.ok_or_else(|| need(request, true))?;
        let projection =
            artifact::admit(&request.target.id, self.client.pin(), self.client, artifact)?;
        if self.client == PluginClient::OpenCode
            && opencode::plan(
                request,
                &projection,
                self.capture.plan.config_digest.clone(),
            )? != self.capture.plan
        {
            return Err(need(request, true));
        }
        Ok((self.capture.plan.clone(), self.capture.fingerprint.clone()))
    }
}
impl ClientPluginProvider {
    pub(super) fn query_observation(
        &self,
    ) -> Option<Box<dyn super::super::protocol::PreparedDeployObservation>> {
        Some(Box::new(ClientObservation {
            client: self.client,
            capture: self.prepared.lock().ok()?.clone()?,
        }))
    }
}
