//! `vibe user init` / `vibe user status` — the personal project's two
//! root-only operations (PROP-074 §2.1).
//!
//! The surface is thin: it resolves the one settings authority, hands the
//! admitted absolute root to the `vibe-workspace` seam, maps the domain
//! observation onto the generated `user_project_report` wire type, and
//! renders it in the mode's own terms. A success writes the report once on
//! stdout (`--json`) or one answered line otherwise; a typed refusal becomes
//! the `user_project` member of the ordinary stderr error envelope. A
//! cancelled init uses only the existing cancellation envelope and emits no
//! second terminal document.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT");

use anyhow::Result;
use specmark::spec;
use vibe_core::cancellation::CancellationToken;
use vibe_core::settings::settings_dir;
use vibe_core::user_project::{
    self, Command, Effects, Outcome, Publication, Report, State, UserProjectRefusal,
};
use vibe_workspace::user_project::{self as project, Initialized, Observation};

use crate::cli::UserCommand;
use crate::output;

/// The generated wire report, which owns every spelling below. From
/// `metadata.rustOptions.package` + `x-rust-type` in
/// `schemas/user_project_report.jtd.json` the generator emits this module at
/// `vibe_wire::generated::user_project_report`, and `generated/mod.rs` gains
/// its `pub mod` line — both by the existing codegen, never by hand.
type Wire = vibe_wire::generated::user_project_report::UserProjectReport;

/// The generated protocol enum, whose one variant is the fixed
/// `vibe-user-project/1` (PROP-074 `#USER-PROJECT-ROOT-REPORT`). If codegen
/// spells the variant differently, only this use compiles short.
type WireProtocol = vibe_wire::generated::user_project_report::UserProjectReportProtocol;

/// The generated verb enum.
type WireCommand = vibe_wire::generated::user_project_report::Command;

/// Dispatch one verb with the invocation's cancellation token.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-ROOT-REPORT")]
pub(crate) fn run(
    ctx: &output::Context,
    command: UserCommand,
    cancellation: &CancellationToken,
) -> Result<()> {
    match command {
        UserCommand::Init => init(ctx, cancellation),
        UserCommand::Enable { path } => {
            super::user_override::set_user_enabled(ctx, &path, true, cancellation)
        }
        UserCommand::Disable { path } => {
            super::user_override::set_user_enabled(ctx, &path, false, cancellation)
        }
        // Status has no checkpoint to stop at: the token arrives unused.
        UserCommand::Status => status(ctx),
        UserCommand::Override(args) => super::user_override::run(ctx, args.command, cancellation),
    }
}

/// Select an initialized personal project for ordinary lifecycle/read owners.
/// This never initializes or repairs an absent or unavailable project.
pub(crate) fn initialized_root() -> Result<std::path::PathBuf> {
    let root = admitted(Command::Status).map_err(|error| anyhow::anyhow!("{error}"))?;
    match project::status(&root) {
        Observation::Initialized { .. } => Ok(root),
        Observation::Absent | Observation::Empty => anyhow::bail!(
            "personal specification project is not initialized; run `vibe user init` first"
        ),
        Observation::Unavailable(refusal) => Err(anyhow::anyhow!("{refusal}")),
    }
}

/// A cancelled run carries the typed refusal, so the existing cancellation
/// envelope reports from the operation's own publication evidence.
#[derive(Debug)]
struct CancelledRun(UserProjectRefusal);

impl std::fmt::Display for CancelledRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for CancelledRun {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// A static refusal rendered as the envelope: the report rides the error.
fn refuse(refusal: UserProjectRefusal) -> anyhow::Error {
    let message = refusal.to_string();
    anyhow::Error::new(refusal).context(message)
}

/// Status has `ok = true` only for `absent` and `initialized`; an unavailable
/// root is a refusal, and under `--json` a refusal is the stderr envelope —
/// never a stdout document (`#USER-PROJECT-ROOT-REPORT-TRANSPORT`).
fn status(ctx: &output::Context) -> Result<()> {
    let root = admitted(Command::Status)?;
    let state = match project::status(&root) {
        Observation::Absent | Observation::Empty => State::Absent,
        Observation::Initialized { .. } => State::Initialized,
        Observation::Unavailable(refusal) => return Err(refuse(*refusal)),
    };
    succeed(ctx, observing(&root, state))
}

fn init(ctx: &output::Context, cancellation: &CancellationToken) -> Result<()> {
    let root = admitted(Command::Init)?;
    // Every step below is a bounded filesystem call; the token is consulted
    // by the seam between them.
    let refusal: UserProjectRefusal = match project::init(&root, cancellation) {
        Ok(Initialized::AlreadyInitialized(report)) | Ok(Initialized::Created(report)) => {
            return succeed(ctx, report);
        }
        Err(refusal) => refusal,
    };
    if cancellation.acknowledged() {
        record(refusal.report(), cancellation);
        let message = refusal.to_string();
        return Err(anyhow::Error::new(CancelledRun(refusal)).context(message));
    }
    Err(refuse(refusal))
}

/// Put the operation's publication evidence on the token, so the cancellation
/// envelope is accurate (`#USER-PROJECT-ROOT-REPORT-TRANSPORT`).
fn record(report: &Report, cancellation: &CancellationToken) {
    match report.publication {
        Publication::Committed => cancellation.committed(),
        Publication::RecoveryRequired => cancellation.publishing(),
        Publication::Restored | Publication::NotStarted => {}
    }
}

/// The status observation of an admitted root.
fn observing(root: &std::path::Path, state: State) -> Report {
    Report::success(
        Command::Status,
        Some(root.to_path_buf()),
        state,
        Effects::None,
        Publication::NotStarted,
    )
}

/// Resolve and admit the root, stamping the verb onto the admission refusal.
fn admitted(command: Command) -> Result<std::path::PathBuf> {
    let root = user_project::admit_root(settings_dir())
        .map_err(|refusal| anyhow::Error::new(with_command(refusal, command)))?;
    for component in root.components() {
        if let std::path::Component::Normal(name) = component {
            let safe = name
                .to_str()
                .is_some_and(|name| vibe_safefs::ensure_safe_component(name).is_ok());
            if !safe {
                return Err(anyhow::Error::new(UserProjectRefusal::new(
                    Report::refused(
                        command,
                        None,
                        "the selected settings path contains an unsupported component",
                        "select an absolute settings path whose components satisfy the safe filesystem rules",
                    ),
                )));
            }
        }
    }
    Ok(root)
}

/// Admission refusals are built for the status verb; re-stamp the verb that
/// actually asked.
fn with_command(refusal: UserProjectRefusal, command: Command) -> UserProjectRefusal {
    let mut corrected = refusal.clone();
    corrected.report_mut().command = command;
    corrected
}

/// Render one success: the wire report in JSON, the answered state otherwise.
/// Non-JSON modes print the same `root` the report carries, never a guess.
fn succeed(ctx: &output::Context, report: Report) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&wire(&report));
    }
    // The same encoding the report itself uses, so both modes agree on the
    // spelling of a root.
    let root = user_project::display(report.root.as_deref())
        .unwrap_or_else(|| "(unknown root)".to_owned());
    let sentence = match report.outcome {
        Outcome::Created => format!("initialized the user project at {root}"),
        Outcome::AlreadyInitialized => format!("the user project at {root} is already initialized"),
        Outcome::Observed if report.state == State::Absent => {
            format!("the user project at {root} is absent; run `vibe user init` to create it")
        }
        Outcome::Observed => format!("the user project at {root} is initialized"),
        // A success never reaches here with these; the arm keeps the match
        // exhaustive over the closed vocabulary.
        Outcome::Refused | Outcome::Incomplete => format!("{root}: {}", state_word(report.state)),
    };
    if ctx.is_quiet() {
        ctx.summary(&sentence);
    } else {
        ctx.block(&sentence);
    }
    Ok(())
}

fn state_word(state: State) -> &'static str {
    match state {
        State::Absent => "absent",
        State::Initialized => "initialized",
        State::Unavailable => "unavailable",
    }
}

/// Map the domain observation onto the generated wire report — the one
/// place that knows the wire spelling.
pub(crate) fn wire(report: &Report) -> Wire {
    Wire {
        protocol: WireProtocol::VibeUserProject,
        ok: report.is_success(),
        command: match report.command {
            Command::Init => WireCommand::UserInit,
            Command::Status => WireCommand::UserStatus,
        },
        root: user_project::display(report.root.as_deref()).map(Box::new),
        state: state(report.state),
        outcome: outcome(report.outcome),
        effects: effects(report.effects),
        publication: publication(report.publication),
        reason: report.reason.clone().map(Box::new),
        remedy: report.remedy.clone().map(Box::new),
    }
}

fn state(value: State) -> vibe_wire::generated::user_project_report::State {
    use vibe_wire::generated::user_project_report::State as WireState;
    match value {
        State::Absent => WireState::Absent,
        State::Initialized => WireState::Initialized,
        State::Unavailable => WireState::Unavailable,
    }
}

fn outcome(value: Outcome) -> vibe_wire::generated::user_project_report::Outcome {
    use vibe_wire::generated::user_project_report::Outcome as WireOutcome;
    match value {
        Outcome::Created => WireOutcome::Created,
        Outcome::AlreadyInitialized => WireOutcome::AlreadyInitialized,
        Outcome::Observed => WireOutcome::Observed,
        Outcome::Refused => WireOutcome::Refused,
        Outcome::Incomplete => WireOutcome::Incomplete,
    }
}

fn effects(value: Effects) -> vibe_wire::generated::user_project_report::Effects {
    use vibe_wire::generated::user_project_report::Effects as WireEffects;
    match value {
        Effects::None => WireEffects::None,
        Effects::Changed => WireEffects::Changed,
        Effects::Unknown => WireEffects::Unknown,
    }
}

fn publication(value: Publication) -> vibe_wire::generated::user_project_report::Publication {
    use vibe_wire::generated::user_project_report::Publication as WirePublication;
    match value {
        Publication::NotStarted => WirePublication::NotStarted,
        Publication::Restored => WirePublication::Restored,
        Publication::Committed => WirePublication::Committed,
        Publication::RecoveryRequired => WirePublication::RecoveryRequired,
    }
}
