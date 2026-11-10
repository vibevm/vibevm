//! `vibe app migrate SLOT FROM TO`: the consent, the run and the report
//! (PROP-075 `#MIGRATION-FAILURE`).
//!
//! The library runs exactly the entries it is handed, so deciding *which*
//! is this adapter's whole job: `--only` and `--all` are explicit consent,
//! an attended terminal offers the declared entries with the `default` ones
//! preselected, `--yes` takes that preselection without asking, and an
//! unattended run that gave none of them refuses before any effect and
//! prints the exact command that would have run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-FAILURE");

use anyhow::Result;
use vibe_app::{Choice, Index, MigrationPlan, MigrationRequest, OperationContext, SettingsRoot};
use vibe_wire::generated::application_migration as wire;

use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use super::wizard::feature_chooser;
use crate::cli::AppMigrateArgs;
use crate::output;
use crate::output::language::{Role, paint};

pub(super) fn run(
    ctx: &output::Context,
    settings: &SettingsRoot,
    args: AppMigrateArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let index = Index::load(settings)?;
    let plan = vibe_app::migration_plan(&index, &args.slot, &args.from, &args.to)?;
    if plan.entries.is_empty() {
        return nothing_declared(ctx, &plan);
    }
    let Some(only) = selection(ctx, &plan, &args)? else {
        // Escape at the offer: no entry was chosen, so nothing ran.
        ctx.summary(&format!(
            "vibe app migrate: nothing chosen; slot `{}` is unchanged",
            plan.slot
        ));
        return Ok(None);
    };
    let source_operation = global::source_operation(Default::default())?;
    let proxy_image = global::proxy_image()?;
    let display = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &display)?.with_source_operation(&source_operation);
    let packages = Packages {
        operation: source_operation.clone(),
        settings: settings.clone(),
        registry: args.registry.clone(),
        embedded: embedded_root.map(std::path::Path::to_path_buf),
        offline,
    };
    let resolver = |spelling: &str| packages.resolve(spelling);
    let document = display.around(ctx, || {
        let operation = OperationContext {
            resolver: Some(&resolver),
            resolver_scope: packages.scope(),
            ..global::operation(ctx, settings, offline, &proxy_image, None, &observe)
        };
        vibe_app::migrate(
            &operation,
            &MigrationRequest {
                slot: &plan.slot,
                from: &plan.from,
                to: &plan.to,
                only: &only,
                resume: args.resume,
            },
        )
    })?;
    report(ctx, &document)?;
    Ok(Some(i32::from(!document.ok)))
}

/// The entries this run carries, or `None` when an attended offer was
/// escaped. A refusal here happens before any effect.
fn selection(
    ctx: &output::Context,
    plan: &MigrationPlan,
    args: &AppMigrateArgs,
) -> Result<Option<Vec<String>>> {
    if args.all {
        return Ok(Some(plan.all()));
    }
    if !args.only.is_empty() {
        return Ok(Some(args.only.clone()));
    }
    if args.yes {
        return Ok(Some(plan.preselected()));
    }
    if !global::attended(ctx) {
        // "Offered, never run without consent": an unattended run that
        // named no entry is told exactly what to run.
        return Err(
            vibe_app::AppError::Step(vibe_app::StepError::MigrationConsent {
                slot: plan.slot.clone(),
                from: plan.from.clone(),
                to: plan.to.clone(),
                command: plan.offer_command(args.resume),
            })
            .into(),
        );
    }
    let items: Vec<Choice> = plan
        .entries
        .iter()
        .map(|entry| Choice {
            name: entry.name.clone(),
            id: Some(entry.id.clone()),
            description: entry.description.clone(),
            detected: entry.default,
            depth: 0,
        })
        .collect();
    let defaults: Vec<bool> = plan.entries.iter().map(|entry| entry.default).collect();
    let prompt = format!(
        "carry which state from {} to {}?",
        plan.from_version, plan.to_version
    );
    let picked = feature_chooser(&prompt, &items, &defaults).map_err(anyhow::Error::msg)?;
    Ok(picked.map(|indexes| {
        indexes
            .into_iter()
            .filter_map(|index| plan.entries.get(index))
            .map(|entry| entry.id.clone())
            .collect()
    }))
}

/// The new version declares nothing to carry, so there is nothing to run
/// and nothing to refuse.
fn nothing_declared(ctx: &output::Context, plan: &MigrationPlan) -> Result<Option<i32>> {
    if ctx.is_json() {
        ctx.emit_json(&wire::ApplicationMigration {
            protocol: vibe_app::MIGRATION_PROTOCOL.to_owned(),
            ok: true,
            slot: plan.slot.clone(),
            from: plan.from.clone(),
            to: plan.to.clone(),
            work: String::new(),
            resumed: false,
            entries: Vec::new(),
            notices: vec![format!(
                "`{}` declares no [[application.migration]] entry",
                plan.to
            )],
        })?;
        return Ok(None);
    }
    ctx.summary(&format!(
        "vibe app migrate: `{}` declares no migration entry, so there is nothing to carry",
        plan.to
    ));
    Ok(None)
}

/// The run's report: the machine document, or one line per entry with what
/// was not carried and how to retry it.
fn report(ctx: &output::Context, document: &wire::ApplicationMigration) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(document);
    }
    for entry in &document.entries {
        let mark = match entry.outcome {
            wire::Outcome::Done => "carried",
            wire::Outcome::ExportFailed => "not carried (export failed)",
            wire::Outcome::ImportFailed => "not carried (import failed)",
            wire::Outcome::Pending => "exported, not yet imported",
            wire::Outcome::Skipped => "skipped",
        };
        ctx.summary(&format!("  {} — {mark}", entry.name));
        if let Some(reason) = &entry.reason
            && !matches!(entry.outcome, wire::Outcome::Skipped)
        {
            ctx.summary(&format!("    {}", super::report::prose(reason)));
        }
        if let Some(log) = &entry.log {
            ctx.summary(&format!("    log {}", paint(Role::Accent, log)));
        }
        if let Some(retry) = &entry.retry {
            ctx.summary(&format!("    retry {}", paint(Role::Accent, retry)));
        }
    }
    if !document.work.is_empty() {
        ctx.summary(&format!(
            "  work directory kept {}",
            paint(Role::Accent, &document.work)
        ));
    }
    ctx.trailer(&format!(
        "vibe app migrate: slot `{}` {} → {}, {}",
        document.slot,
        document.from,
        document.to,
        if document.ok {
            "every selected entry carried"
        } else {
            "some state was not carried"
        }
    ));
    Ok(())
}

/// The offer `install -g`, `update -g` and `app use` make when they give a
/// slot a new version (PROP-075 `#MIGRATION-FAILURE`): one line naming the
/// exact command, attended or not. None of them ever migrates on its own.
pub(crate) fn offer(ctx: &output::Context, settings: &SettingsRoot, slot: &str, key: &str) {
    let Ok(index) = Index::load(settings) else {
        return;
    };
    let Some(plan) = vibe_app::migration_offer(&index, slot, key) else {
        return;
    };
    let entries: Vec<&str> = plan
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    ctx.summary(&format!(
        "  state from {} can be carried over ({}); nothing was migrated — run {}",
        plan.from_version,
        entries.join(", "),
        paint(Role::Accent, &plan.offer_command(false))
    ));
}
