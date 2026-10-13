use super::*;
use vibe_orchestrator::document_analysis::*;
#[test]
fn context_positions_and_gap_use_same_client() {
    let root = tempfile::tempdir().unwrap();
    let raw = "[project]\ngroup='org.demo'\nname='😀'\nversion='1.0.0'\r\n";
    std::fs::write(root.path().join("vibe.toml"), raw).unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        vec![ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        }],
        AnalysisLimits {
            resource_bytes: 4096,
            capture_bytes: 8192,
            open_buffers: 2,
        },
    )
    .unwrap();
    let id = host.resource("vibe.toml").unwrap();
    let context = ServerContext::with_store_root(root.path(), root.path().join("store"))
        .with_document_analysis(host)
        .unwrap();
    let client = context.document_client().unwrap();
    let capture = client.freeze().unwrap();
    let text = capture.raw_text(id).unwrap();
    let offset = raw.find('😀').unwrap() + 4;
    let encoding = negotiate_encoding(&[EditorEncoding::Utf16, EditorEncoding::Utf8]).unwrap();
    let location = text.location(offset, encoding).unwrap();
    assert_eq!(
        text.offset(
            &location.revision,
            &location.basis,
            location.position,
            location.encoding
        ),
        Ok(offset)
    );
    let view = capture.view(id, ViewFilter::Raw).unwrap();
    assert_eq!(view.page(None, 65536).unwrap().bytes, raw.as_bytes());
    client.notify(2, ChangeHint::Disk(id)).unwrap();
    assert_eq!(
        client.freshness(&capture),
        Ok(ViewFreshness::ResnapshotRequired)
    );
    let fresh = client.resnapshot().unwrap();
    assert_eq!(
        client.freshness(&fresh),
        Ok(ViewFreshness::UninvalidatedHint)
    );
    assert_eq!(
        fresh.raw_text(id).unwrap().raw(),
        capture.raw_text(id).unwrap().raw()
    );
    assert_eq!(
        std::fs::read(root.path().join("vibe.toml")).unwrap(),
        raw.as_bytes()
    );
    assert!(!root.path().join("store").exists());
    assert!(!root.path().join("vibe.lock").exists());
}
