//! Manager-owned publication of a compiled per-unit boot lane (PROP-038
//! §2.1; R4.1 atom B) — the narrow seam through which the per-unit emitter
//! hands an already-rendered INDEX / already-compiled STATIC triple to the
//! crash-recoverable transaction manager the node path already uses. No
//! caller can reach the transaction's internals through this cell; the one
//! `pub(crate)` wrapper is the whole surface.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units");

use std::path::Path;

use vibe_core::manifest::SpecFormat;

use super::transaction;
use super::{INDEX_FILE, STATIC_FILE, STATIC_XML_FILE, WorkspaceError, static_file};

/// The generated STATIC spelling a target format does NOT select — the file
/// every publication must leave absent so a lane never carries both.
pub(super) fn stale_static_file(spec_format: SpecFormat) -> &'static str {
    if matches!(spec_format, SpecFormat::Xml) {
        STATIC_FILE
    } else {
        STATIC_XML_FILE
    }
}

/// Publish one package unit's already-compiled boot lane (PROP-038 §2.1;
/// R4.1 atom B) through the same crash-recoverable transaction manager the
/// node path uses: `INDEX.md`, the selected STATIC's presence/bytes, and the
/// stale spelling's absence land in ONE call — or, on any refusal, not at
/// all (a partially applied set rolls back to the recorded before-state).
///
/// `index_text` is the fully rendered `INDEX.md` (fingerprint header
/// included); `static_text` is the compiled static lane, `None` meaning the
/// unit publishes no static artifact (both spellings end absent). Unlike the
/// node path this writes **no** redirect blocks — a dependency package slot
/// is not an agent entry point. Callers hand over compiled bytes only:
/// every fallible render/compile has already happened, so a failure here is
/// a publication failure, never a lost compile, and byte-equal artifacts
/// keep their mtime (the transaction replaces nothing it does not have to).
pub(crate) fn publish_unit_artifacts(
    boot_dir: &Path,
    index_text: &str,
    static_text: Option<&str>,
    spec_format: SpecFormat,
) -> Result<(), WorkspaceError> {
    transaction::write_production_with_selectors(
        transaction::ArtifactWrite {
            index_path: &boot_dir.join(INDEX_FILE),
            index_bytes: index_text.as_bytes(),
            static_path: &boot_dir.join(static_file(spec_format)),
            static_bytes: static_text.map(str::as_bytes),
            stale_path: &boot_dir.join(stale_static_file(spec_format)),
        },
        // No selector work: a package slot writes no redirect blocks.
        |_| Ok(()),
    )
}
