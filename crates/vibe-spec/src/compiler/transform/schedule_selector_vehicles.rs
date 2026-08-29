//! The T8 selector-position vehicles: one recording behavior per
//! selector-legal stage, logging exactly which documents reached it.
//!
//! A pass counter cannot answer this atom's question. "The selector ran" and
//! "the behavior ran on THESE documents" are different claims, and only the
//! second distinguishes a gate that filters from a gate that is consulted
//! and ignored — so each vehicle records the document it was handed, and the
//! tests assert the exact set.
//!
//! Each sighting carries BOTH spellings of the document: the address label
//! and the subject's declared path. They differ for a declared document
//! (`spec://org.demo/alpha/boot/entry#root` versus `boot/alpha.md`), which is
//! what makes a failure message say which of the two a `paths` dimension was
//! matched against.

use std::cell::RefCell;

use crate::compiler::ir::{DocumentIr, SourceIr};
use crate::compiler::worklist::document_key;

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformConfig, TransformStage};

// Per-thread sighting logs: the suite runs tests in parallel, and a
// process-wide static would let one test's vehicles pollute another's set.
thread_local! {
    static SOURCE_SEEN: RefCell<Vec<Sighting>> = const { RefCell::new(Vec::new()) };
    static DOCUMENT_SEEN: RefCell<Vec<Sighting>> = const { RefCell::new(Vec::new()) };
}

/// One document a selector-gated behavior actually saw: `(address label,
/// declared path)`.
pub(super) type Sighting = (String, String);

pub(super) fn reset_selector_sightings() {
    SOURCE_SEEN.with(|seen| seen.borrow_mut().clear());
    DOCUMENT_SEEN.with(|seen| seen.borrow_mut().clear());
}

/// The documents the source position saw, byte-sorted for an exact-set
/// assertion.
pub(super) fn source_sightings() -> Vec<Sighting> {
    sorted(&SOURCE_SEEN)
}

/// The documents the document position saw, byte-sorted.
pub(super) fn document_sightings() -> Vec<Sighting> {
    sorted(&DOCUMENT_SEEN)
}

fn sorted(log: &'static std::thread::LocalKey<RefCell<Vec<Sighting>>>) -> Vec<Sighting> {
    let mut rows = log.with(|seen| seen.borrow().clone());
    rows.sort();
    rows
}

/// One expected sighting, spelled the way a test authors it.
pub(super) fn sighting(address: &str, declared_path: &str) -> Sighting {
    (address.to_string(), declared_path.to_string())
}

/// Records every document the SOURCE position hands it, unchanged.
pub(super) struct RecordingSelectorSource;

impl TransformBehavior for RecordingSelectorSource {
    fn name(&self) -> &str {
        "test-selector-source"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        let row = (
            document_key(input.address()).label(),
            input.subject().declared_path().to_string(),
        );
        SOURCE_SEEN.with(|seen| seen.borrow_mut().push(row));
        Ok(input)
    }
}

/// Records every document the DOCUMENT position hands it, unchanged.
pub(super) struct RecordingSelectorDocument;

impl TransformBehavior for RecordingSelectorDocument {
    fn name(&self) -> &str {
        "test-selector-document"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Document
    }
    fn run_document(
        &self,
        _config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        let source = input.source();
        let row = (
            document_key(source.address()).label(),
            source.subject().declared_path().to_string(),
        );
        DOCUMENT_SEEN.with(|seen| seen.borrow_mut().push(row));
        Ok(input)
    }
}
