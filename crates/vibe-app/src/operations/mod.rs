//! `install -g`, `update -g`, `uninstall -g` and the inventory listing: the
//! slot default rules, replace-in-place for one key, and the index written
//! once per operation after every child succeeded.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

mod apply;
mod deps;
mod lifecycle;
mod listing;
mod modify;
mod outdated;
mod remove;
mod tree;
mod update;

pub use lifecycle::{Change, ChangeVerb, expose, hide, hold, mark, rollback};
pub use listing::{installed, list};
pub use modify::{ModifyRequest, modify};
pub use outdated::outdated;
pub use remove::{Removal, RemoveRequest, autoremove, remove_unrecorded, uninstall};
pub use update::update;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::InstallerOptionDecl;
use vibe_core::progress::Progress;

use crate::agents::AgentHooks;
use crate::alternatives::{check_reservation, reserved_names};
use crate::environment::UserEnvironment;
use crate::error::{AppError, ResolveError};
use crate::index::{Index, Installation, Lifecycle, Location, Mark, Method};
use crate::installer::Interrupt;
use crate::proxies;
use crate::resolve::ResolvedPackage;
use crate::select::{Chooser, InstallChoice, select_option};
use crate::settings::SettingsRoot;
use crate::steps::{
    self, FeatureChooser, FeatureRequest, HostFacts, LicensePrompt, ParameterPrompt, Plan,
    PlanInput, Previous, Prompts, RequirementResolver, Start,
};
use crate::storage::{Availability, Storage};

use apply::{Applied, Mode, Selected};

/// What every mutating operation needs from its caller.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub struct OperationContext<'a> {
    pub settings: &'a SettingsRoot,
    /// Relay child stdout to stderr so a JSON envelope stays clean.
    pub json: bool,
    pub offline: bool,
    /// Present only in an attended terminal.
    pub chooser: Option<Chooser<'a>>,
    /// The `vibe-app-proxy` image copied into `opt/bin` for every reserved
    /// command; the caller locates it beside its own executable.
    pub proxy_image: &'a Path,
    /// PATH and environment lookups for conditions and needs.
    pub facts: HostFacts<'a>,
    /// The attended feature question (`PROP-065#features`).
    pub feature_chooser: Option<FeatureChooser<'a>>,
    /// The attended typed parameter questions (`PROP-065#parameters`).
    pub parameter_prompt: Option<ParameterPrompt<'a>>,
    /// The attended license question (`PROP-065#license`).
    pub license_prompt: Option<LicensePrompt<'a>>,
    /// Shown the plan before the first step runs (`PROP-065#plan`).
    pub on_plan: Option<&'a dyn Fn(&Plan)>,
    /// Step tasks, fetched bytes and child output lines are observed here
    /// (`PROP-065#events`).
    pub progress: &'a Progress,
    /// Child output reaches only the run log and the progress stream.
    pub quiet_children: bool,
    /// Signals the caller received (`PROP-065#cancellation`).
    pub interrupt: Option<&'a dyn Interrupt>,
    /// The agent of `agent` steps, consent and parking (`PROP-064`).
    pub agent: AgentHooks<'a>,
    /// Resolves a required application package by its `group/name@req`
    /// spelling through the same registries as the requiring package
    /// (`PROP-065#app-dependencies`); absent, only installed requirements
    /// are usable.
    pub resolver: Option<PackageResolver<'a>>,
    /// The durable user environment env steps change (`PROP-065#env-step`).
    pub environment: Option<&'a dyn UserEnvironment>,
    /// The caller's PATH, behind the requirements a step's child sees
    /// first (`PROP-065#app-dependencies`).
    pub search_path: Option<&'a OsStr>,
}

/// Resolves one application package spelling to its package tree.
pub type PackageResolver<'a> = &'a dyn Fn(&str) -> Result<ResolvedPackage, AppError>;

impl OperationContext<'_> {
    /// Where child output goes besides the run log.
    pub(crate) fn relay(&self) -> crate::installer::Relay {
        use crate::installer::Relay;
        match (self.quiet_children, self.json) {
            (true, _) => Relay::Quiet,
            (false, true) => Relay::Stderr,
            (false, false) => Relay::Live,
        }
    }
}

/// The selection and placement flags of `install -g` / `update -g`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub struct InstallRequest {
    pub choice: InstallChoice,
    pub library: Option<String>,
    pub force: bool,
    /// `--profile`, `--feature` / `--no-feature`.
    pub features: FeatureRequest,
    /// `--set ID=VALUE` (`PROP-065#parameters`).
    pub parameters: Vec<(String, String)>,
    /// `--accept-license` (`PROP-065#license`).
    pub accept_license: bool,
    /// `--resume` / `--from-step`.
    pub start: Start,
    /// `--hidden`: install without exposure (`PROP-059#hidden-installations`).
    pub hidden: bool,
    /// `manual` for a package named by the user, `auto` for a requirement.
    pub mark: Mark,
    /// `update --purge-previous`: remove the replaced installation at once
    /// instead of keeping it hidden (`PROP-065#update-and-rollback`).
    pub purge_previous: bool,
}

/// The command an outcome reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub enum Command {
    Install,
    Update,
    Uninstall,
    /// Another choice for an installed package (`PROP-065#modify`).
    Modify,
}

/// The result of one operation, for the caller to render.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub struct Outcome {
    pub command: Command,
    pub slot: String,
    pub key: String,
    pub method: Method,
    pub app_root: Option<PathBuf>,
    pub slot_default: Option<String>,
    pub message: String,
}

/// Install one app package into the current user's settings root: its
/// requirements first (`PROP-065#app-dependencies`), then the package.
pub fn install(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Outcome, AppError> {
    let _lock = ctx.settings.lock()?;
    install_locked(ctx, resolved, request, &mut Vec::new())
}

/// [`install`] under the held lock; `stack` names the installations in
/// progress further up, so a requirement cycle is refused.
fn install_locked(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    let key = resolved.package.key();
    deps::enter(stack, &key)?;
    let result = install_entered(ctx, resolved, request, stack);
    stack.pop();
    result
}

fn install_entered(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let option = choose(ctx, resolved, &request.choice)?;
    let key = resolved.package.key();
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let prior = index.installations.get(&key).cloned();
    if let Some(prior) = &prior {
        ensure_available(&key, prior, &storage)?;
    }
    let plan = plan_option(ctx, resolved, &option, request, None, &index)?;
    let paths = deps::install_required(ctx, &plan, request.accept_license, stack)?;
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            resolved,
            option: &option,
            plan: &plan,
            prior: prior.as_ref(),
            library: request.library.as_deref(),
            force: request.force,
            mode: Mode::Install,
            start: &request.start,
            command: Command::Install,
            replaces: None,
            paths,
        },
    )?;
    let kept = prior.as_ref().map(|prior| &prior.lifecycle);
    applied.installation.lifecycle = Lifecycle {
        hidden: request.hidden,
        mark: match (request.mark, kept.map(|kept| kept.mark)) {
            (Mark::Auto, Some(Mark::Manual)) => Mark::Manual,
            (mark, _) => mark,
        },
        requires: deps::recorded(&plan),
        env: std::mem::take(&mut applied.installation.lifecycle.env),
        held: kept.is_some_and(|kept| kept.held),
        replaced: None,
        superseded_by: None,
    };
    if let Some(prior) = &prior {
        retire_env(ctx, &key, prior, &applied.installation)?;
    }
    let replaced = prior.is_some();
    let finish_failure = applied.finish_failure.take();
    let mut index = Index::load(ctx.settings)?;
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        None,
        None,
        Command::Install,
    )?;
    if let Some(failure) = finish_failure {
        return Err(failure);
    }
    Ok(Outcome {
        message: format!(
            "{} {}{} via installer `{}`",
            if replaced { "replaced" } else { "installed" },
            outcome.key,
            if request.hidden { " hidden" } else { "" },
            option.id
        ),
        ..outcome
    })
}

/// Remove the environment entries the previous record of the same key
/// applied and the new one no longer does (`PROP-065#env-step`).
fn retire_env(
    ctx: &OperationContext<'_>,
    key: &str,
    prior: &Installation,
    next: &Installation,
) -> Result<(), AppError> {
    let stale: Vec<_> = prior
        .lifecycle
        .env
        .iter()
        .filter(|entry| {
            !next.lifecycle.env.iter().any(|kept| {
                kept.step == entry.step && kept.op == entry.op && kept.name == entry.name
            })
        })
        .cloned()
        .collect();
    remove::rollback_env(ctx, key, "env", &stale)
}

/// Plan one installation without downloading or running anything
/// (`vibe app plan`, `install --plan-only`): the choice, features,
/// parameters and license acceptance of `request`.
pub fn plan(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Plan, AppError> {
    let option = choose(ctx, resolved, &request.choice)?;
    let index = Index::load(ctx.settings)?;
    plan_option(ctx, resolved, &option, request, None, &index)
}

/// Compute the plan of the selected option and show it.
fn plan_option(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    previous: Option<Previous<'_>>,
    index: &Index,
) -> Result<Plan, AppError> {
    let owner = resolved.package.key();
    let requirements = |spelling: &str, scope, step: Option<&str>| {
        deps::resolve_requirement(
            ctx,
            index,
            &owner,
            spelling,
            scope,
            step,
            request.accept_license,
        )
    };
    let plan = steps::plan(&PlanInput {
        resolved,
        option,
        request: &request.features,
        parameters: &request.parameters,
        accept_license: request.accept_license,
        previous,
        settings: ctx.settings,
        facts: ctx.facts,
        offline: ctx.offline,
        attended: ctx.chooser.is_some(),
        prompts: Prompts {
            chooser: ctx.chooser,
            features: ctx.feature_chooser,
            parameter: ctx.parameter_prompt,
            license: ctx.license_prompt,
        },
        agent: ctx.agent,
        requirements: Some(&requirements as RequirementResolver<'_>),
    })?;
    if let Some(show) = ctx.on_plan {
        show(&plan);
    }
    Ok(plan)
}

/// Record an applied installation, refresh its icon copies, apply the slot
/// default rules, write the index, then delete retired directories and the
/// copies of a replaced key (a failure leaves an orphan).
fn commit(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    index: &mut Index,
    applied: Applied,
    replaces: Option<String>,
    supersede: Option<String>,
    command: Command,
) -> Result<Outcome, AppError> {
    let reserved = reserved_names(index);
    let installation = applied.installation;
    let key = installation.package.key();
    let slot = installation.slot.clone();
    crate::icons::copy_package_icons(ctx.settings, &installation, &applied.package_root)?;
    let mut dropped = None;
    if let Some(old) = replaces.filter(|old| old != &key) {
        dropped = index
            .installations
            .remove(&old)
            .map(|record| record.package);
        if index.default_of(&slot) == Some(old.as_str()) {
            index.slots.insert(slot.clone(), Some(key.clone()));
        }
    }
    if let Some(old) = supersede.filter(|old| old != &key)
        && let Some(record) = index.installations.get_mut(&old)
    {
        record.lifecycle.hidden = true;
        record.lifecycle.superseded_by = Some(key.clone());
        if index.default_of(&slot) == Some(old.as_str()) && !installation.lifecycle.hidden {
            index.slots.insert(slot.clone(), Some(key.clone()));
        }
    }
    let current_default = index
        .default_of(&slot)
        .and_then(|default| index.installations.get(default))
        .map(|record| record.package.clone());
    let take_default = !installation.lifecycle.hidden
        && match (&current_default, command) {
            (None, _) => true,
            (Some(package), Command::Update) => package.same_package(&installation.package),
            _ => false,
        };
    let method = installation.method;
    let app_root = crate::launch::app_root(&installation, storage).ok();
    index.installations.insert(key.clone(), installation);
    if take_default {
        index.slots.insert(slot.clone(), Some(key.clone()));
    }
    normalize_slots(index);
    index.save(ctx.settings)?;
    for directory in applied.retired {
        let _ = fs::remove_dir_all(directory);
    }
    if let Some(package) = dropped {
        crate::icons::remove_package_icons(ctx.settings, &package);
    }
    proxies::sync(
        ctx.settings,
        &reserved,
        &reserved_names(index),
        ctx.proxy_image,
    )?;
    Ok(Outcome {
        command,
        slot_default: index.default_of(&slot).map(str::to_owned),
        slot,
        key,
        method,
        app_root,
        message: String::new(),
    })
}

/// Checks that run before any child: the option's public commands are not
/// reserved by another slot, and the proxy image exists when it is needed.
fn admit(
    ctx: &OperationContext<'_>,
    index: &Index,
    replacing: &[&str],
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
) -> Result<(), AppError> {
    check_reservation(
        index,
        replacing,
        &resolved.application.id,
        &option.public_commands,
    )?;
    if !option.public_commands.is_empty() {
        proxies::require_image(ctx.proxy_image)?;
    }
    Ok(())
}

fn choose(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    choice: &InstallChoice,
) -> Result<InstallerOptionDecl, AppError> {
    if resolved.options.is_empty() {
        return Err(ResolveError::NotGloballyInstallable {
            package: resolved.package.key(),
        }
        .into());
    }
    Ok(select_option(&resolved.options, choice, ctx.chooser)?)
}

/// The installed version an update starts from: the slot default when it is
/// this package, otherwise the newest installed version.
fn previous_installation(
    home: &SettingsRoot,
    index: &Index,
    resolved: &ResolvedPackage,
) -> Result<String, AppError> {
    let mut candidates: Vec<(&String, semver::Version)> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.same_package(&resolved.package)
                && record.lifecycle.superseded_by.is_none()
        })
        .map(|(key, record)| {
            let version = semver::Version::parse(&record.package.version)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
            (key, version)
        })
        .collect();
    if candidates.is_empty() {
        return Err(AppError::not_installed(
            home,
            format!("{}/{}", resolved.package.group, resolved.package.name),
        ));
    }
    if let Some(default) = index.default_of(&resolved.application.id)
        && let Some((key, _)) = candidates.iter().find(|(key, _)| key.as_str() == default)
    {
        return Ok((*key).clone());
    }
    candidates.sort_by(|left, right| left.1.cmp(&right.1));
    Ok(candidates
        .last()
        .map(|(key, _)| (*key).clone())
        .unwrap_or_default())
}

/// An installation whose library is unavailable is never replaced.
fn ensure_available(
    key: &str,
    installation: &Installation,
    storage: &Storage,
) -> Result<(), AppError> {
    if let Location::Owned { library, .. } = &installation.location
        && let Availability::Unavailable(reason) = storage.availability(library)
    {
        return Err(AppError::Unavailable {
            key: key.into(),
            reason,
        });
    }
    Ok(())
}

/// Drop dangling and hidden defaults and slots that have no installation
/// left.
fn normalize_slots(index: &mut Index) {
    let installations = &index.installations;
    index
        .slots
        .retain(|slot, _| installations.values().any(|record| &record.slot == slot));
    for (slot, default) in &mut index.slots {
        if default.as_ref().is_some_and(|key| {
            installations
                .get(key)
                .is_none_or(|record| &record.slot != slot || record.lifecycle.hidden)
        }) {
            *default = None;
        }
    }
    for record in installations.values() {
        index.slots.entry(record.slot.clone()).or_insert(None);
    }
}
