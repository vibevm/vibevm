//! `vibe app modify | outdated | expose | hide | mark | hold | unhold |
//! rollback | autoremove` and the bulk `vibe update -g --all`: argument
//! handling and rendering around the `vibe-app` lifecycle operations
//! (PROP-065 `#modify`, `#outdated`, `#uninstall-tree`,
//! `#update-and-rollback`; PROP-059 `#hidden-installations`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#modify");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#outdated");

use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_app::{FeatureRequest, Index, Mark, ModifyRequest, OperationContext, SettingsRoot};

use super::agents::{AgentSetup, report_parked};
use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use super::report::{render, render_change, render_outdated, render_removal};
use crate::cli::{AppChoiceArgs, UpdateArgs};
use crate::output;

/// The inputs of `vibe app modify`.
pub(crate) struct ModifyArgs<'a> {
    pub package: &'a str,
    pub registry: Option<&'a Path>,
    pub installer: Option<String>,
    pub choice: &'a AppChoiceArgs,
    pub agent: Option<&'a str>,
    pub keep_work: bool,
}

/// `vibe app modify PKG`: the same version, another choice (PROP-065
/// `#modify`); anything not named keeps its recorded value.
pub(crate) fn modify(
    ctx: &output::Context,
    home: &SettingsRoot,
    args: ModifyArgs<'_>,
    embedded_root: Option<&Path>,
    offline: bool,
    source_operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    let user = global::settings_root()?;
    let index = Index::load(home)?;
    let (_, record) = vibe_app::installed(home, &index, args.package)?;
    let package = &record.package;
    let exact = format!("{}/{}@={}", package.group, package.name, package.version);
    let packages = Packages {
        operation: source_operation.clone(),
        settings: user,
        registry: args.registry.map(Path::to_path_buf),
        embedded: embedded_root.map(Path::to_path_buf),
        offline,
    };
    let resolved = packages.resolve(&exact)?;
    let request = ModifyRequest {
        configuration: super::configuration::read(args.choice)?,
        installer: args.installer,
        features: FeatureRequest {
            enable: args.choice.feature.clone(),
            disable: args.choice.no_feature.clone(),
            profile: args.choice.profile.clone(),
        },
        parameters: args.choice.set.clone(),
        accept_license: args.choice.accept_license,
        keep_work: args.keep_work,
    };
    let proxy_image = global::proxy_image()?;
    let agents = AgentSetup::new(ctx, args.agent, false)?;
    let resolver = |spelling: &str| packages.resolve(spelling);
    let show = global::plan_printer(ctx);
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?.with_source_operation(&packages.operation);
    let outcome = flat.around(ctx, || {
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..global::operation(ctx, home, offline, &proxy_image, Some(&show), &observe)
        };
        report_parked(ctx, vibe_app::modify(&operation, &resolved, &request))
    });
    let outcome = flat.report(ctx, outcome)?;
    render(ctx, &outcome)
}

/// `vibe app outdated`: installed versions against the newest versions the
/// registries offer, changing nothing (PROP-065 `#outdated`).
pub(crate) fn outdated(
    ctx: &output::Context,
    home: &SettingsRoot,
    registry: Option<&Path>,
    embedded_root: Option<&Path>,
    offline: bool,
    source_operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    let packages = Packages {
        operation: source_operation.clone(),
        settings: global::settings_root()?,
        registry: registry.map(Path::to_path_buf),
        embedded: embedded_root.map(Path::to_path_buf),
        offline,
    };
    let latest = |package: &vibe_app::PackageCoordinate| {
        packages
            .resolve(&format!("{}/{}", package.group, package.name))
            .map(|resolved| resolved.package.version)
            .map_err(|error| error.to_string())
    };
    let report = ctx.suspend_progress(|| vibe_app::outdated(home, &latest))?;
    render_outdated(ctx, &report)
}

/// The state-changing `vibe app` verbs that run no installer.
pub(crate) enum StateVerb {
    Expose,
    Hide,
    Mark(Mark),
    Hold(bool),
    Rollback,
}

pub(crate) fn change(
    ctx: &output::Context,
    home: &SettingsRoot,
    package: &str,
    verb: StateVerb,
) -> Result<()> {
    let image = global::proxy_image()?;
    let user_home = super::facts::user_home();
    let environment = super::environment::for_host(home.path(), user_home.as_deref());
    let exposure = vibe_app::SlotExposure {
        proxy_image: &image,
        environment: environment.as_deref(),
    };
    let change = match verb {
        StateVerb::Expose => vibe_app::expose(home, package, exposure)?,
        StateVerb::Hide => vibe_app::hide(home, package, exposure)?,
        StateVerb::Mark(mark) => vibe_app::mark(home, package, mark, exposure)?,
        StateVerb::Hold(held) => vibe_app::hold(home, package, held, exposure)?,
        StateVerb::Rollback => vibe_app::rollback(home, package, exposure)?,
    };
    render_change(ctx, &change)
}

/// `vibe app autoremove`: orphans and kept previous installations.
pub(crate) fn autoremove(
    ctx: &output::Context,
    home: &SettingsRoot,
    plan_only: bool,
    offline: bool,
    source_operation: &vibe_package_source::SourceOperation,
) -> Result<()> {
    let proxy_image = global::proxy_image()?;
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?.with_source_operation(source_operation);
    let removal = flat.around(ctx, || {
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..global::operation(ctx, home, offline, &proxy_image, None, &observe)
        };
        vibe_app::autoremove(&operation, plan_only)
    })?;
    render_removal(ctx, &removal)
}

/// `vibe update -g --all`: every current installation whose registries
/// offer a newer version, except held ones (PROP-065
/// `#update-and-rollback`), one ordinary update each.
pub(crate) fn update_all(
    ctx: &output::Context,
    mut args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    super::configuration::preload(&mut args.steps.choice)?;
    let settings = global::settings_root()?;
    let source_operation = global::source_operation(args.source_policy.declaration())?;
    let index = Index::load(&settings)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut skipped = Vec::new();
    let mut updated = 0;
    let mut candidates = Vec::new();
    for record in index.installations.values() {
        let package = &record.package;
        if record.lifecycle.superseded_by.is_some()
            || !seen.insert((package.group.clone(), package.name.clone()))
        {
            continue;
        }
        let spelling = format!("{}/{}", package.group, package.name);
        if index
            .installations
            .values()
            .any(|other| other.package.same_package(package) && other.lifecycle.held)
        {
            skipped.push(spelling);
            continue;
        }
        let resolved = global::resolve_package_with_operation(
            &settings,
            args.registry.as_deref(),
            embedded_root.as_deref(),
            &spelling,
            offline,
            &source_operation,
        )?;
        let installed_newest = index
            .installations
            .values()
            .filter(|other| other.package.same_package(package))
            .filter_map(|other| semver::Version::parse(&other.package.version).ok())
            .max();
        let offered = semver::Version::parse(&resolved.package.version).ok();
        if offered <= installed_newest {
            continue;
        }
        candidates.push(resolved);
    }
    let choice = vibe_app::InstallChoice {
        platform: args.platform.clone(),
        arch: args.arch.clone(),
        installer: args.installer.clone(),
    };
    let selected = super::configuration::select_batch(
        ctx,
        candidates,
        &choice,
        &args.steps.choice.set,
        args.steps.choice.parsed_configuration.as_ref(),
    )?;
    for (resolved, choice) in selected {
        args.platform = choice.platform;
        args.arch = choice.arch;
        args.installer = choice.installer;
        let outcome = global::update_one(
            ctx,
            &args,
            embedded_root.clone(),
            offline,
            &resolved,
            &source_operation,
        )?;
        render(ctx, &outcome)?;
        updated += 1;
    }
    if !ctx.is_json() {
        ctx.summary(&format!(
            "vibe update -g --all: {updated} application(s) updated{}",
            if skipped.is_empty() {
                String::new()
            } else {
                format!("; held and skipped: {}", skipped.join(", "))
            }
        ));
    }
    Ok(())
}
