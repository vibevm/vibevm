//! Profile-owned XML parse and named anchor fold over verified immutable blobs.

use super::{CheckFailure, CheckReport, FindingState, fault, pending};
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::preview::e1::documentation_capture::DocumentationCapture;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-INPUT-CHECK");

pub(super) fn verify(
    input: &DocumentationCapture,
    files: &BTreeMap<String, Vec<u8>>,
    report: &mut CheckReport,
) -> Result<(), CheckFailure> {
    let id = input.capture_id.as_str();
    if input.source_profile != "urn:vibevm:doc:xml" {
        pending(
            report,
            "documentation-profile-unsupported",
            id,
            "Selected source profile has no supported reader in this atom",
            FindingState::Unsupported,
        );
        return Ok(());
    }
    let pages: BTreeSet<_> = input.pages.iter().map(|page| page.path.as_str()).collect();
    for path in files.keys().filter(|path| path.ends_with(".xml")) {
        let excluded = input.exclusions.iter().any(|exclusion| {
            if exclusion.path.ends_with('/') {
                path.starts_with(&exclusion.path)
            } else {
                path == &exclusion.path
            }
        });
        if !pages.contains(path.as_str()) && !excluded {
            return Err(fault(
                "documentation-page-census",
                &format!("{id}:{path}"),
                "XML source file is neither a selected page nor a reasoned exclusion",
            ));
        }
    }
    for page in &input.pages {
        let subject = format!("{id}:{}", page.path);
        let bytes = files.get(&page.path).ok_or_else(|| {
            fault(
                "documentation-page-census",
                &subject,
                "Selected page has no verified captured bytes",
            )
        })?;
        let raw = std::str::from_utf8(bytes).map_err(|_| {
            fault(
                "documentation-xml-encoding",
                &subject,
                "Selected page bytes are not UTF-8",
            )
        })?;
        let ir =
            vibe_specdoc::from_xml_with(raw, vibe_specdoc::Vocabulary::Doc).map_err(|error| {
                CheckFailure::new(
                    "documentation-xml",
                    &subject,
                    format!(
                        "Canonical documentation XML reader refused source line {}",
                        error.line
                    ),
                )
            })?;
        let anchors = vibe_doc::manifest::page::anchors_of(&ir);
        report
            .checked_claims
            .push(format!("documentation-xml-page:{subject}"));
        for anchor in anchors {
            report
                .subject_ids
                .push(format!("documentation-anchor:{subject}#{anchor}"));
        }
        report
            .checked_claims
            .push(format!("documentation-named-anchor-census:{subject}"));
    }
    report
        .checked_claims
        .push(format!("documentation-xml-page-census:{id}"));
    Ok(())
}
