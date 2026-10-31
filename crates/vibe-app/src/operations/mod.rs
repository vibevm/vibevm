//! `install -g`, `update -g`, `uninstall -g` and the inventory listing: the
//! slot default rules, replace-in-place for one key, and the index written
//! once per operation after every child succeeded.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

mod apply;
mod remove;

pub use remove::{installed, list, uninstall};

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{InstallerOptionDecl, result_of};
use vibe_core::progress::Progress;

use crate::agents::AgentHooks;
use crate::alternatives::{check_reservation, reserved_names};
use crate::error::{AppError, ResolveError};
use crate::index::{Index, Location, Method};
use crate::installer::Interrupt;
use crate::proxies;
use crate::resolve::ResolvedPackage;
use crate::select::{Chooser, InstallChoice, select_option};
use crate::settings::SettingsRoot;
use crate::steps::{
    self, FeatureChooser, FeatureRequest, HostFacts, LicensePrompt, ParameterPrompt, Plan,
    PlanInput, Previous, Prompts, Start,
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
}

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
}

/// The command an outcome reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub enum Command {
    Install,
    Update,
    Uninstall,
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

/// Install one app package into the current user's settings root.
pub fn install(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Outcome, AppError> {
    let _lock = ctx.settings.lock()?;
    let mut index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let option = choose(ctx, resolved, &request.choice)?;
    let key = resolved.package.key();
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let prior = index.installations.get(&key).cloned();
    if let Some(prior) = &prior {
        ensure_available(&key, prior, &storage)?;
    }
    let plan = plan_option(ctx, resolved, &option, request, None)?;
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
        },
    )?;
    let replaced = prior.is_some();
    let finish_failure = applied.finish_failure.take();
    let outcome = commit(ctx, &storage, &mut index, applied, None, Command::Install)?;
    if let Some(failure) = finish_failure {
        return Err(failure);
    }
    Ok(Outcome {
        message: format!(
            "{} {} via installer `{}`",
            if replaced { "replaced" } else { "installed" },
            outcome.key,
            option.id
        ),
        ..outcome
    })
}

/// Update one installed app package to the newest resolved version.
pub fn update(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Outcome, AppError> {
    let _lock = ctx.settings.lock()?;
    let mut index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let prior_key = previous_installation(&index, resolved)?;
    let prior = index
        .installations
        .get(&prior_key)
        .cloned()
        .ok_or_else(|| AppError::NotInstalled {
            package: prior_key.clone(),
        })?;
    if prior.slot != resolved.application.id {
        return Err(AppError::SlotChanged {
            key: prior_key,
            from: prior.slot,
            to: resolved.application.id.clone(),
        });
    }
    ensure_available(&prior_key, &prior, &storage)?;
    let explicit = request.choice != InstallChoice::default();
    let choice = if explicit {
        request.choice.clone()
    } else {
        InstallChoice {
            platform: Some(prior.platform.clone()),
            arch: Some(prior.arch.clone()),
            installer: Some(prior.option.clone()),
        }
    };
    let option = choose(ctx, resolved, &choice)?;
    let key = resolved.package.key();
    admit(
        ctx,
        &index,
        &[key.as_str(), prior_key.as_str()],
        resolved,
        &option,
    )?;
    let owned = result_of(&option).is_owned();
    let same_key = index.installations.get(&key).cloned();
    if let Some(existing) = &same_key {
        ensure_available(&key, existing, &storage)?;
    }
    let (target_prior, library, replaces) = if owned {
        let library = request.library.clone().or(match &prior.location {
            Location::Owned { library, .. } => Some(library.clone()),
            Location::External { .. } => None,
        });
        (same_key, library, None)
    } else if matches!(prior.location, Location::Owned { .. }) {
        (same_key, request.library.clone(), None)
    } else {
        (
            Some(prior.clone()),
            request.library.clone(),
            Some(prior_key.clone()),
        )
    };
    let previous = Previous {
        features: &prior.features,
        parameters: &prior.parameters,
        license: prior.license.as_ref().map(|license| license.name.as_str()),
    };
    let plan = plan_option(ctx, resolved, &option, request, Some(previous))?;
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            resolved,
            option: &option,
            plan: &plan,
            prior: target_prior.as_ref(),
            library: library.as_deref(),
            force: request.force,
            mode: Mode::Update,
            start: &request.start,
            command: Command::Update,
        },
    )?;
    let finish_failure = applied.finish_failure.take();
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        replaces,
        Command::Update,
    )?;
    if let Some(failure) = finish_failure {
        return Err(failure);
    }
    Ok(Outcome {
        message: if prior_key == key {
            format!("refreshed {key} in place")
        } else {
            format!("updated {prior_key} to {key}")
        },
        ..outcome
    })
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
    plan_option(ctx, resolved, &option, request, None)
}

/// Compute the plan of the selected option and show it.
fn plan_option(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    previous: Option<Previous<'_>>,
) -> Result<Plan, AppError> {
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
    })?;
    if let Some(show) = ctx.on_plan {
        show(&plan);
    }
    Ok(plan)
}

/// Record an applied installation, apply the slot default rules, write the
/// index, then delete retired directories (a failure leaves an orphan).
fn commit(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    index: &mut Index,
    applied: Applied,
    replaces: Option<String>,
    command: Command,
) -> Result<Outcome, AppError> {
    let reserved = reserved_names(index);
    let installation = applied.installation;
    let key = installation.package.key();
    let slot = installation.slot.clone();
    if let Some(old) = replaces.filter(|old| old != &key) {
        index.installations.remove(&old);
        if index.default_of(&slot) == Some(old.as_str()) {
            index.slots.insert(slot.clone(), Some(key.clone()));
        }
    }
    let current_default = index
        .default_of(&slot)
        .and_then(|default| index.installations.get(default))
        .map(|record| record.package.clone());
    let take_default = match (&current_default, command) {
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
fn previous_installation(index: &Index, resolved: &ResolvedPackage) -> Result<String, AppError> {
    let mut candidates: Vec<(&String, semver::Version)> = index
        .installations
        .iter()
        .filter(|(_, record)| record.package.same_package(&resolved.package))
        .map(|(key, record)| {
            let version = semver::Version::parse(&record.package.version)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
            (key, version)
        })
        .collect();
    if candidates.is_empty() {
        return Err(AppError::NotInstalled {
            package: format!("{}/{}", resolved.package.group, resolved.package.name),
        });
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
    installation: &crate::index::Installation,
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

/// Drop dangling defaults and slots that have no installation left.
fn normalize_slots(index: &mut Index) {
    let installations = &index.installations;
    index
        .slots
        .retain(|slot, _| installations.values().any(|record| &record.slot == slot));
    for (slot, default) in &mut index.slots {
        if default.as_ref().is_some_and(|key| {
            installations
                .get(key)
                .is_none_or(|record| &record.slot != slot)
        }) {
            *default = None;
        }
    }
    for record in installations.values() {
        index.slots.entry(record.slot.clone()).or_insert(None);
    }
}
