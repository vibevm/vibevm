//! Source-bound admission and retained C1/C2 submission for fixed Deploy.
use super::*;

const DEPLOY_MANIFEST_CAP: usize = 1024 * 1024;

/// A non-cloneable trusted-host permit bound to one key. A fresh key carries an
/// authored source witness for one effect; a reserved key can only cold-inspect
/// its retained capsule. Wire data cannot replace roots, profile, target or C1.
pub struct FixedDeployHostAdmission<'a> {
    lease: &'a LifecycleLease,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: String,
    execution: &'a DeployExecution<'a>,
    facts: DeployColdHostFacts<'a>,
    source: Option<StableFileSnapshot>,
    admitted: AdmittedLocalDeployBinding,
    request_mode: deploy::RequestMode,
}

fn authored_fixed_deploy(
    lease: &LifecycleLease,
    execution: &DeployExecution<'_>,
) -> Result<StableFileSnapshot, RetainedMechanismError> {
    let source = lease
        .project()
        .read_file_snapshot_bounded("vibe.toml", DEPLOY_MANIFEST_CAP)
        .map_err(|error| RetainedMechanismError::Refused(error.to_string()))?
        .ok_or_else(|| {
            RetainedMechanismError::Refused("authored deploy manifest is absent".into())
        })?;
    let text = std::str::from_utf8(&source.bytes).map_err(|error| {
        RetainedMechanismError::Refused(format!("deploy manifest is not UTF-8: {error}"))
    })?;
    let manifest = Manifest::parse_str(text).map_err(|error| {
        RetainedMechanismError::Refused(format!("authored deploy manifest: {error}"))
    })?;
    let section = manifest.deploy.as_ref().ok_or_else(|| {
        RetainedMechanismError::Refused("authored deploy section is absent".into())
    })?;
    let os = TargetOs::current()
        .ok_or_else(|| RetainedMechanismError::Refused("host deploy OS is unsupported".into()))?;
    let projected = section
        .project_profile(&execution.selection.profile, os)
        .map_err(|error| {
            RetainedMechanismError::Refused(format!("authored deploy profile: {error}"))
        })?;
    if projected.active_targets() != execution.selection.targets.as_slice()
        || projected.active_targets().len() != 1
        || section.targets.len() != 1
        || execution.targets != section.targets.as_slice()
    {
        return Err(RetainedMechanismError::Refused(
            "host deploy selection or target differs from the authored profile/target".into(),
        ));
    }
    let target = &section.targets[0];
    let fixed_mechanism: MechanismKey = "deploy:vibe-bin".parse().expect("fixed deploy key");
    if target.mechanism != fixed_mechanism
        || target.when.is_some()
        || target
            .depends_on
            .as_ref()
            .is_some_and(|deps| !deps.is_empty())
    {
        return Err(RetainedMechanismError::Refused(
            "authored fixed deploy requires one unconditional independent vibe-bin target".into(),
        ));
    }
    Ok(source)
}

impl<'a> FixedDeployHostAdmission<'a> {
    /// The cooperative host supplies a held lease, C1 authority and physical
    /// roots. A captured authored manifest independently validates its plan.
    pub fn issue_from_trusted_host(
        lease: &'a LifecycleLease,
        namespace: &'a SubmissionNamespace,
        scope: &'a SubmissionScope,
        key: &str,
        execution: &'a DeployExecution<'a>,
        facts: DeployColdHostFacts<'a>,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
    ) -> Result<Self, RetainedMechanismError> {
        Self::issue_with_request_mode(
            lease,
            namespace,
            scope,
            key,
            execution,
            facts,
            current_generation,
            deploy::RequestMode::Legacy,
        )
    }

    /// A profiled request retains the caller's explicit choice in C1 intent.
    /// The legacy constructor above keeps its original canonical bytes.
    pub fn issue_profiled_from_trusted_host(
        lease: &'a LifecycleLease,
        namespace: &'a SubmissionNamespace,
        scope: &'a SubmissionScope,
        key: &str,
        execution: &'a DeployExecution<'a>,
        facts: DeployColdHostFacts<'a>,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
    ) -> Result<Self, RetainedMechanismError> {
        Self::issue_with_request_mode(
            lease,
            namespace,
            scope,
            key,
            execution,
            facts,
            current_generation,
            deploy::RequestMode::Profiled,
        )
    }

    fn issue_with_request_mode(
        lease: &'a LifecycleLease,
        namespace: &'a SubmissionNamespace,
        scope: &'a SubmissionScope,
        key: &str,
        execution: &'a DeployExecution<'a>,
        facts: DeployColdHostFacts<'a>,
        current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
        request_mode: deploy::RequestMode,
    ) -> Result<Self, RetainedMechanismError> {
        lease
            .ensure_root(execution.project_root, "at fixed deploy admission")
            .map_err(|error| RetainedMechanismError::Refused(error.to_string()))?;
        if execution.project_root != facts.project_root
            || execution.settings_root != facts.settings_root
            || execution.user_home != facts.user_home
            || execution.state_home != crate::deploy_state_home(facts.settings_root)
            || scope.context != facts.host_context_id
            || scope.target != facts.instance_ref
            || scope.principal != facts.principal_ref
            || execution.package.is_some()
        {
            return Err(RetainedMechanismError::Refused(
                "fixed deploy roots, state home, package or C1 scope differ from trusted host"
                    .into(),
            ));
        }
        SubmissionReservations::validate_namespace(lease, namespace, scope)?;
        // An existing C1 key is read through its retained capsule, not the
        // present manifest. This also permits cold recovery after source drift.
        let existing = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)?;
        let source = if existing.is_some() {
            None
        } else {
            Some(authored_fixed_deploy(lease, execution)?)
        };
        let admitted = issue_deploy_binding(&facts)?;
        if existing.is_some() {
            // Cold replay needs no present manifest, but it must still carry
            // the original canonical intent. A different profile or target
            // under the same key cannot borrow the retained run.
            let request = match request_mode {
                deploy::RequestMode::Legacy => {
                    deploy::canonical_request(execution, admitted.binding(), facts.principal_ref)
                }
                deploy::RequestMode::Profiled => deploy::canonical_profiled_request(
                    execution,
                    admitted.binding(),
                    facts.principal_ref,
                ),
            }
            .map_err(deploy_error)?;
            SubmissionReservations::inspect_existing_request(
                lease, namespace, scope, key, &request,
            )?;
        }
        if source.is_some() {
            let current = current_generation().map_err(|error| {
                RetainedMechanismError::Refused(format!("current deploy generation: {error}"))
            })?;
            if current.as_deref() != facts.selected_generation_ref {
                return Err(RetainedMechanismError::Refused(
                    "current deploy generation differs from host admission".into(),
                ));
            }
        }
        Ok(Self {
            lease,
            namespace,
            scope,
            key: key.into(),
            execution,
            facts,
            source,
            admitted,
            request_mode,
        })
    }
}

/// An unsuccessful call retains the owner's actual run/partial truth when it
/// can be reobserved. Missing observation is never proof of no effects.
#[derive(Debug, Error)]
#[error("fixed bound deploy submission failed: {source}")]
pub struct FixedDeploySubmitError {
    #[source]
    pub source: RetainedMechanismError,
    pub known_run_id: Option<String>,
    pub owner_observation: Option<DeployInspection>,
    pub effects_may_have_begun: bool,
}

/// Submit only the captured, host-admitted one-target bound vibe-bin plan.
/// The incumbent deploy bridge is the sole C1 reservation/effect journal.
pub fn submit_fixed_bound_vibe_bin(
    admission: FixedDeployHostAdmission<'_>,
    key: &str,
    current_generation: &mut dyn FnMut() -> Result<Option<String>, crate::DeployError>,
) -> Result<DeployInspection, FixedDeploySubmitError> {
    let FixedDeployHostAdmission {
        lease,
        namespace,
        scope,
        key: captured_key,
        execution,
        facts,
        source,
        admitted,
        request_mode,
    } = admission;
    if key != captured_key {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused("fixed deploy permit key differs".into()),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: false,
        });
    }
    // A reserved key belongs to its original owner capsule. Current manifest
    // bytes cannot turn a same-key attach into a fresh effect or erase the
    // retained result; only a new key needs the fresh source witness below.
    let existing = SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
        .map_err(|error| FixedDeploySubmitError {
            source: error.into(),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: true,
        })?;
    if let Some(existing) = existing {
        return match deploy::inspect_host_admitted_bound_vibe_bin_cold(
            lease,
            namespace.id(),
            scope,
            key,
            &admitted,
        ) {
            Ok(Some(observed)) => Ok(map_deploy_observation(Some(observed))),
            Ok(None) => Err(FixedDeploySubmitError {
                source: RetainedMechanismError::Refused(
                    "reserved deploy key has no cold owner observation".into(),
                ),
                known_run_id: Some(existing.domain_run_id),
                owner_observation: None,
                effects_may_have_begun: true,
            }),
            Err(error) => Err(FixedDeploySubmitError {
                source: deploy_error(error),
                known_run_id: Some(existing.domain_run_id),
                owner_observation: None,
                effects_may_have_begun: true,
            }),
        };
    }
    let Some(source) = source else {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused(
                "existing deploy key lost its retained C1 reservation".into(),
            ),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: true,
        });
    };
    let changed = lease
        .project()
        .read_file_snapshot_bounded("vibe.toml", DEPLOY_MANIFEST_CAP)
        .map_err(|error| error.to_string());
    if !matches!(changed, Ok(Some(ref current)) if *current == source) {
        return Err(FixedDeploySubmitError {
            source: RetainedMechanismError::Refused(
                "authored deploy manifest changed before submit".into(),
            ),
            known_run_id: None,
            owner_observation: None,
            effects_may_have_begun: false,
        });
    }
    let binding = admitted.binding().clone();
    let submit = match request_mode {
        deploy::RequestMode::Legacy => deploy::submit_host_admitted_bound_vibe_bin,
        deploy::RequestMode::Profiled => deploy::submit_host_admitted_bound_vibe_bin_profiled,
    };
    match submit(
        lease,
        namespace,
        scope,
        key,
        execution,
        &binding,
        admitted,
        facts.principal_ref,
        current_generation,
    ) {
        Ok(observed) => Ok(map_deploy_observation(Some(observed))),
        Err(error) => {
            let known_run_id =
                SubmissionReservations::inspect_existing_key(lease, namespace, scope, key)
                    .ok()
                    .flatten()
                    .map(|row| row.domain_run_id);
            let owner_observation = issue_deploy_binding(&facts)
                .ok()
                .and_then(|fresh| {
                    deploy::inspect_host_admitted_bound_vibe_bin_cold(
                        lease,
                        namespace.id(),
                        scope,
                        key,
                        &fresh,
                    )
                    .ok()
                })
                .map(map_deploy_observation);
            let known_run_id = known_run_id.or_else(|| match owner_observation.as_ref() {
                Some(
                    DeployInspection::Completed { run_id, .. }
                    | DeployInspection::Uncertain { run_id, .. }
                    | DeployInspection::RecoveryRequired { run_id },
                ) => Some(run_id.clone()),
                Some(DeployInspection::Retired { run_id }) => run_id.clone(),
                Some(DeployInspection::Absent) | None => None,
            });
            Err(FixedDeploySubmitError {
                source: deploy_error(error),
                known_run_id,
                owner_observation,
                effects_may_have_begun: true,
            })
        }
    }
}
