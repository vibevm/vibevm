//! `vibe app extras`: the verbs of a runtime's own package ecosystem, and
//! the editors of the user's own list (PROP-075 `#SLOT-EXTRAS`,
//! `#ECOSYSTEM-ADAPTER`).
//!
//! Three kinds of verb live here, and the split is the point. `list` and
//! `apply` compute the final list and are the ones the two decline flags
//! reach; `remove` and `self-upgrade` run a verb over an already-decided
//! list; and `add`, `drop` and `import` touch **no** installation at all —
//! they edit `~/.vibe/config.toml` and install nothing, which is why they
//! are the only ones here that write a file.
//!
//! The library runs exactly the verb it is handed, and owns the
//! arithmetic of the final list, so this adapter's job is parsing,
//! resolving the version's package tree and rendering. **Item strings
//! pass through untouched**: they are taken from `argv` and handed to
//! `argv` or to the user's own file, and nothing here splits, quotes,
//! escapes or reorders one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use anyhow::Result;
use vibe_app::{EcosystemRequest, ExtrasApply, ExtrasPolicy, OperationContext, SettingsRoot};
use vibe_core::manifest::EcosystemVerb;
use vibe_wire::generated::application_ecosystem as wire;
use vibe_wire::generated::application_extras as extras_wire;

use super::display::StepDisplay;
use super::global::{self, Observation, Packages};
use crate::cli::{AppExtrasArgs, ExtrasCommand, ExtrasTargetArgs};
use crate::output;
use crate::output::language::{Role, paint};

#[path = "extras/settings.rs"]
mod settings;

pub(super) fn run(
    ctx: &output::Context,
    settings: &SettingsRoot,
    args: AppExtrasArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    match args.command {
        ExtrasCommand::List {
            slot,
            decline,
            target,
        } => list(
            ctx,
            settings,
            &slot,
            policy(ctx, decline.no_extras),
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::Apply {
            slot,
            decline,
            target,
        } => apply(
            ctx,
            settings,
            &slot,
            policy(ctx, decline.no_extras),
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::Add {
            slot,
            items,
            target,
        } => settings::add(
            ctx,
            settings,
            &slot,
            &items,
            &target,
            embedded_root,
            offline,
        ),
        ExtrasCommand::Drop { slot, ids, target } => {
            settings::drop(ctx, settings, &slot, &ids, &target, embedded_root, offline)
        }
        ExtrasCommand::Import { slot, file, target } => {
            settings::import(ctx, settings, &slot, &file, &target, embedded_root, offline)
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

/// The two flags, in one answer (PROP-075 `#SLOT-EXTRAS`):
/// `--no-recommends` is the one global flag of
/// `PROP-077#NO-RECOMMENDS-FLAG` and is read off the invocation, never
/// redefined here; `--no-extras` is the verb's own.
pub(super) fn policy(ctx: &output::Context, no_extras: bool) -> ExtrasPolicy {
    ExtrasPolicy {
        no_recommends: ctx.no_recommends(),
        no_extras,
    }
}

/// `vibe app extras list SLOT`: the final list and the observation.
fn list(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    policy: ExtrasPolicy,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let operation = Operation::new(ctx, settings, target, embedded_root, offline)?;
    let document = operation.around(settings, offline, |cx| {
        vibe_app::extras_listing(cx, slot, None, target.ecosystem.as_deref(), true, policy)
    })?;
    report_listing(ctx, &document)?;
    Ok(None)
}

/// `vibe app extras apply SLOT`: compute the final list and install it
/// into the version the slot's default points at.
///
/// Exit status follows the run, because the user asked for exactly this;
/// the same call made *after* an install reports a warning instead and
/// never changes that install's status — [`after_install`].
fn apply(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    policy: ExtrasPolicy,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let operation = Operation::new(ctx, settings, target, embedded_root, offline)?;
    let document = operation.around(settings, offline, |cx| {
        vibe_app::extras_apply(
            cx,
            &ExtrasApply {
                slot,
                version: None,
                ecosystem: target.ecosystem.as_deref(),
                policy,
            },
        )
    })?;
    report_run(ctx, &document)?;
    Ok(Some(i32::from(!document.ok)))
}

/// What `install -g` / `update -g` run once they have placed a **new**
/// version of a slot and written its record (PROP-075 `#SLOT-EXTRAS`).
///
/// It names the installation key it just placed rather than the slot's
/// default, because the default is the user's to move and an install need
/// not have moved it. **It never fails the install**: a refusal or a
/// failing verb is one warning naming the retry command, the version stays
/// installed, the slot default stays where the user put it, and the exit
/// status is the install's own. An extra is an extra.
pub(crate) struct AfterInstall<'a> {
    pub slot: &'a str,
    pub key: &'a str,
    pub registry: Option<&'a std::path::Path>,
    pub embedded: Option<&'a std::path::Path>,
    pub offline: bool,
    pub no_extras: bool,
}

pub(crate) fn after_install(
    ctx: &output::Context,
    settings: &SettingsRoot,
    request: &AfterInstall<'_>,
) {
    let target = ExtrasTargetArgs {
        ecosystem: None,
        registry: request.registry.map(std::path::Path::to_path_buf),
    };
    let run = || -> Result<wire::ApplicationEcosystem> {
        let operation = Operation::new(ctx, settings, &target, request.embedded, request.offline)?;
        operation.around(settings, request.offline, |cx| {
            vibe_app::extras_apply(
                cx,
                &ExtrasApply {
                    slot: request.slot,
                    version: Some(request.key),
                    ecosystem: None,
                    policy: policy(ctx, request.no_extras),
                },
            )
        })
    };
    match run() {
        Ok(document) => {
            // Nothing ran: the package declares no adapter, or every list
            // was empty. Silence is right — the install said everything.
            if document.adapters.is_empty()
                || document
                    .adapters
                    .iter()
                    .all(|row| row.outcome == wire::Outcome::Skipped)
            {
                return;
            }
            let _ = report_run(ctx, &document);
        }
        Err(error) => {
            // A package with no adapter at all is the ordinary case, not a
            // warning — the great majority of applications — so it is the
            // one refusal this path swallows whole, matched by code and
            // never by its prose.
            if error
                .downcast_ref::<vibe_app::AppError>()
                .is_some_and(|error| error.code() == "no_ecosystem")
            {
                return;
            }
            ctx.summary(&format!(
                "  extras were not applied: {}; the version is installed and unchanged — retry \
                 with {}",
                super::report::prose(&error.to_string()),
                paint(
                    Role::Accent,
                    &format!("vibe app extras apply {}", request.slot)
                )
            ));
        }
    }
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
            // A recommendation the user dropped is shown as dropped and
            // not as unchosen: the two are different acts, and only one of
            // them is reversible with `--feature`.
            let mark = match (recommendation.enabled, recommendation.dropped) {
                (_, true) => "[-]",
                (true, false) => "[x]",
                (false, false) => "[ ]",
            };
            ctx.summary(&format!(
                "    {mark} {} — {} ({})",
                paint(Role::Accent, &recommendation.item),
                recommendation.name,
                recommendation.id
            ));
        }
        if !row.add.is_empty() {
            ctx.summary(&format!("    yours {}", row.add.join(" ")));
        }
        if row.items.is_empty() {
            ctx.summary("    no items would be installed");
        } else {
            ctx.summary(&format!("    list {}", row.items.join(" ")));
        }
        if !row.applied.is_empty() {
            ctx.summary(&format!(
                "    last applied {}{}",
                row.applied.join(" "),
                row.applied_at
                    .as_deref()
                    .map(|at| format!(" at {at}"))
                    .unwrap_or_default()
            ));
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
    // A stale line in the user's own settings is reported once, here, and
    // otherwise ignored (PROP-075 `#SLOT-EXTRAS`).
    for notice in &document.notices {
        ctx.summary(&format!("  note: {}", super::report::prose(notice)));
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
    for notice in &document.notices {
        ctx.summary(&format!("  note: {}", super::report::prose(notice)));
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
