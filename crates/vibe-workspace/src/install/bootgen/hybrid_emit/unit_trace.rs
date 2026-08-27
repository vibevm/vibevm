//! One package unit's trace occurrence, and the fresh-output observation that
//! decides whether that occurrence exists at all (PROP-054 `##OBS-TRACE`,
//! R3.4).
//!
//! A unit that the dirty-subgraph proved fresh (PROP-038 §2.8) compiled
//! nothing, so the only honest thing a trace can say about it is the
//! fingerprint of the output that is ALREADY on disk. That reading is a real
//! filesystem act and it can refuse — the file may be missing, a symlink, a
//! hard link, or anything else `vibe-safefs` will not treat as exclusively
//! owned. The order therefore matters, and it is the whole content of this
//! cell:
//!
//! **observe first, declare second.** A successful observation declares one
//! occurrence and immediately reports it `skipped` with that fingerprint. A
//! refusal declares NOTHING: the already-proved boot freshness stands, the
//! artifact and its mtime are untouched, one bounded warning names why, and
//! the run is left with no unterminated occurrence — so `finish(Ok)` still
//! finalises durably.
//!
//! The opposite order (declare, then observe) is what this cell exists to
//! prevent: it leaves a `pending` scope for a compile that never happened, and
//! the run's terminal word then describes work nobody attempted.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::Path;

use vibe_core::layout;
use vibe_core::manifest::SpecFormat;

use crate::boot::hybrid::UnitId;
use crate::compile_trace::{ScopeAcquisition, TraceRun};
use crate::{boot_artifacts, path_to_slash};

/// Everything one unit's traced emission needs, built ONLY when a recorder is
/// present: the base identity it would spend, and the workspace-relative
/// spelling of the output a fresh skip reads.
pub(super) struct UnitTrace<'a> {
    acquisition: ScopeAcquisition<'a>,
    static_rel: String,
}

impl<'a> UnitTrace<'a> {
    pub(super) fn new(
        run: &'a TraceRun,
        unit: &UnitId,
        version: &str,
        spec_format: SpecFormat,
        slot: &str,
    ) -> Self {
        Self {
            acquisition: ScopeAcquisition::unit(run, unit, version, spec_format),
            static_rel: static_rel_path(slot, spec_format),
        }
    }

    /// The occurrence the dirty path acquires at its compile boundary.
    pub(super) const fn acquisition(&self) -> &ScopeAcquisition<'a> {
        &self.acquisition
    }

    /// The fresh path, in its mandated order: observe the existing selected
    /// STATIC artifact no-follow/single-link, and only on success declare the
    /// occurrence and skip it with that fingerprint.
    pub(super) fn record_fresh_skip(&self, workspace_root: &Path) {
        match observe_output_fingerprint(workspace_root, &self.static_rel) {
            Ok(fingerprint) => self.acquisition.skip_observed(&fingerprint),
            Err(reason) => self.acquisition.note_refusal(&reason),
        }
    }
}

/// The workspace-root-relative, forward-slashed path of a unit's selected
/// STATIC artifact — the spelling the no-follow/single-link observation reads.
fn static_rel_path(slot: &str, spec_format: SpecFormat) -> String {
    path_to_slash(
        &Path::new(slot)
            .join(layout::current_boot_dir())
            .join(boot_artifacts::static_file(spec_format)),
    )
}

/// Read the existing selected STATIC artifact no-follow/single-link and
/// fingerprint it through the ONE canonical authority. Any refusal is a
/// bounded reason for a trace diagnostic — never an install error.
fn observe_output_fingerprint(workspace_root: &Path, static_rel: &str) -> Result<String, String> {
    let project = vibe_safefs::Project::open(workspace_root)
        .map_err(|error| format!("opening the workspace root safely: {error:#}"))?;
    match project.read_file(static_rel) {
        Ok(Some(bytes)) => Ok(vibe_spec::emitted_output_fingerprint(&bytes)),
        Ok(None) => Err(format!(
            "the fresh unit's emitted output `{static_rel}` is absent, so no occurrence was \
             declared for it"
        )),
        Err(error) => Err(format!(
            "observing the fresh unit's emitted output `{static_rel}` no-follow/single-link \
             refused, so no occurrence was declared for it: {error:#}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The observed path is the unit's OWN slot lane, forward-slashed, and it
    /// follows the selected format's spelling.
    #[test]
    fn the_observed_path_is_the_slot_s_selected_lane() {
        let md = static_rel_path("vibevm/vibedeps/org.demo/x/1.0.0", SpecFormat::Mixed);
        assert!(md.ends_with("/STATIC.md"), "{md}");
        assert!(md.starts_with("vibevm/vibedeps/org.demo/x/1.0.0/"), "{md}");
        assert!(!md.contains('\\'), "always forward-slashed: {md}");
        let xml = static_rel_path("vibevm/vibedeps/org.demo/x/1.0.0", SpecFormat::Xml);
        assert!(xml.ends_with("/STATIC.xml"), "{xml}");
    }

    /// An absent output is a refusal, not a fingerprint — the arm that must
    /// declare no occurrence.
    #[test]
    fn an_absent_output_refuses_with_a_bounded_reason() {
        let root = tempfile::tempdir().expect("a temporary workspace root");
        let error = observe_output_fingerprint(root.path(), "vibevm/vibespecs/boot/STATIC.md")
            .expect_err("an absent artifact cannot be fingerprinted");
        assert!(error.contains("is absent"), "{error}");
        assert!(error.contains("no occurrence was declared"), "{error}");
    }

    /// A real file fingerprints through the one canonical authority — the same
    /// spelling a dirty compile completes with.
    #[test]
    fn an_observed_file_carries_the_canonical_fingerprint() {
        let root = tempfile::tempdir().expect("a temporary workspace root");
        let rel = "boot/STATIC.md";
        std::fs::create_dir_all(root.path().join("boot")).unwrap();
        std::fs::write(root.path().join(rel), b"# STATIC\n").unwrap();
        assert_eq!(
            observe_output_fingerprint(root.path(), rel).expect("a plain file observes"),
            vibe_spec::emitted_output_fingerprint(b"# STATIC\n")
        );
    }
}
