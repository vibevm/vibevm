//! The closed sum of registered report roots a command boundary may return,
//! and the typed carrier an inner layer uses to hand one outward.
//!
//! ## Why a sum, and why exactly these four
//!
//! The four registered report families — `cli-install-report`,
//! `cli-lifecycle-report`, `cli-update-report`, `cli-reinstall-report` — are
//! the whole set a command boundary may return, and the sum is closed so that
//! adding a fifth is a compile error at every site that decides one.
//!
//! A command's failure root is not a property of the command. `vibe install`
//! reports a slot failure in a `cli-install-report`, but reports a failure of
//! its own post-durability lifecycle callback in a `cli-lifecycle-report` and
//! emits NO install root at all — that is characterised behaviour, and a
//! hosting agent parses it. A phase verb's prerequisite install can likewise
//! fail with an install-shaped root inside a lifecycle command, and a WHOLE
//! `vibe update` returns the install substrate's own slot-failure root rather
//! than an update one.
//!
//! The families are not interchangeable in what they can CARRY, either: the
//! install and lifecycle roots declare a `notices` list and the update and
//! reinstall roots do not. That difference is answered by
//! [`RegisteredReportDraft::absorb_notices`], not by inventing a member.
//!
//! So the root family is decided at the site that first MEASURES the failure —
//! where the rows and progress are still in hand — and travels outward as a
//! value. Nothing downstream reclassifies it, because nothing downstream can:
//! by then all that is left is an `anyhow::Error`, and inferring a report
//! format from an error's Display text is how these two families drifted apart
//! in the first place.
//!
//! ## Why the carrier owns the original error
//!
//! The outer boundary must return the caller's error UNCHANGED — same downcast
//! identity for the exit code, same context chain for stderr. A carrier that
//! stored a formatted string could not do that, so it stores the object, and
//! the boundary that downcasts it takes the object straight back out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt;

use anyhow::Result;
use vibe_wire::generated::shared::CompileTraceReport;

use crate::commands::install::InstallDraft;
use crate::commands::lifecycle::LifecycleDraft;
use crate::commands::reinstall::ReinstallDraft;
use crate::commands::update::UpdateDraft;
use crate::output;

use super::CommandExit;

/// One command's owned, fully-measured report — in whichever registered root
/// family that command's outcome really belongs to.
#[derive(Debug)]
pub(crate) enum RegisteredReportDraft {
    Install(Box<InstallDraft>),
    Lifecycle(Box<LifecycleDraft>),
    Update(Box<UpdateDraft>),
    Reinstall(Box<ReinstallDraft>),
}

impl RegisteredReportDraft {
    /// Whether this outcome is a hosted handoff.
    ///
    /// Read from the typed delegation member, never from a status string: the
    /// deferred-plan rule (park discards its preview, everything else flushes)
    /// hangs off this answer.
    pub(crate) fn parked(&self) -> bool {
        match self {
            Self::Install(draft) => draft.delegation.is_some(),
            Self::Lifecycle(draft) => draft.delegation.is_some(),
            Self::Update(draft) => draft.delegation.is_some(),
            Self::Reinstall(draft) => draft.delegation.is_some(),
        }
    }

    /// Fold owner notices into the root's own `notices` member, and hand back
    /// whatever this family CANNOT carry.
    ///
    /// Called only when there is no trace member to carry them — see the
    /// adapter. It is deliberately not a total operation: `cli-install-report`
    /// and `cli-lifecycle-report` declare a `notices` list, and
    /// `cli-update-report` / `cli-reinstall-report` do not. Inventing one for
    /// them would be a new wire field on a registered format; silently dropping
    /// the notice would delete the only account of a predecessor left running.
    /// So the capability is answered here, by type, and the leftovers travel
    /// back to the adapter, which has a channel every mode can show.
    #[must_use = "a notice this root cannot carry must still be routed somewhere — see the adapter"]
    pub(crate) fn absorb_notices(&mut self, notices: Vec<String>) -> Vec<String> {
        match self {
            Self::Install(draft) => {
                draft.notices.extend(notices);
                Vec::new()
            }
            Self::Lifecycle(draft) => {
                draft.notices.extend(notices);
                Vec::new()
            }
            Self::Update(_) | Self::Reinstall(_) => notices,
        }
    }

    /// Build the generated root with its `trace` member attached, and emit it.
    ///
    /// `quiet_suffix` is appended to the command's ONE summary line, and only
    /// there: a failed root narrates nothing in human mode (it never has), so
    /// its suffix travels on the error line instead.
    pub(crate) fn render(
        self,
        ctx: &output::Context,
        trace: Option<CompileTraceReport>,
        quiet_suffix: &str,
    ) -> Result<()> {
        match self {
            Self::Install(draft) => draft.render(ctx, trace, quiet_suffix),
            Self::Lifecycle(draft) => draft.render(ctx, trace, quiet_suffix),
            Self::Update(draft) => draft.render(ctx, trace, quiet_suffix),
            Self::Reinstall(draft) => draft.render(ctx, trace, quiet_suffix),
        }
    }
}

/// A measured failure travelling outward: the root its site chose, the exact
/// error object to return, and the emission policy that site already had.
#[derive(Debug)]
pub(crate) struct FailedDraft {
    pub(crate) draft: RegisteredReportDraft,
    pub(crate) original: anyhow::Error,
    /// Whether this failure emitted its root when tracing was OFF. A property
    /// of the site — `vibe install` narrates its slot failure, the same
    /// failure under a phase verb's suppressed child context does not — and
    /// never something inferred later.
    pub(crate) emit_when_trace_disabled: bool,
}

impl fmt::Display for FailedDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.original, formatter)
    }
}

impl std::error::Error for FailedDraft {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.original.source()
    }
}

/// Hand a measured failure outward through the ordinary `Result` channel.
///
/// The `anyhow` wrapper is TRANSPORT only: every boundary that can receive one
/// calls [`classify`], which takes the owned value straight back out. Nothing
/// formats it, and it never reaches `main`.
pub(crate) fn carry(
    draft: RegisteredReportDraft,
    original: anyhow::Error,
    emit_when_trace_disabled: bool,
) -> anyhow::Error {
    anyhow::Error::new(FailedDraft {
        draft,
        original,
        emit_when_trace_disabled,
    })
}

/// Attach measured rows to an error that is not carrying any yet.
///
/// The site that runs contributions accumulates rows as it goes, and only ONE
/// of the ways it can fail (a handler that reported a failed transition) knows
/// to freeze them. Every other failure after that point — a state write, a
/// park reconciliation, a checkpoint — would otherwise return an error with
/// the rows still sitting in a local, and the report would claim the run did
/// nothing when it had already done several things successfully.
///
/// Idempotent: an error that already carries a draft keeps the one its own
/// site froze, because that draft is more specific than this one.
pub(crate) fn carry_measured(
    error: anyhow::Error,
    draft: impl FnOnce() -> RegisteredReportDraft,
) -> anyhow::Error {
    if error.downcast_ref::<FailedDraft>().is_some() {
        return error;
    }
    // Historically silent: these failures never emitted a root with tracing
    // off, and adding one now would be a new document on an old path.
    carry(draft(), error, false)
}

/// Prepend rows measured BEFORE the failing site to a carried Lifecycle draft.
///
/// A site that fails freezes what IT measured. It cannot know about work an
/// outer stage already did — a chained clean's contributions, a prerequisite
/// install's slot rows — because those never passed through it. So the outer
/// stage hands them down here, once, and the draft keeps everything else it
/// owns: the same error object, the same root family, the same emission
/// policy.
///
/// An uncarried error is returned untouched. Its caller's fallback builds from
/// the same accumulator, so prepending here would duplicate every row.
pub(crate) fn prepend_lifecycle_rows(
    error: anyhow::Error,
    prefix: Vec<vibe_wire::generated::lifecycle_report::LifecycleContributionReport>,
) -> anyhow::Error {
    if prefix.is_empty() {
        return error;
    }
    match error.downcast::<FailedDraft>() {
        Ok(FailedDraft {
            draft: RegisteredReportDraft::Lifecycle(mut draft),
            original,
            emit_when_trace_disabled,
        }) => {
            let mut rows = prefix;
            rows.append(&mut draft.contributions);
            draft.contributions = rows;
            carry(
                RegisteredReportDraft::Lifecycle(draft),
                original,
                emit_when_trace_disabled,
            )
        }
        // An Install-shaped carrier belongs to a different report family and
        // never carries lifecycle rows; re-carry it exactly as it was.
        Ok(carried) => carry(
            carried.draft,
            carried.original,
            carried.emit_when_trace_disabled,
        ),
        Err(error) => error,
    }
}

/// Whether this error already carries a measured draft.
///
/// Asked, rather than discovered by a failed `downcast`, so a classifier can
/// branch without consuming the error it may need to pass on untouched.
pub(crate) fn is_carried(error: &anyhow::Error) -> bool {
    error.is::<FailedDraft>()
}

/// Take a carried failure apart WITHOUT deciding anything about it.
///
/// Test-only. Production has exactly one consumer of a carrier — [`classify`],
/// at a command boundary that owns a trace session — and a second one that
/// merely unwrapped it would be a second place deciding a root family. It is
/// kept because the sites that BUILD carriers are worth proving directly.
#[cfg(test)]
pub(crate) fn uncarry(error: anyhow::Error) -> Result<FailedDraft, anyhow::Error> {
    error.downcast::<FailedDraft>()
}

/// Turn any error into the one typed failure exit.
///
/// A carried draft is unwrapped to exactly what its site measured. Anything
/// else is a generic stage failure: it gets the draft `fallback` builds for
/// the stage it happened in, and the historical emission policy for such
/// failures, which is silence.
pub(crate) fn classify(
    error: anyhow::Error,
    fallback: impl FnOnce() -> RegisteredReportDraft,
) -> CommandExit<RegisteredReportDraft> {
    match error.downcast::<FailedDraft>() {
        Ok(FailedDraft {
            draft,
            original,
            emit_when_trace_disabled,
        }) => CommandExit::Failed {
            report: draft,
            original_error: original,
            emit_when_trace_disabled,
        },
        Err(original) => CommandExit::Failed {
            report: fallback(),
            original_error: original,
            emit_when_trace_disabled: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("the handler refused")]
    struct Sentinel;

    fn lifecycle_draft() -> RegisteredReportDraft {
        RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
            "build",
            vec!["validate".into(), "install".into(), "build".into()],
            "build",
            Vec::new(),
        )))
    }

    fn row(
        key: &str,
        status: &str,
    ) -> vibe_wire::generated::lifecycle_report::LifecycleContributionReport {
        vibe_wire::generated::lifecycle_report::LifecycleContributionReport {
            flagged: None,
            handler: "script".into(),
            key: key.into(),
            message: None,
            stderr: None,
            stderr_truncated: None,
            stdout: None,
            stdout_truncated: None,
            phase: "build".into(),
            point: "phase:build".into(),
            provider: "org.demo/x".into(),
            reference: None,
            slot_target: None,
            status: status.into(),
            tier: "dependency".into(),
            version: None,
        }
    }

    fn install_draft() -> RegisteredReportDraft {
        RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
            std::path::Path::new("/p"),
            vibe_install::InstallProgress::default(),
            Vec::new(),
        )))
    }

    /// The property the carrier exists for: the object comes back out.
    #[test]
    fn a_carried_failure_returns_its_exact_error_and_policy() {
        let original = anyhow::Error::new(Sentinel).context("phase `build` stopped");
        let rendered = format!("{original:#}");
        let exit = classify(carry(lifecycle_draft(), original, true), install_draft);
        match exit {
            CommandExit::Failed {
                report,
                original_error,
                emit_when_trace_disabled,
            } => {
                assert!(matches!(report, RegisteredReportDraft::Lifecycle(_)));
                assert!(emit_when_trace_disabled, "the site's own policy survives");
                assert_eq!(format!("{original_error:#}"), rendered);
                assert!(original_error.downcast_ref::<Sentinel>().is_some());
            }
            _ => panic!("a carried failure is a failure"),
        }
    }

    /// An uncarried error takes the stage's fallback draft — and silence.
    #[test]
    fn an_uncarried_error_uses_the_stage_fallback_and_stays_silent() {
        let exit = classify(anyhow::anyhow!("planning blew up"), install_draft);
        match exit {
            CommandExit::Failed {
                report,
                emit_when_trace_disabled,
                original_error,
            } => {
                assert!(matches!(report, RegisteredReportDraft::Install(_)));
                assert!(
                    !emit_when_trace_disabled,
                    "historically silent stages stay silent with trace off"
                );
                assert_eq!(original_error.to_string(), "planning blew up");
            }
            _ => panic!("a failure is a failure"),
        }
    }

    /// A GENERIC post-row failure — a state write, a checkpoint, a
    /// reconciliation — keeps every row measured before it.
    ///
    /// This is the branch the failed-transition carrier never covers: those
    /// errors know how to freeze their own rows, and every other one used to
    /// return with the accumulator still sitting in a local. The report then
    /// claimed the run had done nothing when it had already done several
    /// things successfully.
    #[test]
    fn a_generic_post_row_error_still_carries_the_rows_measured_before_it() {
        let measured = vec![row("@vibe/a", "ok"), row("@vibe/b", "cancelled")];
        let carried = carry_measured(
            anyhow::Error::new(Sentinel).context("writing the execution checkpoint"),
            || {
                RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                    "build",
                    vec!["build".into()],
                    "build",
                    measured.clone(),
                )))
            },
        );
        match classify(carried, install_draft) {
            CommandExit::Failed {
                report,
                original_error,
                emit_when_trace_disabled,
            } => {
                let RegisteredReportDraft::Lifecycle(draft) = report else {
                    panic!("this command's own family");
                };
                assert_eq!(
                    draft.contributions.len(),
                    2,
                    "both earlier rows survived the later failure",
                );
                assert_eq!(draft.contributions[1].status, "cancelled");
                assert!(
                    !emit_when_trace_disabled,
                    "a generic stage failure was historically silent",
                );
                assert!(original_error.downcast_ref::<Sentinel>().is_some());
                assert_eq!(
                    format!("{original_error:#}"),
                    "writing the execution checkpoint: the handler refused",
                    "context is neither stripped nor re-added",
                );
            }
            _ => panic!("a failure is a failure"),
        }
    }

    /// A site that already froze its own, more specific rows keeps them.
    #[test]
    fn carrying_rows_never_overwrites_a_draft_its_own_site_measured() {
        let specific = carry(
            RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                "build",
                Vec::new(),
                "build",
                vec![row("@vibe/exact", "fail")],
            ))),
            anyhow::Error::new(Sentinel),
            true,
        );
        let carried = carry_measured(specific, || {
            RegisteredReportDraft::Lifecycle(Box::new(LifecycleDraft::failed(
                "build",
                Vec::new(),
                "build",
                Vec::new(),
            )))
        });
        match classify(carried, install_draft) {
            CommandExit::Failed {
                report,
                emit_when_trace_disabled,
                ..
            } => {
                let RegisteredReportDraft::Lifecycle(draft) = report else {
                    panic!("family");
                };
                assert_eq!(draft.contributions.len(), 1);
                assert_eq!(draft.contributions[0].key, "@vibe/exact");
                assert!(
                    emit_when_trace_disabled,
                    "and its own emission policy, not the generic one",
                );
            }
            _ => panic!("a failure is a failure"),
        }
    }

    /// Park is read from the typed member, not from a status word.
    #[test]
    fn parked_is_the_typed_delegation_member() {
        assert!(!lifecycle_draft().parked());
        let mut parked = LifecycleDraft::failed("build", Vec::new(), "build", Vec::new());
        parked.delegation = Some(
            vibe_wire::generated::lifecycle_report::LifecycleDelegation {
                resume: "vibe build".into(),
                run_id: "0".repeat(32),
                tasks: vec![".vibe/outbox/x/a.md".into()],
            },
        );
        assert!(RegisteredReportDraft::Lifecycle(Box::new(parked)).parked());
    }

    #[test]
    fn notices_fold_into_whichever_root_is_carried() {
        let mut draft = lifecycle_draft();
        let unabsorbed = draft.absorb_notices(vec!["displaced run left running".into()]);
        assert!(unabsorbed.is_empty(), "this family declares `notices`");
        match draft {
            RegisteredReportDraft::Lifecycle(draft) => {
                assert_eq!(
                    draft.notices,
                    vec!["displaced run left running".to_string()]
                );
            }
            _ => panic!("wrong family"),
        }
    }

    /// The capability answer this seam exists for.
    ///
    /// `cli-update-report` and `cli-reinstall-report` have no `notices`
    /// member. The two wrong repairs are equally silent: invent the field (a
    /// new member on a registered format) or drop the notice (delete the only
    /// account of a predecessor left running). So the leftovers come BACK, and
    /// the adapter routes them.
    #[test]
    fn a_root_without_a_notices_member_returns_the_notices_it_cannot_carry() {
        for mut draft in [update_draft(), reinstall_draft()] {
            let unabsorbed = draft.absorb_notices(vec!["displaced run left running".into()]);
            assert_eq!(
                unabsorbed,
                vec!["displaced run left running".to_string()],
                "the notice survives the root that cannot hold it",
            );
            let json = match draft {
                RegisteredReportDraft::Update(draft) => {
                    serde_json::to_string(&draft.into_report(None)).unwrap()
                }
                RegisteredReportDraft::Reinstall(draft) => {
                    serde_json::to_string(&draft.into_report(None)).unwrap()
                }
                _ => panic!("wrong family"),
            };
            assert!(
                !json.contains("notices"),
                "and no `notices` key was invented on the wire: {json}",
            );
        }
    }

    fn update_draft() -> RegisteredReportDraft {
        RegisteredReportDraft::Update(Box::new(crate::commands::update::UpdateDraft::failed(
            &crate::commands::update::UpdateIdentity {
                project_root: std::path::PathBuf::from("/p"),
                scope: vibe_wire::generated::update_report::UpdateReportScope::All,
                packages: Vec::new(),
            },
            0,
            Vec::new(),
            vibe_install::InstallProgress::default(),
            Vec::new(),
        )))
    }

    fn reinstall_draft() -> RegisteredReportDraft {
        RegisteredReportDraft::Reinstall(Box::new(
            crate::commands::reinstall::ReinstallDraft::failed(
                &crate::commands::reinstall::ReinstallIdentity {
                    selected_project_root: std::path::PathBuf::from("/p"),
                    forced: false,
                },
                vibe_install::InstallProgress::default(),
                Vec::new(),
            ),
        ))
    }
}
