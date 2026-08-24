//! Package-overlay materialisation oracle, split from the parent at its test seam.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use super::*;

#[test]
fn package_overlay_is_a_same_format_derivation_input() {
    let src = TempDir::new().unwrap();
    let marked = "# Rules\n\n@fact:A A. <status stage=\"spec\" state=\"work\" action=\"drift\" comment=\"author\"/>\n\n@fact:B B. @status:spec/done\n";
    let untouched = "# Raw\n\n@fact:RAW   Spacing stays.   @status:spec/work\n";
    write(
        src.path(),
        crate::layout_paths::specs_path("RULE.md"),
        marked,
    );
    write(
        src.path(),
        crate::layout_paths::specs_path("RAW.md"),
        untouched,
    );
    let ws = TempDir::new().unwrap();
    write(
        ws.path(),
        crate::layout_paths::vibefacts_path("org.example.overlay.toml"),
        "schema = 1\n\n[[fact]]\naddress = \"spec://org.example/overlay/RULE#A\"\norigin = \"package\"\npackage = \"org.example/overlay\"\nstatus = \"impl/done\"\n\n[[fact]]\naddress = \"spec://org.example/overlay/RULE#B\"\norigin = \"package\"\npackage = \"org.example/overlay\"\n",
    );
    let slot = derive_markdown(ws.path(), src.path(), "overlay");
    let result = fs::read_to_string(slot.join(crate::layout_paths::specs_path("RULE.md"))).unwrap();
    assert!(result.contains("stage=\"impl\" state=\"done\" action=\"drift\" comment=\"author\""));
    assert!(result.contains("@fact:B B. @status:spec/done"));
    assert_eq!(
        fs::read_to_string(slot.join(crate::layout_paths::specs_path("RAW.md"))).unwrap(),
        untouched
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    assert_eq!(
        manifest
            .files
            .iter()
            .find(|file| file.source == crate::layout_paths::specs("RULE.md"))
            .map(|file| file.disposition),
        Some(DerivedFileDisposition::Converted)
    );
    assert_eq!(
        manifest.overlay_hash,
        vibe_facts::overlay_file_hash(ws.path(), "org.example/overlay")
    );
    assert!(format_is_current(&slot, SpecFormat::Markdown));
    fs::write(
        ws.path().join(crate::layout_paths::vibefacts_path(
            "org.example.overlay.toml",
        )),
        "schema = 1\n",
    )
    .unwrap();
    assert!(!format_is_current(&slot, SpecFormat::Markdown));

    let empty = TempDir::new().unwrap();
    write(
        empty.path(),
        crate::layout_paths::vibefacts_path("org.example.plain.toml"),
        "schema = 1\n",
    );
    let with_empty = derive_markdown(empty.path(), src.path(), "plain");
    let empty_bytes =
        fs::read(with_empty.join(crate::layout_paths::specs_path("RULE.md"))).unwrap();
    let absent = TempDir::new().unwrap();
    let without_dir = derive_markdown(absent.path(), src.path(), "plain");
    assert_eq!(
        empty_bytes,
        fs::read(without_dir.join(crate::layout_paths::specs_path("RULE.md"))).unwrap()
    );
}
