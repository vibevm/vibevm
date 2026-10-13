use super::*;
use crate::document_analysis::edit::*;
use crate::document_analysis::*;
use std::{fs, sync::Arc, time::Duration};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
fn fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("docs/custom/changes").join(ID);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    let text = format!(
        "schema=1\n[change]\nid='{ID}'\nselected_node='.'\nslug='local'\ntitle='Local intent'\n[[origins]]\npackage='feat:org.fixture/reusable-intent@=1.0.0'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.fixture/reusable-intent/FEAT-001#DELIVERY'\nsha256='sha256:{}'\n",
        "a".repeat(64),
        "b".repeat(64)
    );
    let mut text = text;
    for (role, path, normative, input) in [
        ("intent", "intent.md", true, true),
        ("refinement", "refinement.md", true, true),
        ("acceptance", "acceptance.md", true, true),
        ("work", "work.toml", false, true),
        ("closure", "closure.md", false, false),
    ] {
        text.push_str(&format!(
            "[[documents]]\nrole='{role}'\npath='{path}'\nnormative={normative}\ninput={input}\n"
        ));
    }
    fs::write(dir.join("change.toml"), text).unwrap();
    for (path, raw) in [
        ("intent.md", "Shared local intent\n"),
        ("refinement.md", "Saved refinement\n"),
        ("acceptance.md", "Independent acceptance\n"),
        ("work.toml", "schema=1\nitems=[]\n"),
        ("closure.md", "Derived closure is not input\n"),
    ] {
        fs::write(dir.join(path), raw).unwrap();
    }
    let mut declarations = vec![ResourceDeclaration {
        path: "vibe.toml".into(),
        role: ResourceRole::Manifest,
    }];
    for path in [
        "change.toml",
        "intent.md",
        "refinement.md",
        "acceptance.md",
        "work.toml",
        "closure.md",
    ] {
        declarations.push(ResourceDeclaration {
            path: format!("docs/custom/changes/{ID}/{path}"),
            role: ResourceRole::Opaque,
        });
    }
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        declarations,
        AnalysisLimits {
            resource_bytes: 1048576,
            capture_bytes: 8388608,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
fn resource(host: &DocumentAnalysisHost, path: &str) -> ResourceId {
    host.resource(&format!("docs/custom/changes/{ID}/{path}"))
        .unwrap()
}
fn view(capture: &AnalysisCapture, node: ResourceId) -> CapturedChangeView<'_> {
    analyze_change(capture, node, Some(Path::new("docs/custom")), ID)
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
                    Some(fs::read(&p).unwrap()),
                ));
            }
        }
    }
    rows.sort();
    rows
}
#[test]
fn two_clients_dirty_refinements_are_isolated() {
    let (root, host) = fixture();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    let id = resource(&host, "refinement.md");
    let node = host.resource("vibe.toml").unwrap();
    let before = inventory(root.path());
    a.open(
        1,
        id,
        1,
        ByteEncoding::Utf8,
        Arc::from(b"Client A refinement\n".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    b.open(
        1,
        id,
        7,
        ByteEncoding::Utf8,
        Arc::from(b"Client B refinement\n".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let ac = a.freeze().unwrap();
    let bc = b.freeze().unwrap();
    let av = view(&ac, node);
    let bv = view(&bc, node);
    assert_eq!(av.raw(id), Some(b"Client A refinement\n".as_slice()));
    assert_eq!(bv.raw(id), Some(b"Client B refinement\n".as_slice()));
    assert_ne!(
        av.analysis().unwrap().identities.intent,
        bv.analysis().unwrap().identities.intent
    );
    assert_eq!(
        av.analysis().unwrap().identities.work,
        bv.analysis().unwrap().identities.work
    );
    assert_eq!(av.analysis().unwrap().manifest.id().as_str(), ID);
    assert_ne!(av.capture_id(), bv.capture_id());
    assert_eq!(av.location_basis(), LocationBasis::OriginalBytes);
    assert_eq!(
        av.revisions()
            .find(|r| r.resource == id)
            .unwrap()
            .buffer_version,
        Some(1)
    );
    assert_eq!(
        bv.revisions()
            .find(|r| r.resource == id)
            .unwrap()
            .buffer_version,
        Some(7)
    );
    assert_eq!(before, inventory(root.path()));
    assert!(!root.path().join(".vibe").exists());
}
#[test]
fn partial_raw_and_historical_analysis_have_no_effects() {
    let (root, host) = fixture();
    let a = host.client().unwrap();
    let node = host.resource("vibe.toml").unwrap();
    let change = resource(&host, "change.toml");
    a.open(
        1,
        change,
        3,
        ByteEncoding::Utf8,
        Arc::from(b"[change\n".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let capture = a.freeze().unwrap();
    fs::write(
        root.path()
            .join("docs/custom/changes")
            .join(ID)
            .join("change.toml"),
        b"poisoned live disk",
    )
    .unwrap();
    let before = inventory(root.path());
    let v = view(&capture, node);
    assert_eq!(v.raw(change), Some(b"[change\n".as_slice()));
    let error = v.analysis().unwrap_err();
    assert!(
        matches!(&error.error,ChangeViewError::Declaration(e) if e.kind==vibe_core::manifest::change::ChangeErrorKind::Syntax)
    );
    assert_eq!(error.revision.as_ref().unwrap().buffer_version, Some(3));
    assert_eq!(error.basis, LocationBasis::OriginalBytes);
    assert!(error.numeric_position_unavailable);
    assert_eq!(before, inventory(root.path()));
    a.change(
        1,
        change,
        3,
        4,
        ByteEncoding::Opaque,
        Arc::from(b"\xff\x00".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let opaque = a.freeze().unwrap();
    let v = view(&opaque, node);
    assert_eq!(v.raw(change), Some(b"\xff\x00".as_slice()));
    assert!(matches!(
        v.analysis().unwrap_err().error,
        ChangeViewError::Encoding
    ));
}
#[test]
fn saved_preparation_and_stale_epochs_reuse_existing_barrier() {
    for mutation in ["none", "buffer", "same-bytes-version", "disk"] {
        let (root, host) = fixture();
        let a = host.client().unwrap();
        let node = host.resource("vibe.toml").unwrap();
        let id = resource(&host, "refinement.md");
        let closure = resource(&host, "closure.md");
        let authority = DocumentEditAuthority::new(host.clone(), vec![id, closure]).unwrap();
        let captured = a.freeze().unwrap();
        let v = view(&captured, node);
        assert!(v.analysis().is_ok());
        assert_eq!(
            prepare_change_edit(
                &v,
                &authority,
                vec![DocumentReplacement {
                    resource: closure,
                    bytes: Arc::from(b"not normative".as_slice())
                }]
            )
            .unwrap_err(),
            EditError::AuthorityRefused
        );
        let prepared = prepare_change_edit(
            &v,
            &authority,
            vec![DocumentReplacement {
                resource: id,
                bytes: Arc::from(b"Future authored refinement\n".as_slice()),
            }],
        )
        .unwrap();
        if mutation == "buffer" {
            a.open(
                1,
                id,
                1,
                ByteEncoding::Utf8,
                Arc::from(b"new dirty version".as_slice()),
                BufferIntent::Replace,
            )
            .unwrap();
        }
        if mutation == "same-bytes-version" {
            a.open(
                1,
                id,
                1,
                ByteEncoding::Utf8,
                Arc::from(b"Saved refinement\n".as_slice()),
                BufferIntent::Replace,
            )
            .unwrap();
        }
        if mutation == "disk" {
            fs::write(
                root.path()
                    .join("docs/custom/changes")
                    .join(ID)
                    .join("refinement.md"),
                b"changed disk",
            )
            .unwrap();
        }
        let fresh = a.freeze().unwrap();
        let before = inventory(root.path());
        let result = prepared.admit(&fresh, Duration::from_secs(10));
        if mutation == "none" {
            let barrier = result.unwrap();
            a.acknowledge_edit(&barrier, true).unwrap();
            drop(barrier);
        } else {
            assert_eq!(result.unwrap_err(), EditError::Stale);
        }
        assert_eq!(before, inventory(root.path()));
        assert!(!root.path().join(".vibe").exists());
    }
}
#[test]
fn old_dirty_capture_stays_save_required_after_explicit_save() {
    let (root, host) = fixture();
    let a = host.client().unwrap();
    let node = host.resource("vibe.toml").unwrap();
    let id = resource(&host, "refinement.md");
    let authority = DocumentEditAuthority::new(host.clone(), vec![id]).unwrap();
    a.open(
        1,
        id,
        1,
        ByteEncoding::Utf8,
        Arc::from(b"Dirty intent\n".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let dirty = a.freeze().unwrap();
    let old = view(&dirty, node);
    assert!(old.analysis().is_ok());
    let replacement = || {
        vec![DocumentReplacement {
            resource: id,
            bytes: Arc::from(b"Next intent\n".as_slice()),
        }]
    };
    assert_eq!(
        prepare_change_edit(&old, &authority, replacement()).unwrap_err(),
        EditError::SaveRequired
    );
    fs::write(
        root.path()
            .join("docs/custom/changes")
            .join(ID)
            .join("refinement.md"),
        b"Dirty intent\n",
    )
    .unwrap();
    assert_eq!(
        prepare_change_edit(&old, &authority, replacement()).unwrap_err(),
        EditError::SaveRequired
    );
    let saved = a.freeze().unwrap();
    let current = view(&saved, node);
    let prepared = prepare_change_edit(&current, &authority, replacement()).unwrap();
    let fresh = a.freeze().unwrap();
    drop(prepared.admit(&fresh, Duration::from_secs(10)).unwrap());
    assert!(!root.path().join(".vibe").exists());
}
