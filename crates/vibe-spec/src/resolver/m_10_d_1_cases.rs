use super::*;
use crate::SpecAddress;
use std::fs;

#[test]
fn repeated_reads_share_payload_while_local_and_selected_upstream_stay_distinct() {
    let root = tempfile::tempdir().unwrap();
    let local = root.path().join("local");
    let upstream = root.path().join("upstream");
    let original = include_bytes!(
        "../../../../fixtures/next/derivatives/vibe-spec/local/vibevm/vibespecs/common/POLICY.xml"
    );
    for directory in [&local, &upstream] {
        fs::create_dir_all(directory.join("spec/common")).unwrap();
        fs::write(directory.join("spec/common/POLICY.xml"), original).unwrap();
    }
    let resolver = FileResolver::with_selected_world(
        &local,
        root.path(),
        BTreeMap::from([(
            ("org.example".into(), "upstream".into()),
            SelectedPackage::new("1.0.0", &upstream),
        )]),
        SelfCoordinate::new(Some("org.example".into()), "local".into()),
    );
    let own = SpecAddress::parse("spec://org.example/local/common/POLICY#RETRY").unwrap();
    let base = SpecAddress::parse("spec://org.example/upstream@1.0.0/common/POLICY#RETRY").unwrap();
    let read = |address: &SpecAddress| {
        let path = resolver.resolve_file(address).unwrap();
        resolver.read_observed_source(address, &path).unwrap()
    };
    let retained = read(&own);
    assert_eq!(retained.sha256, read(&own).sha256);
    assert_eq!(retained.sha256, read(&base).sha256);
    assert_ne!(
        resolver.resolve_file(&own).unwrap(),
        resolver.resolve_file(&base).unwrap()
    );
    let changed = String::from_utf8(original.to_vec())
        .unwrap()
        .replace("Retry at most once", "Retry twice");
    assert_ne!(changed.as_bytes(), original);
    fs::write(local.join("spec/common/POLICY.xml"), changed.as_bytes()).unwrap();
    assert_ne!(read(&own).sha256, retained.sha256);
    assert_eq!(read(&base).bytes, retained.bytes);
    assert_eq!(retained.bytes, original);
}

#[test]
fn competing_representations_and_wrong_base_selection_refuse() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    fs::write(root.path().join("spec/common/POLICY.xml"), b"<spec/>").unwrap();
    fs::write(root.path().join("spec/common/POLICY.md"), b"# Policy").unwrap();
    let resolver = FileResolver::with_selected_world(
        root.path(),
        root.path(),
        BTreeMap::from([(
            ("org.example".into(), "upstream".into()),
            SelectedPackage::new("1.0.0", root.path()),
        )]),
        SelfCoordinate::new(Some("org.example".into()), "local".into()),
    );
    assert!(matches!(
        resolver
            .resolve_file(&SpecAddress::parse("spec://org.example/local/common/POLICY").unwrap()),
        Err(ResolveError::PairCollision { .. })
    ));
    assert!(matches!(
        resolver.resolve_file(
            &SpecAddress::parse("spec://org.example/upstream@2.0.0/common/POLICY").unwrap()
        ),
        Err(ResolveError::SelectedVersionMismatch { .. })
    ));
}
