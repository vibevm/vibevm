//! Workspace-owned portable boot trace descriptors — the ONE place a boot
//! compilation's trace identity is minted (PROP-054 `##OBS-TRACE`, R3.4).
//!
//! Two kinds of real boot compile exist in an install, and each names itself
//! from data the workspace already owns authoritatively:
//!
//! * a **package unit** — the per-unit STATIC lane `emit_package_units`
//!   compiles into a dependency slot. Its base identity is the exact typed
//!   `(group, name)` coordinate plus the artifact target; its label carries
//!   the selected qualified package/version so a reader sees WHICH resolved
//!   package compiled, in a stable printable form.
//! * a **workspace node** — the root or a member's own boot artifacts. Its
//!   identity and label are the canonical workspace-relative path (`.` for
//!   the root), NEVER the absolute `node_dir`: an absolute developer path in
//!   a trace id would make the same project trace differently on two
//!   machines.
//!
//! ## The base spelling, closed
//!
//! ```text
//! unit:<group>/<name>#<artifact>
//! node:<rel>#<artifact>
//! ```
//!
//! The artifact/target spelling is part of the BASE, not just the label: the
//! same unit and the same node rel compile a genuinely different artifact
//! under `static-md` than under `static-xml`. Sharing one base across the two
//! would fold two targets into one attempt series — the second target would
//! read as a re-compilation of the first, and a
//! [`ScopeConflict`](super::TraceError::ScopeConflict) would be the only thing
//! standing between them. A VERSION change is the opposite case and
//! deliberately keeps the base: one adopted run may update a package in place,
//! and its unit is the same artifact recompiled, so it continues that base's
//! attempt series with a new label.
//!
//! Constructors here are pure and cheap, but callers construct a descriptor
//! ONLY when a recorder is present — the no-trace path allocates no id or
//! label strings at all, which is why the traced siblings take a
//! [`ScopeAcquisition`] built under `trace.is_some()` rather than a descriptor.
//!
//! `ScopeKind::Publish` is deliberately unused here: publish/init/uninstall
//! have no lifecycle run owner in this atom.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use vibe_core::manifest::SpecFormat;
use vibe_wire::generated::compiler_trace_index::e1::index::ArtifactTarget;

use crate::boot::hybrid::UnitId;

use super::{ScopeDescriptor, ScopeKind, TraceRun, TraceScope};

/// The trace identity of one package unit's STATIC compilation.
///
/// `version` is the RESOLVED version from the resolution — the label a reader
/// cross-references against the lockfile, and deliberately NOT part of the
/// base.
pub(crate) fn unit_descriptor(
    unit: &UnitId,
    version: &str,
    spec_format: SpecFormat,
) -> ScopeDescriptor {
    let (group, name) = unit;
    let (artifact, target) = static_artifact(spec_format);
    ScopeDescriptor {
        id: format!("unit:{group}/{name}#{artifact}"),
        kind: ScopeKind::Unit,
        label: format!("{group}/{name}@{version}"),
        artifact: artifact.to_string(),
        target,
    }
}

/// The trace identity of one workspace node's own boot artifacts. `node_rel`
/// is the canonical workspace-relative path — `.` for the root, the member's
/// forward-slashed rel path otherwise — never an absolute directory.
pub(crate) fn node_descriptor(node_rel: &str, spec_format: SpecFormat) -> ScopeDescriptor {
    let (artifact, target) = static_artifact(spec_format);
    ScopeDescriptor {
        id: format!("node:{node_rel}#{artifact}"),
        kind: ScopeKind::Node,
        label: node_rel.to_string(),
        artifact: artifact.to_string(),
        target,
    }
}

/// The artifact id and target the selected format compiles to — the only
/// place the two vocabularies are paired, so `static-md`/`static-xml` can
/// never drift between the artifact string and the target enum.
fn static_artifact(spec_format: SpecFormat) -> (&'static str, ArtifactTarget) {
    match spec_format {
        SpecFormat::Xml => ("static-xml", ArtifactTarget::StaticXml),
        SpecFormat::Mixed | SpecFormat::Markdown => ("static-md", ArtifactTarget::StaticMd),
    }
}

/// One artifact compilation's trace occurrence — **before** it is acquired.
///
/// A scope represents an artifact COMPILE, so it is acquired at the compile
/// boundary and nowhere earlier (R3.4 correction §2): INDEX rendering,
/// directory creation and artifact-plan preparation all precede the compiler
/// and may refuse, and a refusal there must leave no pending occurrence for
/// work that was never attempted. This cell is what a traced caller carries
/// through that preparation instead of a live scope — the base identity plus
/// the borrowed run, acquired at the last possible moment.
///
/// It also carries the fresh-artifact law (correction §1): a fresh unit
/// OBSERVES its existing output first, and only a successful observation may
/// declare an occurrence at all — [`skip_observed`](Self::skip_observed) is
/// the only path that declares one, and [`note_refusal`](Self::note_refusal)
/// declares nothing.
pub(crate) struct ScopeAcquisition<'a> {
    run: &'a TraceRun,
    base: ScopeDescriptor,
}

impl<'a> ScopeAcquisition<'a> {
    /// The occurrence one package unit's STATIC compile will spend.
    pub(crate) fn unit(
        run: &'a TraceRun,
        unit: &UnitId,
        version: &str,
        spec_format: SpecFormat,
    ) -> Self {
        Self {
            run,
            base: unit_descriptor(unit, version, spec_format),
        }
    }

    /// The occurrence one workspace node's STATIC compile will spend.
    pub(crate) fn node(run: &'a TraceRun, node_rel: &str, spec_format: SpecFormat) -> Self {
        Self {
            run,
            base: node_descriptor(node_rel, spec_format),
        }
    }

    /// Acquire the occurrence — called when a real compile is about to start,
    /// never before. `None` means the declaration refused and this
    /// compilation runs UNTRACED, with the refusal already retained as a
    /// bounded warning on the run.
    pub(crate) fn acquire(&self) -> Option<TraceScope> {
        self.run.acquire_scope_lossy(&self.base)
    }

    /// A proved-fresh artifact whose existing output was SUCCESSFULLY
    /// observed: declare the occurrence and immediately report it skipped,
    /// carrying the same output fingerprint authority a dirty compile
    /// completes with. Zero events, by construction — nothing was compiled.
    pub(crate) fn skip_observed(&self, fingerprint: &str) {
        if let Some(scope) = self.acquire() {
            scope.skip_lossy(fingerprint);
        }
    }

    /// An observation that refused (absent, symlinked, hard-linked, or any
    /// other safefs refusal). The proved boot freshness stands untouched: no
    /// occurrence is declared for this fresh attempt, and the one bounded
    /// warning names why — so the run can still finalise `ok`.
    pub(crate) fn note_refusal(&self, reason: &str) {
        self.run.note_dropped(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(group: &str, name: &str) -> UnitId {
        (vibe_core::Group::parse(group).unwrap(), name.to_string())
    }

    /// A unit's base identity comes from the exact typed coordinate and the
    /// selected target; the label carries the qualified package/version.
    #[test]
    fn a_unit_names_itself_from_its_coordinate_and_version() {
        let descriptor = unit_descriptor(&unit("org.vibevm", "wal"), "1.2.3", SpecFormat::Mixed);
        assert_eq!(descriptor.id, "unit:org.vibevm/wal#static-md");
        assert_eq!(descriptor.label, "org.vibevm/wal@1.2.3");
        assert_eq!(descriptor.artifact, "static-md");
        assert_eq!(descriptor.target, ArtifactTarget::StaticMd);
        assert!(matches!(descriptor.kind, ScopeKind::Unit));
    }

    /// A node's identity and label are the canonical workspace-relative rel —
    /// `.` for the root — and the format selects the artifact/target pair.
    #[test]
    fn a_node_names_itself_from_its_relative_path_never_an_absolute_one() {
        let root = node_descriptor(".", SpecFormat::Xml);
        assert_eq!(root.id, "node:.#static-xml");
        assert_eq!(root.label, ".");
        assert_eq!(root.artifact, "static-xml");
        assert_eq!(root.target, ArtifactTarget::StaticXml);
        assert!(matches!(root.kind, ScopeKind::Node));

        let member = node_descriptor("members/flow-wal", SpecFormat::Markdown);
        assert_eq!(member.id, "node:members/flow-wal#static-md");
        assert_eq!(member.label, "members/flow-wal");
        assert_eq!(member.artifact, "static-md");
    }

    /// The correction's law: the TARGET belongs to the base. The same unit and
    /// the same node rel compiled to Markdown and to XML are two artifacts and
    /// therefore two bases — never one attempt series wearing two targets.
    #[test]
    fn one_unit_and_one_node_rel_have_distinct_bases_per_target() {
        let md = unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Markdown);
        let xml = unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Xml);
        assert_ne!(md.id, xml.id, "one unit, two targets, two bases");
        assert_ne!(md.target, xml.target);

        let node_md = node_descriptor("members/alpha", SpecFormat::Markdown);
        let node_xml = node_descriptor("members/alpha", SpecFormat::Xml);
        assert_ne!(node_md.id, node_xml.id, "one rel, two targets, two bases");
        assert_ne!(node_md.target, node_xml.target);

        // `Mixed` and `Markdown` select the SAME artifact, so they are one
        // base: the pair is a target vocabulary, not a manifest vocabulary.
        assert_eq!(
            unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Mixed).id,
            md.id
        );
    }

    /// A version change under ONE target keeps the base: an update inside one
    /// adopted run recompiles the same artifact, and its occurrences belong to
    /// the same attempt series with a new label.
    #[test]
    fn a_version_change_under_one_target_keeps_the_base() {
        let one = unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Mixed);
        let two = unit_descriptor(&unit("org.a", "x"), "2.0.0", SpecFormat::Mixed);
        assert_eq!(one.id, two.id);
        assert_ne!(one.label, two.label);
        assert_eq!(one.target, two.target);
    }

    /// Two units, two nodes and two formats are all distinct bases: no
    /// collision is possible between any pair of them.
    #[test]
    fn distinct_bases_cannot_collide() {
        let ids = [
            unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Mixed).id,
            unit_descriptor(&unit("org.a", "y"), "1.0.0", SpecFormat::Mixed).id,
            unit_descriptor(&unit("org.b", "x"), "1.0.0", SpecFormat::Mixed).id,
            unit_descriptor(&unit("org.a", "x"), "1.0.0", SpecFormat::Xml).id,
            node_descriptor(".", SpecFormat::Mixed).id,
            node_descriptor(".", SpecFormat::Xml).id,
            node_descriptor("x", SpecFormat::Mixed).id,
        ];
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b, "{a} collides with {b}");
            }
        }
    }
}
