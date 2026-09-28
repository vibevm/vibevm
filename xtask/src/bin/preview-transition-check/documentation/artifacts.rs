//! Exact retained artifact reads and identity reuse through the shared boundary.

use super::{CheckFailure, CheckReport, capture, fault};
use std::collections::BTreeMap;
use vibe_safefs::Project;
use vibe_wire::generated::preview::e1::documentation_capture::DocumentationCapture;
use vibe_wire::generated::preview::e1::documentation_map::DocumentationMap;
use vibe_wire::generated::preview::e1::transition::{DocumentationCaptureRef, SourceDigestRef};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-INPUT-CHECK");

pub(super) struct Retained<'a> {
    release: &'a Project,
    identities: BTreeMap<String, String>,
    captures: BTreeMap<(String, String), DocumentationCapture>,
}

impl<'a> Retained<'a> {
    pub(super) fn new(release: &'a Project) -> Self {
        Self {
            release,
            identities: BTreeMap::new(),
            captures: BTreeMap::new(),
        }
    }

    pub(super) fn capture(
        &mut self,
        reference: &DocumentationCaptureRef,
        report: &mut CheckReport,
    ) -> Result<DocumentationCapture, CheckFailure> {
        if self
            .identities
            .get(&reference.capture_id)
            .is_some_and(|digest| digest != &reference.capture_file_sha256)
        {
            return Err(fault(
                "documentation-capture-conflict",
                &reference.capture_id,
                "One documentation identity names conflicting capture bytes",
            ));
        }
        let key = (
            reference.source.clone(),
            reference.capture_file_sha256.clone(),
        );
        let capture = if let Some(capture) = self.captures.get(&key) {
            capture.clone()
        } else {
            let bytes = capture::read_within(
                self.release,
                &reference.source,
                &reference.capture_id,
                "documentation-capture-read",
            )?;
            if capture::sha256(&bytes) != reference.capture_file_sha256 {
                return Err(fault(
                    "documentation-capture-digest",
                    &reference.capture_id,
                    "Documentation capture bytes differ from selected digest",
                ));
            }
            let capture: DocumentationCapture = serde_json::from_slice(&bytes).map_err(|_| {
                fault(
                    "documentation-capture-shape",
                    &reference.capture_id,
                    "Documentation capture does not match its registered shape",
                )
            })?;
            capture.validate_documentation_capture().map_err(|error| {
                fault(
                    "documentation-capture-meaning",
                    &reference.capture_id,
                    error.reason,
                )
            })?;
            let raw = capture::digest_within(
                self.release,
                &capture.source_record,
                &reference.capture_id,
                "documentation-raw-read",
            )?;
            if raw != capture.source_record_sha256 {
                return Err(fault(
                    "documentation-raw-digest",
                    &reference.capture_id,
                    "Raw documentation observation bytes changed",
                ));
            }
            self.captures.insert(key, capture.clone());
            capture
        };
        if capture.capture_id != reference.capture_id {
            return Err(fault(
                "documentation-capture-binding",
                &reference.capture_id,
                "Capture payload identity differs from selected reference",
            ));
        }
        self.identities.insert(
            reference.capture_id.clone(),
            reference.capture_file_sha256.clone(),
        );
        report.subject_ids.push(reference.capture_id.clone());
        report.checked_claims.extend([
            format!("documentation-capture-bytes:{}", reference.capture_id),
            format!("documentation-raw-bytes:{}", reference.capture_id),
        ]);
        Ok(capture)
    }

    pub(super) fn map(
        &self,
        reference: &SourceDigestRef,
        report: &mut CheckReport,
    ) -> Result<DocumentationMap, CheckFailure> {
        let bytes = capture::read_within(
            self.release,
            &reference.source,
            "documentation-map",
            "documentation-map-read",
        )?;
        if capture::sha256(&bytes) != reference.sha256 {
            return Err(fault(
                "documentation-map-digest",
                "documentation-map",
                "Map bytes differ from selected digest",
            ));
        }
        let map: DocumentationMap = serde_json::from_slice(&bytes).map_err(|_| {
            fault(
                "documentation-map-shape",
                "documentation-map",
                "Documentation map does not match its registered shape",
            )
        })?;
        map.validate_preview().map_err(|error| {
            fault(
                "documentation-map-meaning",
                "documentation-map",
                error.reason,
            )
        })?;
        report.checked_claims.push("documentation-map-bytes".into());
        Ok(map)
    }
}
