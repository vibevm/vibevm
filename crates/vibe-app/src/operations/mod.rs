//! `install -g`, `update -g`, `uninstall -g` and the inventory listing: the
//! slot default rules, replace-in-place for one key, and the index written
//! once per operation after every child succeeded.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#alternatives");

mod apply;
pub(crate) mod import;
mod install;
mod planning;
pub use import::{
    ImportResumeInfo, ImportScope, import_cohort, import_full, import_resume_info,
    inspect_import_resume, resume_full_import, resume_import,
};
pub use install::install;
use install::install_locked;
pub use planning::{configuration_plan, plan};
use planning::{plan_option, plan_option_with_modes};
mod deps;
mod lifecycle;
#[path = "recommends.rs"]
mod recommends;
pub use recommends::{Recommendation, RecommendationOutcome};
mod listing;
mod modify;
mod outdated;
pub(crate) mod pending;
pub use pending::{ResumeInfo, ResumeRequest, resume, resume_info};
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
use crate::index::{Index, Installation, Location, Mark, Method, PackageCoordinate};
use crate::installer::{Interrupt, StepLogs, StepPanel};
use crate::proxies;
use crate::resolve::ResolvedPackage;
use crate::select::{Chooser, InstallChoice, select_option};
use crate::settings::SettingsRoot;
use crate::steps::{
    FeatureChooser, FeatureRequest, HostFacts, LicensePrompt, ParameterPrompt, Plan, RequiredBy,
    Start,
};
use crate::storage::{Availability, Storage};

use apply::Applied;

/// What every mutating operation needs from its caller.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub struct OperationContext<'a> {
    /// Shared lazy managed-contact owner of this operation, never reloaded from payload metadata.
    pub http_client: Option<&'a vibe_package_source::OperationHttpClient>,
    /// Immutable public bindings of one verified finite import, with ephemeral scoped answers.
    pub import_scope: Option<&'a crate::ImportScope<'a>>,
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
    /// Shown a one-line notice before a run changes something the user
    /// asked to keep (`PROP-065#work`).
    pub on_notice: Option<&'a dyn Fn(&str)>,
    /// Step tasks, fetched bytes and child output lines are observed here
    /// (`PROP-065#events`).
    pub progress: &'a Progress,
    /// Child output reaches only the run log and the progress stream.
    pub quiet_children: bool,
    /// The caller draws a step panel over this run (`PROP-065#display`):
    /// children get no terminal of their own and their output reaches only
    /// the run log and the progress stream, and the panel is released
    /// around a child that takes the terminal.
    pub panel: Option<&'a dyn StepPanel>,
    /// Told the output log of each step that runs a child, before it starts
    /// (`PROP-065#display`): the caller names it or shows its tail.
    pub on_step_log: Option<&'a dyn StepLogs>,
    /// Signals the caller received (`PROP-065#cancellation`).
    pub interrupt: Option<&'a dyn Interrupt>,
    /// The agent of `agent` steps, consent and parking (`PROP-064`).
    pub agent: AgentHooks<'a>,
    /// Resolves a required application package by its `group/name@req`
    /// spelling through the same registries as the requiring package
    /// (`PROP-065#app-dependencies`); absent, only installed requirements
    /// are usable.
    pub resolver: Option<PackageResolver<'a>>,
    pub resolver_scope: ResolverScope<'a>,
    /// Re-admit a retained project-slot source through its project/lock owner.
    pub source_reader: Option<SavedSourceReader<'a>>,
    /// The durable user environment env steps change (`PROP-065#env-step`).
    pub environment: Option<&'a dyn UserEnvironment>,
    /// The caller's PATH, behind the requirements a step's child sees
    /// first (`PROP-065#app-dependencies`).
    pub search_path: Option<&'a OsStr>,
    /// The user's own `config.toml` — the file
    /// `[slot.<slot>.extras.<ecosystem>]` lives in
    /// (`PROP-075#SLOT-EXTRAS`).
    ///
    /// **This is the `vibe-app` → user-config seam P7c introduces**, and it
    /// is a *path* rather than a resolved config for the reason this crate's
    /// settings root is threaded in the same way: this crate never reads the
    /// environment itself, and the one chokepoint that resolves
    /// `$VIBE_SETTINGS` / `VIBEVM_USER_CONFIG` into a file belongs to the
    /// caller. Reading and interpreting the file is this crate's, so a test
    /// that leaves this `None` behaves exactly as a home with no `[slot]`
    /// table does — which is also exactly how every command that does not
    /// consume extras behaves.
    pub user_config: Option<&'a Path>,
}

/// What an operation that changes a slot's global exposure needs: the
/// `vibe-app-proxy` image its per-command proxies are copied from, and the
/// durable user environment its link directories go on
/// (`PROP-059#path-directories`). Both come from the caller, which owns the
/// process boundary.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
pub struct SlotExposure<'a> {
    pub proxy_image: &'a Path,
    pub environment: Option<&'a dyn UserEnvironment>,
}

impl<'a> SlotExposure<'a> {
    /// The exposure of one operation context.
    pub fn of(ctx: &OperationContext<'a>) -> Self {
        Self {
            proxy_image: ctx.proxy_image,
            environment: ctx.environment,
        }
    }

    /// An exposure with the proxy image alone: usable only where no slot
    /// can gain or lose a payload directory on PATH.
    pub fn image_only(proxy_image: &'a Path) -> Self {
        Self {
            proxy_image,
            environment: None,
        }
    }
}

/// Existing package-lookup owners, retained for descendants not planned before a park.
#[derive(Debug, Clone, Copy, Default)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub enum ResolverScope<'a> {
    #[default]
    Caller,
    Registry(&'a Path),
    Chain {
        embedded: Option<&'a Path>,
    },
    Project(&'a Path),
}

/// Resolves one application package spelling to its package tree.
pub type PackageResolver<'a> = &'a dyn Fn(&str) -> Result<ResolvedPackage, AppError>;
/// Typed callback for source owners outside the app crate; never a newest lookup.
pub type SavedSourceReader<'a> = &'a dyn Fn(
    &vibe_wire::generated::shared::ApplicationPendingSource,
    &PackageCoordinate,
) -> Result<ResolvedPackage, AppError>;

impl OperationContext<'_> {
    /// Where child output goes besides the run log.
    pub(crate) fn relay(&self) -> crate::installer::Relay {
        use crate::installer::Relay;
        match (self.quiet_children || self.panel.is_some(), self.json) {
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
    /// Historical portable meaning for target reassessment, never fresh default approval.
    pub transfer: Option<crate::configuration::PortableTransfer>,
    /// Complete common input; absent maintenance input reuses the retained original.
    pub configuration: Option<crate::configuration::ConfigurationSubmission>,
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
    /// `--keep-work`: leave the work directory of a successful run in
    /// place for the next `--from-step` (`PROP-065#work`).
    pub keep_work: bool,
    /// `--hidden`: install without exposure (`PROP-059#hidden-installations`).
    pub hidden: bool,
    /// `manual` for a package named by the user, `auto` for a requirement.
    pub mark: Mark,
    /// `update --purge-previous`: remove the replaced installation at once
    /// instead of keeping it hidden (`PROP-065#update-and-rollback`).
    pub purge_previous: bool,
    /// Set by the requirement installer, never by a command: the package
    /// and step that asked for this one, so its plan says whose it is
    /// (`PROP-065#app-dependencies`).
    pub required_by: Option<RequiredBy>,
    /// Whether this installation's `[recommends]` are walked after its own
    /// record is committed (`PROP-077#APP-RECOMMENDS-MECHANICS`). Set by
    /// the command the user typed and `--no-recommends` clears it; a
    /// requirement install, an import replay, a resume and each
    /// recommendation of the walk itself leave it `false`, because the walk
    /// enumerates the next level itself and must not recurse through the
    /// installer.
    pub recommends: bool,
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
    /// The work directory `--keep-work` kept (`PROP-065#work`).
    pub work: Option<PathBuf>,
    /// What became of each `[recommends]` reference the walk reached, in
    /// the order it reached them (`PROP-077#APP-RECOMMENDS`). Empty when
    /// nothing recommended anything and when the walk was not entered.
    pub recommended: Vec<Recommendation>,
    /// The `[suggests]` of the packages this operation installed, reported
    /// once and installed nowhere (`PROP-077#SUGGESTS-UNCHANGED`).
    pub suggested: Vec<String>,
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

/// Record an applied installation, refresh its icon copies, apply the slot
/// default rules, write the index, then delete retired directories and the
/// copies of a replaced key (a failure leaves an orphan).
struct Publication<'a> {
    lock: &'a crate::MutationLock,
    command: Command,
    preserve_default: bool,
}
fn commit(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    index: &mut Index,
    applied: Applied,
    replaces: Option<String>,
    supersede: Option<String>,
    publication: Publication<'_>,
) -> Result<Outcome, AppError> {
    let Publication {
        lock,
        command,
        preserve_default,
    } = publication;
    let reserved = reserved_names(index);
    let work = applied.work;
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
            index.set_default(&slot, Some(key.clone()));
        }
    }
    if let Some(old) = supersede.filter(|old| old != &key)
        && let Some(record) = index.installations.get_mut(&old)
    {
        record.lifecycle.hidden = true;
        record.lifecycle.superseded_by = Some(key.clone());
        if index.default_of(&slot) == Some(old.as_str()) && !installation.lifecycle.hidden {
            index.set_default(&slot, Some(key.clone()));
        }
    }
    let current_default = index
        .default_of(&slot)
        .and_then(|default| index.installations.get(default))
        .map(|record| record.package.clone());
    let take_default = !preserve_default
        && !installation.lifecycle.hidden
        && match (&current_default, command) {
            (None, _) => true,
            (Some(package), Command::Update) => package.same_package(&installation.package),
            _ => false,
        };
    let method = installation.method;
    let app_root = crate::launch::app_root(&installation, storage).ok();
    index.installations.insert(key.clone(), installation);
    if take_default {
        index.set_default(&slot, Some(key.clone()));
    }
    normalize_slots(index);
    let changed = match command {
        Command::Install => crate::catalog::EventKind::Installed,
        Command::Update | Command::Modify | Command::Uninstall => {
            crate::catalog::EventKind::Updated
        }
    };
    let image = proxies::for_publication(ctx.settings, index, ctx.proxy_image)?;
    index.publish_locked(
        ctx.settings,
        &crate::catalog::HomeEvent::about(changed, key.clone()),
        lock,
        image.as_ref(),
    )?;
    // The user layer follows the inventory: the collection this package's
    // `menu_group` names gains its menu launchers (`PROP-059#launcher-catalog`).
    if let Some(record) = index.installations.get(&key) {
        crate::catalog::seed_package(ctx.settings, record)?;
    }
    for directory in applied.retired {
        let _ = fs::remove_dir_all(directory);
    }
    if let Some(package) = dropped {
        crate::icons::remove_package_icons(ctx.settings, &package);
    }
    proxies::sync_captured(
        ctx.settings,
        &ctx.settings.proxy_dir(),
        &reserved,
        &reserved_names(index),
        image.as_ref(),
        lock,
    )?;
    // Whole payload directories on PATH follow the inventory the same way
    // (`PROP-059#path-directories`): first install of a slot creates its
    // links and puts them on the durable PATH, an update or a modify
    // repoints them at the new payload.
    crate::paths::apply_locked(ctx.settings, index, storage, ctx.environment, lock)?;
    Ok(Outcome {
        command,
        slot_default: index.default_of(&slot).map(str::to_owned),
        slot,
        key,
        method,
        app_root,
        message: String::new(),
        work,
        recommended: Vec::new(),
        suggested: Vec::new(),
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
/// left — except a slot that still owes bookkeeping: while its link
/// directories are on the user's durable PATH, its record is what says
/// which entries to take out again (`PROP-059#path-directories`), so it
/// stays until the paths reconciliation has cleared them.
fn normalize_slots(index: &mut Index) {
    let installations = &index.installations;
    index.slots.retain(|slot, state| {
        installations.values().any(|record| &record.slot == slot) || !state.path_env.is_empty()
    });
    for (slot, state) in &mut index.slots {
        if state.default.as_ref().is_some_and(|key| {
            installations
                .get(key)
                .is_none_or(|record| &record.slot != slot || record.lifecycle.hidden)
        }) {
            state.default = None;
        }
    }
    for record in installations.values() {
        index.slots.entry(record.slot.clone()).or_default();
    }
}
