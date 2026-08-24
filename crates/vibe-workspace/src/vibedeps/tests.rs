//! Out-of-line unit oracles for the vibedeps cell.

use super::*;
use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;

#[cfg(test)]
fn version(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

#[cfg(test)]
fn g(s: &str) -> Group {
    Group::parse(s).unwrap()
}

#[cfg(test)]
fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[cfg(test)]
fn derive_markdown(ws: &Path, src: &Path, name: &str) -> PathBuf {
    materialise_with_spec_format(
        ws,
        &g("org.example"),
        name,
        &version("1.0.0"),
        src,
        CopyMode::Copy,
        SpecFormat::Markdown,
        "sha256:source",
    )
    .unwrap();
    slot_abs_path(ws, &g("org.example"), name, &version("1.0.0"))
}

#[test]
fn slot_rel_path_is_group_name_version() {
    let rel = slot_rel_path(&g("org.vibevm"), "wal", &version("0.3.0"));
    assert_eq!(rel, crate::layout_paths::vibedeps("org.vibevm.wal/0.3.0"));
}

#[test]
fn slot_abs_path_joins_under_workspace_root() {
    let root = Path::new("ws-root");
    let abs = slot_abs_path(root, &g("org.vibevm"), "rust", &version("2.1.0"));
    assert!(abs.starts_with(root));
    assert!(abs.ends_with(crate::layout_paths::vibedeps_path("org.vibevm.rust/2.1.0")));
}

#[test]
fn materialise_copies_the_tree_verbatim() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(
        src.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n",
    );
    write(src.path(), "boot/10-flow-wal.md", "# boot");
    write(
        src.path(),
        crate::layout_paths::specs_path("flows/wal/WAL.md"),
        "# protocol",
    );

    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();

    let slot = ws
        .path()
        .join(crate::layout_paths::vibedeps_path("org.vibevm.wal/0.3.0"));
    assert_eq!(
        fs::read_to_string(slot.join("vibe.toml")).unwrap(),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("boot/10-flow-wal.md")).unwrap(),
        "# boot"
    );
    assert_eq!(
        fs::read_to_string(slot.join(crate::layout_paths::specs_path("flows/wal/WAL.md"))).unwrap(),
        "# protocol"
    );
    // The returned footprint is slot-relative, forward-slashed, sorted.
    assert_eq!(
        written,
        vec![
            PathBuf::from("boot/10-flow-wal.md"),
            crate::layout_paths::specs_path("flows/wal/WAL.md"),
            PathBuf::from("vibe.toml"),
        ]
    );
}

#[test]
fn materialise_skips_dot_git() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    write(src.path(), ".git/config", "[core]");
    write(src.path(), ".git/objects/ab/cdef", "blob");
    // A `.git` nested deeper than the root is skipped too.
    write(src.path(), "boot/.git/HEAD", "ref: refs/heads/main");
    write(src.path(), "boot/snippet.md", "# snippet");

    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "w",
        &version("1.0.0"),
        src.path(),
    )
    .unwrap();

    let slot = ws
        .path()
        .join(crate::layout_paths::vibedeps_path("org.vibevm.w/1.0.0"));
    assert!(slot.join("vibe.toml").is_file());
    assert!(slot.join("boot/snippet.md").is_file());
    assert!(!slot.join(".git").exists());
    assert!(!slot.join("boot/.git").exists());
    assert_eq!(
        written,
        vec![PathBuf::from("boot/snippet.md"), PathBuf::from("vibe.toml")]
    );
}

#[test]
fn materialise_is_idempotent_and_clears_stale_files() {
    let ws = TempDir::new().unwrap();
    let src1 = TempDir::new().unwrap();
    write(src1.path(), "vibe.toml", "v1");
    write(src1.path(), "stale.md", "remove me");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "auth",
        &version("0.1.0"),
        src1.path(),
    )
    .unwrap();

    // Re-materialise from a source that no longer carries `stale.md`.
    let src2 = TempDir::new().unwrap();
    write(src2.path(), "vibe.toml", "v2");
    let written = materialise(
        ws.path(),
        &g("org.vibevm"),
        "auth",
        &version("0.1.0"),
        src2.path(),
    )
    .unwrap();

    let slot = ws
        .path()
        .join(crate::layout_paths::vibedeps_path("org.vibevm.auth/0.1.0"));
    assert_eq!(fs::read_to_string(slot.join("vibe.toml")).unwrap(), "v2");
    assert!(
        !slot.join("stale.md").exists(),
        "stale file must be cleared"
    );
    assert_eq!(written, vec![PathBuf::from("vibe.toml")]);
}

#[test]
fn materialise_errors_when_source_missing() {
    let ws = TempDir::new().unwrap();
    let missing = ws.path().join("no-such-source");
    let err = materialise(
        ws.path(),
        &g("org.vibevm"),
        "ghost",
        &version("0.1.0"),
        &missing,
    )
    .unwrap_err();
    assert!(matches!(err, WorkspaceError::Io { .. }), "{err}");
}

#[test]
fn is_materialised_reflects_slot_presence() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    assert!(!is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0")
    ));
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0"),
        src.path(),
    )
    .unwrap();
    assert!(is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "fmt",
        &version("1.0.0")
    ));
}

#[test]
fn remove_slot_deletes_and_reports() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();

    assert!(remove_slot(ws.path(), &g("org.vibevm"), "wal", &version("0.3.0")).unwrap());
    assert!(!is_materialised(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0")
    ));
    // A second removal finds nothing to do.
    assert!(!remove_slot(ws.path(), &g("org.vibevm"), "wal", &version("0.3.0")).unwrap());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink",
    r = 1
)]
fn materialise_hardlink_mode_places_the_full_tree() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    write(src.path(), "boot/s.md", "# s");
    let written = materialise_with(
        ws.path(),
        &g("org.vibevm"),
        "w",
        &version("1.0.0"),
        src.path(),
        CopyMode::Hardlink,
    )
    .unwrap();
    let slot = ws
        .path()
        .join(crate::layout_paths::vibedeps_path("org.vibevm.w/1.0.0"));
    // Hardlinked (or copy-fallback) — either way the content is present
    // and the footprint matches a copy materialisation.
    assert_eq!(fs::read_to_string(slot.join("vibe.toml")).unwrap(), "x");
    assert!(slot.join("boot/s.md").is_file());
    assert_eq!(
        written,
        vec![PathBuf::from("boot/s.md"), PathBuf::from("vibe.toml")]
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn in_place_slot_path_is_unversioned() {
    let rel = in_place_slot_rel_path(&g("org.vibevm"), "chromium");
    assert_eq!(rel, crate::layout_paths::vibedeps("org.vibevm.chromium"));
    let abs = in_place_slot_abs_path(Path::new("ws"), &g("org.vibevm"), "chromium");
    assert!(abs.ends_with(crate::layout_paths::vibedeps_path("org.vibevm.chromium")));
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn materialise_in_place_moves_the_clone_keeping_git() {
    let ws = TempDir::new().unwrap();
    let clone = TempDir::new().unwrap();
    write(clone.path(), "vibe.toml", "[package]\n");
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "src/main.rs", "fn main() {}");

    materialise_in_place(ws.path(), &g("org.vibevm"), "giant", clone.path()).unwrap();

    let slot = ws
        .path()
        .join(crate::layout_paths::vibedeps_path("org.vibevm.giant"));
    assert!(slot.join("vibe.toml").is_file());
    assert!(slot.join("src/main.rs").is_file());
    assert!(slot.join(".git/HEAD").is_file());
    assert!(is_in_place_slot(ws.path(), &g("org.vibevm"), "giant"));
    assert!(!clone.path().join("vibe.toml").exists());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn is_in_place_slot_false_for_a_versioned_copy() {
    let ws = TempDir::new().unwrap();
    // A versioned `copy` slot has no `.git` at the <group>.<name> level,
    // so it is not mistaken for an in-place slot.
    let src = TempDir::new().unwrap();
    write(src.path(), "vibe.toml", "x");
    materialise(
        ws.path(),
        &g("org.vibevm"),
        "wal",
        &version("0.3.0"),
        src.path(),
    )
    .unwrap();
    assert!(!is_in_place_slot(ws.path(), &g("org.vibevm"), "wal"));
}

#[test]
fn remove_in_place_slot_deletes_and_reports() {
    let ws = TempDir::new().unwrap();
    let clone = TempDir::new().unwrap();
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "f", "x");
    materialise_in_place(ws.path(), &g("org.vibevm"), "big", clone.path()).unwrap();
    assert!(remove_in_place_slot(ws.path(), &g("org.vibevm"), "big").unwrap());
    assert!(!is_in_place_slot(ws.path(), &g("org.vibevm"), "big"));
    // A second removal finds nothing to do.
    assert!(!remove_in_place_slot(ws.path(), &g("org.vibevm"), "big").unwrap());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#vendoring",
    r = 1
)]
fn ensure_gitignored_appends_once() {
    let ws = TempDir::new().unwrap();
    let entry = crate::layout_paths::vibedeps("org.vibevm.giant");
    ensure_gitignored(ws.path(), &entry).unwrap();
    let gi = fs::read_to_string(ws.path().join(".gitignore")).unwrap();
    assert!(gi.contains(&format!("{entry}/")), "{gi}");
    // Idempotent — a second call does not duplicate the entry.
    ensure_gitignored(ws.path(), &entry).unwrap();
    let gi2 = fs::read_to_string(ws.path().join(".gitignore")).unwrap();
    assert_eq!(gi2.matches(&entry).count(), 1, "{gi2}");
}

#[test]
fn mixed_format_is_the_legacy_verbatim_tree_without_identity_record() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "README.md", "# Package\n");
    write(
        src.path(),
        crate::layout_paths::specs_path("guide.md"),
        "# Guide\n",
    );

    materialise_with_spec_format(
        ws.path(),
        &g("org.vibevm"),
        "mixed",
        &version("1.0.0"),
        src.path(),
        CopyMode::Copy,
        SpecFormat::Mixed,
        "",
    )
    .unwrap();

    let slot = slot_abs_path(ws.path(), &g("org.vibevm"), "mixed", &version("1.0.0"));
    assert_eq!(fs::read(slot.join("README.md")).unwrap(), b"# Package\n");
    assert_eq!(
        fs::read(slot.join(crate::layout_paths::specs_path("guide.md"))).unwrap(),
        b"# Guide\n"
    );
    assert!(!slot.join(DERIVED_MANIFEST_FILENAME).exists());
    assert!(format_is_current(&slot, SpecFormat::Mixed));
}

#[test]
fn xml_format_converts_only_spec_genre_and_records_every_file() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write(src.path(), "README.md", "# Package\n\nOverview.\n");
    write(
        src.path(),
        crate::layout_paths::specs_path("guide.md"),
        "# Guide\n\nText.\n",
    );
    write(src.path(), "LICENSE.md", "license bytes\n");
    write(src.path(), "vibe.toml", "[package]\nname = \"sample\"\n");

    materialise_with_spec_format(
        ws.path(),
        &g("org.vibevm"),
        "xml",
        &version("1.0.0"),
        src.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        "sha256:source",
    )
    .unwrap();

    let slot = slot_abs_path(ws.path(), &g("org.vibevm"), "xml", &version("1.0.0"));
    assert!(slot.join("README.xml").is_file());
    assert!(
        slot.join(crate::layout_paths::specs_path("guide.xml"))
            .is_file()
    );
    assert!(!slot.join("README.md").exists());
    assert!(
        !slot
            .join(crate::layout_paths::specs_path("guide.md"))
            .exists()
    );
    assert_eq!(
        fs::read(slot.join("LICENSE.md")).unwrap(),
        b"license bytes\n"
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    assert_eq!(manifest.source_hash, "sha256:source");
    assert_eq!(manifest.output_format, SpecFormat::Xml);
    assert_eq!(manifest.converter_recipe, vibe_specdoc::CONVERTER_RECIPE);
    assert_eq!(manifest.derived_hash, compute_derived_hash(&slot).unwrap());
    assert_eq!(
        manifest
            .files
            .iter()
            .filter(|file| file.disposition == DerivedFileDisposition::Converted)
            .count(),
        2
    );
    assert_eq!(manifest.files.len(), 4);
}

#[test]
fn markdown_format_converts_xml_and_copies_a_rejected_candidate_verbatim() {
    let ws = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    let xml = vibe_specdoc::to_xml(&vibe_specdoc::from_markdown("# Valid\n\nBody.\n").unwrap());
    write(
        src.path(),
        crate::layout_paths::specs_path("valid.xml"),
        &xml,
    );
    write(
        src.path(),
        crate::layout_paths::specs_path("rejected.xml"),
        "<not-closed",
    );

    materialise_with_spec_format(
        ws.path(),
        &g("org.vibevm"),
        "markdown",
        &version("1.0.0"),
        src.path(),
        CopyMode::Copy,
        SpecFormat::Markdown,
        "sha256:source",
    )
    .unwrap();

    let slot = slot_abs_path(ws.path(), &g("org.vibevm"), "markdown", &version("1.0.0"));
    assert!(
        slot.join(crate::layout_paths::specs_path("valid.md"))
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(slot.join(crate::layout_paths::specs_path("rejected.xml"))).unwrap(),
        "<not-closed"
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    let rejected = manifest
        .files
        .iter()
        .find(|file| file.source == crate::layout_paths::specs("rejected.xml"))
        .unwrap();
    assert_eq!(rejected.output, crate::layout_paths::specs("rejected.xml"));
    assert_eq!(rejected.disposition, DerivedFileDisposition::Copied);
}

#[test]
fn redbook_materialises_fully_xml_with_md_readmes_converted() {
    // The live redbook's XML sources copy through; Markdown READMEs convert.
    let ws = TempDir::new().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(vibe_core::layout::current_packages_root())
        .join("org.vibevm.world/redbook/v1.0.0");
    materialise_with_spec_format(
        ws.path(),
        &g("org.vibevm.world"),
        "redbook",
        &version("1.0.0"),
        &source,
        CopyMode::Copy,
        SpecFormat::Xml,
        "sha256:redbook-source",
    )
    .unwrap();

    let slot = slot_abs_path(
        ws.path(),
        &g("org.vibevm.world"),
        "redbook",
        &version("1.0.0"),
    );
    let manifest = read_derived_manifest(&slot).unwrap();
    let converted = manifest
        .files
        .iter()
        .filter(|file| file.disposition == DerivedFileDisposition::Converted)
        .count();
    let copied = manifest
        .files
        .iter()
        .filter(|file| file.disposition == DerivedFileDisposition::Copied)
        .count();
    assert_eq!(converted, 2);
    assert_eq!(copied, 8);
    assert!(slot.join("README.xml").is_file());
    assert!(
        slot.join(crate::layout_paths::boot_path("03-flow-redbook.xml"))
            .is_file()
    );
    assert!(
        slot.join(crate::layout_paths::specs_path(
            "book/ru/chapter-3-memory-individual.xml"
        ))
        .is_file()
    );
    assert!(slot.join("LICENSE.md").is_file());
    assert!(slot.join("vibe.toml").is_file());
    assert_eq!(manifest.derived_hash, compute_derived_hash(&slot).unwrap());
}

#[test]
fn generated_boot_artifacts_stay_outside_the_derived_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let slot = dir.path();
    std::fs::create_dir_all(slot.join(vibe_core::layout::current_boot_dir())).expect("mkdir");
    std::fs::write(
        slot.join(crate::layout_paths::specs_path("a.xml")),
        "<spec xmlns=\"https://vibevm.org/spec/1\"/>",
    )
    .unwrap();
    let before = derived::compute_derived_hash(slot).expect("hash");
    std::fs::write(
        slot.join(vibe_core::layout::current_boot_static_md()),
        "# generated
",
    )
    .expect("write");
    std::fs::write(
        slot.join(vibe_core::layout::current_boot_static_xml()),
        "generated XML\n",
    )
    .unwrap();
    std::fs::write(
        slot.join(vibe_core::layout::current_boot_index()),
        "schema = 1\n",
    )
    .unwrap();
    let after = derived::compute_derived_hash(slot).expect("hash");
    assert_eq!(before, after, "generated artifacts must not move the hash");
    assert!(derived::is_generated_boot_artifact(
        slot,
        &slot.join(vibe_core::layout::current_boot_static_md())
    ));
    assert!(derived::is_generated_boot_artifact(
        slot,
        &slot.join(vibe_core::layout::current_boot_static_xml())
    ));
    assert!(!derived::is_generated_boot_artifact(
        slot,
        &slot.join(crate::layout_paths::boot_path("03-flow.md"))
    ));
}

mod overlay;
