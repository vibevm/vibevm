//! Requirements between application packages (`PROP-065#app-dependencies`):
//! resolution through the home's index first and the caller's package
//! resolver otherwise, installation of the missing ones ahead of the
//! requiring package under the same consent, and the exposure of every
//! requirement on the requiring steps' PATH.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#license");

use std::collections::BTreeSet;

use vibe_core::manifest::RequirementScope;
use vibe_core::progress::Requires;

use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::exposure::exposure_of;
use crate::index::{Index, Installation, Mark, Requirement};
use crate::resolve::qualified_ref;
use crate::select::{InstallChoice, select_option};
use crate::steps::{Plan, PlannedDependency, RequiredBy, StepPaths, admit_license};
use crate::storage::Storage;

use super::{InstallRequest, OperationContext};

/// Resolve one `requires` entry of `owner`: an installed version that
/// satisfies it is used as is (the newest one that is not a kept previous
/// installation), otherwise the caller's resolver names the package and
/// its installer option for this host is chosen without asking. A license
/// of a requirement to install is consented to like the owner's
/// (`PROP-065#license`): outside an attended terminal only
/// `accept_license` admits it, and its absence refuses the whole plan
/// before anything is fetched; an attended terminal asks once, when the
/// requirement itself is planned.
pub(super) fn resolve_requirement(
    ctx: &OperationContext<'_>,
    index: &Index,
    owner: &str,
    spelling: &str,
    scope: RequirementScope,
    step: Option<&str>,
    accept_license: bool,
) -> Result<PlannedDependency, AppError> {
    let unresolved = |reason: String| -> AppError {
        LifecycleError::RequirementUnresolved {
            key: owner.into(),
            requirement: spelling.into(),
            reason,
        }
        .into()
    };
    let requested = qualified_ref(spelling).map_err(|error| unresolved(error.to_string()))?;
    let group = requested
        .group
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    if let Some(import) = ctx.import_scope
        && let Some(binding) = import.dependency(owner, spelling, scope, step)?
    {
        let key = binding.application.key.clone();
        let package = super::pending::coordinate(&key)?;
        let record = index.installations.get(&key);
        let completed = record.is_some() && import.completed_binding(ctx.settings, &key, index)?;
        imported_work_ready(ctx, index, &package, completed)?;
        if let Some(record) = record.filter(|_| completed) {
            return Ok(PlannedDependency {
                package: spelling.into(),
                scope,
                step: step.map(str::to_owned),
                key,
                installed: true,
                hidden: record.lifecycle.hidden,
                commands: record
                    .public_commands
                    .iter()
                    .map(|c| c.name.clone())
                    .collect(),
                option: Some(record.option.clone()),
                resolved: None,
            });
        }
        let (resolved, option) = import.admit(ctx, binding)?;
        if let Some(record) = record {
            import.compatible(ctx, binding, &option, record)?;
        } else if ctx.chooser.is_none() {
            admit_license(&option, &key, &resolved.root, accept_license, None, None)?;
        }
        return Ok(PlannedDependency {
            package: spelling.into(),
            scope,
            step: step.map(str::to_owned),
            key,
            installed: record.is_some(),
            hidden: record.map_or(binding.application.hidden, |r| r.lifecycle.hidden),
            commands: record
                .map(|r| r.public_commands.iter().map(|c| c.name.clone()).collect())
                .unwrap_or_else(|| {
                    option
                        .public_commands
                        .iter()
                        .map(|c| c.name.clone())
                        .collect()
                }),
            option: Some(option.id.clone()),
            resolved: record.is_none().then(|| Box::new(resolved)),
        });
    }
    let mut candidates: Vec<(semver::Version, &String, &Installation)> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == group
                && record.package.name == requested.name.as_str()
                && record.lifecycle.superseded_by.is_none()
        })
        .filter_map(|(key, record)| {
            let version = semver::Version::parse(&record.package.version).ok()?;
            requested
                .version
                .matches(&version)
                .then_some((version, key, record))
        })
        .collect();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    if let Some((_, key, record)) = candidates.last() {
        if let Some(target) = &record.pending {
            super::pending::load(ctx.settings, &record.package, Some(&target.operation))?;
            return Err(unresolved(format!(
                "dependency has unfinished work; continue its actual operation with `{}` before starting a different parent",
                super::pending::command(target)
            )));
        }
        return Ok(PlannedDependency {
            package: spelling.into(),
            scope,
            step: step.map(str::to_owned),
            key: (*key).clone(),
            installed: true,
            hidden: record.lifecycle.hidden,
            commands: record
                .public_commands
                .iter()
                .map(|command| command.name.clone())
                .collect(),
            option: Some(record.option.clone()),
            resolved: None,
        });
    }
    let resolver = ctx
        .resolver
        .ok_or_else(|| unresolved("no package resolver is available in this context".into()))?;
    if let Some(imported) = ctx.import_scope {
        imported.admit_lookup(ctx)?;
    }
    let resolved = resolver(spelling).map_err(|error| unresolved(error.to_string()))?;
    if let Some(state) =
        crate::steps::StepsFile::load(&super::pending::work(ctx.settings, &resolved.package))?
        && let Some(op) = state.operation.as_ref().filter(|op| {
            op.phase != vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
                || op.parent.is_some()
        })
    {
        super::pending::load(ctx.settings, &resolved.package, Some(&op.target.operation))?;
        return Err(unresolved(format!(
            "dependency has retained work from another operation; continue `{}` before starting a different parent",
            super::pending::command(&op.target)
        )));
    }
    let option = select_option(&resolved.options, &InstallChoice::default(), None)
        .map_err(|error| unresolved(error.to_string()))?;
    if ctx.chooser.is_none() {
        admit_license(
            &option,
            &resolved.package.key(),
            &resolved.root,
            accept_license,
            None,
            None,
        )?;
    }
    Ok(PlannedDependency {
        package: spelling.into(),
        scope,
        step: step.map(str::to_owned),
        key: resolved.package.key(),
        installed: false,
        hidden: scope == RequirementScope::Build,
        commands: option
            .public_commands
            .iter()
            .map(|command| command.name.clone())
            .collect(),
        option: Some(option.id.clone()),
        resolved: Some(Box::new(resolved)),
    })
}

/// Refuse a requirement cycle: `key` is already being installed further up.
pub(super) fn enter(stack: &mut Vec<String>, key: &str) -> Result<(), AppError> {
    if stack.iter().any(|seen| seen == key) {
        let mut chain = stack.clone();
        chain.push(key.to_owned());
        return Err(LifecycleError::RequirementCycle {
            chain: chain.join(" -> "),
        }
        .into());
    }
    stack.push(key.to_owned());
    Ok(())
}

/// Install every requirement of `plan` that is not installed yet, in plan
/// order, hidden for build scope and exposed for runtime scope, marked
/// `auto`, under the requiring command's `--accept-license`; then expose
/// each requirement for the steps that need it.
pub(super) fn install_required(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    plan: &Plan,
    accept_license: bool,
    stack: &mut Vec<String>,
    parent: Option<&vibe_wire::generated::shared::ApplicationPendingTarget>,
) -> Result<StepPaths, AppError> {
    let mut done = BTreeSet::new();
    for dependency in &plan.dependencies {
        if !done.insert(dependency.key.clone()) {
            continue;
        }
        let imported = ctx
            .import_scope
            .map(|scope| {
                scope.dependency(
                    &plan.package.key(),
                    &dependency.package,
                    dependency.scope,
                    dependency.step.as_deref(),
                )
            })
            .transpose()?
            .flatten();
        if imported.is_some_and(|binding| binding.application.key != dependency.key) {
            return Err(LifecycleError::RequirementUnresolved {
                key: plan.package.key(),
                requirement: dependency.package.clone(),
                reason: "imported exact key differs from retained dependency plan".into(),
            }
            .into());
        }
        if let Some(parent) = parent
            && let Some(target) = super::pending::bound_child(ctx, parent, &dependency.key)?
        {
            let package = super::pending::coordinate(&target.key)?;
            let state = super::pending::load(ctx.settings, &package, Some(&target.operation))?;
            let op = state.operation.as_ref().ok_or_else(|| {
                super::pending::failure(
                    ctx.settings,
                    &package,
                    "missing bound child operation".into(),
                )
            })?;
            if op.phase != vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
            {
                if op.parent.as_ref() != Some(&super::pending::application_reference(parent))
                    || op.kind
                        != vibe_wire::generated::shared::ApplicationPendingOperationKind::Install
                {
                    return Err(super::pending::failure(
                        ctx.settings,
                        &package,
                        format!(
                            "continue the dependency's actual operation separately with `{}`",
                            super::pending::command(&target)
                        ),
                    ));
                }
                let answers = imported
                    .and(ctx.import_scope)
                    .map_or(&[][..], |scope| scope.answers(&dependency.key));
                if let Err(error) = super::pending::continue_target_with_parameters(
                    lock, ctx, &target, answers, stack,
                ) {
                    if let Some(scope) = imported.and(ctx.import_scope) {
                        return Err(scope.input_error(&dependency.key, error));
                    }
                    if matches!(
                        &error,
                        AppError::Configuration(_)
                            | AppError::Step(crate::StepError::ParameterInvalid { .. })
                    ) {
                        return Err(LifecycleError::RequirementUnresolved{key:parent.key.clone(),requirement:dependency.package.clone(),reason:format!("{error}; supply this child's ephemeral input using `{}` (declared secret IDs: {})",super::pending::command(&target),op.secret_required.join(", "))}.into());
                    }
                    return Err(error);
                }
            }
            super::pending::ack_child(ctx.settings, parent, &dependency.key)?;
            continue;
        }
        if dependency.installed {
            continue;
        }
        let Some(resolved) = dependency.resolved.as_deref() else {
            continue;
        };
        let option = resolved
            .options
            .iter()
            .find(|option| Some(&option.id) == dependency.option.as_ref());
        let mut request = InstallRequest {
            choice: InstallChoice {
                platform: option.map(|option| option.platform.clone()),
                arch: option.map(|option| option.arch.clone()),
                installer: dependency.option.clone(),
            },
            hidden: dependency.hidden,
            mark: Mark::Auto,
            accept_license,
            required_by: Some(RequiredBy {
                package: plan.package.key(),
                step: dependency.step.clone(),
            }),
            ..InstallRequest::default()
        };
        if let Some(import) = ctx.import_scope
            && let Some(binding) = imported
        {
            if binding.application.key != dependency.key {
                return Err(LifecycleError::RequirementUnresolved {
                    key: plan.package.key(),
                    requirement: dependency.package.clone(),
                    reason: "imported exact dependency differs from the prepared plan".into(),
                }
                .into());
            }
            let (current, selected) = import.admit(ctx, binding)?;
            if current.package != resolved.package {
                return Err(LifecycleError::RequirementUnresolved {
                    key: plan.package.key(),
                    requirement: dependency.package.clone(),
                    reason: "imported target source changed before dependency execution".into(),
                }
                .into());
            }
            let required_by = request.required_by.take();
            request = import.request(ctx, binding, &selected, accept_license)?;
            request.required_by = required_by;
        }
        // The requirement's whole install is observed under one task, so a
        // step panel draws its steps as a subtree of the requirement
        // instead of mixing them in among the requiring package's own
        // (`PROP-065#display`).
        // The structured `requires` lets a consumer of the machine stream key
        // the subtree against the plan's `dependencies[]` without parsing
        // the label (PROP-060 `#MACHINE-STREAM`).
        let task = ctx.progress.task_requiring(
            requirement_label(dependency),
            Requires {
                key: dependency.key.clone(),
                scope: dependency.scope.as_str().to_owned(),
                step: dependency.step.clone(),
            },
        );
        let progress = task.progress();
        let nested = OperationContext {
            progress: &progress,
            ..*ctx
        };
        match super::install_locked(
            lock,
            &nested,
            resolved,
            &request,
            stack,
            None,
            parent.map(|p| super::pending::Parent::Dependency {
                target: p,
                requirement: dependency.package.as_str(),
            }),
        ) {
            Ok(_) => {
                if let Some(parent) = parent {
                    super::pending::ack_child(ctx.settings, parent, &dependency.key)?;
                }
                task.finish();
            }
            Err(error) => {
                task.fail(error.to_string());
                if let Some(scope) = imported.and(ctx.import_scope) {
                    return Err(scope.input_error(&dependency.key, error));
                }
                if ctx.import_scope.is_some()
                    && matches!(
                        &error,
                        AppError::Configuration(
                            crate::configuration::ConfigurationError::Unresolved { .. }
                        )
                    )
                {
                    return Err(LifecycleError::RequirementUnresolved {
                        key: plan.package.key(),
                        requirement: dependency.package.clone(),
                        reason: format!("target-only dependency `{}` needs ordinary configuration input ({}); continue in an attended session or configure that dependency explicitly; it has no imported binding", dependency.key, error.code()),
                    }.into());
                }
                return Err(error);
            }
        }
    }
    let mut paths = StepPaths::default();
    if plan.dependencies.is_empty() {
        return Ok(paths);
    }
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    for dependency in &plan.dependencies {
        let Some(record) = index.installations.get(&dependency.key) else {
            continue;
        };
        let exposure = exposure_of(
            ctx.settings,
            &storage,
            &dependency.key,
            record,
            ctx.proxy_image,
            lock,
        )?;
        match &dependency.step {
            None => paths.option.push(exposure),
            Some(step) => paths.steps.entry(step.clone()).or_default().push(exposure),
        }
    }
    Ok(paths)
}

/// Full bindings cannot reuse an old index row while an unindexed replacement
/// of that package family is unfinished. Ordinary no-import selection is unchanged.
fn imported_work_ready(
    ctx: &OperationContext<'_>,
    index: &Index,
    package: &crate::PackageCoordinate,
    completed_by_cohort: bool,
) -> Result<(), AppError> {
    let fail = |reason: &str| super::pending::failure(ctx.settings, package, reason.into());
    let name = package.directory_name();
    let prefix = name
        .strip_suffix(&package.version)
        .ok_or_else(|| fail("invalid canonical work identity"))?;
    let entries = match std::fs::read_dir(ctx.settings.app_work_dir()) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(fail("cannot inspect imported dependency work")),
    };
    for entry in entries {
        let entry = entry.map_err(|_| fail("imported dependency work enumeration failed"))?;
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        let path = entry.path();
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            return Err(fail("imported dependency has unsafe or unknown work"));
        }
        let state = crate::steps::StepsFile::load(&path)
            .map_err(|_| fail("imported dependency work is unreadable"))?
            .ok_or_else(|| fail("imported dependency work has no known completion"))?;
        if !state.package.same_package(package)
            || path != super::pending::work(ctx.settings, &state.package)
        {
            return Err(fail("imported dependency work identity differs"));
        }
        if let Some(op) = &state.operation {
            super::pending::load(ctx.settings, &state.package, Some(&op.target.operation))
                .map_err(|_| fail("imported dependency work has invalid home or generation"))?;
            if let Some(scope) = ctx.import_scope
                && scope.released_refusal(ctx.settings, &state)?
            {
                continue;
            }
            if op.phase != vibe_wire::generated::shared::ApplicationPendingOperationPhase::Complete
                || op.publication_record.is_none()
                || op.parent.is_some() && !(completed_by_cohort && state.package == *package)
            {
                return Err(fail(
                    "imported dependency has unfinished, unpublished or retained work; continue its exact owner first",
                ));
            }
        } else if !index.installations.contains_key(&state.package.key())
            || state.steps.iter().any(|s| {
                !matches!(
                    s.outcome,
                    crate::index::StepOutcome::Done | crate::index::StepOutcome::Skipped
                )
            })
        {
            return Err(fail(
                "imported dependency legacy work is not known complete",
            ));
        }
    }
    Ok(())
}

/// The progress row one required package's install hangs under: its key,
/// its scope and what asked for it (`PROP-065#display`).
fn requirement_label(dependency: &PlannedDependency) -> String {
    let scope = dependency.scope.as_str();
    match &dependency.step {
        Some(step) => format!("requires {} ({scope}) for step {step}", dependency.key),
        None => format!("requires {} ({scope})", dependency.key),
    }
}

/// The requirements an installation records from its plan.
pub(super) fn recorded(plan: &Plan) -> Vec<Requirement> {
    plan.dependencies
        .iter()
        .map(|dependency| Requirement {
            package: dependency.package.clone(),
            key: dependency.key.clone(),
            scope: dependency.scope,
            step: dependency.step.clone(),
            // A planned dependency is a declared requirement
            // (`PROP-065#app-dependencies`); only the recommendation walk
            // records a weak edge (`PROP-077#APP-RECOMMENDS-MECHANICS`).
            weak: false,
        })
        .collect()
}
