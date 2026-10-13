use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use vibe_core::{
    ContentHash,
    manifest::{
        Lockfile, SpecFormat,
        change::{ChangeOrigin, ChangeOriginDocument},
    },
};
use vibe_workspace::source_query::{RetainedOriginCapture, SourceQueryService, SourceStrength};
const ADDRESS: &str = "spec://org.demo/dep/DOC#DELIVERY";
const ORIGINAL: &str = "# Origin {#root}\n\n@fact:DELIVERY Original delivery. @status:impl/done\n";
fn hash(raw: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(raw))
}
struct Fixture {
    _root: tempfile::TempDir,
    host: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let host = root.path().join("host");
        let source = root.path().join("source");
        fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
        fs::create_dir_all(&host).unwrap();
        fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
        fs::write(source.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        let fixture = Self {
            _root: root,
            host,
            source,
        };
        fixture.install(ORIGINAL, 'a');
        fixture
    }
    fn install(&self, body: &str, letter: char) {
        self.install_format(body, letter, SpecFormat::Mixed);
    }
    fn install_format(&self, body: &str, letter: char, format: SpecFormat) {
        fs::write(self.source.join("vibevm/vibespecs/DOC.md"), body).unwrap();
        let group = "org.demo".parse().unwrap();
        let version = "1.0.0".parse().unwrap();
        let selected =
            ContentHash::parse(&format!("sha256:{}", letter.to_string().repeat(64))).unwrap();
        let slot = vibe_workspace::vibedeps::slot_abs_path(&self.host, &group, "dep", &version);
        if slot.exists() {
            fs::remove_dir_all(&slot).unwrap();
        }
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &self.host,
            &group,
            "dep",
            &version,
            &self.source,
            vibe_workspace::vibedeps::CopyMode::Copy,
            format,
            &selected,
        )
        .unwrap();
        let mut lock = Lockfile::empty("fixture", "fixture");
        lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{selected}'\n")).unwrap());
        lock.write(self.host.join("vibe.lock")).unwrap();
    }
    fn retained(&self, addresses: &[&str]) -> Arc<RetainedOriginCapture> {
        let service = SourceQueryService::prepare_host(&self.host).unwrap();
        Arc::new(
            service
                .retain_origin_capture(
                    "org.demo",
                    "dep",
                    &addresses.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                )
                .unwrap(),
        )
    }
    fn declaration(&self) -> ChangeOrigin {
        // Expected bytes are read independently from authored fixture leaves,
        // not copied from the carrier's own advertised hashes.
        ChangeOrigin {
            package: "flow:org.demo/dep@=1.0.0".into(),
            selected_content_hash: Some(format!("sha256:{}", "a".repeat(64))),
            manifest_sha256: hash(&fs::read(self.source.join("vibe.toml")).unwrap()),
            documents: vec![ChangeOriginDocument {
                address: ADDRESS.into(),
                sha256: hash(ORIGINAL.as_bytes()),
            }],
        }
    }
}
fn inventory(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut pending = vec![root.to_path_buf()];
    let mut rows = vec![];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                pending.push(p.clone());
                rows.push((p.strip_prefix(root).unwrap().into(), None));
            } else {
                rows.push((
                    p.strip_prefix(root).unwrap().into(),
                    Some(fs::read(p).unwrap()),
                ));
            }
        }
    }
    rows.sort();
    rows
}
fn preserved(before: &Vec<(PathBuf, Option<Vec<u8>>)>, root: &Path, label: &str) {
    let after = inventory(root);
    assert_eq!(before, &after);
    println!(
        "C1_SNAPSHOT_PAIR {}",
        serde_json::json!({"label":label,"before":before,"after":after})
    );
}
#[test]
fn retained_origin_survives_reinstall() {
    let fixture = Fixture::new();
    let initial = inventory(fixture._root.path());
    let retained = fixture.retained(&[ADDRESS]);
    let old_service = SourceQueryService::prepare_host(&fixture.host).unwrap();
    let accepted = accept_origin(&fixture.declaration(), Some(retained.clone())).unwrap();
    preserved(&initial, fixture._root.path(), "initial-retention");
    assert_eq!(accepted.identity().package, "flow:org.demo/dep@=1.0.0");
    assert_eq!(
        accepted.identity().facts[ADDRESS].strength,
        SourceStrength::RecordConsistent
    );
    let old_snapshot = retained.documents()["DOC"].snapshot().clone();
    fixture.install(ORIGINAL, 'a');
    let before = inventory(fixture._root.path());
    let current = fixture.retained(&[ADDRESS]);
    assert!(matches!(
        compare_origin(&accepted, Some(&current)),
        OriginComparison::Unchanged { .. }
    ));
    assert_eq!(retained.documents()["DOC"].snapshot(), &old_snapshot);
    preserved(&before, fixture._root.path(), "reinstall-retention");
    fixture.install(
        "# Origin {#root}\n\n@fact:DELIVERY Changed same-version delivery. @status:impl/done\n",
        'b',
    );
    let after_setup = inventory(fixture._root.path());
    let changed = fixture.retained(&[ADDRESS]);
    assert!(matches!(
        old_service.retain_origin_capture("org.demo", "dep", &[ADDRESS.into()]),
        Err(vibe_workspace::source_query::SourceQueryUnavailable::StaleWorld("selected lock"))
    ));
    let OriginComparison::OriginDrift {
        accepted: old,
        current: new,
        affected,
    } = compare_origin(&accepted, Some(&changed))
    else {
        panic!("expected same-version origin-drift")
    };
    assert_eq!(old.package, new.package);
    assert_ne!(old.selected_content_hash, new.selected_content_hash);
    assert_eq!(
        affected,
        [OriginFactChange {
            address: ADDRESS.into(),
            kind: OriginChangeKind::Changed
        }]
    );
    assert_eq!(
        retained.documents()["DOC"].snapshot().bytes,
        ORIGINAL.as_bytes()
    );
    assert_eq!(retained.documents()["DOC"].snapshot(), &old_snapshot);
    preserved(&after_setup, fixture._root.path(), "drift-historical");
    assert!(!before.is_empty());
    println!("ORIGIN_REPORT old={old:?} current={new:?} affected={affected:?}");
}

#[test]
fn missing_retained_origin_never_uses_newest() {
    let fixture = Fixture::new();
    let declaration = fixture.declaration();
    let retained = fixture.retained(&[ADDRESS]);
    let accepted = accept_origin(&declaration, Some(retained)).unwrap();
    let newer = vibe_workspace::vibedeps::slot_abs_path(
        &fixture.host,
        &"org.demo".parse().unwrap(),
        "dep",
        &"9.0.0".parse().unwrap(),
    );
    fs::create_dir_all(newer.join("vibevm/vibespecs")).unwrap();
    fs::write(newer.join("vibe.toml"), "NEWEST_INVALID = [").unwrap();
    fs::write(newer.join("vibevm/vibespecs/DOC.md"), "NEWEST_POISON").unwrap();
    let before = inventory(fixture._root.path());
    let current = fixture.retained(&[ADDRESS]);
    assert_eq!(current.pin().to_string(), "flow:org.demo/dep@=1.0.0");
    assert_eq!(
        accept_origin(&declaration, None).unwrap_err(),
        OriginUnavailable::MissingRetained
    );
    assert!(matches!(
        compare_origin(&accepted, None),
        OriginComparison::Unavailable(OriginUnavailable::MissingCurrent)
    ));
    assert_eq!(before, inventory(fixture._root.path()));
}

#[test]
fn declaration_and_selected_fact_mismatches_refuse() {
    let fixture = Fixture::new();
    let retained = fixture.retained(&[ADDRESS]);
    let declaration = fixture.declaration();
    let before = inventory(fixture._root.path());
    let mut wrong = declaration.clone();
    wrong.package = "flow:org.demo/dep@=2.0.0".into();
    assert_eq!(
        accept_origin(&wrong, Some(retained.clone())).unwrap_err(),
        OriginUnavailable::Pin
    );
    wrong = declaration.clone();
    wrong.manifest_sha256 = format!("sha256:{}", "f".repeat(64));
    assert_eq!(
        accept_origin(&wrong, Some(retained.clone())).unwrap_err(),
        OriginUnavailable::Manifest
    );
    wrong = declaration.clone();
    wrong.documents[0].sha256 = format!("sha256:{}", "f".repeat(64));
    assert_eq!(
        accept_origin(&wrong, Some(retained.clone())).unwrap_err(),
        OriginUnavailable::Document(ADDRESS.into())
    );
    wrong = declaration.clone();
    wrong.documents.push(wrong.documents[0].clone());
    assert_eq!(
        accept_origin(&wrong, Some(retained.clone())).unwrap_err(),
        OriginUnavailable::Selection
    );
    let missing = "spec://org.demo/dep/DOC#MISSING";
    let held = fixture.retained(&[missing]);
    assert!(held.facts()[missing].canonical.is_none());
    wrong = declaration.clone();
    wrong.documents[0].address = missing.into();
    assert_eq!(
        accept_origin(&wrong, Some(held)).unwrap_err(),
        OriginUnavailable::Fact(missing.into())
    );
    assert_eq!(before, inventory(fixture._root.path()));
}

#[test]
fn copied_media_and_transformed_original_gap_are_explicit() {
    let fixture = Fixture::new();
    fixture.install_format(ORIGINAL, 'a', SpecFormat::Xml);
    let held = fixture.retained(&[ADDRESS]);
    let doc = &held.documents()["DOC"];
    assert_eq!(
        doc.provenance().materialized_media.as_deref(),
        Some("application/xml")
    );
    assert_eq!(doc.provenance().output_path, "vibevm/vibespecs/DOC.xml");
    assert_eq!(doc.provenance().source_path, "vibevm/vibespecs/DOC.md");
    assert!(held.facts()[ADDRESS].canonical.is_some());
    let mut claimed = fixture.declaration();
    claimed.documents[0].sha256 = format!("sha256:{}", doc.snapshot().sha256);
    assert_eq!(
        accept_origin(&claimed, Some(held)).unwrap_err(),
        OriginUnavailable::OriginalUnavailable(ADDRESS.into())
    );
    let path = fixture.source.join("vibe.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("media='text/markdown'", "media='application/xml'");
    fs::write(&path, text).unwrap();
    let xml = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><p><DELIVERY fact=\"true\" status=\"impl/done\">Original delivery.</DELIVERY></p></spec>";
    fixture.install(xml, 'c');
    let before = inventory(fixture._root.path());
    let held = fixture.retained(&[ADDRESS]);
    assert_eq!(
        held.documents()["DOC"].provenance().output_path,
        "vibevm/vibespecs/DOC.md"
    );
    assert_eq!(
        held.documents()["DOC"]
            .provenance()
            .materialized_media
            .as_deref(),
        Some("application/xml")
    );
    claimed = fixture.declaration();
    claimed.selected_content_hash = Some(format!("sha256:{}", "c".repeat(64)));
    claimed.documents[0].sha256 = hash(xml.as_bytes());
    let accepted = accept_origin(&claimed, Some(held)).unwrap();
    assert_eq!(
        accepted.capture().documents()["DOC"].snapshot().bytes,
        xml.as_bytes()
    );
    assert_eq!(
        accepted.identity().facts[ADDRESS].strength,
        SourceStrength::RecordConsistent
    );
    assert_eq!(before, inventory(fixture._root.path()));
}

#[test]
fn selected_content_identity_is_authored() {
    let fixture = Fixture::new();
    let declaration = fixture.declaration();
    let old = accept_origin(&declaration, Some(fixture.retained(&[ADDRESS]))).unwrap();
    fs::write(
        fixture.source.join("README.md"),
        "Changed non-document package content\n",
    )
    .unwrap();
    fixture.install(ORIGINAL, 'b');
    let before = inventory(fixture._root.path());
    let next = fixture.retained(&[ADDRESS]);
    let review = review_origin_rebase(&old, next).unwrap();
    assert_ne!(
        review.old.selected_content_hash,
        review.new.selected_content_hash
    );
    assert_eq!(review.old.manifest_sha256, review.new.manifest_sha256);
    assert!(review.affected.is_empty());
    assert_eq!(
        review.old.facts[ADDRESS].document_sha256,
        review.new.facts[ADDRESS].document_sha256
    );
    assert_eq!(before, inventory(fixture._root.path()));
    assert_ne!(
        review.next_origin().declaration(),
        old.declaration(),
        "accepted selected content is absent from authored origin identity"
    );
}

#[test]
fn unbound_mismatch_initial_binding_and_lost_carrier_are_distinct() {
    let fixture = Fixture::new();
    let held = fixture.retained(&[ADDRESS]);
    let declaration = fixture.declaration();
    let before = inventory(fixture._root.path());
    let mut legacy = declaration.clone();
    legacy.selected_content_hash = None;
    assert_eq!(
        accept_origin(&legacy, Some(held.clone())).unwrap_err(),
        OriginUnavailable::Unbound
    );
    let initial = prepare_initial_origin_binding(&legacy, Some(held.clone())).unwrap();
    assert_eq!(initial.previous(), &legacy);
    assert_eq!(initial.next_origin().declaration(), &declaration);
    let mut wrong = declaration.clone();
    wrong.selected_content_hash = Some(format!("sha256:{}", "f".repeat(64)));
    assert_eq!(
        accept_origin(&wrong, Some(held.clone())).unwrap_err(),
        OriginUnavailable::ContentMismatch
    );
    assert_eq!(
        prepare_initial_origin_binding(&declaration, Some(held)).unwrap_err(),
        OriginUnavailable::AlreadyBound
    );
    assert_eq!(
        accept_origin(initial.next_origin().declaration(), None).unwrap_err(),
        OriginUnavailable::MissingRetained
    );
    preserved(
        &before,
        fixture._root.path(),
        "initial-unbound-lost-retention",
    );
}
