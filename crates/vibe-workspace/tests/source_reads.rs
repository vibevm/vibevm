//! Supported materialization is read through the actual spec consumer.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use vibe_core::Group;
use vibe_spec::{
    FileResolver, FsSectionSource, SectionSource, SelectedPackage, SelfCoordinate, SpecAddress,
};
use vibe_workspace::vibedeps;

#[cfg(test)]
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[cfg(test)]
fn names(root: &Path) -> Vec<std::path::PathBuf> {
    fn visit(root: &Path, current: &Path, paths: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            paths.push(path.strip_prefix(root).unwrap().to_path_buf());
            if path.is_dir() {
                visit(root, &path, paths);
            }
        }
    }
    let mut paths = vec![];
    visit(root, root, &mut paths);
    paths.sort();
    paths
}

#[cfg(unix)]
#[test]
fn actual_hardlink_materialization_reads_md_and_xml_without_cow() {
    use std::os::unix::fs::MetadataExt;
    for extension in ["md", "xml"] {
        let workspace = tempfile::tempdir().unwrap();
        let original = tempfile::tempdir().unwrap();
        write(
            &original.path().join("vibe.toml"),
            "[package]\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nkind='flow'\nmaterialization='hardlink'\n",
        );
        let document = format!("spec/DOC.{extension}");
        let body = if extension == "xml" {
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">SHARED_SOURCE</title></spec>"
        } else {
            "# Shared {#root}\nSHARED_SOURCE\n"
        };
        write(&original.path().join(&document), body);
        let group = Group::parse("org.demo").unwrap();
        let version = semver::Version::parse("1.0.0").unwrap();
        let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
        vibedeps::materialise_with(
            workspace.path(),
            &group,
            "dep",
            &version,
            original.path(),
            vibedeps::CopyMode::Hardlink,
            &hash,
        )
        .unwrap();
        let slot = vibedeps::slot_abs_path(workspace.path(), &group, "dep", &version);
        let original_meta = fs::metadata(original.path().join(&document)).unwrap();
        let slot_meta = fs::metadata(slot.join(&document)).unwrap();
        assert_eq!(
            (original_meta.dev(), original_meta.ino()),
            (slot_meta.dev(), slot_meta.ino()),
            "copy fallback is not hardlink evidence"
        );
        assert!(slot_meta.nlink() >= 2);
        let before = names(workspace.path());
        let record = fs::read(slot.join(".vibe-slot.toml")).unwrap();
        let source = FsSectionSource::new(FileResolver::new(
            workspace.path(),
            SelfCoordinate::new(None, "host".into()),
        ));
        let address = SpecAddress::parse("spec://org.demo/dep@1.0.0/DOC#root").unwrap();
        assert!(
            source
                .section_text(&address)
                .unwrap()
                .contains("SHARED_SOURCE")
        );
        assert_eq!(names(workspace.path()), before);
        assert_eq!(fs::read(slot.join(".vibe-slot.toml")).unwrap(), record);
        assert_eq!(
            fs::read_to_string(original.path().join(&document)).unwrap(),
            body
        );
        let after = fs::metadata(slot.join(&document)).unwrap();
        assert_eq!(
            (after.dev(), after.ino(), after.nlink()),
            (slot_meta.dev(), slot_meta.ino(), slot_meta.nlink())
        );
    }
}

#[test]
fn selected_unversioned_in_place_checkout_is_its_own_read_authority() {
    let workspace = tempfile::tempdir().unwrap();
    let original = tempfile::tempdir().unwrap();
    fs::create_dir(original.path().join(".git")).unwrap();
    write(
        &original.path().join("spec/DOC.md"),
        "# In place {#root}\nIN_PLACE\n",
    );
    let group = Group::parse("org.demo").unwrap();
    vibedeps::materialise_in_place(workspace.path(), &group, "dep", original.path()).unwrap();
    let slot = vibedeps::in_place_slot_abs_path(workspace.path(), &group, "dep");
    let selected = BTreeMap::from([(
        ("org.demo".into(), "dep".into()),
        SelectedPackage::new("1.0.0", &slot),
    )]);
    let source = FsSectionSource::new(FileResolver::with_selected_world(
        workspace.path(),
        workspace.path(),
        selected,
        SelfCoordinate::new(None, "host".into()),
    ));
    let before = names(workspace.path());
    let address = SpecAddress::parse("spec://org.demo/dep@1.0.0/DOC#root").unwrap();
    assert!(source.section_text(&address).unwrap().contains("IN_PLACE"));
    assert_eq!(names(workspace.path()), before);
}
