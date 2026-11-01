//! `vibe app export` and `vibe app import`, the portable set of a home's
//! applications (PROP-059 `#portable-set`). The library decides each row;
//! this adapter resolves packages, runs the ordinary `install -g` operation
//! row after row with its plan, panel and trailer, and reports.
//!
//! Not to be confused with `export.rs`, the system-menu export of
//! `vibe app launchers --export-system`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_app::{
    FeatureRequest, ImportAction, ImportInstall, Index, InstallRequest, OperationContext,
    PortableExport, ResolvedPackage, SettingsRoot,
};
use vibe_wire::generated::application_import::{ApplicationImport, Refused, Skipped};
use vibe_wire::generated::apps_manifest::{ApplicationMark, AppsManifest};

use super::agents::{AgentSetup, report_parked};
use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use super::report::{prose, render};
use crate::cli::{AppExportArgs, AppImportArgs, AppStepArgs};
use crate::output;
use crate::output::language::{Role, Stream, paint};

/// `vibe app export`: the set as TOML on stdout or in `--output`, or as
/// JSON with `--json`.
pub(super) fn export(
    ctx: &output::Context,
    home: &SettingsRoot,
    args: &AppExportArgs,
) -> Result<()> {
    let request = PortableExport {
        all: args.all,
        arrangement: !args.installations_only,
    };
    let set = vibe_app::portable_set(home, &request)?;
    let text = if ctx.is_json() {
        format!("{}\n", serde_json::to_string_pretty(&set)?)
    } else {
        vibe_app::portable_toml(&set)?
    };
    let Some(path) = &args.output else {
        if ctx.is_json() {
            return ctx.emit_json(&set);
        }
        print!("{text}");
        return Ok(());
    };
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
    if ctx.is_json() {
        return ctx.emit_json(&set);
    }
    ctx.trailer(&format!(
        "exported {} application(s) · {}",
        set.application.len(),
        path.display()
    ));
    Ok(())
}

/// The rows of one import as they end.
#[derive(Default)]
struct Tally {
    /// Installed, or under `--dry-run` to be installed, with the installer
    /// the row will use.
    installed: Vec<(String, String)>,
    skipped: Vec<(String, String)>,
    refused: Vec<(String, String, String)>,
}

impl Tally {
    fn refuse(&mut self, key: &str, error: &vibe_app::AppError) {
        self.refused
            .push((key.to_owned(), error.code().to_owned(), error.to_string()));
    }
}

/// `vibe app import FILE`: manual rows first, each through the ordinary
/// install; `auto` rows come as requirements; the arrangement last.
/// Answers exit status 1 when any row was refused.
pub(super) fn import(
    ctx: &output::Context,
    args: &AppImportArgs,
    explicit_path: bool,
    offline: bool,
) -> Result<Option<i32>> {
    if explicit_path {
        bail!(
            "vibe app import installs into the user home; a project records its applications in \
             vibe.toml (violates spec://org.vibevm.core/vibevm/common/PROP-059#portable-set; fix: \
             drop --path, and add project applications with `vibe install`)"
        );
    }
    let file = args.file.display().to_string();
    let text = std::fs::read_to_string(&args.file)
        .map_err(|error| vibe_app::AppError::from(portable_invalid(&file, &error)))?;
    let set = vibe_app::parse_portable(&file, &text)?;
    let settings = global::settings_root()?;
    // `--registry` comes first and a package it does not carry falls
    // through to the configured registries — the embedded-registry rule.
    let packages = Packages {
        settings: settings.clone(),
        registry: None,
        embedded: args.registry.clone(),
        offline,
    };
    let proxy_image = global::proxy_image()?;
    let agents = AgentSetup::new(ctx, None, false)?;
    let mut tally = Tally::default();
    for row in set
        .application
        .iter()
        .filter(|row| row.mark == ApplicationMark::Manual)
    {
        let resolved = match packages.resolve(&exact(&row.key)) {
            Ok(resolved) => resolved,
            Err(error) => {
                tally.refuse(&row.key, &error);
                continue;
            }
        };
        let index = Index::load(&settings)?;
        match vibe_app::import_action(&index, row, &resolved.options, args.skip_installed) {
            ImportAction::Skip(reason) => tally.skipped.push((row.key.clone(), reason)),
            ImportAction::Refuse(error) => tally.refuse(&row.key, &error),
            ImportAction::Install(install) => {
                let using = install
                    .option
                    .clone()
                    .unwrap_or_else(|| "chosen on this host".into());
                if args.dry_run {
                    tally.installed.push((row.key.clone(), using));
                    continue;
                }
                let target = Target {
                    settings: &settings,
                    packages: &packages,
                    proxy_image: &proxy_image,
                    agents: &agents,
                };
                let outcome = install_row(ctx, &target, &resolved, &install, args, row.hidden)?
                    .and_then(|outcome| {
                        if row.held {
                            vibe_app::hold(&settings, &row.key, true, &proxy_image)?;
                        }
                        Ok(outcome)
                    });
                match outcome {
                    Ok(outcome) => {
                        if !ctx.is_json() {
                            render(ctx, &outcome)?;
                        }
                        tally.installed.push((row.key.clone(), using));
                    }
                    Err(error) => tally.refuse(&row.key, &error),
                }
            }
        }
    }
    let index = Index::load(&settings)?;
    for row in set
        .application
        .iter()
        .filter(|row| row.mark == ApplicationMark::Auto)
    {
        let reason = if args.dry_run {
            "a requirement; installed only when a manual row requires it"
        } else if index.installations.contains_key(&row.key) {
            "a requirement; installed with what requires it"
        } else {
            "a requirement nothing in the set required"
        };
        tally.skipped.push((row.key.clone(), reason.into()));
    }
    if !args.dry_run {
        tally
            .skipped
            .extend(vibe_app::restore_arrangement(&settings, &set)?);
    }
    report(ctx, &settings, &set, args.dry_run, &tally)?;
    Ok((!tally.refused.is_empty()).then_some(1))
}

/// `group/name@version` as the exact requirement `group/name@=version`:
/// the set names a version, not a range to resolve anew.
fn exact(key: &str) -> String {
    match key.rsplit_once('@') {
        Some((package, version)) => format!("{package}@={version}"),
        None => key.to_owned(),
    }
}

fn portable_invalid(file: &str, error: &dyn std::fmt::Display) -> vibe_app::LifecycleError {
    vibe_app::LifecycleError::PortableInvalid {
        file: file.into(),
        reason: error.to_string(),
    }
}

/// What every row of one import installs into and with.
struct Target<'a> {
    settings: &'a SettingsRoot,
    packages: &'a Packages,
    proxy_image: &'a Path,
    agents: &'a AgentSetup,
}

/// One row through the ordinary `install -g` operation: its plan, step
/// panel, licence question and requirements, exactly as `install -g` with
/// the row's `--installer`, `--feature`, `--no-feature` and `--set`. The
/// outer error stops the whole import; the inner one refuses this row.
fn install_row(
    ctx: &output::Context,
    target: &Target<'_>,
    resolved: &ResolvedPackage,
    install: &ImportInstall,
    args: &AppImportArgs,
    hidden: bool,
) -> Result<Result<vibe_app::Outcome, vibe_app::AppError>> {
    let request = InstallRequest {
        choice: install.choice.clone(),
        features: FeatureRequest {
            enable: install.enable.clone(),
            disable: install.disable.clone(),
            profile: None,
        },
        parameters: install.parameters.clone(),
        accept_license: args.accept_license,
        hidden,
        ..InstallRequest::default()
    };
    let packages = target.packages;
    let resolver = |spelling: &str| packages.resolve(spelling);
    let show = global::plan_printer(ctx);
    let display = StepDisplay::open(ctx, &AppStepArgs::default());
    let observe = Observation::new(ctx, true, &display)?;
    Ok(display.around(ctx, || {
        let operation = OperationContext {
            agent: target.agents.hooks(),
            resolver: Some(&resolver),
            ..global::operation(
                ctx,
                target.settings,
                packages.offline,
                target.proxy_image,
                Some(&show),
                &observe,
            )
        };
        report_parked(ctx, vibe_app::install(&operation, resolved, &request))
    }))
}

fn report(
    ctx: &output::Context,
    settings: &SettingsRoot,
    set: &AppsManifest,
    dry_run: bool,
    tally: &Tally,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&ApplicationImport {
            protocol: vibe_app::IMPORT_PROTOCOL.into(),
            home: settings.path().display().to_string(),
            dry_run,
            installed: tally.installed.iter().map(|(key, _)| key.clone()).collect(),
            skipped: tally
                .skipped
                .iter()
                .map(|(key, reason)| Skipped {
                    key: key.clone(),
                    reason: reason.clone(),
                })
                .collect(),
            refused: tally
                .refused
                .iter()
                .map(|(key, code, message)| Refused {
                    key: key.clone(),
                    code: code.clone(),
                    message: message.clone(),
                })
                .collect(),
        });
    }
    let mut lines: Vec<String> = Vec::new();
    if dry_run {
        for (key, using) in &tally.installed {
            lines.push(format!(
                "install  {key}  {}",
                paint(Role::Dim, &format!("installer {using}"))
            ));
        }
    }
    for (key, reason) in &tally.skipped {
        lines.push(format!("skip     {key}  {}", paint(Role::Dim, reason)));
    }
    for (key, code, message) in &tally.refused {
        lines.push(format!(
            "{}   {key}  {}",
            paint(Role::Alert, "refuse"),
            paint(Role::Dim, &format!("{code} · {}", prose(message)))
        ));
    }
    if dry_run && !(set.collection.is_empty() && set.pin.is_empty() && set.launcher.is_empty()) {
        lines.push(paint(
            Role::Dim,
            &format!(
                "arrangement: {} collection(s), {} pinned list(s), {} override(s) restored after the installations",
                set.collection.len(),
                set.pin.len(),
                set.launcher.len()
            ),
        ));
    }
    if !lines.is_empty() {
        ctx.block(&lines.join("\n"));
    }
    let verb = if dry_run {
        "would install"
    } else {
        "installed"
    };
    ctx.trailer_on(
        Stream::Out,
        tally.refused.is_empty(),
        &format!(
            "import {verb} {} · skipped {} · refused {}",
            tally.installed.len(),
            tally.skipped.len(),
            tally.refused.len()
        ),
    );
    Ok(())
}
