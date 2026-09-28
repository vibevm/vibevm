//! Selected documentation byte/page subclaims; full intake remains pending.

use super::{
    CheckFailure, CheckReport, CheckReportVerdict, CheckRequest, Finding, FindingState, capture,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use vibe_wire::generated::preview::e1::documentation_capture::DocumentationCapture;
use vibe_wire::generated::preview::e1::documentation_map::{
    DocumentationMap, TopicDisposition, TopicSourceKind,
};
use vibe_wire::generated::preview::e1::transition::{
    DocumentationCaptureRef, PendingCaptureState, PendingDocumentationState, Transition,
};

mod artifacts;
mod git;
mod xml;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-INPUT-CHECK");

fn fault(code: &'static str, subject: &str, reason: &str) -> CheckFailure {
    CheckFailure::new(code, subject, reason)
}

fn pending(report: &mut CheckReport, code: &str, subject: &str, reason: &str, state: FindingState) {
    report.findings.push(Finding {
        code: code.into(),
        subject: subject.into(),
        reason: reason.into(),
        state,
    });
}

pub(super) fn check(
    request: &CheckRequest,
    transition: &Transition,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    report.verdict = CheckReportVerdict::Pending;
    request
        .validate_preview()
        .map_err(|error| fault("request-meaning", error.field, error.reason))?;
    let path = Path::new(&request.transition);
    let release_dir = path.parent().unwrap_or(Path::new("."));
    let release = capture::open_project(release_dir)?;
    let root_bytes = capture::git(release_dir, &["rev-parse", "--show-toplevel"])?;
    let root_text = std::str::from_utf8(&root_bytes).map_err(|_| {
        fault(
            "documentation-product-root",
            "product",
            "Product root cannot be represented",
        )
    })?;
    let product_root = PathBuf::from(root_text.strip_suffix('\n').unwrap_or(root_text));
    let mut retained = artifacts::Retained::new(&release);
    let current_ref = if matches!(
        transition.documentation.state,
        PendingDocumentationState::PendingOwnerBinding
    ) {
        pending(
            report,
            "documentation-selection-missing",
            "documentation",
            "Owner-current documentation capture has not been selected",
            FindingState::Pending,
        );
        None
    } else {
        let (Some(id), Some(source), Some(digest)) = (
            &transition.documentation.capture_id,
            &transition.documentation.source,
            &transition.documentation.capture_file_sha256,
        ) else {
            return Err(fault(
                "documentation-selection",
                "documentation",
                "Current documentation reference is incomplete",
            ));
        };
        Some(DocumentationCaptureRef {
            capture_id: id.clone(),
            source: source.clone(),
            capture_file_sha256: digest.clone(),
        })
    };
    let current = current_ref
        .as_ref()
        .map(|reference| retained.capture(reference, report))
        .transpose()?;
    let base = transition
        .documentation_base
        .as_ref()
        .map(|reference| retained.capture(reference, report))
        .transpose()?;
    let map = if let Some(reference) = &transition.documentation_map {
        Some(retained.map(reference, report)?)
    } else {
        pending(
            report,
            "documentation-map-missing",
            "documentation",
            "An exact documentation map has not been supplied",
            FindingState::Pending,
        );
        None
    };
    let mut verified = BTreeMap::new();
    for input in current.iter().chain(base.iter()) {
        if !verified.contains_key(&input.capture_id) {
            let source = git::verify(input, request, &product_root, report)?;
            if let Some(source) = &source {
                xml::verify(input, &source.files, report)?;
            }
            verified.insert(input.capture_id.clone(), source);
        }
    }
    let references = transition
        .capture_references()
        .map_err(|error| fault("documentation-product-reference", error.field, error.reason))?;
    let mut products = BTreeSet::new();
    for input in current.iter().chain(base.iter()) {
        product(
            &input.product_baseline_capture_id,
            &references,
            &transition.dp2.state,
            release_dir,
            &release,
            &mut products,
            report,
        )?;
    }
    if let Some(map) = &map {
        product(
            &map.product_capture_id,
            &references,
            &transition.dp2.state,
            release_dir,
            &release,
            &mut products,
            report,
        )?;
        if let Some(current) = &current {
            if map.documentation_capture_id != current.capture_id {
                return Err(fault(
                    "documentation-map-capture",
                    "documentation-map",
                    "Map documentation identity differs from selected current capture",
                ));
            }
            if verified
                .get(&current.capture_id)
                .and_then(Option::as_ref)
                .is_some()
            {
                membership(map, current, transition, report)?;
            } else {
                pending(
                    report,
                    "documentation-map-membership-unverified",
                    &current.capture_id,
                    "Map page membership awaits supported verified source bytes",
                    FindingState::Pending,
                );
            }
        }
    }
    for (code, reason) in [
        (
            "documentation-topic-denominator-missing",
            "Independent complete product/topic denominator has not been bound",
        ),
        (
            "documentation-anchor-bindings-missing",
            "Typed documentation page/anchor claim bindings have not been supplied",
        ),
        (
            "documentation-editorial-proof-later",
            "Source subclaims do not establish editorial review or a three-way update",
        ),
        (
            "documentation-build-link-example-runtime-later",
            "Build, link, example and runtime verification remain separate obligations",
        ),
    ] {
        pending(report, code, "documentation", reason, FindingState::Pending);
    }
    Ok(())
}

fn product(
    id: &str,
    references: &[vibe_wire::generated::preview::e1::transition::CaptureRef],
    state: &PendingCaptureState,
    release_dir: &Path,
    release: &vibe_safefs::Project,
    seen: &mut BTreeSet<(String, String)>,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let mut found = false;
    for reference in references
        .iter()
        .filter(|reference| reference.capture_id == id)
    {
        found = true;
        let key = (id.into(), reference.source.clone());
        if seen.insert(key) {
            let verified = capture::verify_ref(
                release_dir,
                release,
                reference,
                &format!("documentation-product:{id}"),
            )?;
            report.checked_claims.extend(verified.claims);
            report.subject_ids.push(id.into());
        }
    }
    if !found {
        if !matches!(state, PendingCaptureState::Pending) {
            return Err(fault(
                "documentation-product-capture-missing",
                id,
                "Required product capture has no selected retained reference",
            ));
        }
        pending(
            report,
            "documentation-product-capture-missing",
            id,
            "Required product capture is unresolved while DP2 remains pending",
            FindingState::Pending,
        );
    }
    Ok(())
}

fn membership(
    map: &DocumentationMap,
    capture: &DocumentationCapture,
    transition: &Transition,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let selected: BTreeSet<_> = transition
        .change_records
        .iter()
        .map(String::as_str)
        .collect();
    for topic in &map.topics {
        if topic
            .change_ids
            .iter()
            .any(|id| !selected.contains(id.as_str()))
        {
            return Err(fault(
                "documentation-map-change",
                &topic.topic_id,
                "Map references an unselected permanent change ID",
            ));
        }
        if !topic.change_ids.is_empty() {
            pending(
                report,
                "documentation-record-bytes-unverified",
                &topic.topic_id,
                "Selected change IDs do not by themselves prove retained record-byte linkage",
                FindingState::Pending,
            );
        }
        if let Some(source) = &topic.source_page {
            let Some(page) = capture.pages.iter().find(|page| &page.path == source) else {
                return Err(fault(
                    "documentation-map-page",
                    &topic.topic_id,
                    "Map source page is absent from verified captured page membership",
                ));
            };
            if topic.source_kind != page.source_kind {
                return Err(fault(
                    "documentation-map-provenance",
                    &topic.topic_id,
                    "Map and capture source-kind declarations disagree",
                ));
            }
            report.checked_claims.push(format!(
                "documentation-declared-page-membership:{}:{}",
                capture.capture_id, topic.topic_id
            ));
        }
        if matches!(
            topic.disposition,
            TopicDisposition::Pending | TopicDisposition::Create
        ) || matches!(topic.source_kind, TopicSourceKind::Unbound)
        {
            pending(
                report,
                "documentation-topic-edit-pending",
                &topic.topic_id,
                "Topic has a pending or page-to-create source obligation",
                FindingState::Pending,
            );
        }
    }
    Ok(())
}
