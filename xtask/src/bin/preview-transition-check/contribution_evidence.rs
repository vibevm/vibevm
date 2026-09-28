//! Current required evidence selection; historical attempts remain retained data.

use super::{CheckFailure, CheckReport, FindingState, capture, fault, pending};
use std::collections::BTreeSet;
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::release_impact::{
    EvidenceRefVerdict, ImplementationSubjectKind, ReleaseImpact,
};
use vibe_wire::generated::preview::e1::transition::ContributionScope;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#CONTRIBUTION-HISTORY-SELECTION");

pub(super) fn check(
    project: &Project,
    record: &ReleaseImpact,
    required: Option<&[String]>,
    current: &BTreeSet<String>,
    scope: &ContributionScope,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let Some(required) = required else {
        pending(
            report,
            "required-claims-missing",
            &record.change_id,
            "Required current evidence claims have not been selected",
            FindingState::Pending,
        );
        return Ok(());
    };
    for claim in required {
        let subject = format!("{}:{claim}", record.change_id);
        let mut matching = record
            .evidence
            .iter()
            .filter(|entry| &entry.claim_id == claim);
        let Some(entry) = matching.next() else {
            return Err(fault(
                "evidence-claim-missing",
                &subject,
                "Required proof claim is absent from the retained record",
            ));
        };
        if matching.next().is_some() {
            return Err(fault(
                "evidence-claim-ambiguous",
                &subject,
                "Required proof claim does not resolve uniquely",
            ));
        }
        match entry.verdict {
            EvidenceRefVerdict::Failed => {
                return Err(fault(
                    "evidence-failed",
                    &subject,
                    "Required evidence records a failed result",
                ));
            }
            EvidenceRefVerdict::Unavailable => {
                pending(
                    report,
                    "evidence-unavailable",
                    &subject,
                    "Required evidence is unavailable",
                    FindingState::Unavailable,
                );
                continue;
            }
            EvidenceRefVerdict::Unverified => {
                pending(
                    report,
                    "evidence-unverified",
                    &subject,
                    "Required evidence remains unverified",
                    FindingState::Pending,
                );
                continue;
            }
            EvidenceRefVerdict::Passed => {}
        }
        if !current.contains(&entry.subject_id)
            && entry.subject_id != scope.before_capture_id
            && entry.subject_id != scope.after_capture_id
        {
            if record.implementation_subjects.iter().any(|implementation| {
                implementation.identity == entry.subject_id
                    && !matches!(implementation.kind, ImplementationSubjectKind::Commit)
            }) {
                pending(
                    report,
                    "evidence-subject-unsupported",
                    &subject,
                    "Required nonlocal subject authority cannot be established by this scope",
                    FindingState::Unsupported,
                );
            } else {
                return Err(fault(
                    "evidence-subject",
                    &subject,
                    "Required passing proof does not bind a current implementation or selected scope capture",
                ));
            }
        }
        let Some(expected) = &entry.sha256 else {
            pending(
                report,
                "evidence-digest-missing",
                &subject,
                "Required passing evidence has no retained SHA-256 binding",
                FindingState::Pending,
            );
            continue;
        };
        match capture::digest_within(project, &entry.source, &subject, "evidence-unavailable") {
            Ok(actual) if &actual == expected => report
                .checked_claims
                .push(format!("evidence-bytes:{subject}")),
            Ok(_) => {
                return Err(fault(
                    "evidence-digest",
                    &subject,
                    "Required evidence bytes differ from the selected digest",
                ));
            }
            Err(error) if error.code == "evidence-unavailable" => pending(
                report,
                error.code,
                &subject,
                &error.reason,
                FindingState::Unavailable,
            ),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
