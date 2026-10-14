use super::*;
use crate::SpecAddress;
use std::fs;

fn fixture(side: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/next/derivatives/vibe-spec")
        .join(side)
}

#[test]
fn local_and_upstream_addresses_keep_independent_raw_and_projection_bases() {
    let resolver = FileResolver::with_selected_world(
        fixture("local"),
        fixture("base"),
        BTreeMap::from([(
            ("org.example".into(), "upstream".into()),
            SelectedPackage::new("1.0.0", fixture("base")),
        )]),
        SelfCoordinate::new(Some("org.example".into()), "local".into()),
    );
    let mut hashes = Vec::new();
    for (owner, phrase) in [
        ("local", "Retry at most once"),
        ("upstream@1.0.0", "Retry at most three"),
    ] {
        let address =
            SpecAddress::parse(&format!("spec://org.example/{owner}/common/POLICY#RETRY")).unwrap();
        let path = resolver.resolve_file(&address).unwrap();
        let raw = resolver.read_observed_source(&address, &path).unwrap();
        let inspected = inspect_vibe_source(&path, &raw.bytes);
        assert_eq!(inspected.raw_sha256, format!("sha256:{}", raw.sha256));
        let (projected, digest) = inspected.projection.unwrap();
        assert!(projected.contains(phrase));
        assert!(projected.contains("DO_NOT_EXECUTE"));
        assert_ne!(digest, inspected.raw_sha256);
        let markdown = inspect_vibe_source(Path::new("POLICY.md"), projected.as_bytes());
        assert_eq!(markdown.projection.unwrap().0, projected);
        hashes.push(inspected.raw_sha256);
    }
    assert_ne!(hashes[0], hashes[1]);
}

#[test]
fn simultaneous_representations_refuse_and_bad_projection_retains_raw_digest() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    let xml = fs::read(fixture("local").join("vibevm/vibespecs/common/POLICY.xml")).unwrap();
    let md = inspect_vibe_source(Path::new("POLICY.xml"), &xml)
        .projection
        .unwrap()
        .0;
    fs::write(root.path().join("spec/common/POLICY.xml"), xml).unwrap();
    fs::write(root.path().join("spec/common/POLICY.md"), md).unwrap();
    let resolver = FileResolver::new(
        root.path(),
        SelfCoordinate::new(Some("org.example".into()), "local".into()),
    );
    let address = SpecAddress::parse("spec://org.example/local/common/POLICY").unwrap();
    assert!(matches!(
        resolver.resolve_file(&address),
        Err(ResolveError::PairCollision { .. })
    ));
    let failed = inspect_vibe_source(Path::new("POLICY.xml"), b"\xff\0");
    assert!(failed.projection.is_err());
    assert!(failed.raw_sha256.starts_with("sha256:"));
}
