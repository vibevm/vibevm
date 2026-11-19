//! Opt-in top-level handoff. Tokens stay opaque and never enter error reports.
use super::*;

pub enum InstallPreparation {
    Completed(Outcome),
    NeedsNode(Box<NodeRemediation>),
}

pub struct NodeRemediation {
    prepared: crate::steps::PreparedSelection,
    request: InstallRequest,
    settings: crate::SettingsRoot,
    prior: Option<crate::Installation>,
    slot: Option<crate::SlotState>,
}
impl std::fmt::Debug for NodeRemediation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRemediation")
            .field("selection", &"[protected ephemeral token]")
            .finish()
    }
}
impl NodeRemediation {
    pub fn runtime(&self) -> &vibe_core::manifest::NpmRuntimeDecl {
        self.prepared
            .option
            .npm_runtime
            .as_ref()
            .expect("planner admitted runtime binding")
    }
    pub fn option_id(&self) -> &str {
        &self.prepared.option.id
    }
    pub fn missing_command(&self) -> &str {
        &self.prepared.command
    }

    /// Acquire only the bound metadata input through the ordinary installer owner.
    /// The operation-owned temporary file contains no prepared answers.
    pub fn verify_runtime_input(
        &self,
        ctx: &OperationContext<'_>,
        resolved: &ResolvedPackage,
    ) -> Result<(), AppError> {
        self.prepared.revalidate(resolved)?;
        if ctx.settings != &self.settings {
            return Err(crate::ResolveError::Manifest {
                package: resolved.package.key(),
                reason: "runtime metadata context changed".into(),
            }
            .into());
        }
        let binding = self.runtime();
        let steps = vibe_core::manifest::steps_of(&self.prepared.option);
        let step = steps
            .iter()
            .find(|s| s.id == binding.step)
            .expect("validated binding step");
        let temporary =
            tempfile::tempdir_in(ctx.settings.path()).map_err(|source| crate::StateError::Io {
                action: "creating runtime metadata scratch",
                path: ctx.settings.path().display().to_string(),
                source,
            })?;
        let destination = temporary.path().join("input.tgz");
        let key = resolved.package.key();
        let watch = crate::installer::FetchWatch {
            http_client: ctx.http_client,
            interrupt: ctx.interrupt,
            download_headers: ctx.download_headers,
            ..Default::default()
        };
        use vibe_core::manifest::StepKind;
        use vibe_core::remap::{Artifact, ArtifactKind};
        match &step.kind {
            StepKind::Sources { sources } => {
                let source = sources
                    .iter()
                    .find(|s| Some(&s.name) == binding.input.as_ref())
                    .expect("validated source name");
                let artifact = Artifact::new(ArtifactKind::AppInput)
                    .with_package(&key)
                    .with_step(&step.id);
                let (fetched, _) = crate::installer::source_build::fetch(
                    &ctx.settings.app_sources_cache(),
                    &resolved.root,
                    source,
                    ctx.offline,
                    &watch,
                    vibe_core::remap::active(),
                    &artifact,
                )?;
                crate::verify_npm_runtime_archive_interruptible(
                    &self.prepared.option,
                    binding,
                    &fetched.file,
                    ctx.interrupt,
                )
            }
            StepKind::Archive {
                source,
                sha256,
                size,
                ..
            } => {
                let artifact = Artifact::new(ArtifactKind::AppInput)
                    .with_package(&key)
                    .with_step(&step.id)
                    .with_url(source)
                    .with_digest(sha256);
                crate::installer::fetch_located(
                    &crate::installer::Located {
                        locator: vibe_core::remap::active(),
                        artifact: &artifact,
                        declared: source,
                        sha256,
                        filename: None,
                    },
                    &resolved.root,
                    &destination,
                    ctx.offline,
                    &crate::installer::FetchWatch {
                        size: *size,
                        ..watch
                    },
                )?;
                crate::verify_npm_runtime_archive_interruptible(
                    &self.prepared.option,
                    binding,
                    &destination,
                    ctx.interrupt,
                )
            }
            _ => unreachable!("validated npm input kind"),
        }
        .map_err(|reason| {
            crate::ResolveError::Manifest {
                package: key,
                reason,
            }
            .into()
        })
    }
}

pub fn install_preparing_node(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<InstallPreparation, AppError> {
    if request.start != crate::Start::Fresh
        || request.required_by.is_some()
        || ctx.import_scope.is_some()
    {
        return install(ctx, resolved, request).map(InstallPreparation::Completed);
    }
    let held = ctx.settings.lock()?;
    if let Some(outcome) = pending::manual(&held, ctx, resolved, request)? {
        return Ok(InstallPreparation::Completed(outcome));
    }
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let option = choose(ctx, resolved, &request.choice)?;
    let key = resolved.package.key();
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let prior = index.installations.get(&key).cloned();
    if let Some(prior) = &prior {
        ensure_available(&key, prior, &storage)?;
    }
    let previous = prior
        .as_ref()
        .filter(|p| p.option == option.id)
        .map(|p| Previous {
            secret_origins: &p.secret_origins,
            configuration: p.configuration.as_ref(),
            features: &p.features,
            parameters: &p.parameters,
            license: p.license.as_ref().map(|l| l.name.as_str()),
            ask: false,
        });
    match super::super::planning::prepare_option(
        ctx,
        resolved,
        &option,
        request,
        previous,
        &index,
        super::super::planning::PreparedContinuation {
            pending: None,
            modes: None,
            request_node: true,
            prepared: None,
        },
    )? {
        crate::steps::PreparedInstall::Ready(plan) => {
            let outcome = apply_plan(
                &held,
                ctx,
                resolved,
                request,
                &mut vec![key],
                ApplicationPlan {
                    option,
                    plan: *plan,
                    prior,
                    saved: None,
                    parent: None,
                    inherited_held: None,
                },
            )?;
            Ok(InstallPreparation::Completed(
                super::super::recommends::after(&held, ctx, resolved, request, outcome),
            ))
        }
        crate::steps::PreparedInstall::NeedsNode(prepared) => {
            Ok(InstallPreparation::NeedsNode(Box::new(NodeRemediation {
                prepared: *prepared,
                request: request.clone(),
                settings: ctx.settings.clone(),
                prior,
                slot: index.slots.get(&resolved.application.id).cloned(),
            })))
        }
    }
}

/// One-shot continuation: lock and all mutable operation facts belong to this call.
pub fn continue_after_node(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    token: NodeRemediation,
) -> Result<Outcome, AppError> {
    let held = ctx.settings.lock()?;
    if ctx.settings != &token.settings {
        return Err(crate::ResolveError::Manifest {
            package: resolved.package.key(),
            reason: "prepared target settings or store changed".into(),
        }
        .into());
    }
    token.prepared.revalidate(resolved)?;
    let option = token.prepared.option.clone();
    let key = resolved.package.key();
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let prior = index.installations.get(&key).cloned();
    if prior != token.prior || index.slots.get(&resolved.application.id) != token.slot.as_ref() {
        return Err(crate::ResolveError::Manifest {
            package: key,
            reason: "prepared target installation or default changed; new selection required"
                .into(),
        }
        .into());
    }

    if let Some(prior) = &prior {
        ensure_available(&key, prior, &storage)?;
    }
    let plan = match super::super::planning::prepare_option(
        ctx,
        resolved,
        &option,
        &token.request,
        None,
        &index,
        super::super::planning::PreparedContinuation {
            pending: None,
            modes: None,
            request_node: false,
            prepared: Some(token.prepared),
        },
    )? {
        crate::steps::PreparedInstall::Ready(plan) => *plan,
        crate::steps::PreparedInstall::NeedsNode(_) => {
            unreachable!("continuation cannot request another provider")
        }
    };
    let outcome = apply_plan(
        &held,
        ctx,
        resolved,
        &token.request,
        &mut vec![key],
        ApplicationPlan {
            option,
            plan,
            prior,
            saved: None,
            parent: None,
            inherited_held: None,
        },
    )?;
    Ok(super::super::recommends::after(
        &held,
        ctx,
        resolved,
        &token.request,
        outcome,
    ))
}
