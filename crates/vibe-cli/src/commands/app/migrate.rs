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
use crate::output::language::{
    Block, Glyph, LabelValue, Line, Meta, Role, hint, paint, short_path, viewport,
};

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
        ctx.hint(&format!("nothing chosen; slot {} is unchanged", plan.slot));
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
    ctx.hint(&format!(
        "{} declares no migration entry, so there is nothing to carry",
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
    let text = migration_text(document, viewport());
    if !text.is_empty() {
        ctx.block(&text);
    }
    let (ok, what) = migration_trailer(document);
    ctx.trailer_verdict_at(ok, &what, &document.work);
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
    ctx.block(&offer_text(
        &plan.from_version,
        &entries,
        &plan.offer_command(false),
    ));
}

/// The version after the last at sign, or an unqualified key as given.
fn version_of(key: &str) -> &str {
    key.rsplit_once('@').map_or(key, |(_, version)| version)
}

/// One glyph block per entry, with measured time only for valid ordered endpoints.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn migration_text(document: &wire::ApplicationMigration, width: usize) -> String {
    let w = document
        .entries
        .iter()
        .map(|entry| entry.id.chars().count())
        .max()
        .unwrap_or(0);
    let blocks = document
        .entries
        .iter()
        .map(|entry| {
            let (glyph, mut meta) = match entry.outcome {
                wire::Outcome::Done => (Glyph::Done, Vec::new()),
                wire::Outcome::ExportFailed => {
                    (Glyph::Failed, vec![Meta::Alert("export failed".into())])
                }
                wire::Outcome::ImportFailed => {
                    (Glyph::Failed, vec![Meta::Alert("import failed".into())])
                }
                wire::Outcome::Pending => (
                    Glyph::Skipped,
                    vec![Meta::note("exported, not yet imported")],
                ),
                wire::Outcome::Skipped => (Glyph::Skipped, vec![Meta::note("skipped")]),
            };
            if let (Some(started), Some(finished)) = (&entry.started, &entry.finished)
                && let (Ok(started), Ok(finished)) = (
                    chrono::DateTime::parse_from_rfc3339(started),
                    chrono::DateTime::parse_from_rfc3339(finished),
                )
                && let Ok(duration) = finished.signed_duration_since(started).to_std()
            {
                meta.insert(0, Meta::Time(duration));
            }
            if let Some(log) = &entry.log {
                meta.push(Meta::note(format!("log {}", short_path(log))));
            }
            let mut body = Vec::new();
            if matches!(
                entry.outcome,
                wire::Outcome::ExportFailed | wire::Outcome::ImportFailed | wire::Outcome::Pending
            ) {
                if let Some(reason) = &entry.reason {
                    body.push(Line::Label(
                        "why".into(),
                        vec![LabelValue::Text(super::report::prose(reason))],
                    ));
                }
                if let Some(retry) = &entry.retry {
                    body.push(Line::Label(
                        "retry".into(),
                        vec![LabelValue::Command(retry.clone())],
                    ));
                }
            }
            Block::new(glyph, format!("{:<w$}  {}", entry.id, entry.name))
                .with_meta(meta)
                .with_body(body)
                .render_at(width)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let hints = document
        .notices
        .iter()
        .map(|notice| format!("  {}", hint(&super::report::prose(notice))))
        .collect::<Vec<_>>()
        .join("\n");
    [blocks, hints]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The document verdict and the selected versions, naming every entry not carried.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
fn migration_trailer(document: &wire::ApplicationMigration) -> (bool, String) {
    let from = version_of(&document.from);
    let to = version_of(&document.to);
    let missing = document
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.outcome,
                wire::Outcome::ExportFailed | wire::Outcome::ImportFailed | wire::Outcome::Pending
            )
        })
        .count();
    let carried = document
        .entries
        .iter()
        .filter(|entry| matches!(entry.outcome, wire::Outcome::Done))
        .count();
    let slot = &document.slot;
    let text = if document.ok {
        format!("migrated {slot} {from} → {to}")
    } else if carried > 0 {
        format!("migrated {slot} {from} → {to} partly · {missing} not carried")
    } else {
        format!("migrated nothing of {slot} {from} → {to} · {missing} not carried")
    };
    (document.ok, text)
}

/// The offer is a dim explanation followed by its exact command on an accent line.
fn offer_text(from_version: &str, names: &[&str], command: &str) -> String {
    format!(
        "  {}\n  {}",
        hint(&format!(
            "state from {from_version} can be carried over ({}); nothing was migrated",
            names.join(", ")
        )),
        paint(Role::Accent, command)
    )
}

#[cfg(test)]
#[path = "migrate/tests.rs"]
mod tests;
