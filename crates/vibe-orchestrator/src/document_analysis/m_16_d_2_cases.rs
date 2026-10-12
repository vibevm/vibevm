use super::*;
use vibe_spec::editor_positions::{EditorEncoding, EditorPosition};
fn fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.demo'\nname='app'\nversion='1.0.0'\n[[project.sources]]\nfile='doc.xml'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n").unwrap();
    std::fs::write(root.path().join("text.md"), "a😀e\u{301}\r\nЖ\n").unwrap();
    let md = "# Root {#root}\n\n@fact:ONE One. @status:impl/done\n\n## Child {#child}\n\nTwo.\n";
    std::fs::write(
        root.path().join("doc.xml"),
        vibe_specdoc::to_xml(&vibe_specdoc::from_markdown(md).unwrap()),
    )
    .unwrap();
    std::fs::write(root.path().join("opaque.bin"), b"\xff\x00").unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "text.md".into(),
                role: ResourceRole::Opaque,
            },
            ResourceDeclaration {
                path: "opaque.bin".into(),
                role: ResourceRole::Opaque,
            },
            ResourceDeclaration {
                path: "doc.xml".into(),
                role: ResourceRole::Source {
                    manifest: "vibe.toml".into(),
                },
            },
        ],
        AnalysisLimits {
            resource_bytes: 1048576,
            capture_bytes: 8388608,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
#[test]
fn unicode_location_uses_actual_frozen_resource() {
    let (root, host) = fixture();
    let client = host.client().unwrap();
    let id = host.resource("text.md").unwrap();
    client
        .open(
            1,
            id,
            7,
            ByteEncoding::Utf8,
            Arc::from("a😀e\u{301}\r\nЖ\n".as_bytes()),
            BufferIntent::Replace,
        )
        .unwrap();
    let capture = client.freeze().unwrap();
    let text = capture.raw_text(id).unwrap();
    let loc = text.location(5, EditorEncoding::Utf16).unwrap();
    assert_eq!(
        loc.position,
        EditorPosition {
            line: 0,
            character: 3
        }
    );
    assert_eq!(loc.revision.buffer_version, Some(7));
    assert_eq!(loc.revision.capture, capture.id());
    assert_eq!(
        text.offset(&loc.revision, &loc.basis, loc.position, loc.encoding),
        Ok(5)
    );
    assert_eq!(text.original_offset(&loc), Ok(5));
    assert_eq!(
        std::fs::read(root.path().join("text.md")).unwrap(),
        text.raw()
    );
}
#[test]
fn projection_partial_and_non_utf8_locations_are_honest() {
    let (root, host) = fixture();
    let client = host.client().unwrap();
    let capture = client.freeze().unwrap();
    let xml = host.resource("doc.xml").unwrap();
    let text = capture.projected_text(xml).unwrap();
    let loc = text.location(0, EditorEncoding::Utf8).unwrap();
    assert!(matches!(
        loc.basis,
        LocationBasis::MarkdownProjection {
            recipe: "vibe-specdoc-to-markdown/1",
            ..
        }
    ));
    assert_eq!(
        text.original_offset(&loc),
        Err(LocationError::ReverseMapUnavailable)
    );
    let raw_xml = capture.raw_text(xml).unwrap();
    assert_eq!(
        raw_xml.offset(&loc.revision, &loc.basis, loc.position, loc.encoding),
        Err(LocationError::RevisionMismatch)
    );
    let opaque = capture
        .raw_text(host.resource("opaque.bin").unwrap())
        .unwrap();
    assert_eq!(opaque.raw(), b"\xff\x00");
    assert_eq!(
        opaque.location(0, EditorEncoding::Utf8),
        Err(LocationError::EncodingUnavailable)
    );
    client
        .open(
            1,
            xml,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"<spec".as_slice()),
            BufferIntent::Replace,
        )
        .unwrap();
    let partial = client.freeze().unwrap();
    assert_eq!(partial.raw_text(xml).unwrap().raw(), b"<spec");
    assert!(matches!(
        partial.projected_text(xml),
        Err(LocationError::ProjectionUnavailable)
    ));
    let actual = partial
        .parser_diagnostics(xml, EditorEncoding::Utf16)
        .unwrap();
    assert!(!actual.is_empty());
    assert!(actual.iter().all(|d| d.revision.capture == partial.id()
        && d.revision.buffer_version == Some(1)
        && d.basis == LocationBasis::OriginalBytes));
    let mut diagnostic = partial.diagnostic(xml, "partial", "actual partial source");
    assert_eq!(
        diagnostic.source_revision.as_ref().unwrap().buffer_version,
        Some(1)
    );
    let bound = partial.bind_diagnostic(diagnostic.clone()).unwrap();
    assert_eq!(bound.revision.unwrap().buffer_version, Some(1));
    diagnostic.capture = capture.id().into();
    assert!(matches!(
        partial.bind_diagnostic(diagnostic),
        Err(LocationError::RevisionMismatch)
    ));
    assert_ne!(
        std::fs::read(root.path().join("doc.xml")).unwrap(),
        b"<spec"
    );
}
#[test]
fn pages_bind_filter_capture_and_reconstruct_all_nodes_and_bytes() {
    let (_root, host) = fixture();
    let client = host.client().unwrap();
    let capture = client.freeze().unwrap();
    let id = host.resource("doc.xml").unwrap();
    let outline = capture.outline(id).unwrap();
    assert_eq!(
        &outline.source_revision,
        capture.projected_text(id).unwrap().revision()
    );
    let selected = capture.select(id, Some("root"), true).unwrap();
    assert_eq!(selected.source_revision, outline.source_revision);
    assert_eq!(selected.projection_digest, outline.projection_digest);
    let full = outline.nodes;
    let view = capture
        .view(id, ViewFilter::Outline { prefix: vec![] })
        .unwrap();
    let first = view.page(None, 1).unwrap();
    let cursor = first.next.clone().unwrap();
    let mut all = first.nodes;
    let mut next = Some(cursor.clone());
    while let Some(c) = next {
        let p = view.page(Some(&c), 1).unwrap();
        all.extend(p.nodes);
        next = p.next;
    }
    assert_eq!(all, full);
    let filtered = capture
        .view(
            id,
            ViewFilter::Outline {
                prefix: vec!["root".into()],
            },
        )
        .unwrap();
    assert!(matches!(
        filtered.page(Some(&cursor), 1),
        Err(ViewError::ForeignCursor)
    ));
    let newer = client
        .freeze()
        .unwrap()
        .view(id, ViewFilter::Outline { prefix: vec![] })
        .unwrap();
    assert!(matches!(
        newer.page(Some(&cursor), 1),
        Err(ViewError::ForeignCursor)
    ));
    let raw = capture.view(id, ViewFilter::Raw).unwrap();
    let mut p = raw.page(None, 3).unwrap();
    let mut bytes = p.bytes;
    while let Some(c) = p.next {
        p = raw.page(Some(&c), 3).unwrap();
        bytes.extend(&p.bytes);
    }
    assert_eq!(bytes, capture.resource(id).unwrap().raw().unwrap());
}
#[test]
fn watch_gap_and_overlay_preserve_historical_capture_without_effects() {
    let (root, host) = fixture();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    let id = host.resource("text.md").unwrap();
    let old = a.freeze().unwrap();
    let other = b.freeze().unwrap();
    let raw = old.raw_text(id).unwrap();
    std::fs::write(root.path().join("text.md"), b"changed by fixture").unwrap();
    let event = a.notify(1, ChangeHint::Disk(id)).unwrap();
    assert!(event.affected.contains(&id));
    assert_eq!(a.freshness(&old), Ok(ViewFreshness::Historical));
    assert_eq!(b.freshness(&other), Ok(ViewFreshness::UninvalidatedHint));
    assert_eq!(old.raw_text(id).unwrap().raw(), raw.raw());
    a.notify(3, ChangeHint::Lock(host.resource("vibe.toml").unwrap()))
        .unwrap();
    assert_eq!(a.freshness(&old), Ok(ViewFreshness::ResnapshotRequired));
    let during_gap = a.freeze().unwrap();
    let fresh = a.resnapshot().unwrap();
    assert_eq!(fresh.raw_text(id).unwrap().raw(), b"changed by fixture");
    assert_eq!(a.freshness(&fresh), Ok(ViewFreshness::UninvalidatedHint));
    assert_eq!(a.freshness(&during_gap), Ok(ViewFreshness::Historical));
    for (sequence, hint) in [
        (4, ChangeHint::Policy),
        (5, ChangeHint::Target),
        (6, ChangeHint::Overlay(id)),
        (7, ChangeHint::Gap),
    ] {
        a.notify(sequence, hint).unwrap();
    }
    assert_eq!(a.freshness(&fresh), Ok(ViewFreshness::ResnapshotRequired));
    assert_eq!(
        std::fs::read(root.path().join("text.md")).unwrap(),
        b"changed by fixture"
    );
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("vibe.lock").exists());
    let clean = a.resnapshot().unwrap();
    a.open(
        1,
        id,
        1,
        ByteEncoding::Utf8,
        Arc::from(b"dirty".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    assert_eq!(a.freshness(&clean), Ok(ViewFreshness::Historical));
    assert_eq!(
        a.freshness(&other),
        Err(DocumentAnalysisError::InvalidDeclaration)
    );
}
