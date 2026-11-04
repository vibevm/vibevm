//! Initial checkpoint admission, exact input capture and legacy prefix provenance.
use super::*;
use crate::OperationContext;
use std::sync::atomic::{AtomicU64, Ordering};
pub(in crate::operations) fn generation() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.9fZ"),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
pub(in crate::operations) struct Checkpoint<'a> {
    pub kind: w::ApplicationPendingOperationKind,
    pub retirements: Vec<w::ApplicationPendingRetirement>,
    pub predecessor: Option<&'a str>,
    pub parent: Option<Parent<'a>>,
}
pub(in crate::operations) fn begin(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    plan: &Plan,
    checkpoint: Checkpoint<'_>,
) -> Result<w::ApplicationPendingTarget, AppError> {
    let Checkpoint {
        kind,
        retirements,
        predecessor,
        parent,
    } = checkpoint;
    let settings = ctx.settings;
    let directory = work(settings, &plan.package);
    let old = if request.start != Start::Fresh {
        Some(StepsFile::load(&directory)?.ok_or_else(|| {
            failure(
                settings,
                &plan.package,
                "there is no kept work to resume or restart".into(),
            )
        })?)
    } else {
        None
    };
    if let Some(old) = &old {
        if old.package != plan.package || old.option != plan.option || old.features != plan.features
        {
            return Err(failure(
                settings,
                &plan.package,
                "kept package, installer or features differ from the requested replay".into(),
            ));
        }
        if let Start::FromStep(id) = &request.start
            && !plan.steps.iter().any(|s| &s.step.id == id)
        {
            return Err(crate::StepError::UnknownStep {
                option: plan.option.clone(),
                step: id.clone(),
                steps: plan
                    .steps
                    .iter()
                    .map(|s| s.step.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            }
            .into());
        }
        if request.start == Start::Resume
            && old
                .operation
                .as_ref()
                .is_none_or(|o| o.phase == w::ApplicationPendingOperationPhase::Complete)
            && plan.steps.iter().all(|s| {
                old.get(&s.step.id).is_some_and(|p| {
                    matches!(
                        p.outcome,
                        crate::StepOutcome::Done | crate::StepOutcome::Skipped
                    )
                })
            })
        {
            return Err(crate::StepError::NothingToResume {
                key: plan.package.key(),
                work: directory.display().to_string(),
                reason: "every planned step in kept work is already settled".into(),
            }
            .into());
        }
    }
    let inherited = old
        .as_ref()
        .is_some_and(|old| inherited_unknown(old, plan, &request.start));
    if inherited && plan.configuration.is_some() {
        return Err(failure(settings,&plan.package,"a retained legacy prefix has no original common intent; use a fresh full replay with the chosen complete common input".into()));
    }
    if let Some(existing) = StepsFile::load(&directory)? {
        super::work_retirement::before_replacement(lock, settings, &existing)?;
    }
    // Retirement may clear only the old generation's pending marker. Bind the
    // new predecessor/dependency identities to the actual post-retirement index.
    let current_index = Index::load(settings)?;
    let index = &current_index;
    let home = actual_home(settings)?;
    let project = if settings.is_project() {
        settings
            .path()
            .parent()
            .and_then(Path::parent)
            .map(|p| {
                p.canonicalize()
                    .map_err(|e| failure(settings, &plan.package, e.to_string()))
                    .and_then(|p| lossless(&p))
            })
            .transpose()?
    } else {
        None
    };
    let target = w::ApplicationPendingTarget {
        home,
        key: plan.package.key(),
        operation: generation(),
        project,
    };
    let mut source = resolved.source.clone();
    source.directory = lossless(
        &resolved
            .root
            .canonicalize()
            .map_err(|e| failure(settings, &plan.package, e.to_string()))?,
    )?;
    let kind_name = kind_name(&kind);
    let op = w::ApplicationPendingOperation {
        retirements,
        lookup: lookup(ctx)?,
        publication_record: None,
        target: target.clone(),
        kind,
        source,
        phase: w::ApplicationPendingOperationPhase::Prepared,
        execution_identity: identity::execution(
            settings,
            resolved,
            option,
            plan,
            request.force,
            kind_name,
        )?,
        agent_modes: plan
            .steps
            .iter()
            .filter_map(|s| {
                s.agent_mode.map(|mode| {
                    (
                        s.step.id.clone(),
                        match mode {
                            crate::RunMode::Interactive => w::ApplicationAgentMode::Interactive,
                            crate::RunMode::Unattended => w::ApplicationAgentMode::Unattended,
                        },
                    )
                })
            })
            .collect(),
        prompts: plan
            .steps
            .iter()
            .filter_map(|p| {
                p.prepared_prompt
                    .as_ref()
                    .map(|text| (p.step.id.clone(), text.clone()))
            })
            .collect(),
        secret_required: plan.secret_provided.clone(),
        children: plan
            .dependencies
            .iter()
            .map(|d| {
                Ok(w::ApplicationPendingChild {
                    requirement: d.package.clone(),
                    key: d.key.clone(),
                    scope: match d.scope {
                        vibe_core::manifest::RequirementScope::Build => {
                            w::ApplicationPendingChildScope::Build
                        }
                        vibe_core::manifest::RequirementScope::Runtime => {
                            w::ApplicationPendingChildScope::Runtime
                        }
                    },
                    step: d.step.clone(),
                    option: d.option.clone().ok_or_else(|| {
                        failure(
                            settings,
                            &plan.package,
                            "a dependency has no selected installer".into(),
                        )
                    })?,
                    commands: d.commands.clone(),
                    hidden: d.hidden,
                    installed: d.installed,
                    source: d.resolved.as_ref().map(|r| r.source.clone()),
                    record_identity: identity::predecessor(index, &d.key)
                        .map_err(|e| failure(settings, &plan.package, e))?,
                    target: None,
                    completed: d.installed,
                })
            })
            .collect::<Result<_, AppError>>()?,
        parent: parent.map(|parent| parent.reference()),
        predecessor_launchers: predecessor
            .and_then(|key| index.installations.get(key))
            .map(|r| r.launchers.iter().map(|l| l.id.clone()).collect())
            .unwrap_or_default(),
        predecessor: predecessor.map(str::to_owned),
        predecessor_identity: predecessor
            .map(|key| identity::predecessor(index, key))
            .transpose()
            .map_err(|e| failure(settings, &plan.package, e))?
            .flatten(),
        provenance: w::ApplicationPendingOperationProvenance::Captured,
        request: w::ApplicationPendingRequest {
            secret_origins: plan.secret_origins.clone(),
            agent_external: ctx
                .agent
                .selected
                .is_some_and(|a| a.source == crate::AgentSource::External),
            offline: ctx.offline,
            hosted: ctx.agent.hosted,
            assume_yes: ctx.agent.assume_yes,
            agent: ctx.agent.selected.map(|a| a.id.clone()),
            platform: plan.platform.clone(),
            arch: plan.arch.clone(),
            installer: plan.option.clone(),
            force: request.force,
            accept_license: request.accept_license || plan.license.is_some(),
            keep_work: request.keep_work,
            hidden: request.hidden,
            mark: match request.mark {
                crate::Mark::Manual => w::ApplicationPendingRequestMark::Manual,
                crate::Mark::Auto => w::ApplicationPendingRequestMark::Auto,
            },
            purge_previous: request.purge_previous,
            enabled: request.features.enable.clone(),
            disabled: request.features.disable.clone(),
            profile: request.features.profile.clone(),
            library: request.library.clone(),
            features: plan.features.clone(),
            parameters: plan
                .parameters
                .iter()
                .filter(|p| !p.secret)
                .map(|p| (p.id.clone(), p.value.clone()))
                .collect(),
            secret_provided: plan.secret_provided.clone(),
            configuration: plan.configuration.clone(),
            public_set: if plan.configuration.is_some() {
                Default::default()
            } else {
                request
                    .parameters
                    .iter()
                    .filter(|(id, _)| option.parameters.iter().any(|p| &p.id == id && !p.secret))
                    .cloned()
                    .collect()
            },
            restart_from: match &request.start {
                Start::FromStep(id) => Some(id.clone()),
                _ => None,
            },
            required_by: request
                .required_by
                .as_ref()
                .map(|r| w::ApplicationPendingRequiredBy {
                    package: r.package.clone(),
                    step: r.step.clone(),
                }),
        },
    };
    let steps = old.map(|s| s.steps).unwrap_or_default();
    let mut op = op;
    if inherited {
        op.provenance = w::ApplicationPendingOperationProvenance::LegacyEffectiveOnly;
        for origin in op.request.secret_origins.values_mut() {
            if !matches!(origin, w::ConfigurationParameterOrigin::Explicit(_)) {
                *origin = w::ConfigurationParameterOrigin::LegacyEffective(Box::new(
                    w::ConfigurationParameterOriginLegacyEffective {},
                ));
            }
        }
    }
    if request.start == Start::Fresh && directory.exists() {
        if let Some(notice) = ctx.on_notice {
            notice(&format!(
                "replacing kept work directory {}",
                directory.display()
            ));
        }
        fs::remove_dir_all(&directory)
            .map_err(|e| failure(settings, &plan.package, e.to_string()))?;
    }
    fs::create_dir_all(&directory).map_err(|e| failure(settings, &plan.package, e.to_string()))?;
    StepsFile {
        operation: Some(op),
        package: plan.package.clone(),
        option: plan.option.clone(),
        features: plan.features.clone(),
        steps,
    }
    .save(&directory)?;
    Ok(target)
}
fn inherited_unknown(old: &StepsFile, plan: &Plan, start: &Start) -> bool {
    if old.operation.as_ref().is_some_and(|o| {
        o.provenance != w::ApplicationPendingOperationProvenance::LegacyEffectiveOnly
    }) {
        return false;
    }
    let first = match start {
        Start::Fresh => return false,
        Start::FromStep(id) => plan
            .steps
            .iter()
            .position(|s| &s.step.id == id)
            .unwrap_or(0),
        Start::Resume => plan
            .steps
            .iter()
            .position(|s| {
                old.get(&s.step.id).is_none_or(|s| {
                    !matches!(
                        s.outcome,
                        crate::StepOutcome::Done | crate::StepOutcome::Skipped
                    )
                })
            })
            .unwrap_or(plan.steps.len()),
    };
    let finish = *start == Start::Resume
        && plan
            .steps
            .get(first)
            .is_some_and(|s| s.step.phase == vibe_core::manifest::StepPhase::Finish)
        && plan
            .steps
            .iter()
            .filter(|s| s.step.phase == vibe_core::manifest::StepPhase::Prepare)
            .all(|s| {
                old.get(&s.step.id).is_some_and(|s| {
                    matches!(
                        s.outcome,
                        crate::StepOutcome::Done | crate::StepOutcome::Skipped
                    )
                })
            });
    plan.steps.iter().enumerate().any(|(i, s)| {
        (i < first || (finish && s.step.phase == vibe_core::manifest::StepPhase::Prepare))
            && old.get(&s.step.id).is_some()
    })
}

pub(in crate::operations) fn lookup(
    ctx: &OperationContext<'_>,
) -> Result<w::ApplicationPendingLookup, AppError> {
    let path = |path: &Path| {
        path.canonicalize()
            .map_err(|e| {
                crate::StepError::ResumeMismatch {
                    work: path.display().to_string(),
                    reason: format!("resolver root cannot be retained: {e}"),
                }
                .into()
            })
            .and_then(|p| lossless(&p))
    };
    Ok(match ctx.resolver_scope {
        crate::ResolverScope::Caller => {
            w::ApplicationPendingLookup::Caller(Box::new(w::ApplicationPendingLookupCaller {}))
        }
        crate::ResolverScope::Registry(root) => {
            w::ApplicationPendingLookup::Registry(Box::new(w::ApplicationPendingLookupRegistry {
                root: path(root)?,
            }))
        }
        crate::ResolverScope::Chain { embedded } => {
            w::ApplicationPendingLookup::Chain(Box::new(w::ApplicationPendingLookupChain {
                embedded: embedded.map(path).transpose()?,
            }))
        }
        crate::ResolverScope::Project(root) => {
            w::ApplicationPendingLookup::Project(Box::new(w::ApplicationPendingLookupProject {
                root: path(root)?,
            }))
        }
    })
}
