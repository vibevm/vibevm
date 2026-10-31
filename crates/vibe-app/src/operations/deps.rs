//! Requirements between application packages (`PROP-065#app-dependencies`):
//! resolution through the home's index first and the caller's package
//! resolver otherwise, installation of the missing ones ahead of the
//! requiring package under the same consent, and the exposure of every
//! requirement on the requiring steps' PATH.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#app-dependencies");

use std::collections::BTreeSet;

use vibe_core::manifest::RequirementScope;

use crate::error::AppError;
use crate::error_lifecycle::LifecycleError;
use crate::exposure::exposure_of;
use crate::index::{Index, Installation, Mark, Requirement};
use crate::resolve::qualified_ref;
use crate::select::{InstallChoice, select_option};
use crate::steps::{Plan, PlannedDependency, StepPaths};
use crate::storage::Storage;

use super::{InstallRequest, OperationContext};

/// Resolve one `requires` entry of `owner`: an installed version that
/// satisfies it is used as is (the newest one that is not a kept previous
/// installation), otherwise the caller's resolver names the package and
/// its installer option for this host is chosen without asking.
pub(super) fn resolve_requirement(
    ctx: &OperationContext<'_>,
    index: &Index,
    owner: &str,
    spelling: &str,
    scope: RequirementScope,
    step: Option<&str>,
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
    let resolved = resolver(spelling).map_err(|error| unresolved(error.to_string()))?;
    let option = select_option(&resolved.options, &InstallChoice::default(), None)
        .map_err(|error| unresolved(error.to_string()))?;
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
/// `auto`; then expose each requirement for the steps that need it.
pub(super) fn install_required(
    ctx: &OperationContext<'_>,
    plan: &Plan,
    stack: &mut Vec<String>,
) -> Result<StepPaths, AppError> {
    let mut done = BTreeSet::new();
    for dependency in &plan.dependencies {
        if dependency.installed || !done.insert(dependency.key.clone()) {
            continue;
        }
        let Some(resolved) = dependency.resolved.as_deref() else {
            continue;
        };
        let option = resolved
            .options
            .iter()
            .find(|option| Some(&option.id) == dependency.option.as_ref());
        let request = InstallRequest {
            choice: InstallChoice {
                platform: option.map(|option| option.platform.clone()),
                arch: option.map(|option| option.arch.clone()),
                installer: dependency.option.clone(),
            },
            hidden: dependency.hidden,
            mark: Mark::Auto,
            ..InstallRequest::default()
        };
        super::install_locked(ctx, resolved, &request, stack)?;
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
        )?;
        match &dependency.step {
            None => paths.option.push(exposure),
            Some(step) => paths.steps.entry(step.clone()).or_default().push(exposure),
        }
    }
    Ok(paths)
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
        })
        .collect()
}
