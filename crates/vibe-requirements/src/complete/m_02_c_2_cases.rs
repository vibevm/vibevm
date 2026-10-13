use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
use vibe_core::{
    ContentHash,
    manifest::{Lockfile, SpecFormat},
};
use vibe_workspace::source_query::SourceQueryService;
struct Fixture {
    root: tempfile::TempDir,
    host: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let host = root.path().join("host");
        let source = root.path().join("source");
        fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
        fs::create_dir(&host).unwrap();
        fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/large'='=1.0.0'\n").unwrap();
        fs::write(source.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='large'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
        fs::write(source.join("vibevm/vibespecs/DOC.md"), body).unwrap();
        let result = Self { root, host, source };
        result.install('a');
        result
    }
    fn install(&self, letter: char) {
        self.install_format(letter, SpecFormat::Mixed);
    }
    fn install_format(&self, letter: char, format: SpecFormat) {
        let group = "org.demo".parse().unwrap();
        let version = "1.0.0".parse().unwrap();
        let content =
            ContentHash::parse(&format!("sha256:{}", letter.to_string().repeat(64))).unwrap();
        let slot = vibe_workspace::vibedeps::slot_abs_path(&self.host, &group, "large", &version);
        if slot.exists() {
            fs::remove_dir_all(&slot).unwrap();
        }
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &self.host,
            &group,
            "large",
            &version,
            &self.source,
            vibe_workspace::vibedeps::CopyMode::Copy,
            format,
            &content,
        )
        .unwrap();
        let mut lock = Lockfile::empty("fixture", "fixture");
        lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='large'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{content}'\n")).unwrap());
        lock.write(self.host.join("vibe.lock")).unwrap();
    }
    fn observe(&self) -> CapturedFactSet {
        let inventory = SourceQueryService::prepare_host(&self.host)
            .unwrap()
            .retain_source_inventory("org.demo", "large")
            .unwrap();
        CapturedFactSet::observe(Arc::new(inventory)).unwrap()
    }
}
fn inventory(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut result = vec![];
    let mut pending = vec![root.to_path_buf()];
    while let Some(p) = pending.pop() {
        for e in fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                pending.push(p.clone());
                result.push((p.strip_prefix(root).unwrap().into(), None));
            } else {
                result.push((
                    p.strip_prefix(root).unwrap().into(),
                    Some(fs::read(p).unwrap()),
                ));
            }
        }
    }
    result.sort();
    result
}
fn large_body() -> String {
    let mut text = "# Native scope {#root}\n\n".to_string();
    for i in 0..320 {
        text.push_str(&format!(
            "@fact:F{i:03} Requirement {i}. @requires:specification @status:spec/done\n\n"
        ));
    }
    text
}
fn preserved(before: &Vec<(PathBuf, Option<Vec<u8>>)>, root: &Path, label: &str) {
    let after = inventory(root);
    assert_eq!(before, &after);
    println!(
        "C2_SNAPSHOT_PAIR {}",
        serde_json::json!({"label":label,"before":before,"after":after})
    );
}
fn addresses() -> Vec<String> {
    (0..320)
        .map(|i| format!("spec://org.demo/large/DOC#F{i:03}"))
        .collect()
}
fn change_for(source: &CapturedFactSet, address: &str) -> crate::change::ParsedCapturedChange {
    change_with_binding(source, address, true)
}
fn change_with_binding(
    source: &CapturedFactSet,
    address: &str,
    bound: bool,
) -> crate::change::ParsedCapturedChange {
    let node = vibe_core::manifest::Manifest::parse_str(
        "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    let context =
        vibe_core::manifest::change::ChangeContext::for_node(&node, ".", Some(Path::new("spec")))
            .unwrap();
    let id = vibe_core::manifest::change::ChangeId::parse("chg-0123456789abcdef0123456789abcdef")
        .unwrap();
    let dir = context.directory(&id);
    let digest = &source.documents()["vibevm/vibespecs/DOC.md"]
        .witness
        .as_ref()
        .unwrap()
        .digest;
    let text = format!(
        "schema=1\n[change]\nid='{}'\nselected_node='.'\nslug='scope'\ntitle='Selected'\n[[origins]]\npackage='{}'\nselected_content_hash='{}'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='{address}'\nsha256='{digest}'\n[[documents]]\nrole='intent'\npath='intent.md'\nnormative=true\ninput=true\n[[documents]]\nrole='acceptance'\npath='acceptance.md'\nnormative=true\ninput=true\n",
        id.as_str(),
        source.inventory().pin(),
        source.inventory().content_hash(),
        source.inventory().manifest().sha256
    );
    let text = if bound {
        text
    } else {
        text.replacen(
            &format!(
                "selected_content_hash='{}'\n",
                source.inventory().content_hash()
            ),
            "",
            1,
        )
    };
    let intent = dir.join("intent.md");
    let acceptance = dir.join("acceptance.md");
    crate::change::analyze_change_bytes(
        &text,
        &context,
        &dir,
        &[
            (intent.as_path(), b"Local intent"),
            (acceptance.as_path(), b"Local acceptance"),
        ],
    )
    .unwrap()
}
#[test]
fn connected_large_capture_exceeds_query_page_without_truncating_change_scope() {
    let fixture = Fixture::new(&large_body());
    let before = inventory(fixture.root.path());
    let captured = fixture.observe();
    let expected = addresses();
    assert_eq!(
        captured.facts().keys().cloned().collect::<Vec<_>>(),
        expected
    );
    let selection = captured.select_all();
    assert!(selection.is_complete());
    assert_eq!(selection.observed().len(), 320);
    for fact in selection.observed() {
        assert_eq!(fact.capture_id, captured.capture_id());
        assert_eq!(fact.reader, "vibe-specdoc:markdown/spec");
        assert_eq!(
            fact.authored
                .requirements
                .as_ref()
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            [progress_core::model::ArtifactKind::Specification]
        );
    }
    let pages = (0..320)
        .step_by(64)
        .map(|start| selection.page(start, 64).unwrap())
        .collect::<Vec<_>>();
    let assembled = selection.assemble(&pages).unwrap();
    assert!(assembled.is_complete());
    assert_eq!(
        assembled
            .rows()
            .iter()
            .map(|f| f.authored.address.clone())
            .collect::<Vec<_>>(),
        expected
    );
    let query = crate::RequirementsQuery::try_new(None, 256, false).unwrap();
    let report = crate::query(
        &query,
        &crate::QueryContext {
            selected_root: fixture.host.clone(),
            observed_at: "2026-10-13T00:00:00Z".parse().unwrap(),
            lifecycle_run_id: None,
        },
        None,
    )
    .unwrap();
    assert!(report.truncated);
    assert_eq!(report.rows.len(), 256);
    assert_eq!(selection.observed().len(), 320);
    let node = vibe_core::manifest::Manifest::parse_str(
        "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    let context =
        vibe_core::manifest::change::ChangeContext::for_node(&node, ".", Some(Path::new("spec")))
            .unwrap();
    let id = vibe_core::manifest::change::ChangeId::parse("chg-0123456789abcdef0123456789abcdef")
        .unwrap();
    let dir = context.directory(&id);
    let mut text = format!(
        "schema=1\n[change]\nid='{}'\nselected_node='.'\nslug='scope'\ntitle='Complete scope'\n[[origins]]\npackage='{}'\nselected_content_hash='{}'\nmanifest_sha256='sha256:{}'\n",
        id.as_str(),
        captured.inventory().pin(),
        captured.inventory().content_hash(),
        captured.inventory().manifest().sha256
    );
    let raw_digest = vibe_facts::SourceFileWitness::of("DOC", large_body().as_bytes()).digest;
    for address in &expected {
        text.push_str(&format!(
            "[[origins.documents]]\naddress='{address}'\nsha256='{raw_digest}'\n"
        ));
    }
    for (role, path) in [("intent", "intent.md"), ("acceptance", "acceptance.md")] {
        text.push_str(&format!(
            "[[documents]]\nrole='{role}'\npath='{path}'\nnormative=true\ninput=true\n"
        ));
    }
    let intent = dir.join("intent.md");
    let acceptance = dir.join("acceptance.md");
    let change = crate::change::analyze_change_bytes(
        &text,
        &context,
        &dir,
        &[
            (intent.as_path(), b"Local intent"),
            (acceptance.as_path(), b"Local acceptance"),
        ],
    )
    .unwrap();
    let selected = change.select_origin_facts(0, &captured).unwrap();
    assert!(selected.is_complete());
    assert_eq!(selected.observed().len(), 320);
    preserved(
        &before,
        fixture.root.path(),
        "large-complete-and-neutral-query",
    );
    println!(
        "C2_LARGE rows={} public_rows={} capture={} filter={}",
        selected.observed().len(),
        report.rows.len(),
        captured.capture_id(),
        selected.filter_id()
    );
}

#[test]
fn truncated_mixed_filter_and_changed_membership_pages_refuse() {
    let fixture = Fixture::new(&large_body());
    let old = fixture.observe();
    let selected = old.select_all();
    let partial = selected
        .assemble(&[selected.page(0, 256).unwrap()])
        .unwrap();
    assert!(!partial.is_complete());
    assert_eq!(partial.gaps().len(), 64);
    assert!(
        partial
            .gaps()
            .iter()
            .all(|g| g.kind == SelectionGapKind::Truncated)
    );
    assert_eq!(
        partial
            .gaps()
            .iter()
            .map(|g| g.address.clone())
            .collect::<Vec<_>>(),
        addresses()[256..].to_vec()
    );
    let exact = old.select_exact(&addresses()[..1]);
    assert!(exact.is_complete());
    assert_eq!(exact.omitted_addresses().len(), 319);
    assert_eq!(
        selected.assemble(&[exact.page(0, 1).unwrap()]).unwrap_err(),
        PageError::MixedFilter
    );
    assert_eq!(
        selected
            .assemble(&[selected.page(0, 64).unwrap(), selected.page(0, 64).unwrap()])
            .unwrap_err(),
        PageError::DuplicateRow
    );
    let manifest = fs::read_to_string(fixture.source.join("vibe.toml")).unwrap()
        + "\n[[package.sources]]\nfile='vibevm/vibespecs/MORE.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n";
    fs::write(fixture.source.join("vibe.toml"), manifest).unwrap();
    fs::write(fixture.source.join("vibevm/vibespecs/MORE.md"),"# More\n\n@fact:EXTRA Explicit new membership. @requires:specification @status:spec/done\n").unwrap();
    fixture.install('b');
    let before = inventory(fixture.root.path());
    let new = fixture.observe();
    let new_selection = new.select_all();
    assert_ne!(old.capture_id(), new.capture_id());
    assert_eq!(new.facts().len(), 321);
    assert_eq!(old.facts().len(), 320);
    assert_eq!(
        selected
            .assemble(&[
                selected.page(0, 256).unwrap(),
                new_selection.page(256, 65).unwrap()
            ])
            .unwrap_err(),
        PageError::MixedCapture
    );
    assert!(selected.is_complete()); // explicitly historical, never current-new membership
    preserved(&before, fixture.root.path(), "mixed-pages-new-membership");
}

#[test]
fn exact_gaps_and_inert_resources_are_explicit() {
    let fixture =
        Fixture::new("# D\n\n@fact:ONE Selected. @requires:specification @status:spec/done\n");
    let mut manifest = fs::read_to_string(fixture.source.join("vibe.toml")).unwrap();
    for (file, media, interpretation, role, body) in [
        (
            "ORDINARY.md",
            "text/markdown",
            "commonmark",
            "documentation",
            Some("# Plain\n\n@fact:FALSE literal annotation. @status:impl/done\n"),
        ),
        (
            "RAW.xml",
            "application/xml",
            "xml-data",
            "resource",
            Some("<invalid-xml @fact:FALSE"),
        ),
        (
            "BROKEN.md",
            "text/markdown",
            "vibe-spec",
            "spec",
            Some("# D\n\nPlain.\n\n@status:impl/done\n\nStranded.\n"),
        ),
    ] {
        manifest.push_str(&format!("\n[[package.sources]]\nfile='vibevm/vibespecs/{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"));
        if let Some(body) = body {
            fs::write(fixture.source.join("vibevm/vibespecs").join(file), body).unwrap();
        }
    }
    fs::write(fixture.source.join("vibe.toml"), manifest).unwrap();
    fs::write(
        fixture.source.join("vibevm/vibespecs/UNCLASSIFIED.md"),
        "@fact:FALSE Unclassified must not mint. @status:impl/done\n",
    )
    .unwrap();
    fixture.install('b');
    let before = inventory(fixture.root.path());
    let captured = fixture.observe();
    assert_eq!(
        captured.facts().keys().cloned().collect::<Vec<_>>(),
        ["spec://org.demo/large/DOC#ONE"]
    );
    assert_eq!(
        captured.documents()["vibevm/vibespecs/ORDINARY.md"].state,
        DocumentState::Inert
    );
    assert_eq!(
        captured.documents()["vibevm/vibespecs/RAW.xml"].state,
        DocumentState::Inert
    );
    assert_eq!(
        captured.documents()["vibevm/vibespecs/BROKEN.md"].state,
        DocumentState::Invalid
    );
    assert_eq!(
        captured.documents()["vibevm/vibespecs/UNCLASSIFIED.md"].state,
        DocumentState::Unclassified
    );
    let requests = [
        ("DOC#ABSENT", SelectionGapKind::Missing),
        ("BROKEN#X", SelectionGapKind::Unavailable),
        ("UNCLASSIFIED#FALSE", SelectionGapKind::Unclassified),
        ("ORDINARY#FALSE", SelectionGapKind::OutOfScope),
        ("RAW#FALSE", SelectionGapKind::OutOfScope),
    ];
    for (suffix, kind) in requests {
        let address = format!("spec://org.demo/large/{suffix}");
        let selected = captured.select_exact(&[address.clone()]);
        assert!(!selected.is_complete());
        assert_eq!(selected.gaps(), [SelectionGap { address, kind }]);
    }
    assert!(!captured.select_all().is_complete());
    let all = captured.select_all();
    let exact = captured.select_exact(&["spec://org.demo/large/DOC#ONE".into()]);
    assert_ne!(
        all.filter_id(),
        exact.filter_id(),
        "whole-source scope and explicit selected scope must not share page filter identity"
    );
    assert_eq!(
        exact.assemble(&[all.page(0, 1).unwrap()]).unwrap_err(),
        PageError::MixedFilter
    );
    preserved(
        &before,
        fixture.root.path(),
        "precise-gaps-inert-unclassified",
    );
}

#[test]
fn missing_live_capture_refuses_without_recreating_slot_bytes() {
    let fixture = Fixture::new(&large_body());
    let old = fixture.observe();
    let slot = vibe_workspace::vibedeps::slot_abs_path(
        &fixture.host,
        &"org.demo".parse().unwrap(),
        "large",
        &"1.0.0".parse().unwrap(),
    );
    fs::remove_file(slot.join("vibevm/vibespecs/DOC.md")).unwrap();
    let before = inventory(fixture.root.path());
    let error = SourceQueryService::prepare_host(&fixture.host).unwrap_err();
    assert_eq!(
        error.code,
        vibe_wire::generated::source_query::e1::error::ErrorCode::UnavailableWorld
    );
    assert_eq!(error.boundary, "selected descriptor/output map");
    assert_eq!(old.facts().len(), 320);
    assert!(old.select_all().is_complete());
    preserved(&before, fixture.root.path(), "live-missing-no-recreation");
}

#[test]
fn directory_census_and_converted_original_remain_unavailable() {
    let fixture =
        Fixture::new("# D\n\n@fact:ONE Original. @requires:specification @status:spec/done\n");
    let manifest = fs::read_to_string(fixture.source.join("vibe.toml")).unwrap()
        + "\n[[package.sources]]\ndirectory='other'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n";
    fs::write(fixture.source.join("vibe.toml"), manifest).unwrap();
    fixture.install('b');
    let captured = fixture.observe();
    assert_eq!(
        captured.capture_gaps(),
        [CaptureGap::DirectoryCensus("other".into())]
    );
    assert!(!captured.select_all().is_complete());
    fixture.install_format('c', SpecFormat::Xml);
    let before = inventory(fixture.root.path());
    let converted = fixture.observe();
    assert!(converted.facts().is_empty());
    let doc = &converted.documents()["vibevm/vibespecs/DOC.md"];
    assert_eq!(doc.state, DocumentState::Unavailable);
    assert!(doc.original_unavailable);
    assert!(!converted.select_all().is_complete());
    preserved(&before, fixture.root.path(), "converted-original-gap");
}

#[test]
fn change_consumer_preserves_gaps_and_ignores_same_stem_inert_hash() {
    let fixture =
        Fixture::new("# D\n\n@fact:ONE Selected. @requires:specification @status:spec/done\n");
    let manifest = fs::read_to_string(fixture.source.join("vibe.toml")).unwrap()
        + "\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n";
    fs::write(fixture.source.join("vibe.toml"), manifest).unwrap();
    fs::write(
        fixture.source.join("vibevm/vibespecs/DOC.xml"),
        "<not-vibe @fact:ONE",
    )
    .unwrap();
    fixture.install('b');
    let before = inventory(fixture.root.path());
    let source = fixture.observe();
    let address = "spec://org.demo/large/DOC#ONE";
    let change = change_for(&source, address);
    let scope = change.select_origin_facts(0, &source).unwrap();
    assert!(scope.is_complete());
    assert_eq!(scope.observed().len(), 1);
    let missing = change_for(&source, "spec://org.demo/large/DOC#ABSENT");
    let result = missing.select_origin_facts(0, &source).unwrap();
    assert!(!result.is_complete());
    assert_eq!(result.gaps()[0].kind, SelectionGapKind::Missing);
    let unbound = change_with_binding(&source, address, false);
    assert_eq!(
        unbound.select_origin_facts(0, &source).unwrap_err(),
        crate::change::ChangeFactSelectionError::Unbound
    );
    let original = source.inventory().content_hash().to_string();
    fixture.install('c');
    let new = fixture.observe();
    assert_ne!(new.inventory().content_hash().as_str(), original);
    assert_eq!(
        change.select_origin_facts(0, &new).unwrap_err(),
        crate::change::ChangeFactSelectionError::Content
    );
    assert!(
        change
            .select_origin_facts(0, &source)
            .unwrap()
            .is_complete()
    );
    // Deliberate reinstall above is setup, not a pure-query state change.
    let after_setup = inventory(fixture.root.path());
    assert!(!before.is_empty());
    let body = change_for(&new, address);
    assert!(body.select_origin_facts(0, &new).unwrap().is_complete());
    preserved(
        &after_setup,
        fixture.root.path(),
        "same-stem-change-binding",
    );
}
