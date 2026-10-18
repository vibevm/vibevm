use super::*;
use vibe_orchestrator::{
    document_analysis::{AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole},
    machine_operation::boot_debugger::{
        BootDebuggerRoute, CapturedKind, debugger_request as q, debugger_result as r,
    },
};
use vibe_resolver::resolvo_engine::request::NormalizedRequest;

fn fixture() -> (
    tempfile::TempDir,
    InstallResolver,
    BootDebuggerRoute,
    String,
    NormalizedRequest,
) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), "[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'={version='=1.0.0',link='static'}\n").unwrap();
    for name in ["vibe.lock", "INDEX.md", "STATIC.md"] {
        std::fs::write(root.join(name), format!("canonical {name}")).unwrap();
    }
    std::fs::create_dir(root.join("slots")).unwrap();
    std::fs::write(root.join("slots/sentinel"), b"installed slot bytes").unwrap();
    let boot = root.join(vibe_core::layout::current_boot_dir());
    std::fs::create_dir_all(&boot).unwrap();
    for name in ["STATIC.md", "STATIC.xml", "INDEX.md"] {
        std::fs::write(boot.join(name), format!("canonical boot {name}")).unwrap();
    }
    let slot = root.join("vibevm/vibedeps/org.example.app/0.9.0");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(
        slot.join("vibe.toml"),
        b"installed dependency slot sentinel",
    )
    .unwrap();
    let registry = temp.path().join("registry");
    let package = registry.join("org.example/app/v1.0.0");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n").unwrap();
    std::fs::write(
        package.join("boot.md"),
        b"# Pure builtin fixture\n\nsource bytes\n",
    )
    .unwrap();
    let resolver = InstallResolver::Local(LocalRegistry::new(&registry).unwrap(), None);
    let host = DocumentAnalysisHost::new(
        &root,
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "INDEX.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 100_000,
            capture_bytes: 200_000,
            open_buffers: 2,
        },
    )
    .unwrap();
    let artifact = host.resource("INDEX.md").unwrap();
    let client = host.client().unwrap();
    let capture = client.freeze().unwrap();
    let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap();
    let id = route.retain_capture(capture).unwrap();
    let roots = route
        .hypothetical_host(&id)
        .unwrap()
        .manifest
        .requires
        .packages;
    let request = NormalizedRequest::legacy(&roots, &Default::default());
    (temp, resolver, route, id, request)
}
fn project_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = vec![];
    fn walk(root: &Path, directory: &Path, files: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    std::fs::read(path).unwrap(),
                ));
            }
        }
    }
    walk(root, root, &mut files);
    files.sort();
    files
}
#[test]
fn real_source_binds_shared_route_and_preserves_project() {
    let (temp, resolver, route, id, normalized) = fixture();
    let before = project_bytes(&temp.path().join("project"));
    let request = resolver
        .bind_boot_preview(
            &route,
            &id,
            &id,
            normalized.clone(),
            normalized,
            "os:linux".into(),
        )
        .unwrap();
    let q::Command::Preview(binding) = &request.command else {
        panic!()
    };
    println!(
        "prepared scenario={} baseline={} candidate={} catalog={}",
        binding.scenario_id,
        binding.expected_baseline,
        binding.expected_candidate,
        binding.expected_catalog
    );
    let first = route.request(request.clone()).unwrap();
    let r::Body::Preview(body) = &first.body else {
        panic!("preview result")
    };
    assert_eq!(body.status, r::BodyPreviewStatus::Complete, "{body:?}");
    assert!(body.artifact.as_ref().unwrap().contains("source bytes"));
    assert!(body.compiler_carrier_json.is_some());
    assert!(body.measurement_json.is_some());
    assert_eq!(before, project_bytes(&temp.path().join("project")));
    // Remove live registry input: evaluation still uses only retained bytes.
    std::fs::remove_file(temp.path().join("registry/org.example/app/v1.0.0/boot.md")).unwrap();
    assert_eq!(first, route.request(request).unwrap());
    assert_eq!(before, project_bytes(&temp.path().join("project")));
}
#[test]
fn shared_route_refuses_stale_and_illegal_loading() {
    let (temp, resolver, route, id, normalized) = fixture();
    let request = resolver
        .bind_boot_preview(
            &route,
            &id,
            &id,
            normalized.clone(),
            normalized.clone(),
            "os:linux".into(),
        )
        .unwrap();
    let mut illegal = request.clone();
    let q::Command::Preview(body) = &mut illegal.command else {
        panic!()
    };
    body.loading_overrides
        .insert("org.example/app".into(), "dynamic".into());
    let r::Body::Preview(result) = route.request(illegal).unwrap().body else {
        panic!()
    };
    assert_eq!(result.status, r::BodyPreviewStatus::Incompatible);
    assert!(result.artifact.is_none());
    let mut stale = request.clone();
    let q::Command::Preview(body) = &mut stale.command else {
        panic!()
    };
    body.expected_catalog.push_str("changed");
    let r::Body::Preview(result) = route.request(stale).unwrap().body else {
        panic!()
    };
    assert_eq!(result.status, r::BodyPreviewStatus::Stale);
    assert!(result.measurement_json.is_none());
    // An actual moving source changes both the catalog and the frozen world;
    // replaying the old expected witness against that new binding is stale.
    std::fs::write(
        temp.path().join("registry/org.example/app/v1.0.0/boot.md"),
        b"changed candidate source",
    )
    .unwrap();
    let mut recaptured = resolver
        .bind_boot_preview(
            &route,
            &id,
            &id,
            normalized.clone(),
            normalized.clone(),
            "os:linux".into(),
        )
        .unwrap();
    let q::Command::Preview(old) = &request.command else {
        panic!()
    };
    let q::Command::Preview(new) = &mut recaptured.command else {
        panic!()
    };
    assert_ne!(old.expected_catalog, new.expected_catalog);
    assert_ne!(old.expected_candidate, new.expected_candidate);
    new.expected_catalog = old.expected_catalog.clone();
    new.expected_candidate = old.expected_candidate.clone();
    let r::Body::Preview(result) = route.request(recaptured).unwrap().body else {
        panic!()
    };
    assert_eq!(result.status, r::BodyPreviewStatus::Stale);
    let wrong = NormalizedRequest::legacy(&[], &Default::default());
    assert!(
        resolver
            .bind_boot_preview(&route, &id, &id, wrong, normalized, "os:linux".into())
            .is_err()
    );
}

#[test]
fn normal_closure_is_compiled_from_frozen_sources() {
    let (temp, resolver, route, id, normalized) = fixture();
    let package = temp.path().join("registry/org.example/app/v1.0.0");
    std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[boot_snippet]\nsource='vibevm/vibespecs/boot/entry.md'\n").unwrap();
    std::fs::create_dir_all(package.join("vibevm/vibespecs/boot")).unwrap();
    std::fs::write(
        package.join("vibevm/vibespecs/boot/entry.md"),
        "# Entry {#root}\n\n#use spec://org.example/app/common/base#root\n\nnormal entry body\n",
    )
    .unwrap();
    std::fs::create_dir_all(package.join("vibevm/vibespecs/common")).unwrap();
    std::fs::write(
        package.join("vibevm/vibespecs/common/base.md"),
        "# Base {#root}\n\nretained closure body\n",
    )
    .unwrap();
    let request = resolver
        .bind_boot_preview(
            &route,
            &id,
            &id,
            normalized.clone(),
            normalized,
            "os:linux".into(),
        )
        .unwrap();
    let first = route.request(request.clone()).unwrap();
    let r::Body::Preview(body) = &first.body else {
        panic!()
    };
    assert_eq!(body.status, r::BodyPreviewStatus::Complete, "{body:?}");
    assert!(
        body.artifact
            .as_ref()
            .unwrap()
            .contains("retained closure body"),
        "{body:?}"
    );
    assert!(
        body.compiler_carrier_json
            .as_ref()
            .unwrap()
            .contains("normal")
    );
    std::fs::remove_file(package.join("vibevm/vibespecs/common/base.md")).unwrap();
    assert_eq!(first, route.request(request).unwrap());
}
