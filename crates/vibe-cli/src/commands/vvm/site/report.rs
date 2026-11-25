//! The `vibe self apply` report (`vibe-self-apply/1`) and its text form
//! (`PROP-019#site-profile`): what changed in which file, the effective
//! installation, where a person puts releases and payloads, the PATH step.
//!
//! The document itself is the registered wire type
//! `vibe_wire::generated::self_apply_report` (`cli-self-apply-report`); this
//! module owns the projection of the command's own observations into it and
//! the human rendering of the result.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use vibe_wire::generated::self_apply_report::{
    ApplyAction, ConsentSource, EnvBlockStatus, EnvEffectiveness, EnvObservationStatus,
    EnvPathEntryStatus, EnvProbeStatus, EnvTargetKind, EnvironmentObservation, LoginShell,
    PathFixOutcome, PathReport, SelfApplyReport, SkipReason,
};

use crate::commands::vvm::env::{self, PathFixObservation};
use crate::output;

#[path = "report/effective.rs"]
pub(super) mod effective;

/// The JSON protocol name of the report.
pub(super) const PROTOCOL: &str = "vibe-self-apply/1";

/// The command name every form of the report carries.
pub(super) const COMMAND: &str = "self:apply";

/// Project a performed PATH step into the report's `path` member: the
/// observation's own members, with the planning ones absent.
pub(super) fn path_done(observation: PathFixObservation) -> PathReport {
    PathReport {
        shim_dir: observation.shim_dir,
        outcome: Some(match observation.outcome {
            env::PathFixOutcome::AlreadyOnPath => PathFixOutcome::AlreadyOnPath,
            env::PathFixOutcome::Applied => PathFixOutcome::Applied,
            env::PathFixOutcome::Skipped => PathFixOutcome::Skipped,
        }),
        was_on_durable_path: Some(observation.was_on_durable_path),
        on_durable_path: Some(observation.on_durable_path),
        planned: None,
        consent: observation.consent.map(consent_source),
        skip_reason: observation.skip_reason.map(skip_reason),
        changed: Some(observation.changed),
        activation_hint: observation.activation_hint,
        fix_hint: observation.fix_hint,
        durable_environment: Some(environment(observation.durable_environment)),
        target: None,
    }
}

/// The wire value of one consent source.
pub(super) fn consent_source(source: env::ConsentSource) -> ConsentSource {
    match source {
        env::ConsentSource::Flag => ConsentSource::Flag,
        env::ConsentSource::Profile => ConsentSource::Profile,
        env::ConsentSource::Interactive => ConsentSource::Interactive,
    }
}

/// The wire value of one skip reason.
pub(super) fn skip_reason(reason: env::SkipReason) -> SkipReason {
    match reason {
        env::SkipReason::FlagDeclined => SkipReason::FlagDeclined,
        env::SkipReason::ProfileNever => SkipReason::ProfileNever,
        env::SkipReason::NotInteractive => SkipReason::NotInteractive,
        env::SkipReason::Declined => SkipReason::Declined,
    }
}

/// The wire value of one performed PATH outcome, spelled for the receipt
/// and for a human.
pub(super) fn outcome_label(outcome: &PathFixOutcome) -> &'static str {
    match outcome {
        PathFixOutcome::AlreadyOnPath => "already_on_path",
        PathFixOutcome::Applied => "applied",
        PathFixOutcome::Skipped => "skipped",
    }
}

/// The wire form of the durable-environment observation the PATH step read.
fn environment(observation: env::EnvironmentObservation) -> EnvironmentObservation {
    EnvironmentObservation {
        target_kind: match observation.target_kind {
            env::TargetKind::RcFile => EnvTargetKind::RcFile,
            env::TargetKind::WindowsRegistry => EnvTargetKind::WindowsRegistry,
            env::TargetKind::Unavailable => EnvTargetKind::Unavailable,
        },
        target: observation.target,
        // `and_then`, not `map`: cmd is not a login shell with an rc file
        // this step could have read, so there is nothing to report for it.
        shell: observation.shell.and_then(|shell| match shell {
            env::Shell::Bash => Some(LoginShell::Bash),
            env::Shell::Zsh => Some(LoginShell::Zsh),
            env::Shell::Fish => Some(LoginShell::Fish),
            env::Shell::Pwsh => Some(LoginShell::Pwsh),
            env::Shell::Posix => Some(LoginShell::Posix),
            env::Shell::Cmd => None,
        }),
        observation: match observation.observation {
            env::ObservationStatus::Present => EnvObservationStatus::Present,
            env::ObservationStatus::Missing => EnvObservationStatus::Missing,
            env::ObservationStatus::Unavailable => EnvObservationStatus::Unavailable,
            env::ObservationStatus::Malformed => EnvObservationStatus::Malformed,
        },
        managed_block: match observation.managed_block {
            env::BlockStatus::Present => EnvBlockStatus::Present,
            env::BlockStatus::Absent => EnvBlockStatus::Absent,
            env::BlockStatus::Malformed => EnvBlockStatus::Malformed,
            env::BlockStatus::NotApplicable => EnvBlockStatus::NotApplicable,
            env::BlockStatus::Unknown => EnvBlockStatus::Unknown,
        },
        path_entry: match observation.path_entry {
            env::PathEntryStatus::Correct => EnvPathEntryStatus::Correct,
            env::PathEntryStatus::Missing => EnvPathEntryStatus::Missing,
            env::PathEntryStatus::Different => EnvPathEntryStatus::Different,
            env::PathEntryStatus::Ambiguous => EnvPathEntryStatus::Ambiguous,
            env::PathEntryStatus::Unknown => EnvPathEntryStatus::Unknown,
        },
        login_probe: match observation.login_probe {
            env::ProbeStatus::NotRun => EnvProbeStatus::NotRun,
            env::ProbeStatus::Ran => EnvProbeStatus::Ran,
            env::ProbeStatus::Failed => EnvProbeStatus::Failed,
            env::ProbeStatus::TimedOut => EnvProbeStatus::TimedOut,
            env::ProbeStatus::Unsupported => EnvProbeStatus::Unsupported,
        },
        login_effectiveness: match observation.login_effectiveness {
            env::Effectiveness::Unknown => EnvEffectiveness::Unknown,
            env::Effectiveness::Effective => EnvEffectiveness::Effective,
            env::Effectiveness::NotEffective => EnvEffectiveness::NotEffective,
        },
        error: observation.error,
    }
}

/// Emit the report: the document itself under `--json`, its text form and
/// the diagnostics otherwise.
pub(super) fn emit(report: &SelfApplyReport, ctx: &output::Context) -> anyhow::Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    for warning in &report.warnings {
        ctx.diagnostic(warning);
    }
    let text = super::report_text::render(
        report,
        output::language::viewport(),
        output::language::rule_width(),
    );
    if !text.is_empty() {
        ctx.block(&text);
    }
    if let Some(what) = super::report_text::trailer_what(report) {
        let receipt = if report.action == ApplyAction::Apply {
            vibe_core::settings::site_receipt_path().map(|path| path.display().to_string())
        } else {
            report
                .profile
                .as_ref()
                .map(|profile| profile.stored.clone())
        };
        if let Some(receipt) = receipt {
            ctx.trailer_at(&what, &receipt);
        } else {
            ctx.trailer(&what);
        }
    } else if ctx.is_quiet() {
        let headline = super::report_text::headline_text(report);
        if !headline.is_empty() {
            ctx.summary(&headline);
        }
    }
    Ok(())
}
