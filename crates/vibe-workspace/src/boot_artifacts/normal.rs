//! Derive the normal-format whole-document seed for the compiler binder.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline");

use vibe_spec::SpecAddress;

use crate::layout_paths;

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
pub(super) fn normal_seed(origin: &str, path: &str) -> Option<SpecAddress> {
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

/// The honest document witness for a hoisted simple or normal boot snippet.
pub(super) fn hoisted_seed(origin: &str, path: &str) -> Option<SpecAddress> {
    if let Some(seed) = normal_seed(origin, path) {
        return Some(seed);
    }
    let coord = origin.split_whitespace().next()?;
    let (group, name) = coord.split_once('/')?;
    let (_, boot_rest) = path.rsplit_once("/boot/")?;
    let doc = boot_rest
        .strip_suffix(".md")
        .or_else(|| boot_rest.strip_suffix(".xml"))
        .unwrap_or(boot_rest);
    SpecAddress::parse(&format!("spec://{group}/{name}/boot/{doc}")).ok()
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

    #[test]
    fn hoisted_seed_accepts_the_legacy_slot_boot_shape() {
        let seed = hoisted_seed(
            "org.vibevm/shared",
            "vibevm/vibedeps/org.vibevm.shared/1.0.0/boot/shared.md",
        )
        .unwrap();
        assert_eq!(seed.without_pin(), "spec://org.vibevm/shared/boot/shared");
    }
}
