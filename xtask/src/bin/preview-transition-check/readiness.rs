//! Stage prerequisites only; declarations never establish an owner result.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#BOOTSTRAP-STAGE-READINESS");

use super::{
    CheckFailure, CheckReport, CheckReportVerdict, CheckRequest, CheckRequestMode,
    CheckRequestStage, Finding, FindingState,
};
use vibe_wire::generated::preview::e1::transition::Transition;

fn pending(report: &mut CheckReport, code: &str, subject: &str, reason: &str) {
    report.findings.push(Finding {
        code: code.into(),
        subject: subject.into(),
        reason: reason.into(),
        state: FindingState::Pending,
    });
}

fn unsupported(report: &mut CheckReport, code: &str, reason: &str) {
    report.findings.push(Finding {
        code: code.into(),
        subject: "bindings".into(),
        reason: reason.into(),
        state: FindingState::Unsupported,
    });
}

fn roles(transition: &Transition, report: &mut CheckReport) {
    if transition.change_records.is_empty() {
        pending(
            report,
            "bindings-record-selection-missing",
            "bindings",
            "Permanent change records have not been selected; an empty declared list does not establish native adoption or complete record coverage.",
        );
    } else {
        pending(
            report,
            "bindings-record-selection-unverified",
            "bindings",
            "Selected permanent change IDs require exact retained record-byte and actual native subject inspection; this readiness branch does not verify that selection.",
        );
    }
    if transition.contribution_scope.is_none() {
        pending(
            report,
            "bindings-contribution-scope-missing",
            "contribution",
            "Exact before/after contribution captures and selected record/evidence bindings have not been supplied; an absent scope is not completed proof.",
        );
    } else {
        pending(
            report,
            "bindings-contribution-scope-unverified",
            "contribution",
            "The selected contribution scope requires its owning capture, record-byte, actual delta and required-claim predicates; declared bindings or producer completion flags are not that result.",
        );
    }
    unsupported(
        report,
        "bindings-roles-owner-unavailable",
        "This declaration-only readiness branch has not run the authoritative native role/meaningful-identity adapter; the bindings roles entrypoint performs that inspection. Roles requires actual non-normative discovery, immutable ID/byte preservation and unchanged intent/work identities; declarations or equal producer hashes do not prove them.",
    );
}

fn linked(transition: &Transition, report: &mut CheckReport) {
    roles(transition, report);
    unsupported(
        report,
        "bindings-linked-capability-owner-unavailable",
        "Linked requires the roles predecessor and complete canonical capability/interface joins through the actual owner; that inspection adapter is not connected.",
    );
    unsupported(
        report,
        "bindings-linked-migration-owner-unavailable",
        "Linked additionally requires canonical format-break and migration joins with exact supported semantics through their actual owners; those adapters are not connected. A prose recipe is not an implemented converter.",
    );
}

fn product(transition: &Transition, report: &mut CheckReport) {
    if transition.dp2.capture_id.is_none() {
        pending(
            report,
            "product-candidate-capture-missing",
            "dp2",
            "An exact DP2 candidate capture has not been selected; no final candidate subject can be inferred from earlier captures or completion flags.",
        );
    } else {
        pending(
            report,
            "product-candidate-capture-unverified",
            "dp2",
            "The selected DP2 reference is a declaration. Its exact capture bytes, commit/tree and scope require the owning capture predicate; this readiness branch does not verify them.",
        );
    }
}

fn documentation(transition: &Transition, report: &mut CheckReport) {
    if transition.documentation.capture_id.is_none() {
        pending(
            report,
            "documentation-current-capture-missing",
            "documentation",
            "An exact owner-current documentation capture has not been selected.",
        );
    } else {
        pending(
            report,
            "documentation-current-capture-unverified",
            "documentation",
            "Selected current documentation bytes and source provenance require the intake owner result; a captured/updated/verified label is not that result.",
        );
    }
    if transition.documentation_map.is_none() {
        pending(
            report,
            "documentation-map-missing",
            "documentation",
            "An exact documentation map source/digest has not been selected.",
        );
    } else {
        pending(
            report,
            "documentation-map-unverified",
            "documentation",
            "The selected map requires verified bytes and an independent complete product/topic and document-anchor binding; declared topics and checked labels are not proof.",
        );
    }
}

fn updated(transition: &Transition, report: &mut CheckReport) {
    product(transition, report);
    documentation(transition, report);
    if transition.documentation_base.is_none() {
        pending(
            report,
            "documentation-base-capture-missing",
            "documentation-base",
            "An exact documentation base subject for the reviewed three-way delta has not been selected; current bytes are not an inferred base.",
        );
    } else {
        pending(
            report,
            "documentation-base-capture-unverified",
            "documentation-base",
            "The selected base capture requires its owning byte/source predicate; a reference or equal declared digest does not verify a base.",
        );
    }
    pending(
        report,
        "documentation-intake-predecessor-unverified",
        "documentation",
        "Updated requires the intake predecessor: exact supplied captures/binding, independent complete topic census and typed document anchors. This readiness branch has not established that result.",
    );
    pending(
        report,
        "documentation-reviewed-delta-unverified",
        "documentation",
        "Updated requires the exact base/current/product three-way topic delta and subject-bound editorial review preserving independent edits and generated-source authority; changed files or an updated label do not establish it.",
    );
}

fn guide(transition: &Transition, report: &mut CheckReport) {
    if transition.migration_guide.source.is_none() {
        pending(
            report,
            "migration-guide-selection-missing",
            "migration-guide",
            "An exact migration guidance source/digest has not been selected.",
        );
    } else {
        pending(
            report,
            "migration-guide-selection-unverified",
            "migration-guide",
            "The selected guide bytes, reviewed action/limitation matrix and exact scoped recipes require their owning predicates; draft/reviewed/verified labels do not establish them.",
        );
    }
}

fn reviewed(transition: &Transition, report: &mut CheckReport) {
    product(transition, report);
    documentation(transition, report);
    guide(transition, report);
    linked(transition, report);
    pending(
        report,
        "release-bindings-predecessor-unverified",
        "release",
        "Reviewed requires the actual roles/linked predecessor results; unrelated passing evidence and producer completion flags do not establish these canonical joins.",
    );
    pending(
        report,
        "release-documentation-predecessor-unverified",
        "documentation",
        "Reviewed requires the current verified documentation predecessor on the exact candidate; documentation_verified is only a producer assertion.",
    );
    pending(
        report,
        "release-net-reconciliation-unverified",
        "release",
        "Reviewed requires independent complete DP1-to-candidate source/package/interface net reconciliation against permanent records, including additions, removals, same-version content, reverts and supersession; intermediate labels or counts are not a net oracle.",
    );
    pending(
        report,
        "release-guidance-review-unverified",
        "migration-guide",
        "Reviewed additionally requires subject-bound review of guidance and every audience/platform action, preservation boundary, prerequisite, repeat and partial-failure recovery recipe; selecting prose alone is not review.",
    );
}

fn rehearsed(report: &mut CheckReport) {
    pending(
        report,
        "release-reviewed-predecessor-unverified",
        "release",
        "Verified requires the actual independent net reconciliation and reviewed guidance predecessor on matching exact subjects.",
    );
    pending(
        report,
        "release-rehearsal-unverified",
        "migration-rehearsal",
        "Verified additionally requires actual isolated subject-bound rehearsal, applicable positive/negative cases, repeat behavior, preservation and partial-failure recovery on exact recipe/capture/platform inputs. migration_rehearsed and unrelated success cannot establish those results.",
    );
}

/// Report the selected late stage's unmet prerequisites without I/O or proof claims.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-081#BOOTSTRAP-STAGE-READINESS"
)]
pub(super) fn check(
    request: &CheckRequest,
    transition: &Transition,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    request
        .validate_preview()
        .map_err(|error| CheckFailure::new("request-meaning", error.field, error.reason))?;
    transition
        .validate_preview()
        .map_err(|error| CheckFailure::new("transition-meaning", error.field, error.reason))?;
    match (&request.mode, &request.stage) {
        (CheckRequestMode::Bindings, Some(CheckRequestStage::Roles)) => {
            roles(transition, report);
            pending(
                report,
                "bindings-linked-proof-later",
                "bindings",
                "Roles does not establish the later complete capability/migration linkage predicate.",
            );
        }
        (CheckRequestMode::Bindings, Some(CheckRequestStage::Linked)) => {
            linked(transition, report);
            pending(
                report,
                "bindings-documentation-release-proof-later",
                "bindings",
                "Linked does not establish later editorial, documentation build/claim, guide, rehearsal or final closure proof.",
            );
        }
        (CheckRequestMode::Documentation, Some(CheckRequestStage::Updated)) => {
            updated(transition, report);
            pending(
                report,
                "documentation-verified-proof-later",
                "documentation",
                "Updated does not establish the later current build/link/example/claim verification predicate.",
            );
        }
        (CheckRequestMode::Documentation, Some(CheckRequestStage::Verified)) => {
            updated(transition, report);
            pending(
                report,
                "documentation-current-verification-unverified",
                "documentation",
                "Verified additionally requires actual current build/link/example/claim results bound to exact documentation/product/build inputs and honest platform limits; a successful build alone or checked topic flags cannot establish them.",
            );
            pending(
                report,
                "documentation-release-proof-later",
                "documentation",
                "Documentation verification does not establish later net guidance, migration rehearsal or scaffold-free closure proof.",
            );
        }
        (CheckRequestMode::Release, Some(CheckRequestStage::Reviewed)) => {
            reviewed(transition, report);
            pending(
                report,
                "release-rehearsal-proof-later",
                "migration-rehearsal",
                "Reviewed does not establish the later actual rehearsal/repeat/recovery predicate or finalize DP2.",
            );
        }
        (CheckRequestMode::Release, Some(CheckRequestStage::Verified)) => {
            reviewed(transition, report);
            rehearsed(report);
            pending(
                report,
                "release-retirement-proof-later",
                "retirement",
                "Verified does not establish actual scaffold removal, permanent final-product proof or later attestation.",
            );
        }
        (CheckRequestMode::Retired, None) => {
            reviewed(transition, report);
            rehearsed(report);
            pending(
                report,
                "retired-scaffold-free-candidate-unverified",
                "retirement",
                "Candidate eligibility requires the actual source/documentation commit C with temporary scaffold absent and all permanent retained inputs independent of campaign/private state; a sealed DP2 label does not establish it.",
            );
            pending(
                report,
                "retired-permanent-reference-product-proof-unverified",
                "retirement",
                "Retired requires actual permanent reference/format/final-product proof on that exact scaffold-free candidate; final_product_panel_passed is only a producer assertion.",
            );
            pending(
                report,
                "retired-attestation-after-candidate",
                "final-attestation",
                "Later narrow metadata-only attestation must identify C and retained hashes without product/documentation-claim/recipe delta. It is separate from candidate eligibility; readiness claims neither eligibility nor final closure.",
            );
        }
        _ => {
            return Err(CheckFailure::new(
                "readiness-stage",
                "preview-transition",
                "This request belongs to an existing owning checker, not the late-stage readiness branch",
            ));
        }
    }
    report.verdict = CheckReportVerdict::Pending;
    Ok(())
}
