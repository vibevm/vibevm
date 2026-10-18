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
fn registered_local_fetch_captures_real_bytes_and_reads_after_source_revocation() {
    use vibe_orchestrator::machine_operation::boot_debugger::preparation::*;
    let (temp, resolver, route, id, _) = fixture();
    let before = project_bytes(route.root());
    let cache = temp.path().join("preview-cache");
    std::fs::create_dir(&cache).unwrap();
    let (service, plan) = resolver
        .bind_boot_preparation(&route, &id, &cache, "host".into(), Some(60_000))
        .unwrap();
    let r::Body::Preparation(p) = route.request(plan).unwrap().body else {
        panic!()
    };
    assert!(p.plan_json.contains("content=sha256:"));
    assert!(!cache.join("sources").exists());
    assert_eq!(service.provider_invocations(), 0);
    let make = |command| q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command,
    };
    let prepared = route
        .request(make(q::Command::Prepare(Box::new(q::CommandPrepare {
            plan_id: service.id().into(),
        }))))
        .unwrap();
    let r::Body::Preparation(p) = &prepared.body else {
        panic!()
    };
    assert_eq!(p.status, r::BodyPreparationStatus::Complete, "{p:?}");
    assert!(cache.join("sources").exists());
    let outcome = service.read().unwrap().unwrap();
    assert!(
        matches!(&outcome.stages[0], StageOutcome::Captured { evidence } if evidence.files["boot.md"].ends_with(b"source bytes\n"))
    );
    service.revoke().unwrap();
    std::fs::remove_dir_all(temp.path().join("registry")).unwrap();
    let reopened =
        PreparationService::open(route.root(), service.plan().clone(), "reader".into()).unwrap();
    assert_eq!(Some(outcome), reopened.read().unwrap());
    for _ in 0..2 {
        assert_eq!(
            prepared,
            route
                .request(make(q::Command::PreparationRead(Box::new(
                    q::CommandPreparationRead {
                        plan_id: service.id().into()
                    }
                ))))
                .unwrap()
        );
        route
            .request(make(q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: id.clone(),
            }))))
            .unwrap();
        route
            .request(make(q::Command::Compare(Box::new(q::CommandCompare {
                snapshot_id: id.clone(),
                other_snapshot_id: id.clone(),
            }))))
            .unwrap();
    }
    assert_eq!(service.provider_invocations(), 1);
    assert_eq!(before, project_bytes(route.root()));
}
#[test]
fn changed_local_source_is_typed_failed_and_never_retried() {
    use vibe_orchestrator::machine_operation::boot_debugger::preparation::*;
    let (temp, resolver, route, id, _) = fixture();
    let cache = temp.path().join("preview-cache");
    std::fs::create_dir(&cache).unwrap();
    let (service, _) = resolver
        .bind_boot_preparation(&route, &id, &cache, "host".into(), Some(60_000))
        .unwrap();
    std::fs::write(
        temp.path().join("registry/org.example/app/v1.0.0/boot.md"),
        b"drift",
    )
    .unwrap();
    let q = q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command: q::Command::Prepare(Box::new(q::CommandPrepare {
            plan_id: service.id().into(),
        })),
    };
    let result = route.request(q.clone()).unwrap();
    assert!(matches!(
        service.read().unwrap().unwrap().stages[0],
        StageOutcome::Failed { .. }
    ));
    assert!(!cache.join("sources").exists());
    assert_eq!(result, route.request(q).unwrap());
}

#[test]
fn real_fetch_then_missing_native_producer_retains_partial_evidence() {
    use vibe_orchestrator::machine_operation::boot_debugger::preparation::*;
    let (temp, resolver, route, id, _) = fixture();
    let manifest = temp
        .path()
        .join("registry/org.example/app/v1.0.0/vibe.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[[extension]]\nid='native'\npoint='compile:emitted'\nhandler={kind='native',crate_dir='never-built'}\n");
    std::fs::write(manifest, text).unwrap();
    let before = project_bytes(route.root());
    let cache = temp.path().join("preview-cache");
    std::fs::create_dir(&cache).unwrap();
    let (service, plan) = resolver
        .bind_boot_preparation(&route, &id, &cache, "host".into(), Some(60_000))
        .unwrap();
    let r::Body::Preparation(p) = route.request(plan).unwrap().body else {
        panic!()
    };
    assert!(p.plan_json.contains("native"));
    let make = |command| q::Request {
        schema: 1,
        context_id: route.context_id().into(),
        command,
    };
    let prepared = route
        .request(make(q::Command::Prepare(Box::new(q::CommandPrepare {
            plan_id: service.id().into(),
        }))))
        .unwrap();
    let r::Body::Preparation(p) = &prepared.body else {
        panic!()
    };
    assert_eq!(p.status, r::BodyPreparationStatus::Partial);
    let outcome = service.read().unwrap().unwrap();
    assert!(
        matches!(&outcome.stages[0], StageOutcome::Captured { evidence } if evidence.files["boot.md"].ends_with(b"source bytes\n"))
    );
    assert!(
        matches!(&outcome.stages[1], StageOutcome::Unavailable { reason } if reason.contains("Native"))
    );
    service.revoke().unwrap();
    std::fs::remove_dir_all(temp.path().join("registry")).unwrap();
    assert_eq!(
        prepared,
        route
            .request(make(q::Command::PreparationRead(Box::new(
                q::CommandPreparationRead {
                    plan_id: service.id().into()
                }
            ))))
            .unwrap()
    );
    assert_eq!(service.provider_invocations(), 2);
    assert_eq!(before, project_bytes(route.root()));
    assert!(!cache.join("never-built").exists());
}
