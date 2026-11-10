//! `vibe app extras list|remove|self-upgrade`: the explicit verbs of a
//! runtime's own package ecosystem (PROP-075 `#SLOT-EXTRAS`,
//! `#ECOSYSTEM-ADAPTER`).
//!
//! The library runs exactly the verb it is handed with exactly the items
//! it is given, so this adapter's whole job is parsing, resolving the
//! version's package tree and rendering. **Item strings pass through
//! untouched**: they are taken from `argv` and handed to `argv`, and
//! nothing here splits, quotes, escapes or reorders one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use anyhow::Result;
use vibe_app::{EcosystemRequest, OperationContext, SettingsRoot};
use vibe_core::manifest::EcosystemVerb;
use vibe_wire::generated::application_ecosystem as wire;
use vibe_wire::generated::application_extras as extras_wire;

use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use crate::cli::{AppExtrasArgs, ExtrasCommand, ExtrasTargetArgs};
use crate::output;
use crate::output::language::{Role, paint};

pub(super) fn run(
    ctx: &output::Context,
    settings: &SettingsRoot,
    args: AppExtrasArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    match args.command {
        ExtrasCommand::List { slot, target } => {
            list(ctx, settings, &slot, &target, embedded_root, offline)
        }
        ExtrasCommand::Remove {
            slot,
            items,
            target,
        } => verb(
            ctx,
            settings,
            &slot,
            EcosystemVerb::Remove,
            &items,
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::SelfUpgrade { slot, target } => verb(
            ctx,
            settings,
            &slot,
            EcosystemVerb::SelfUpgrade,
            &[],
            &target,
            embedded_root,
            offline,
        ),
    }
}

/// Everything the library needs that is the same for every verb: the
/// resolver over the configured (or named) registries, the step display
/// and the observation.
struct Operation<'a> {
    packages: Packages,
    source_operation: vibe_package_source::SourceOperation,
    proxy_image: std::path::PathBuf,
    display: StepDisplay,
    ctx: &'a output::Context,
}

impl<'a> Operation<'a> {
    fn new(
        ctx: &'a output::Context,
        settings: &SettingsRoot,
        target: &ExtrasTargetArgs,
        embedded_root: Option<&std::path::Path>,
        offline: bool,
    ) -> Result<Self> {
        let source_operation = global::source_operation(Default::default())?;
        Ok(Self {
            packages: Packages {
                operation: source_operation.clone(),
                settings: settings.clone(),
                registry: target.registry.clone(),
                embedded: embedded_root.map(std::path::Path::to_path_buf),
                offline,
            },
            source_operation,
            proxy_image: global::proxy_image()?,
            display: StepDisplay::flat(),
            ctx,
        })
    }

    /// Run `body` with a resolving operation context, inside the step
    /// display an adapter verb's child streams into.
    fn around<T>(
        &self,
        settings: &SettingsRoot,
        offline: bool,
        body: impl FnOnce(&OperationContext<'_>) -> Result<T, vibe_app::AppError>,
    ) -> Result<T> {
        let observe = Observation::new(self.ctx, true, &self.display)?
            .with_source_operation(&self.source_operation);
        let resolver = |spelling: &str| self.packages.resolve(spelling);
        let document = self.display.around(self.ctx, || {
            let operation = OperationContext {
                resolver: Some(&resolver),
                resolver_scope: self.packages.scope(),
                ..global::operation(
                    self.ctx,
                    settings,
                    offline,
                    &self.proxy_image,
                    None,
                    &observe,
                )
            };
            body(&operation)
        })?;
        Ok(document)
    }
}

/// `vibe app extras list SLOT`: the final list and the observation.
fn list(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let operation = Operation::new(ctx, settings, target, embedded_root, offline)?;
    let document = operation.around(settings, offline, |cx| {
        vibe_app::extras_listing(cx, slot, None, target.ecosystem.as_deref(), true)
    })?;
    report_listing(ctx, &document)?;
    Ok(None)
}

/// `vibe app extras remove|self-upgrade SLOT`: one verb of one or every
/// declared adapter.
#[allow(clippy::too_many_arguments)]
fn verb(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    which: EcosystemVerb,
    items: &[String],
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let operation = Operation::new(ctx, settings, target, embedded_root, offline)?;
    let document = operation.around(settings, offline, |cx| {
        vibe_app::ecosystem_run(
            cx,
            &EcosystemRequest {
                slot,
                version: None,
                ecosystem: target.ecosystem.as_deref(),
                verb: which,
                items,
            },
        )
    })?;
    report_run(ctx, &document)?;
    Ok(Some(i32::from(!document.ok)))
}

/// The listing's report: the machine document, or one block per adapter.
fn report_listing(ctx: &output::Context, document: &extras_wire::ApplicationExtras) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(document);
    }
    for row in &document.ecosystems {
        ctx.summary(&format!("  {} ({})", row.name, row.id));
        for recommendation in &row.recommendations {
            ctx.summary(&format!(
                "    {} {} — {}",
                if recommendation.enabled { "[x]" } else { "[ ]" },
                paint(Role::Accent, &recommendation.item),
                recommendation.name
            ));
        }
        if row.items.is_empty() {
            ctx.summary("    no items would be installed");
        } else {
            ctx.summary(&format!("    list {}", row.items.join(" ")));
        }
        match (&row.observed_failed, row.observed.is_empty()) {
            (Some(reason), _) => {
                ctx.summary(&format!(
                    "    not observed: {}",
                    super::report::prose(reason)
                ));
            }
            (None, true) => ctx.summary("    installed none"),
            (None, false) => {
                ctx.summary(&format!("    installed {}", row.observed.join(" ")));
            }
        }
    }
    ctx.trailer(&format!(
        "vibe app extras list: slot `{}` at {}, {} ecosystem(s)",
        document.slot,
        document.version,
        document.ecosystems.len()
    ));
    Ok(())
}

/// A run's report: the machine document, or one line per adapter with what
/// failed and how to retry exactly that. **Nothing is ever rolled back.**
fn report_run(ctx: &output::Context, document: &wire::ApplicationEcosystem) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(document);
    }
    for row in &document.adapters {
        let mark = match row.outcome {
            wire::Outcome::Done => "done",
            wire::Outcome::Failed => "failed",
            wire::Outcome::Skipped => "skipped",
        };
        ctx.summary(&format!("  {} — {mark}", row.name));
        if let Some(reason) = &row.reason
            && row.outcome != wire::Outcome::Skipped
        {
            ctx.summary(&format!("    {}", super::report::prose(reason)));
        }
        if !row.observed.is_empty() {
            ctx.summary(&format!("    installed {}", row.observed.join(" ")));
        }
        if let Some(log) = &row.log {
            ctx.summary(&format!("    log {}", paint(Role::Accent, log)));
        }
        if let Some(retry) = &row.retry {
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
        "vibe app extras: slot `{}` {}",
        document.slot,
        if document.ok {
            "done"
        } else {
            "some ecosystem work did not finish; the version is installed and unchanged"
        }
    ));
    Ok(())
}
