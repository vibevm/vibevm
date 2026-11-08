//! `vibe app export` and `vibe app import`, the portable set of a home's
//! applications (PROP-059 `#portable-set`). The library decides each row;
//! this adapter resolves packages, runs the ordinary `install -g` operation
//! row after row with its plan, panel and trailer, and reports.
//!
//! Not to be confused with `export.rs`, the system-menu export of
//! `vibe app launchers --export-system`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#portable-set");

use anyhow::{Context, Result, bail};
use vibe_app::{ImportAction, Index, OperationContext, SettingsRoot};
use vibe_wire::generated::application_import::{ApplicationImport, License, Refused, Skipped};
use vibe_wire::generated::apps_manifest::ApplicationMark;

use super::agents::{AgentSetup, report_parked};
use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use super::report::prose;
use crate::cli::{AppExportArgs, AppImportArgs, AppStepArgs};
use crate::output;
use crate::output::language::{Role, Stream, paint};

#[path = "portable/export.rs"]
mod export;
#[path = "portable/full.rs"]
mod full;
#[path = "portable/pending.rs"]
mod pending;
/// Render a portable export through the separately owned output adapter.
pub(super) fn export(
    ctx: &output::Context,
    home: &SettingsRoot,
    args: &AppExportArgs,
) -> Result<()> {
    export::run(ctx, home, args)
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
        self.add_rows(cohort.results());
    }
    fn add_rows(&mut self, rows: &[vibe_wire::generated::shared::ApplicationPendingImportResult]) {
        use vibe_wire::generated::shared::ApplicationPendingImportResultAction as Action;
        for row in rows {
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
    let stdin_count = [
        args.file.as_deref(),
        args.target_input.as_deref(),
        args.secret_input.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|p| *p == std::path::Path::new("-"))
    .count();
    if stdin_count > 1 {
        bail!("only one portable/target/secret input may consume stdin");
    }
    if let Some(id) = &args.resume {
        return resume_import(ctx, id, offline, args.secret_input.as_deref());
    }
    let secrets = if let Some(path) = &args.secret_input {
        vibe_app::parse_import_secrets(&read_secret_input(path)?)?
    } else {
        vibe_app::ImportSecrets::default()
    };
    let target = if let Some(path) = &args.target_input {
        vibe_app::parse_import_target(&read_input(path)?)?
    } else {
        vibe_wire::generated::application::e1::import_input::ImportTargetInput {
            protocol: "vibe-app-import-input/1".into(),
            applications: Default::default(),
        }
    };
    let path = args
        .file
        .as_ref()
        .context("import needs FILE or --resume OPERATION_ID")?;
    let file = path.display().to_string();
    let bytes = read_input(path)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| portable_invalid(&file, &"portable file must be UTF-8"))?;
    let set = match vibe_app::parse_portable_document(&file, text)? {
        vibe_app::PortableDocument::Full(set) => {
            return full::run(ctx, args, offline, &set, &target, &secrets);
        }
        vibe_app::PortableDocument::Legacy(set) => {
            if args.target_input.is_some() || args.secret_input.is_some() {
                bail!(
                    "scoped portable inputs require a full --configuration export; legacy import does not consume them"
                );
            }
            set
        }
    };
    let settings = global::settings_root()?;
    // `--registry` comes first and a package it does not carry falls
    // through to the configured registries — the embedded-registry rule.
    let packages = Packages {
        operation: global::source_operation(Default::default())?,
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
        let observe =
            Observation::new(ctx, true, &display)?.with_source_operation(&packages.operation);
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
    report(
        ctx,
        &settings,
        (set.collection.len(), set.pin.len(), set.launcher.len()),
        args.dry_run,
        &tally,
    )?;
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
    report(ctx, settings, (0, 0, 0), false, &tally)?;
    Ok((!tally.refused.is_empty()).then_some(1))
}

fn report(
    ctx: &output::Context,
    settings: &SettingsRoot,
    arrangement: (usize, usize, usize),
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
            pending: None,
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
    if dry_run && arrangement != (0, 0, 0) {
        lines.push(paint(
            Role::Dim,
            &format!(
                "arrangement: {} collection(s), {} pinned list(s), {} override(s) restored after the installations",
                arrangement.0,
                arrangement.1,
                arrangement.2
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

fn resume_import(
    ctx: &output::Context,
    id: &str,
    offline: bool,
    secret_input: Option<&std::path::Path>,
) -> Result<Option<i32>> {
    use vibe_wire::generated::shared::ApplicationPendingLookup as Lookup;
    let settings = global::settings_root()?;
    let inspected = vibe_app::inspect_import_resume(&settings, id)?;
    if !inspected.full && secret_input.is_some() {
        bail!(
            "legacy cohort does not consume scoped secret files; continue its exact application operation"
        );
    }
    let secrets = if inspected.cleanup_only {
        // Finalization does not reopen an ephemeral answer file that may no longer exist.
        vibe_app::ImportSecrets::default()
    } else if let Some(path) = secret_input {
        vibe_app::parse_import_secrets(&read_secret_input(path)?)?
    } else {
        vibe_app::ImportSecrets::default()
    };
    let agents = AgentSetup::for_import_resume(ctx, &inspected)?;
    let info = &inspected.context;
    let packages = Packages {
        operation: global::source_operation(Default::default())?,
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
    let observe = Observation::new(ctx, true, &display)?.with_source_operation(&packages.operation);
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
        if inspected.full {
            report_parked(ctx, vibe_app::resume_full_import(&operation, id, &secrets))
                .map(Resumed::Full)
        } else {
            report_parked(ctx, vibe_app::resume_import(&operation, id)).map(Resumed::Legacy)
        }
    });
    if let Err(error) = &result {
        let prefix = vibe_app::inspect_import_resume(&settings, id)?.results;
        if let Some(exit) = emit_input_required(ctx, &settings, error, &prefix)? {
            return Ok(Some(exit));
        }
    }
    let cohort = display.report(ctx, result)?;
    match cohort {
        Resumed::Legacy(cohort) => report_retained(ctx, &settings, &cohort),
        Resumed::Full(cohort) => {
            let mut tally = Tally::default();
            tally.add_rows(cohort.results());
            report(ctx, &settings, (0, 0, 0), false, &tally)?;
            Ok((!tally.refused.is_empty()).then_some(1))
        }
    }
}
enum Resumed {
    Legacy(vibe_app::PreparedImport),
    Full(vibe_app::PreparedFullImport),
}

fn read_input(path: &std::path::Path) -> Result<Vec<u8>> {
    if path == std::path::Path::new("-") {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes)?;
        Ok(bytes)
    } else {
        Ok(std::fs::read(path).with_context(|| format!("reading {}", path.display()))?)
    }
}

fn emit_input_required(
    ctx: &output::Context,
    settings: &SettingsRoot,
    error: &vibe_app::AppError,
    prefix: &[vibe_wire::generated::shared::ApplicationPendingImportResult],
) -> Result<Option<i32>> {
    if let vibe_app::AppError::Configuration(
        vibe_app::configuration::ConfigurationError::ImportInputRequired {
            target,
            key,
            parameters,
        },
    ) = error
    {
        return pending::emit_pending(
            ctx,
            settings,
            target,
            &vibe_wire::generated::shared::ApplicationImportWaitingInput {
                key: key.clone(),
                parameters: parameters.clone(),
            },
            prefix,
        );
    }
    Ok(None)
}

/// Secret input locations and I/O error chains are outside public diagnostics.
fn read_secret_input(path: &std::path::Path) -> Result<Vec<u8>> {
    let result = if path == std::path::Path::new("-") {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes).map(|_| bytes)
    } else {
        std::fs::read(path)
    };
    result.map_err(|_| anyhow::anyhow!("cannot read scoped secret input; supply a readable secret document or use standard input"))
}
