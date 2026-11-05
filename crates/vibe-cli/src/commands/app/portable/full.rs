//! Full JSON/TOML input reaches classified private2 preparation and the ordinary runner.
use super::*;
use vibe_app::{FullImportCandidate, FullImportPreparation, ImportSecrets};
use vibe_wire::generated::application::{e1::import_input::ImportTargetInput, e2::apps};
pub(super) fn run(
    ctx: &output::Context,
    args: &AppImportArgs,
    offline: bool,
    set: &apps::AppsManifest,
    target: &ImportTargetInput,
    secrets: &ImportSecrets,
) -> Result<Option<i32>> {
    let settings = global::settings_root()?;
    let packages = Packages {
        settings: settings.clone(),
        registry: None,
        embedded: args.registry.clone(),
        offline,
    };
    let active = vibe_app::selected_import_keys(set)?;
    let mut selected = Vec::new();
    for row in set
        .application
        .iter()
        .filter(|row| active.contains(&row.key))
    {
        let resolved = packages.resolve(&exact(&row.key))?;
        let explicit = target
            .applications
            .get(&row.key)
            .and_then(|t| t.installer.clone());
        let source_offered = resolved.options.iter().any(|o| {
            o.id == row.installer
                && Some(o.platform.as_str()) == vibe_app::detected_platform()
                && o.arch == vibe_app::detected_arch()
        });
        let choice = vibe_app::InstallChoice {
            installer: explicit.or_else(|| source_offered.then(|| row.installer.clone())),
            ..Default::default()
        };
        let option = vibe_app::select_option(
            &resolved.options,
            &choice,
            global::attended(ctx)
                .then_some(&super::super::wizard::chooser as vibe_app::Chooser<'_>),
        )
        .map_err(vibe_app::AppError::from)?;
        selected.push((row, resolved, option));
    }
    let candidates = selected
        .iter()
        .map(|(application, resolved, option)| FullImportCandidate {
            application,
            resolved,
            option,
        })
        .collect::<Vec<_>>();
    let resolver = |name: &str| packages.resolve(name);
    let proxy = global::proxy_image()?;
    let agents = AgentSetup::new(ctx, None, false)?;
    let show = global::plan_printer(ctx);
    let display = StepDisplay::open(ctx, &AppStepArgs::default());
    let observe = Observation::new(ctx, true, &display)?;
    let operation = OperationContext {
        agent: agents.hooks(),
        resolver: Some(&resolver),
        resolver_scope: packages.scope(),
        ..global::operation(ctx, &settings, offline, &proxy, Some(&show), &observe)
    };
    let mut prepared = vibe_app::prepare_full_import(FullImportPreparation {
        document: set,
        candidates: &candidates,
        target,
        secrets,
        context: &operation,
        skip_installed: args.skip_installed,
        accept_license: args.accept_license,
    })?;
    let mut tally = Tally::default();
    tally.skipped.extend(set.application.iter().filter(|row|!active.contains(&row.key)).map(|row|(row.key.clone(),"carried source data; no selected root requires this application, so its configuration was not consumed".into())));
    if args.dry_run {
        for row in vibe_app::preview_full_import(&operation, &prepared, secrets)? {
            match row {
                vibe_app::FullImportPreview::Install { key, installer } => {
                    tally.installed.push((key, installer))
                }
                vibe_app::FullImportPreview::Skip { key, reason } => {
                    tally.skipped.push((key, reason))
                }
                vibe_app::FullImportPreview::Refused { key, error } => tally.refuse(&key, &error),
            }
        }
    } else {
        let result = display.around(ctx, || {
            report_parked(
                ctx,
                vibe_app::import_full(&operation, &mut prepared, secrets),
            )
        });
        if let Err(error) = &result
            && let Some(exit) = emit_input_required(ctx, &settings, error, prepared.results())?
        {
            return Ok(Some(exit));
        }
        display.report(ctx, result)?;
        for row in prepared.results() {
            match row.action {
                vibe_wire::generated::shared::ApplicationPendingImportResultAction::Installed => {
                    tally
                        .installed
                        .push((row.key.clone(), row.option.clone().unwrap_or_default()))
                }
                vibe_wire::generated::shared::ApplicationPendingImportResultAction::Skipped => {
                    tally.skipped.push((row.key.clone(), row.message.clone()))
                }
                vibe_wire::generated::shared::ApplicationPendingImportResultAction::Refused => {
                    tally.refused.push((
                        row.key.clone(),
                        row.code.clone().unwrap_or_default(),
                        row.message.clone(),
                    ))
                }
            }
        }
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
