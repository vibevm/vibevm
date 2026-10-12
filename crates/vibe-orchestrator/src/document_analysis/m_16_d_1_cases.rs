use super::*;
use std::fs;
const BEFORE: &str = "[package]\ngroup='org.overlay'\nname='app'\nkind='flow'\nversion='1.0.0'\n[features]\ndefault=['saved']\nsaved=[]\ndirty=[]\n";
fn fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), BEFORE).unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        vec![ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        }],
        AnalysisLimits {
            resource_bytes: 1024 * 1024,
            capture_bytes: 8 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
#[test]
fn two_clients_preview_captured_manifest() {
    let (root, host) = fixture();
    let saved_bytes = fs::read(root.path().join("vibe.toml")).unwrap();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    let manifest = host.resource("vibe.toml").unwrap();
    let dirty = BEFORE.replace("default=['saved']", "default=['dirty']");
    a.open(
        1,
        manifest,
        1,
        ByteEncoding::Utf8,
        Arc::from(dirty.as_bytes()),
        BufferIntent::Replace,
    )
    .unwrap();
    let a_capture = a.freeze().unwrap();
    let b_capture = b.freeze().unwrap();
    let a_preview = a_capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    let b_preview = b_capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(
        a_preview.diagnostics.is_empty(),
        "{:?}",
        a_preview.diagnostics
    );
    assert!(
        b_preview.diagnostics.is_empty(),
        "{:?}",
        b_preview.diagnostics
    );
    assert!(
        a_preview
            .features
            .as_ref()
            .unwrap()
            .active_features
            .contains("dirty")
    );
    assert!(
        !a_preview
            .features
            .as_ref()
            .unwrap()
            .active_features
            .contains("saved")
    );
    assert!(
        b_preview
            .features
            .as_ref()
            .unwrap()
            .active_features
            .contains("saved")
    );
    assert!(
        !b_preview
            .features
            .as_ref()
            .unwrap()
            .active_features
            .contains("dirty")
    );
    assert_ne!(a_capture.client(), b_capture.client());
    assert_ne!(a_capture.id(), b_capture.id());
    assert_eq!(
        a_capture.resource(manifest).unwrap().raw().unwrap(),
        dirty.as_bytes()
    );
    assert_eq!(
        b_capture.resource(manifest).unwrap().raw().unwrap(),
        saved_bytes
    );
    assert_eq!(
        fs::read(root.path().join("vibe.toml")).unwrap(),
        saved_bytes
    );
}

fn resource_fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("docs")).unwrap();
    let text = "[project]\ngroup='org.overlay'\nname='app'\nversion='1.0.0'\n[[project.sources]]\ndirectory='docs'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='scan'\n[[project.sources]]\nfile='docs/data.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n";
    std::fs::write(root.path().join("vibe.toml"), text).unwrap();
    std::fs::write(
        root.path().join("docs/a.md"),
        "# Ordinary\n\n@fact:FAKE Literal. @status:impl/done\n",
    )
    .unwrap();
    std::fs::write(root.path().join("docs/data.xml"), "<data><broken>").unwrap();
    let mut rows = vec![ResourceDeclaration {
        path: "vibe.toml".into(),
        role: ResourceRole::Manifest,
    }];
    for path in ["docs/a.md", "docs/new.md", "docs/moved.md", "docs/data.xml"] {
        rows.push(ResourceDeclaration {
            path: path.into(),
            role: ResourceRole::Source {
                manifest: "vibe.toml".into(),
            },
        });
    }
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        rows,
        AnalysisLimits {
            resource_bytes: 1024 * 1024,
            capture_bytes: 8 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
#[test]
fn buffer_versions_create_delete_move_and_close() {
    let (root, host) = resource_fixture();
    let client = host.client().unwrap();
    let new = host.resource("docs/new.md").unwrap();
    let old = host.resource("docs/a.md").unwrap();
    let moved = host.resource("docs/moved.md").unwrap();
    assert_eq!(
        client.open(
            9,
            new,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"new".as_slice()),
            BufferIntent::Create
        ),
        Err(DocumentAnalysisError::UnknownEpoch)
    );
    client
        .open(
            1,
            new,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"new".as_slice()),
            BufferIntent::Create,
        )
        .unwrap();
    client
        .open(
            1,
            old,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"moved".as_slice()),
            BufferIntent::Move { target: moved },
        )
        .unwrap();
    let frozen = client.freeze().unwrap();
    assert_eq!(frozen.resource(new).unwrap().raw(), Some(b"new".as_slice()));
    assert_eq!(frozen.resource(old).unwrap().raw(), None);
    assert_eq!(
        frozen.resource(old).unwrap().buffer_bytes(),
        Some(b"moved".as_slice())
    );
    assert_eq!(
        frozen.resource(moved).unwrap().raw(),
        Some(b"moved".as_slice())
    );
    assert_eq!(
        client.change(
            1,
            new,
            7,
            8,
            ByteEncoding::Utf8,
            Arc::from(b"wrong".as_slice()),
            BufferIntent::Create
        ),
        Err(DocumentAnalysisError::StaleVersion)
    );
    client.close(1, old, 1).unwrap();
    client
        .open(
            1,
            old,
            2,
            ByteEncoding::Utf8,
            Arc::from([]),
            BufferIntent::Delete,
        )
        .unwrap();
    assert_eq!(client.freeze().unwrap().resource(old).unwrap().raw(), None);
    client.close(1, new, 1).unwrap();
    assert_eq!(
        client.open(
            1,
            new,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"old".as_slice()),
            BufferIntent::Create
        ),
        Err(DocumentAnalysisError::StaleVersion)
    );
    assert!(!root.path().join("docs/new.md").exists());
    assert!(!root.path().join("docs/moved.md").exists());
    assert!(root.path().join("docs/a.md").exists());
    let collision = host.client().unwrap();
    let second = host.resource("docs/data.xml").unwrap();
    collision
        .open(
            1,
            old,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"first".as_slice()),
            BufferIntent::Move { target: moved },
        )
        .unwrap();
    collision
        .open(
            1,
            second,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"second".as_slice()),
            BufferIntent::Move { target: moved },
        )
        .unwrap();
    assert!(matches!(
        collision.freeze(),
        Err(DocumentAnalysisError::InvalidIntent)
    ));
    assert!(!root.path().join("docs/moved.md").exists());
    let (_, foreign) = resource_fixture();
    assert_eq!(
        client.open(
            1,
            foreign.resource("docs/a.md").unwrap(),
            1,
            ByteEncoding::Utf8,
            Arc::from(b"x".as_slice()),
            BufferIntent::Replace
        ),
        Err(DocumentAnalysisError::UnknownResource)
    );
}
#[test]
fn historical_capture_and_partial_raw_survive() {
    let (root, host) = resource_fixture();
    let client = host.client().unwrap();
    let manifest = host.resource("vibe.toml").unwrap();
    let ordinary = host.resource("docs/a.md").unwrap();
    let data = host.resource("docs/data.xml").unwrap();
    let initial = client.freeze().unwrap();
    let old = initial.resource(manifest).unwrap().raw().unwrap().to_vec();
    client
        .open(
            1,
            manifest,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"[project]\nname=".as_slice()),
            BufferIntent::Replace,
        )
        .unwrap();
    let partial = client.freeze().unwrap();
    assert_eq!(
        partial.manifest(manifest).unwrap_err().code,
        "manifest-syntax"
    );
    assert_eq!(
        partial.resource(manifest).unwrap().raw(),
        Some(b"[project]\nname=".as_slice())
    );
    assert_eq!(initial.resource(manifest).unwrap().raw().unwrap(), old);
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='disk-changed'\nversion='1.0.0'\n",
    )
    .unwrap();
    assert_eq!(initial.resource(manifest).unwrap().raw().unwrap(), old);
    let view = initial.inspect(ordinary).unwrap();
    assert!(matches!(
        view.inspection.view,
        Some(vibe_specdoc::captured::StructuredView::Commonmark(_))
    ));
    let view = initial.inspect(data).unwrap();
    assert_eq!(view.inspection.raw(), b"<data><broken>");
    assert!(matches!(
        view.inspection.view,
        Some(vibe_specdoc::captured::StructuredView::XmlData(
            vibe_specdoc::xml_data::DataInspection::Unavailable(_)
        ))
    ));
}
fn boot_fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    let specs = "vibevm/vibespecs";
    std::fs::create_dir_all(root.path().join(format!("dep/{specs}/boot"))).unwrap();
    std::fs::create_dir_all(root.path().join(format!("{specs}/boot"))).unwrap();
    let primary = "[project]\ngroup='org.overlay'\nname='app'\nversion='1.0.0'\n[requires.packages]\n'flow:org.overlay/dep'='=1.0.0'\n[[project.sources]]\ndirectory='vibevm/vibespecs'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n";
    let dep = "[package]\ngroup='org.overlay'\nname='dep'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[[package.sources]]\ndirectory='vibevm/vibespecs'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[boot_snippet]\nsource='vibevm/vibespecs/boot/dep.md'\nlink='static'\n[[boot_snippet.fragment]]\nsource='vibevm/vibespecs/boot/extra.md'\nwhen='installed:org.overlay/dep'\n";
    std::fs::write(root.path().join("vibe.toml"), primary).unwrap();
    std::fs::write(root.path().join("dep/vibe.toml"), dep).unwrap();
    for (path, text) in [
        ("vibevm/vibespecs/boot/10-own.md", "# Own {#own}\n"),
        (
            "dep/vibevm/vibespecs/boot/dep.md",
            "# Dep {#root}\n\n#embed spec://org.overlay/dep/RULE#solution\n",
        ),
        (
            "dep/vibevm/vibespecs/boot/extra.md",
            "# Extra {#extra}\n\nDistinct fragment.\n",
        ),
        (
            "dep/vibevm/vibespecs/RULE.md",
            "# Rule {#root}\n\n## Solution {#solution}\n\n@fact:ONE Captured rule. @status:impl/done\n",
        ),
    ] {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let mut rows = vec![
        ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        },
        ResourceDeclaration {
            path: "dep/vibe.toml".into(),
            role: ResourceRole::Manifest,
        },
    ];
    for (path, owner) in [
        ("vibevm/vibespecs/boot/10-own.md", "vibe.toml"),
        ("dep/vibevm/vibespecs/boot/dep.md", "dep/vibe.toml"),
        ("dep/vibevm/vibespecs/boot/extra.md", "dep/vibe.toml"),
        ("dep/vibevm/vibespecs/RULE.md", "dep/vibe.toml"),
    ] {
        rows.push(ResourceDeclaration {
            path: path.into(),
            role: ResourceRole::Source {
                manifest: owner.into(),
            },
        });
    }
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        rows,
        AnalysisLimits {
            resource_bytes: 1024 * 1024,
            capture_bytes: 8 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
#[test]
fn captured_configuration_solver_and_boot_share_inputs() {
    let (root, host) = boot_fixture();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    let rule = host.resource("dep/vibevm/vibespecs/RULE.md").unwrap();
    let text =
        "# Rule {#root}\n\n## Solution {#solution}\n\n@fact:ONE Unsaved rule. @status:impl/done\n";
    a.open(
        1,
        rule,
        1,
        ByteEncoding::Utf8,
        Arc::from(text.as_bytes()),
        BufferIntent::Replace,
    )
    .unwrap();
    let captured = a.freeze().unwrap();
    let other = b.freeze().unwrap();
    let preview = captured.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    assert_eq!(preview.dependencies.as_ref().unwrap().packages.len(), 1);
    assert_eq!(preview.boot.as_ref().unwrap().entries.len(), 3);
    let compiled = std::str::from_utf8(preview.compiled_boot.as_ref().unwrap()).unwrap();
    assert!(compiled.contains("Unsaved rule."));
    assert!(compiled.contains("Distinct fragment."));
    assert!(!compiled.contains("#embed"));
    let other = other.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(
        std::str::from_utf8(other.compiled_boot.as_ref().unwrap())
            .unwrap()
            .contains("Captured rule.")
    );
    std::fs::write(
        root.path().join("dep/vibevm/vibespecs/RULE.md"),
        "poisoned disk after capture",
    )
    .unwrap();
    assert!(
        std::str::from_utf8(
            captured
                .configuration_preview(&vibe_resolver::FeatureRequest::default())
                .compiled_boot
                .as_ref()
                .unwrap()
        )
        .unwrap()
        .contains("Unsaved rule.")
    );
    assert!(!root.path().join("vibevm/vibespecs/boot/STATIC.md").exists());
}
fn inventory(root: &std::path::Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut Vec<(String, Option<Vec<u8>>)>,
    ) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                out.push((name, None));
                walk(root, &path, out)
            } else {
                out.push((name, Some(std::fs::read(path).unwrap())));
            }
        }
    }
    let mut out = vec![];
    walk(root, root, &mut out);
    out
}
#[test]
fn readonly_preview_preserves_fixture_state() {
    let (root, host) = boot_fixture();
    std::fs::create_dir_all(root.path().join("settings/cache/profiles")).unwrap();
    std::fs::write(
        root.path().join("settings/cache/profiles/sentinel"),
        "unchanged settings/profile/cache",
    )
    .unwrap();
    let before = inventory(root.path());
    let client = host.client().unwrap();
    let capture = client.freeze().unwrap();
    let preview = capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    for (id, _) in capture.resources() {
        let _ = capture.inspect(id);
        let _ = capture.outline(id);
    }
    assert_eq!(before, inventory(root.path()));
}

#[test]
fn configured_builtin_and_native_preparation_are_not_ignored() {
    let (root, host) = boot_fixture();
    let client = host.client().unwrap();
    let primary = host.resource("vibe.toml").unwrap();
    let original = std::fs::read_to_string(root.path().join("vibe.toml")).unwrap();
    let xml = original.replace("version='1.0.0'", "version='1.0.0'\nspec_format='xml'");
    let configured = format!(
        "{xml}\n[[extension]]\nid='minify'\npoint='compile:emitted'\nhandler={{kind='builtin',name='xml-minify'}}\n"
    );
    client
        .open(
            1,
            primary,
            1,
            ByteEncoding::Utf8,
            Arc::from(configured.as_bytes()),
            BufferIntent::Replace,
        )
        .unwrap();
    let capture = client.freeze().unwrap();
    let preview = capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    let bytes = preview.compiled_boot.unwrap();
    assert!(!bytes.is_empty());
    assert!(!std::str::from_utf8(&bytes).unwrap().contains("\n  <"));
    client.change(1,primary,1,2,ByteEncoding::Utf8,Arc::from(format!("{original}\n[[extension]]\nid='native'\npoint='compile:emitted'\nhandler={{kind='native',crate_dir='never-built'}}\n").as_bytes()),BufferIntent::Replace).unwrap();
    let preview = client
        .freeze()
        .unwrap()
        .configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.compiled_boot.is_none());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "native-preparation-required"),
        "{:?}",
        preview.diagnostics
    );
    assert!(!root.path().join("never-built").exists());
}

#[path = "m_16_d_1_boot_cases.rs"]
mod boot_cases;
