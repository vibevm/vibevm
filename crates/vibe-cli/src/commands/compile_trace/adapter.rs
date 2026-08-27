//! The ONE post-finalize adapter: everything that happens between "the trace
//! is closed" and "the process has printed".
//!
//! Four commands share this order, and no caller may re-implement any step of
//! it — that is the whole reason it is a function rather than a convention:
//!
//! 1. attach the `trace` member to the selected root, exactly once;
//! 2. honour the funnel's TYPED plan disposition — flush for success and
//!    failure, discard for a park;
//! 3. emit at most one registered root;
//! 4. if a FAILED command's report rendering also fails, that is two
//!    independent failures: the report one becomes a secondary diagnostic and
//!    the command still returns its ORIGINAL error;
//! 5. surface the owner's notices EXACTLY once, through the one channel each
//!    mode can actually show them in.
//!
//! ## The notice mapping, and why quiet is not an afterthought
//!
//! Notices are produced before the command's own session exists (closing a
//! displaced predecessor), so they belong to no document by nature. Where they
//! can be SEEN differs per mode, and the mapping is explicit because every
//! implicit version of it loses them somewhere:
//!
//! ```text
//! member present            → the member's warnings, and nothing else
//! JSON, no member, a root   → the registered root's `notices`, once —
//!                             unless that root has no `notices` member, and
//!                             then stderr, once (see below)
//! JSON, no member, NO root  → stderr diagnostics (a root that is never
//!                             emitted carries nothing)
//! human, no member          → one bounded stderr diagnostic per notice
//! quiet, no member          → a COUNT, folded into the single line quiet owns
//! ```
//!
//! The "unless" is a capability, not a special case. `cli-update-report` and
//! `cli-reinstall-report` declare no `notices` list, and the two obvious
//! repairs are both silent corruptions: inventing the field puts a member on a
//! registered format nobody agreed to, and dropping the notice deletes the only
//! account of, say, a predecessor run left `running` on disk. So
//! [`RegisteredReportDraft::absorb_notices`] answers by type and hands back
//! what it could not take, and the leftovers reach stderr here — the one
//! channel that exists in every mode and is not part of the document stream a
//! machine reader parses.
//!
//! Quiet is the sharp case. Its contract is exactly one line, and a `notices`
//! member is invisible on a terminal while a diagnostic line would be a second
//! line. So quiet gets a count in its summary suffix (on success) or in its
//! error suffix (on failure) — never a line of its own. The packet's explicit
//! exception for a bounded secondary diagnostic is likewise folded there.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use anyhow::Result;

use vibe_wire::generated::shared::CompileTraceReport;

use crate::output;

use super::bounded::BoundedDiagnostic;
use super::draft::RegisteredReportDraft;
use super::{FinalizedCommand, PlanDisposition, present, quiet};

/// Consume the join and produce the command's whole observable output.
///
/// This function does exactly one thing: name the production sink. Every
/// decision and every emission lives in [`render_finalized_with_sink`], which
/// the tests drive with a capturing sink — so deleting an `absorb_into` or an
/// `emit` CALL is red, not merely unobserved.
pub(crate) fn render_finalized(
    ctx: &output::Context,
    finalized: FinalizedCommand<RegisteredReportDraft>,
) -> Result<()> {
    render_finalized_with_sink(ctx, finalized, &mut CtxSink(ctx))
}

/// The whole adapter, with its diagnostic channel injected.
pub(crate) fn render_finalized_with_sink(
    ctx: &output::Context,
    finalized: FinalizedCommand<RegisteredReportDraft>,
    sink: &mut dyn DiagnosticSink,
) -> Result<()> {
    let FinalizedCommand {
        mut report,
        plan,
        trace,
        original_error,
        emit_report,
        trace_requested,
        notices,
    } = finalized;
    // The landed law, restated where it could still be violated: a member can
    // only exist for a request.
    debug_assert!(trace_requested || trace.is_none());
    // The funnel's own answer and the report's typed handoff must agree. They
    // are computed from different facts on purpose — one from the exit arm,
    // one from the measured delegation — so a disagreement is a real defect in
    // whichever produced it, not a style question.
    debug_assert_eq!(
        report.parked(),
        plan == PlanDisposition::Discard,
        "the exit's park and the report's handoff disagree",
    );

    let quiet_mode = ctx.is_quiet() && !ctx.is_json();
    // ONE seam: planning decides where each notice can be seen AND performs
    // the report mutation; execution performs the emission through a sink.
    // The tests below drive the same two functions with a captured sink, so
    // deleting either half is red rather than merely unobserved.
    let notice_plan = plan_notices(&trace, quiet_mode, ctx.is_json(), emit_report, notices);
    let NoticeResidue {
        folded: folded_notices,
        unabsorbed,
    } = notice_plan.absorb_into(&mut report);

    let mut suffix = match trace.as_ref() {
        Some(trace) if quiet_mode => present::quiet_suffix(trace),
        _ => String::new(),
    };
    if folded_notices > 0 {
        suffix.push_str(&format!(", {folded_notices} trace notice(s)"));
    }
    // On success the suffix rides the command's own summary; on failure the
    // only line is `main`'s error, so it travels with the error instead.
    let (summary_suffix, mut error_suffix) = if original_error.is_some() {
        (String::new(), suffix)
    } else {
        (suffix, String::new())
    };

    let rendered = if emit_report {
        let deferred = match plan {
            PlanDisposition::Discard => {
                ctx.discard_json_plans();
                Ok(())
            }
            PlanDisposition::Flush => ctx.flush_json_plans(),
        };
        deferred.and_then(|()| sink.render_root(ctx, report, trace.clone(), &summary_suffix))
    } else {
        // A historically silent failure emits no root at all and leaves its
        // previews for `main`'s existing failure flush.
        Ok(())
    };

    // Human mode reads the member as a table. JSON carries it inside the root
    // and quiet carries it as a suffix — printing it here would be the second
    // copy each of those modes forbids.
    if let Some(trace) = trace.as_ref()
        && !ctx.is_json()
        && !quiet_mode
    {
        present::render_human(ctx, trace);
    }
    notice_plan.emit(sink);
    // The one routing the plan could not decide alone: whether the SELECTED
    // root can hold a notice at all is a property of its generated type, and
    // the plan is built before the root is known. Emitted here, once, and only
    // for the notices `absorb_into` really refused.
    for notice in &unabsorbed {
        sink.diagnostic(notice);
    }

    match (rendered, original_error) {
        (Ok(()), None) => Ok(()),
        (Ok(()), Some(error)) => Err(quiet::attach(error, &error_suffix)),
        (Err(error), None) => Err(error),
        // Two independent failures. The command's error is the one that
        // decides the exit code and the one the operator is chasing; the
        // report's refusal is a secondary diagnostic — a line of its own where
        // there is room for one, and a count in the suffix where there is not.
        (Err(refusal), Some(error)) => {
            if quiet_mode {
                error_suffix.push_str(", 1 report refusal");
            } else {
                sink.diagnostic(
                    BoundedDiagnostic::new(format_args!(
                        "this command's report could not be written: {refusal:#}"
                    ))
                    .as_str(),
                );
            }
            Err(quiet::attach(error, &error_suffix))
        }
    }
}

/// The adapter's two output channels, injectable together.
///
/// Both halves are here for the same reason: a plan that DECIDES correctly and
/// never EXECUTES is a defect no routing-tuple test can see. With the sink
/// injected, deleting either the `emit` call or the `absorb_into` call in the
/// executor is red — the captured diagnostics go empty, or the captured root
/// arrives without its notice.
pub(crate) trait DiagnosticSink {
    /// A bounded diagnostic that belongs to no document.
    fn diagnostic(&mut self, text: &str);

    /// The one registered root, already carrying whatever was folded into it.
    fn render_root(
        &mut self,
        ctx: &output::Context,
        report: RegisteredReportDraft,
        trace: Option<CompileTraceReport>,
        quiet_suffix: &str,
    ) -> Result<()>;
}

/// The production sink: stderr, and the command's own renderer.
struct CtxSink<'a>(&'a output::Context);

impl DiagnosticSink for CtxSink<'_> {
    fn diagnostic(&mut self, text: &str) {
        self.0.diagnostic(text);
    }

    fn render_root(
        &mut self,
        ctx: &output::Context,
        report: RegisteredReportDraft,
        trace: Option<CompileTraceReport>,
        quiet_suffix: &str,
    ) -> Result<()> {
        report.render(ctx, trace, quiet_suffix)
    }
}

/// What this mode will do with the owner's notices — decided once, executed
/// once, and testable as a unit.
pub(crate) struct NoticePlan {
    /// Folded into the registered root's own `notices` member.
    root: Vec<String>,
    /// Written to the diagnostic sink, one line each.
    stderr: Vec<String>,
    /// Counted into quiet's single line, because that is all it can hold.
    folded: usize,
}

/// What absorbing left over: the quiet count, and the notices the selected
/// root's generated type has no member for.
pub(crate) struct NoticeResidue {
    /// Counted into quiet's single line.
    pub(crate) folded: usize,
    /// Refused by the root, and therefore still owed a channel.
    pub(crate) unabsorbed: Vec<String>,
}

impl NoticePlan {
    /// Perform the report half, and report what is left.
    ///
    /// The mutation lives HERE rather than at the call site so that "plan says
    /// root, root actually gets it" is one function a test can drive.
    pub(crate) fn absorb_into(&self, report: &mut RegisteredReportDraft) -> NoticeResidue {
        let unabsorbed = if self.root.is_empty() {
            Vec::new()
        } else {
            report.absorb_notices(self.root.clone())
        };
        NoticeResidue {
            folded: self.folded,
            unabsorbed,
        }
    }

    /// Perform the diagnostic half.
    pub(crate) fn emit(&self, sink: &mut dyn DiagnosticSink) {
        for notice in &self.stderr {
            sink.diagnostic(notice);
        }
    }
}

/// Where this mode can actually SHOW a notice.
///
/// Pure, and separate from the rendering, because the interesting cases are
/// all "which of three channels" questions that a captured-stdout test would
/// answer only indirectly.
///
/// The JSON arm is split on `emit_report` for a reason that cost a whole class
/// of notice once already: a root that is never emitted carries nothing. A
/// historically silent failure emits no document at all, so folding into its
/// `notices` member is the same as deleting them — they take stderr instead,
/// which is a diagnostic channel in every mode and not part of the document
/// stream a machine reader parses.
pub(crate) fn plan_notices(
    trace: &Option<CompileTraceReport>,
    quiet_mode: bool,
    json: bool,
    emit_report: bool,
    notices: Vec<String>,
) -> NoticePlan {
    let (root, stderr, folded) = match (trace, quiet_mode, json) {
        // The member already carries them, verbatim, in its warnings.
        (Some(_), _, _) => (Vec::new(), Vec::new(), 0),
        // A count is all a single line can hold — and it still says there IS
        // something to look at.
        (None, true, _) => (Vec::new(), Vec::new(), notices.len()),
        (None, false, true) if emit_report => (notices, Vec::new(), 0),
        (None, false, _) => (Vec::new(), notices, 0),
    };
    NoticePlan {
        root,
        stderr,
        folded,
    }
}

#[cfg(test)]
#[path = "adapter/tests.rs"]
mod tests;
