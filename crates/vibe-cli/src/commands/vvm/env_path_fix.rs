//! Consented PATH repair (PROP-019 §2.6; corporate-env campaign §3.5): put the
//! stable shim directory on the durable PATH only when the user agreed —
//! an explicit flag, a site-profile policy, or a yes to an interactive
//! question whose default is yes on Windows and no elsewhere. The write goes
//! through the platform's [`EnvPersister`], so on Windows it is the direct
//! registry API with `WM_SETTINGCHANGE` and never a child process.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3);

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{EnvPersister, EnvironmentObservation, Persisted, profile::PathEntryStatus};

/// The site-profile `[path] fix` policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PathFixPolicy {
    /// Ask when a terminal is attached; otherwise leave PATH alone.
    Ask,
    /// Fix without asking, attended or not.
    Always,
    /// Never touch the durable PATH.
    Never,
}

/// Everything that decides whether the durable PATH may be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PathFixConsent {
    /// An explicit command-line answer (`--fix-path` is `Some(true)`); it
    /// wins over the policy and is never second-guessed by a question.
    pub(crate) flag: Option<bool>,
    pub(crate) policy: PathFixPolicy,
    /// Whether a question can be asked: not unattended and on a terminal.
    pub(crate) interactive: bool,
    /// What the question defaults to.
    pub(crate) default_answer: bool,
}

impl PathFixConsent {
    /// Consent on this host: the question defaults to yes on Windows, where
    /// fixing PATH is the purpose of the command, and to no elsewhere.
    pub(crate) fn new(flag: Option<bool>, policy: PathFixPolicy, interactive: bool) -> Self {
        Self {
            flag,
            policy,
            interactive,
            default_answer: cfg!(windows),
        }
    }

    /// Consent the caller already obtained on its own terms.
    pub(crate) fn granted() -> Self {
        Self::new(Some(true), PathFixPolicy::Ask, false)
    }
}

/// Who agreed to the change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConsentSource {
    Flag,
    Profile,
    Interactive,
}

/// Why the durable PATH was left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SkipReason {
    /// The explicit flag said no.
    FlagDeclined,
    /// The site profile says `fix = "never"`.
    ProfileNever,
    /// No terminal to ask and no flag or `always` policy.
    NotInteractive,
    /// The interactive answer was no.
    Declined,
}

/// The decision before any question is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathFixDecision {
    Apply(ConsentSource),
    Ask { default: bool },
    Skip(SkipReason),
}

/// Flag first, then the profile policy, then a question when one can be
/// asked; an unattended run without a flag or `always` never writes.
pub(crate) fn decide(consent: &PathFixConsent) -> PathFixDecision {
    match (consent.flag, consent.policy) {
        (Some(true), _) => PathFixDecision::Apply(ConsentSource::Flag),
        (Some(false), _) => PathFixDecision::Skip(SkipReason::FlagDeclined),
        (None, PathFixPolicy::Always) => PathFixDecision::Apply(ConsentSource::Profile),
        (None, PathFixPolicy::Never) => PathFixDecision::Skip(SkipReason::ProfileNever),
        (None, PathFixPolicy::Ask) if consent.interactive => PathFixDecision::Ask {
            default: consent.default_answer,
        },
        (None, PathFixPolicy::Ask) => PathFixDecision::Skip(SkipReason::NotInteractive),
    }
}

/// What happened to the durable PATH.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PathFixOutcome {
    /// The shim directory was already the canonical first entry.
    AlreadyOnPath,
    /// The persister wrote the durable target.
    Applied,
    /// No consent; nothing was written.
    Skipped,
}

/// The structured result for text and JSON reports.
#[derive(Debug, Serialize)]
pub(crate) struct PathFixObservation {
    pub(crate) shim_dir: String,
    pub(crate) outcome: PathFixOutcome,
    pub(crate) was_on_durable_path: bool,
    pub(crate) on_durable_path: bool,
    pub(crate) consent: Option<ConsentSource>,
    pub(crate) skip_reason: Option<SkipReason>,
    /// Whether the write changed the target (an equivalent but differently
    /// spelled entry can make an applied fix a no-op).
    pub(crate) changed: bool,
    /// How to make the change visible now, after an applied fix.
    pub(crate) activation_hint: Option<String>,
    /// How to fix it later, after a skip.
    pub(crate) fix_hint: Option<String>,
    /// The durable target after the run.
    pub(crate) durable_environment: EnvironmentObservation,
}

/// A consented durable-PATH write that the persister could not complete.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(crate) enum PathFixError {
    #[error(
        "putting `{shim_dir}` on the durable PATH at {target} failed: {detail} (violates REQ spec://org.vibevm.core/vibevm/common/PROP-019#path); fix: make {target} writable by this user and re-run, or add `{shim_dir}` to PATH yourself"
    )]
    Apply {
        shim_dir: String,
        target: String,
        detail: String,
    },
}

/// Report whether `shim_dir` is on the durable PATH and, with consent, put
/// it there through `persister`. `ask` poses the yes/no question with its
/// default and is called only for an interactive `ask` policy without a flag.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-019#path",
    r = 3
)]
pub(crate) fn ensure_shim_dir_on_durable_path(
    persister: &dyn EnvPersister,
    shim_dir: &Path,
    consent: &PathFixConsent,
    ask: &mut dyn FnMut(&str, bool) -> bool,
) -> Result<PathFixObservation, PathFixError> {
    let before = persister.inspect_path(shim_dir);
    let target = before
        .target
        .clone()
        .unwrap_or_else(|| "the durable environment".to_string());
    let shown = shim_dir.display().to_string();
    let mut report = PathFixObservation {
        shim_dir: shown.clone(),
        outcome: PathFixOutcome::AlreadyOnPath,
        was_on_durable_path: before.path_entry == PathEntryStatus::Correct,
        on_durable_path: before.path_entry == PathEntryStatus::Correct,
        consent: None,
        skip_reason: None,
        changed: false,
        activation_hint: None,
        fix_hint: None,
        durable_environment: before,
    };
    if report.was_on_durable_path {
        return Ok(report);
    }
    let consent_source = match decide(consent) {
        PathFixDecision::Apply(source) => Ok(source),
        PathFixDecision::Skip(reason) => Err(reason),
        PathFixDecision::Ask { default } => {
            let question =
                format!("Put `{shown}` on your PATH at {target} so new terminals find vibe?");
            if ask(&question, default) {
                Ok(ConsentSource::Interactive)
            } else {
                Err(SkipReason::Declined)
            }
        }
    };
    let source = match consent_source {
        Ok(source) => source,
        Err(reason) => {
            report.outcome = PathFixOutcome::Skipped;
            report.skip_reason = Some(reason);
            report.fix_hint = Some(format!(
                "re-run with --fix-path, or add `{shown}` to PATH at {target} yourself"
            ));
            return Ok(report);
        }
    };
    let persisted = persister
        .ensure_on_path(shim_dir)
        .map_err(|error| PathFixError::Apply {
            shim_dir: shown,
            target,
            detail: format!("{error:#}"),
        })?;
    report.durable_environment = persister.inspect_path(shim_dir);
    report.on_durable_path = report.durable_environment.path_entry == PathEntryStatus::Correct;
    report.outcome = PathFixOutcome::Applied;
    report.consent = Some(source);
    report.changed = persisted == Persisted::Changed;
    report.activation_hint = Some(persister.activation_hint());
    Ok(report)
}

#[cfg(test)]
#[path = "env_path_fix_tests.rs"]
mod tests;
