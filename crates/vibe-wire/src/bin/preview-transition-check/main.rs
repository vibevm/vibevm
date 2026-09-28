//! Read-only checker for the permanent Developer Preview transition records.

mod capture;

use std::fs;
use std::io::{self, Read};
use std::path::Path;

use vibe_wire::generated::preview::e1::check_report::{
    CheckReport, CheckReportMode, CheckReportStage, CheckReportVerdict, Finding, FindingState,
};
use vibe_wire::generated::preview::e1::check_request::{
    CheckRequest, CheckRequestMode, CheckRequestStage,
};
use vibe_wire::generated::preview::e1::transition::Transition;

pub(crate) struct CheckFailure {
    code: &'static str,
    subject: String,
    reason: String,
}

impl CheckFailure {
    pub(crate) fn new(
        code: &'static str,
        subject: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            code,
            subject: subject.into(),
            reason: reason.into(),
        }
    }
}

fn report_mode(mode: &CheckRequestMode) -> CheckReportMode {
    match mode {
        CheckRequestMode::Capture => CheckReportMode::Capture,
        CheckRequestMode::Contribution => CheckReportMode::Contribution,
        CheckRequestMode::Bindings => CheckReportMode::Bindings,
        CheckRequestMode::Documentation => CheckReportMode::Documentation,
        CheckRequestMode::Release => CheckReportMode::Release,
        CheckRequestMode::Retired => CheckReportMode::Retired,
    }
}

fn report_stage(stage: &Option<CheckRequestStage>) -> Option<CheckReportStage> {
    stage.as_ref().map(|stage| match stage {
        CheckRequestStage::Roles => CheckReportStage::Roles,
        CheckRequestStage::Linked => CheckReportStage::Linked,
        CheckRequestStage::Intake => CheckReportStage::Intake,
        CheckRequestStage::Updated => CheckReportStage::Updated,
        CheckRequestStage::Verified => CheckReportStage::Verified,
        CheckRequestStage::Reviewed => CheckReportStage::Reviewed,
    })
}

fn empty_report(mode: CheckReportMode, stage: Option<CheckReportStage>) -> CheckReport {
    CheckReport {
        schema: 1,
        mode,
        stage,
        verdict: CheckReportVerdict::Failed,
        subject_ids: Vec::new(),
        checked_claims: Vec::new(),
        findings: Vec::new(),
    }
}

fn fail(report: &mut CheckReport, error: CheckFailure) {
    report.verdict = CheckReportVerdict::Failed;
    report.findings.push(Finding {
        code: error.code.into(),
        subject: error.subject,
        reason: error.reason,
        state: FindingState::Failed,
    });
}

fn run(request: &CheckRequest) -> CheckReport {
    let mut report = empty_report(report_mode(&request.mode), report_stage(&request.stage));
    let transition_path = Path::new(&request.transition);
    let bytes = match fs::read(transition_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            fail(
                &mut report,
                CheckFailure::new(
                    "transition-read",
                    "transition",
                    "Transition file could not be read",
                ),
            );
            return report;
        }
    };
    let transition: Transition = match serde_json::from_slice(&bytes) {
        Ok(transition) => transition,
        Err(error) => {
            fail(
                &mut report,
                CheckFailure::new("transition-shape", "transition", error.to_string()),
            );
            return report;
        }
    };
    if let Err(error) = transition.validate_preview() {
        fail(
            &mut report,
            CheckFailure::new("transition-meaning", error.field, error.reason),
        );
        return report;
    }
    report.subject_ids.push(transition.dp1.capture_id.clone());
    match request.mode {
        CheckRequestMode::Capture => match capture::verify(transition_path, &transition) {
            Ok(claims) => {
                report.verdict = CheckReportVerdict::Passed;
                report.checked_claims = claims;
                if !transition.proof.documentation_verified {
                    report.findings.push(Finding {
                        code: "documentation-proof-later".into(),
                        subject: "documentation".into(),
                        reason: "Documentation proof belongs to a later stage.".into(),
                        state: FindingState::Pending,
                    });
                }
                if !transition.proof.final_product_panel_passed {
                    report.findings.push(Finding {
                        code: "final-product-proof-later".into(),
                        subject: "dp2".into(),
                        reason: "The final product panel has not run for this candidate.".into(),
                        state: FindingState::Pending,
                    });
                }
            }
            Err(error) => fail(&mut report, error),
        },
        _ => {
            report.verdict = CheckReportVerdict::Pending;
            report.findings.push(Finding {
                code: "stage-pending".into(),
                subject: "preview-transition".into(),
                reason: "This stage has not yet established its required domain evidence.".into(),
                state: FindingState::Pending,
            });
        }
    }
    report
        .findings
        .sort_by(|a, b| (&a.code, &a.subject).cmp(&(&b.code, &b.subject)));
    report
}

fn main() {
    let mut input = String::new();
    let mut invalid = empty_report(CheckReportMode::InvalidRequest, None);
    if let Err(error) = io::stdin().take(1_048_577).read_to_string(&mut input) {
        fail(
            &mut invalid,
            CheckFailure::new("request-read", "request", error.to_string()),
        );
        emit(invalid);
        return;
    }
    if input.len() > 1_048_576 {
        fail(
            &mut invalid,
            CheckFailure::new("request-size", "request", "request exceeds 1 MiB"),
        );
        emit(invalid);
        return;
    }
    let request: CheckRequest = match serde_json::from_str(&input) {
        Ok(request) => request,
        Err(error) => {
            fail(
                &mut invalid,
                CheckFailure::new("request-shape", "request", error.to_string()),
            );
            emit(invalid);
            return;
        }
    };
    if let Err(error) = request.validate_preview() {
        fail(
            &mut invalid,
            CheckFailure::new("request-meaning", error.field, error.reason),
        );
        emit(invalid);
        return;
    }
    emit(run(&request));
}

fn emit(report: CheckReport) {
    let code = match report.verdict {
        CheckReportVerdict::Passed => 0,
        CheckReportVerdict::Failed => 1,
        CheckReportVerdict::Pending => 2,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("generated report serializes")
    );
    std::process::exit(code);
}
