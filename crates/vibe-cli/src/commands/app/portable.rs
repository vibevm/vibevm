//! `vibe app export` and `vibe app import`, the portable set of a home's
//! applications (PROP-059 `#portable-set`). The library decides each row;
//! this adapter resolves packages, runs the ordinary `install -g` operation
//! row after row with its plan, panel and trailer, and reports.
//!
//! Not to be confused with `export.rs`, the system-menu export of
//! `vibe app launchers --export-system`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use anyhow::{Context, Result, bail};
use vibe_app::{ImportAction, Index, OperationContext, PortableExport, SettingsRoot};
use vibe_wire::generated::application_import::{ApplicationImport, License, Refused, Skipped};
use vibe_wire::generated::apps_manifest::{ApplicationMark, AppsManifest};

use super::agents::{AgentSetup, report_parked};
use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use super::report::prose;
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
    /// The licences of the installing rows nobody accepted yet: what a dry
    /// run would ask, what a real run accepted or refused on.
    licenses: Vec<License>,
}

impl Tally {
    fn add_results(&mut self, cohort: &vibe_app::PreparedImport) {
        use vibe_wire::generated::shared::ApplicationPendingImportResultAction as Action;
        for row in cohort.results() {
            match row.action {
                Action::Installed => self
                    .installed
                    .push((row.key.clone(), row.option.clone().unwrap_or_default())),
                Action::Skipped => self.skipped.push((row.key.clone(), row.message.clone())),
                Action::Refused => self.refused.push((
                    row.key.clone(),
                    row.code.clone().unwrap_or_else(|| "import_refused".into()),
                    row.message.clone(),
                )),
            }
        }
    }

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
    if let Some(id) = &args.resume {
        return resume_import(ctx, id, offline);
    }
    let path = args
        .file
        .as_ref()
        .context("import needs FILE or --resume OPERATION_ID")?;
    let file = path.display().to_string();
    let text = std::fs::read_to_string(path)
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
    // Resolve and classify every selected manual row before the first mutation.
    // A later rich/secret row must not be discovered after an earlier install.
    let mut selected = Vec::new();
    for row in set
        .application
        .iter()
        .filter(|r| r.mark == ApplicationMark::Manual)
    {
        let resolved = match packages.resolve(&exact(&row.key)) {
            Ok(r) => r,
            Err(e) => {
                tally.refuse(&row.key, &e);
                continue;
            }
        };
        let choice = vibe_app::import_choice(row, &resolved.options);
        let option = match vibe_app::select_option(
            &resolved.options,
            &choice,
            global::attended(ctx).then_some(&super::wizard::chooser as vibe_app::Chooser<'_>),
        ) {
            Ok(o) => o,
            Err(e) => {
                tally.refuse(&row.key, &e.into());
                continue;
            }
        };
        selected.push((row, resolved, option));
    }
    let candidates: Vec<_> = selected
        .iter()
        .map(|(row, resolved, option)| vibe_app::ImportCandidate {
            row,
            resolved,
            option,
        })
        .collect();
    let mut prepared = vibe_app::prepare_import(
        &set,
        &candidates,
        args.skip_installed,
        args.accept_license,
        args.registry.as_deref(),
        global::attended(ctx),
    )?;
    let index = Index::load(&settings)?;
    for row in prepared.rows() {
        let original = vibe_app::import_row(row);
        let resolved = &selected
            .iter()
            .find(|(r, _, _)| r.key == row.key)
            .ok_or_else(|| {
                anyhow::anyhow!("classified import row `{}` has no selected source", row.key)
            })?
            .1;
        match vibe_app::import_action(&index, &original, &resolved.options, args.skip_installed) {
            ImportAction::Install(install) => {
                if let Some(license) = vibe_app::import_license(
                    &index,
                    &original,
                    &install,
                    resolved,
                    args.accept_license,
                ) {
                    tally.licenses.push(License {
                        key: row.key.clone(),
                        name: license.name,
                        file: license.file,
                    });
                }
                if args.dry_run {
                    tally
                        .installed
                        .push((row.key.clone(), row.installer.clone()));
                }
            }
            ImportAction::Skip(reason) => {
                if args.dry_run {
                    tally.skipped.push((row.key.clone(), reason));
                }
            }
            ImportAction::Refuse(error) => {
                if args.dry_run {
                    tally.refuse(&row.key, &error);
                }
            }
        }
    }
    if args.dry_run {
        tally.skipped.extend(prepared.automatic().iter().map(|key| {
            (
                key.clone(),
                "a requirement; installed only when a manual row requires it".into(),
            )
        }));
    } else {
        for (key, code, message) in tally.refused.drain(..) {
            prepared.note_refusal(key, code, message);
        }
        let resolver = |name: &str| packages.resolve(name);
        let show = global::plan_printer(ctx);
        let display = StepDisplay::open(ctx, &AppStepArgs::default());
        let observe = Observation::new(ctx, true, &display)?;
        display.around(ctx, || {
            let operation = OperationContext {
                agent: agents.hooks(),
                resolver: Some(&resolver),
                resolver_scope: packages.scope(),
                ..global::operation(ctx, &settings, offline, &proxy_image, Some(&show), &observe)
            };
            report_parked(ctx, vibe_app::import_cohort(&operation, &mut prepared))
        })?;
        tally.add_results(&prepared);
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

/// A completed exact import continuation uses the established cohort report.
pub(super) fn report_retained(
    ctx: &output::Context,
    settings: &SettingsRoot,
    cohort: &vibe_app::PreparedImport,
) -> Result<Option<i32>> {
    let mut tally = Tally::default();
    tally.add_results(cohort);
    let set = AppsManifest {
        protocol: vibe_app::APPS_PROTOCOL.into(),
        exported: String::new(),
        home: vibe_wire::generated::apps_manifest::AppsManifestHome::User,
        platform: String::new(),
        arch: String::new(),
        application: vec![],
        collection: vec![],
        pin: vec![],
        launcher: vec![],
    };
    report(ctx, settings, &set, false, &tally)?;
    Ok((!tally.refused.is_empty()).then_some(1))
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
            licenses: tally.licenses.clone(),
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
    if dry_run {
        for license in &tally.licenses {
            lines.push(format!(
                "license  {}  {}",
                license.key,
                paint(Role::Dim, &license.name)
            ));
        }
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

fn resume_import(ctx: &output::Context, id: &str, offline: bool) -> Result<Option<i32>> {
    use vibe_wire::generated::shared::ApplicationPendingLookup as Lookup;
    let settings = global::settings_root()?;
    let info = vibe_app::import_resume_info(&settings, id)?;
    let agents = AgentSetup::for_import(ctx, &info)?;
    let packages = Packages {
        settings: settings.clone(),
        registry: match &info.lookup {
            Lookup::Registry(r) => Some(r.root.clone().into()),
            _ => None,
        },
        embedded: match &info.lookup {
            Lookup::Chain(r) => r.embedded.clone().map(Into::into),
            _ => None,
        },
        offline: offline || info.offline,
    };
    let resolver = |name: &str| match &info.lookup {
        Lookup::Project(p) => {
            super::project::resolve_saved_requirement(std::path::Path::new(&p.root), name)
        }
        Lookup::Caller(_) => Err(vibe_app::ResolveError::Registry {
            spelling: name.into(),
            reason: "saved caller-owned resolver cannot be reconstructed by this CLI".into(),
        }
        .into()),
        _ => packages.resolve(name),
    };
    let proxy = global::proxy_image()?;
    let display = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &display)?;
    let show = global::plan_printer(ctx);
    let result = display.around(ctx, || {
        let operation = OperationContext {
            agent: agents.hooks(),
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..global::operation(
                ctx,
                &settings,
                offline || info.offline,
                &proxy,
                Some(&show),
                &observe,
            )
        };
        report_parked(ctx, vibe_app::resume_import(&operation, id))
    });
    let cohort = display.report(ctx, result)?;
    report_retained(ctx, &settings, &cohort)
}
