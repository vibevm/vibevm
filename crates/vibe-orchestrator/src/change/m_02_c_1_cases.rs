use super::*;
use crate::document_analysis::{edit::*, *};
use sha2::{Digest, Sha256};
use std::{fs, sync::Arc, time::Duration};
use vibe_core::{
    ContentHash,
    manifest::{
        Lockfile, SpecFormat,
        change::{ChangeOrigin, ChangeOriginDocument},
    },
};
use vibe_requirements::change::{AcceptedOrigin, OriginChangeKind, accept_origin};
use vibe_workspace::source_query::{RetainedOriginCapture, SourceQueryService};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const ADDRESS: &str = "spec://org.demo/dep/DOC#DELIVERY";
const NEXT: &str = "spec://org.demo/dep/DOC#UPDATED";
const EXTRA: &str = "spec://org.demo/dep/DOC#EXTRA";
const OLD: &str = "# D {#root}\n\n@fact:DELIVERY Original delivery. @status:impl/done\n";
const NEW: &str = "# D {#root}\n\n@fact:UPDATED New explicit delivery. @status:impl/done\n\n@fact:EXTRA Additional explicitly selected fact. @status:impl/done\n";
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
struct Fixture {
    root: tempfile::TempDir,
    host: Arc<DocumentAnalysisHost>,
    old: AcceptedOrigin,
    next: Arc<RetainedOriginCapture>,
    text: String,
}
fn install(host: &Path, source: &Path, body: &str, letter: char) {
    fs::write(source.join("vibevm/vibespecs/DOC.md"), body).unwrap();
    let group = "org.demo".parse().unwrap();
    let version = "1.0.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", letter.to_string().repeat(64))).unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(host, &group, "dep", &version);
    if slot.exists() {
        fs::remove_dir_all(&slot).unwrap();
    }
    vibe_workspace::vibedeps::materialise_with_spec_format(
        host,
        &group,
        "dep",
        &version,
        source,
        vibe_workspace::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let mut lock = Lockfile::empty("fixture", "fixture");
    lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
    lock.write(host.join("vibe.lock")).unwrap();
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let host = root.path().join("host");
        fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
        fs::create_dir(&host).unwrap();
        fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
        let manifest = "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n";
        fs::write(source.join("vibe.toml"), manifest).unwrap();
        install(&host, &source, OLD, 'a');
        let retained = Arc::new(
            SourceQueryService::prepare_host(&host)
                .unwrap()
                .retain_origin_capture("org.demo", "dep", &[ADDRESS.into()])
                .unwrap(),
        );
        let declaration = ChangeOrigin {
            package: "flow:org.demo/dep@=1.0.0".into(),
            selected_content_hash: Some(format!("sha256:{}", "a".repeat(64))),
            manifest_sha256: hash(manifest.as_bytes()),
            documents: vec![ChangeOriginDocument {
                address: ADDRESS.into(),
                sha256: hash(OLD.as_bytes()),
            }],
        };
        let old = accept_origin(&declaration, Some(retained)).unwrap();
        install(&host, &source, NEW, 'b');
        let next = Arc::new(
            SourceQueryService::prepare_host(&host)
                .unwrap()
                .retain_origin_capture("org.demo", "dep", &[NEXT.into(), EXTRA.into()])
                .unwrap(),
        );
        let dir = host.join("docs/custom/changes").join(ID);
        fs::create_dir_all(&dir).unwrap();
        let text=format!("# Owner preamble\nschema=1\n[change]\nid='{ID}'\nselected_node='.'\nslug='cosmetic'\ntitle='Local intent'\n[[origins]]\npackage='{}' # exact selected pin\nmanifest_sha256='{}' # retain comment\n[[origins.documents]]\naddress='{ADDRESS}' # preserve row comment\nsha256='{}'\n[[documents]]\nrole='intent'\npath='intent.md'\nnormative=true\ninput=true\n[[documents]]\nrole='acceptance'\npath='acceptance.md'\nnormative=true\ninput=true\n[metadata]\nowner_value='untouched' # opaque user value\n",declaration.package,declaration.manifest_sha256,declaration.documents[0].sha256).replace('\n',"\r\n");
        let text = text.replacen(
            "manifest_sha256=",
            &format!(
                "selected_content_hash='{}'\r\nmanifest_sha256=",
                declaration.selected_content_hash.as_ref().unwrap()
            ),
            1,
        );
        assert_eq!(text.matches("selected_content_hash=").count(), 1);
        fs::write(dir.join("change.toml"), &text).unwrap();
        fs::write(dir.join("intent.md"), "Independent local intent\n").unwrap();
        fs::write(dir.join("acceptance.md"), "Independent local acceptance\n").unwrap();
        let mut declarations = vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "vibe.lock".into(),
                role: ResourceRole::Lockfile,
            },
        ];
        for name in ["change.toml", "intent.md", "acceptance.md"] {
            declarations.push(ResourceDeclaration {
                path: format!("docs/custom/changes/{ID}/{name}"),
                role: ResourceRole::Opaque,
            });
        }
        let held = DocumentAnalysisHost::new(
            &host,
            "vibe.toml",
            declarations,
            AnalysisLimits {
                resource_bytes: 1048576,
                capture_bytes: 8388608,
                open_buffers: 8,
            },
        )
        .unwrap();
        Self {
            root,
            host: held,
            old,
            next,
            text,
        }
    }
    fn resource(&self, name: &str) -> ResourceId {
        self.host
            .resource(&format!("docs/custom/changes/{ID}/{name}"))
            .unwrap()
    }
    fn view<'a>(&self, capture: &'a AnalysisCapture) -> CapturedChangeView<'a> {
        analyze_change(
            capture,
            self.host.resource("vibe.toml").unwrap(),
            Some(Path::new("docs/custom")),
            ID,
        )
    }
    fn authority(&self) -> Arc<DocumentEditAuthority> {
        DocumentEditAuthority::new(self.host.clone(), vec![self.resource("change.toml")]).unwrap()
    }
}
fn inventory(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut rows = vec![];
    let mut pending = vec![root.to_path_buf()];
    while let Some(p) = pending.pop() {
        for entry in fs::read_dir(p).unwrap() {
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
#[test]
fn saved_rebase_names_old_new_and_prepares_existing_barrier() {
    let fixture = Fixture::new();
    let client = fixture.host.client().unwrap();
    let captured = client.freeze().unwrap();
    let view = fixture.view(&captured);
    let authority = fixture.authority();
    let before = inventory(fixture.root.path());
    let prepared =
        prepare_origin_rebase(&view, &fixture.old, fixture.next.clone(), &authority).unwrap();
    assert_eq!(prepared.proposal().before, fixture.text);
    assert_ne!(
        prepared.new_intent(),
        &view.analysis().unwrap().identities.intent
    );
    assert_eq!(
        prepared
            .review()
            .old
            .facts
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        [ADDRESS]
    );
    assert_eq!(
        prepared
            .review()
            .new
            .facts
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        [EXTRA, NEXT]
    );
    assert_eq!(
        prepared
            .review()
            .affected
            .iter()
            .map(|d| (&d.address, &d.kind))
            .collect::<Vec<_>>(),
        [
            (&ADDRESS.to_string(), &OriginChangeKind::Removed),
            (&EXTRA.to_string(), &OriginChangeKind::Added),
            (&NEXT.to_string(), &OriginChangeKind::Added)
        ]
    );
    let proposal = prepared.proposal();
    let mut cursor = 0;
    for edit in &proposal.edits {
        let prefix = &proposal.before[cursor..edit.before_span.start];
        assert!(proposal.after.contains(prefix));
        cursor = edit.before_span.end;
    }
    assert!(proposal.after.ends_with(&proposal.before[cursor..]));
    assert!(proposal.after.contains("# preserve row comment\r\n"));
    assert!(
        proposal
            .after
            .contains("owner_value='untouched' # opaque user value\r\n")
    );
    assert_eq!(
        proposal.checked.origins()[0],
        *prepared.review().next_origin().declaration()
    );
    let node_manifest = captured
        .manifest(fixture.host.resource("vibe.toml").unwrap())
        .unwrap();
    let context =
        ChangeContext::for_node(&node_manifest, ".", Some(Path::new("docs/custom"))).unwrap();
    let current = prepared.review().next_origin().declaration();
    let mut smaller = current.clone();
    smaller.documents.retain(|d| d.address == NEXT);
    let with_comment = proposal.after.replace(
        &format!("address=\"{NEXT}\"\r\n"),
        &format!("address=\"{NEXT}\" # removed-row comment survives\r\n"),
    );
    let shrunk = vibe_core::manifest::change::plan_origin_rebase(
        &with_comment,
        &context,
        view.analysis().unwrap().manifest.directory(),
        current,
        &smaller,
    )
    .unwrap();
    assert_eq!(shrunk.checked.origins()[0], smaller);
    assert!(shrunk.after.contains("# removed-row comment survives\r\n"));
    assert!(shrunk.after.contains("# preserve row comment\r\n"));
    assert!(
        shrunk
            .after
            .contains("owner_value='untouched' # opaque user value\r\n")
    );
    let mut wrong = fixture.old.declaration().clone();
    wrong.manifest_sha256 = format!("sha256:{}", "f".repeat(64));
    let error = vibe_core::manifest::change::plan_origin_rebase(
        &fixture.text,
        &context,
        view.analysis().unwrap().manifest.directory(),
        &wrong,
        current,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        vibe_core::manifest::change::ChangeErrorKind::Origin
    );
    assert_eq!(error.field, "rebase.origin");
    println!(
        "REBASE_REPORT old={:?} new={:?} affected={:?}",
        prepared.review().old,
        prepared.review().new,
        prepared.review().affected
    );
    let fresh = client.freeze().unwrap();
    let barrier = prepared.admit(&fresh, Duration::from_secs(30)).unwrap();
    client.acknowledge_edit(&barrier, true).unwrap();
    drop(barrier);
    assert_eq!(before, inventory(fixture.root.path()));
    assert_eq!(
        fixture.old.capture().documents()["DOC"].snapshot().bytes,
        OLD.as_bytes()
    );
}
#[test]
fn dirty_or_stale_rebase_refuses() {
    for mutation in ["dirty-saved", "buffer-version", "disk"] {
        let fixture = Fixture::new();
        let client = fixture.host.client().unwrap();
        let input = fixture.resource("intent.md");
        let authority = fixture.authority();
        if mutation == "dirty-saved" {
            client
                .open(
                    1,
                    input,
                    1,
                    ByteEncoding::Utf8,
                    Arc::from(b"Dirty local intent\n".as_slice()),
                    BufferIntent::Replace,
                )
                .unwrap();
        }
        let captured = client.freeze().unwrap();
        let view = fixture.view(&captured);
        if mutation == "dirty-saved" {
            assert!(matches!(
                prepare_origin_rebase(&view, &fixture.old, fixture.next.clone(), &authority),
                Err(ChangeRebaseError::Edit(EditError::SaveRequired))
            ));
            fs::write(
                fixture
                    .root
                    .path()
                    .join("host/docs/custom/changes")
                    .join(ID)
                    .join("intent.md"),
                "Dirty local intent\n",
            )
            .unwrap();
            assert!(matches!(
                prepare_origin_rebase(&view, &fixture.old, fixture.next.clone(), &authority),
                Err(ChangeRebaseError::Edit(EditError::SaveRequired))
            ));
            let fresh = client.freeze().unwrap();
            let saved = fixture.view(&fresh);
            assert!(
                prepare_origin_rebase(&saved, &fixture.old, fixture.next.clone(), &authority)
                    .is_ok()
            );
        } else {
            let plan = prepare_origin_rebase(&view, &fixture.old, fixture.next.clone(), &authority)
                .unwrap();
            if mutation == "buffer-version" {
                client
                    .open(
                        1,
                        input,
                        2,
                        ByteEncoding::Utf8,
                        Arc::from(b"Independent local intent\n".as_slice()),
                        BufferIntent::Replace,
                    )
                    .unwrap();
            } else {
                fs::write(
                    fixture
                        .root
                        .path()
                        .join("host/docs/custom/changes")
                        .join(ID)
                        .join("acceptance.md"),
                    "Disk changed\n",
                )
                .unwrap();
            }
            let before = inventory(fixture.root.path());
            let fresh = client.freeze().unwrap();
            assert_eq!(
                plan.admit(&fresh, Duration::from_secs(30)).unwrap_err(),
                EditError::Stale
            );
            assert_eq!(before, inventory(fixture.root.path()));
        }
    }
}

#[test]
fn initial_legacy_binding_uses_saved_capture_without_circular_acceptance() {
    let fixture = Fixture::new();
    let client = fixture.host.client().unwrap();
    let path = fixture
        .root
        .path()
        .join("host/docs/custom/changes")
        .join(ID)
        .join("change.toml");
    let token = format!(
        "selected_content_hash='{}'\r\n",
        fixture
            .old
            .declaration()
            .selected_content_hash
            .as_ref()
            .unwrap()
    );
    assert_eq!(fixture.text.matches(&token).count(), 1);
    let legacy = fixture.text.replacen(&token, "", 1);
    fs::write(&path, &legacy).unwrap();
    let capture = client.freeze().unwrap();
    let view = fixture.view(&capture);
    let authority = fixture.authority();
    assert_eq!(
        view.analysis().unwrap().identities.intent.recipe(),
        vibe_core::manifest::change::INTENT_RECIPE
    );
    let retained = Arc::new(fixture.old.capture().clone());
    let before = inventory(fixture.root.path());
    let plan = prepare_initial_change_origin_binding(&view, 0, Some(retained.clone()), &authority)
        .unwrap();
    assert!(plan.binding().previous().selected_content_hash.is_none());
    assert_eq!(
        plan.binding().next_origin().declaration(),
        fixture.old.declaration()
    );
    assert_eq!(
        plan.new_intent().recipe(),
        vibe_core::manifest::change::BOUND_INTENT_RECIPE
    );
    assert_eq!(plan.proposal().edits.len(), 1);
    assert_eq!(plan.proposal().edits[0].field, "selected_content_hash");
    assert!(plan.proposal().after.contains("# retain comment\r\n"));
    assert!(plan.proposal().after.contains("# preserve row comment\r\n"));
    assert!(
        plan.proposal()
            .after
            .contains("owner_value='untouched' # opaque user value\r\n")
    );
    assert_eq!(
        vibe_requirements::change::accept_origin(plan.binding().next_origin().declaration(), None)
            .unwrap_err(),
        vibe_requirements::change::OriginUnavailable::MissingRetained
    );
    let fresh = client.freeze().unwrap();
    let barrier = plan.admit(&fresh, Duration::from_secs(30)).unwrap();
    client.acknowledge_edit(&barrier, true).unwrap();
    drop(barrier);
    assert_eq!(before, inventory(fixture.root.path()));
    client
        .open(
            1,
            fixture.resource("intent.md"),
            4,
            ByteEncoding::Utf8,
            Arc::from(b"Unsaved initial intent\n".as_slice()),
            BufferIntent::Replace,
        )
        .unwrap();
    let dirty = client.freeze().unwrap();
    let view = fixture.view(&dirty);
    assert!(matches!(
        prepare_initial_change_origin_binding(&view, 0, Some(retained), &authority),
        Err(ChangeRebaseError::Edit(EditError::SaveRequired))
    ));
    assert_eq!(before, inventory(fixture.root.path()));
}

#[test]
fn uncited_content_rebase_changes_bound_intent_with_empty_fact_delta() {
    let fixture = Fixture::new();
    let source = fixture.root.path().join("source");
    let host = fixture.root.path().join("host");
    fs::write(source.join("README.md"), "Uncited package bytes changed\n").unwrap();
    install(&host, &source, OLD, 'c');
    let next = Arc::new(
        SourceQueryService::prepare_host(&host)
            .unwrap()
            .retain_origin_capture("org.demo", "dep", &[ADDRESS.into()])
            .unwrap(),
    );
    let client = fixture.host.client().unwrap();
    let capture = client.freeze().unwrap();
    let view = fixture.view(&capture);
    let authority = fixture.authority();
    let before = inventory(fixture.root.path());
    let plan = prepare_origin_rebase(&view, &fixture.old, next, &authority).unwrap();
    assert!(plan.review().affected.is_empty());
    assert_eq!(plan.review().old.facts, plan.review().new.facts);
    assert_ne!(
        plan.review().old.selected_content_hash,
        plan.review().new.selected_content_hash
    );
    assert_ne!(
        plan.new_intent(),
        &view.analysis().unwrap().identities.intent
    );
    assert_eq!(
        plan.new_intent().recipe(),
        vibe_core::manifest::change::BOUND_INTENT_RECIPE
    );
    assert_eq!(plan.proposal().edits.len(), 1);
    assert_eq!(plan.proposal().edits[0].field, "selected_content_hash");
    assert_eq!(
        plan.proposal().checked.origins()[0]
            .selected_content_hash
            .as_deref(),
        Some(format!("sha256:{}", "c".repeat(64)).as_str())
    );
    let fresh = client.freeze().unwrap();
    let barrier = plan.admit(&fresh, Duration::from_secs(30)).unwrap();
    client.acknowledge_edit(&barrier, true).unwrap();
    drop(barrier);
    assert_eq!(before, inventory(fixture.root.path()));
    assert_eq!(
        fixture.old.capture().documents()["DOC"].snapshot().bytes,
        OLD.as_bytes()
    );
}
