//! `normal + static` compilation (PROP-035 §8) — the branch of the static
//! renderer ([`super::render_static`]) that compiles a `normal`-format
//! package's contribution to its `#use` / `#source`-resolved, tree-shaken
//! closure, rather than concatenating the file verbatim (the `simple` path).
//!
//! The hard algorithmic work is [`vibe_spec::compile_static_qualified`]; this
//! cell only derives the closure's seed from a [`BootEntry`] and adapts the
//! compiler's error into a REQ-citing [`WorkspaceError`].

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline");

use std::path::Path;

use specmark::spec;
use vibe_spec::{
    FileResolver, FsSectionSource, RenameEntry, SelfCoordinate, SpecAddress,
    compile_static_qualified,
};

use crate::boot::BootEntry;
use crate::{WorkspaceError, layout_paths};

/// Compile a `normal` package's static contribution (PROP-035 §8): the
/// `#use` / `#source`-resolved, tree-shaken, topologically-ordered closure
/// reachable from the entry's boot-snippet contract, rather than the file
/// concatenated verbatim. Resolution runs against the materialised
/// dependency tree (the same [`FileResolver`] the `#embed` pass uses), so the
/// closure may span `source/` and other packages the contract `#use`s.
///
/// Qualification is **per-node** (PROP-035 §8 phase 5, B-006 rider): every node
/// in the closure is qualified under its own authoring origin — derived from its
/// topo key — so a node spliced in from another package keeps its true
/// provenance, never the entry's; the returned rename map is `(origin, rename)`
/// per node, ready for the lane tombstone.
///
/// Errors are surfaced as [`WorkspaceError::InlineCompile`] naming the package
/// and the governing requirement (PROP-035 §8) — a structured, REQ-citing
/// diagnostic the installer prints rather than a bare compiler string.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline",
    r = 1
)]
pub(super) fn compile_normal_entry(
    entry: &BootEntry,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
) -> Result<(String, Vec<(String, RenameEntry)>), WorkspaceError> {
    let seed =
        normal_seed(&entry.origin, &entry.path).ok_or_else(|| WorkspaceError::InlineCompile {
            reason: format!(
                "cannot derive a spec:// seed for the normal package `{}` at `{}` \
                 (PROP-035 §8): expected a `<group>/<name>` origin and a path under a \
                 package's `{}` root",
                entry.origin,
                entry.path,
                layout_paths::slot_specs("<slot>", "")
            ),
        })?;
    let source = FsSectionSource::new(FileResolver::new(workspace_root, self_coord.clone()));
    compile_static_qualified(&seed, &source).map_err(|e| WorkspaceError::InlineCompile {
        reason: format!(
            "compiling the normal package `{}` closure (PROP-035 §8): {e}",
            entry.origin
        ),
    })
}

/// Derive the `spec://` seed for a `normal` static entry — the whole-document
/// address of its boot-snippet contract, from which [`compile_static_qualified`]
/// walks the `#use` / `#source` closure (PROP-035 §6/§8).
///
/// `origin` is the entry's `<group>/<name>` provenance (a hoisted entry may
/// append a ` [shared by …]` suffix, dropped here); `path` is the
/// workspace-relative path of the contract inside the package's dependency
/// slot. The doc-path is the segment after the slot's live specs root minus
/// the serialisation extension — `.md` or `.xml`, since a document's address
/// does not change with its serialisation (PROP-045) — so
/// `contract/greeting.md` and `boot/10-flow-wal.xml` both address
/// extensionless. The seed carries no anchor, so it names the whole document
/// (`DocTree` resolves an empty anchor to the root). Returns `None` when the
/// origin or path is not the expected package shape.
fn normal_seed(origin: &str, path: &str) -> Option<SpecAddress> {
    let coord = origin.split_whitespace().next()?;
    let (group, name) = coord.split_once('/')?;
    let specs_delimiter = format!("/{}/", layout_paths::specs(""));
    let (_, doc_rest) = path.split_once(&specs_delimiter)?;
    let doc_path = doc_rest
        .strip_suffix(".md")
        .or_else(|| doc_rest.strip_suffix(".xml"))
        .unwrap_or(doc_rest);
    SpecAddress::parse(&format!("spec://{group}/{name}/{doc_path}")).ok()
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#addressing")]
    fn normal_seed_derives_the_whole_doc_contract_address() {
        // The seed is the contract's whole-document address — no anchor, so the
        // compiler walks from the root (PROP-035 §6): `<group>/<name>` from the
        // origin, doc-path from the slot's live specs root minus `.md`.
        let slot = layout_paths::vibedeps("com.example.hello.greeter/1.0.0");
        let greeting = layout_paths::slot_specs(&slot, "contract/greeting.md");
        let s = normal_seed("com.example.hello/greeter", &greeting).unwrap();
        assert_eq!(
            s.without_pin(),
            "spec://com.example.hello/greeter/contract/greeting"
        );

        // A hoisted entry's ` [shared by …]` origin suffix is dropped.
        let h = normal_seed("com.example.hello/greeter [shared by a/b]", &greeting).unwrap();
        assert_eq!(
            h.without_pin(),
            "spec://com.example.hello/greeter/contract/greeting"
        );

        // A path with no live specs root, or a nameless origin, is not derivable.
        assert!(normal_seed("com.example.hello/greeter", "some/other/path.md").is_none());
        let invalid = layout_paths::slot_specs(layout_paths::vibedeps("x/1.0.0"), "a.md");
        assert!(normal_seed("nogroup", &invalid).is_none());
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#addressing")]
    fn normal_seed_strips_the_xml_serialisation_too() {
        // A document's address does not change with its serialisation
        // (PROP-045): an XML-materialised contract must yield the same
        // extensionless doc-path a Markdown one does, or the seed carries
        // `.xml` into the address and resolution dies on a double extension.
        let slot = layout_paths::vibedeps("org.vibevm.world.wal/1.0.0");
        let snippet = layout_paths::slot_specs(&slot, "boot/10-flow-wal.xml");
        let s = normal_seed("org.vibevm.world/wal", &snippet).unwrap();
        assert_eq!(
            s.without_pin(),
            "spec://org.vibevm.world/wal/boot/10-flow-wal"
        );
    }
}
