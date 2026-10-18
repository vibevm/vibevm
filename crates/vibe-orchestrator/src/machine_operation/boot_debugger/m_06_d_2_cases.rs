use super::*;
use crate::document_analysis::*;
use preparation::*;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Provider(Arc<AtomicUsize>);
impl PreparationProvider for Provider {
    fn prepare(&self, stage: &PreparationStage, _: &Path) -> StageOutcome {
        self.0.fetch_add(1, Ordering::SeqCst);
        match stage.kind {
            StageKind::Fetch => StageOutcome::Captured {
                evidence: StageEvidence {
                    files: [("boot.md".into(), b"exact captured bytes".to_vec())].into(),
                },
            },
            StageKind::Native => StageOutcome::Failed {
                reason: "native failed after fetch".into(),
            },
            StageKind::Hydration => StageOutcome::Unavailable {
                reason: "scoped hydration provider absent".into(),
            },
        }
    }
}
fn fixture(
    kinds: &[StageKind],
) -> (
    tempfile::TempDir,
    BootDebuggerRoute,
    Arc<PreparationService>,
    String,
) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let cache = temp.path().join("preview");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&cache).unwrap();
    std::fs::write(root.join("vibe.toml"), "[project]\n").unwrap();
    std::fs::write(root.join("STATIC.md"), "canonical").unwrap();
    let host = DocumentAnalysisHost::new(
        &root,
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "STATIC.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 4096,
            capture_bytes: 8192,
            open_buffers: 2,
        },
    )
    .unwrap();
    let artifact = host.resource("STATIC.md").unwrap();
    let capture = host.client().unwrap().freeze().unwrap();
    let route = BootDebuggerRoute::new(host, artifact, CapturedKind::Static).unwrap();
    let id = route.retain_capture(capture).unwrap();
    let service = Arc::new(
        PreparationService::open(
            &root,
            PreparationPlan {
                context: route.context_id().into(),
                capture: id.clone(),
                cache: cache.to_string_lossy().into(),
                stages: kinds
                    .iter()
                    .map(|kind| PreparationStage {
                        kind: *kind,
                        input: format!("exact {kind:?} capture"),
                        effects: vec!["preview cache only".into()],
                    })
                    .collect(),
            },
            "host-principal".into(),
        )
        .unwrap(),
    );
    route.register_preparation(service.clone()).unwrap();
    (temp, route, service, id)
}
fn request(route: &BootDebuggerRoute, command: q::Command) -> r::Result {
    route
        .request(q::Request {
            schema: 1,
            context_id: route.context_id().into(),
            command,
        })
        .unwrap()
}
fn prepare(service: &PreparationService) -> q::Command {
    q::Command::Prepare(Box::new(q::CommandPrepare {
        plan_id: service.id().into(),
    }))
}
fn read(service: &PreparationService) -> q::Command {
    q::Command::PreparationRead(Box::new(q::CommandPreparationRead {
        plan_id: service.id().into(),
    }))
}
#[test]
fn admitted_prepare_then_pure_inspect_compare_and_reopen_after_revocation() {
    let (temp, route, service, id) = fixture(&[StageKind::Fetch]);
    let calls = Arc::new(AtomicUsize::new(0));
    service
        .admit(Arc::new(Provider(calls.clone())), 60_000)
        .unwrap();
    let plan = request(
        &route,
        q::Command::PreparationPlan(Box::new(q::CommandPreparationPlan {
            plan_id: service.id().into(),
        })),
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(service.provider_invocations(), 0);
    let r::Body::Preparation(p) = plan.body else {
        panic!()
    };
    assert!(p.plan_json.contains("exact Fetch capture"));
    let first = request(&route, prepare(&service));
    let r::Body::Preparation(body) = &first.body else {
        panic!()
    };
    assert_eq!(body.status, r::BodyPreparationStatus::Complete);
    for _ in 0..2 {
        assert_eq!(first, request(&route, read(&service)));
        request(
            &route,
            q::Command::Inspect(Box::new(q::CommandInspect {
                snapshot_id: id.clone(),
            })),
        );
        request(
            &route,
            q::Command::Compare(Box::new(q::CommandCompare {
                snapshot_id: id.clone(),
                other_snapshot_id: id.clone(),
            })),
        );
    }
    service.revoke().unwrap();
    let reopened =
        PreparationService::open(route.root(), service.plan().clone(), "reader".into()).unwrap();
    assert_eq!(reopened.read().unwrap(), service.read().unwrap());
    assert_eq!(first, request(&route, prepare(&service)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.provider_invocations(), 1);
    assert_eq!(
        std::fs::read(temp.path().join("project/STATIC.md")).unwrap(),
        b"canonical"
    );
    assert!(!temp.path().join("project/.vibe").exists());
}
#[test]
fn partial_failure_preserves_valid_stage_durably_without_reinvocation() {
    let (_temp, route, service, _) = fixture(&[StageKind::Fetch, StageKind::Native]);
    let calls = Arc::new(AtomicUsize::new(0));
    service
        .admit(Arc::new(Provider(calls.clone())), 60_000)
        .unwrap();
    let result = request(&route, prepare(&service));
    let r::Body::Preparation(body) = &result.body else {
        panic!()
    };
    assert_eq!(body.status, r::BodyPreparationStatus::Partial);
    let outcome = service.read().unwrap().unwrap();
    assert!(
        matches!(&outcome.stages[0], StageOutcome::Captured { evidence } if evidence.files["boot.md"] == b"exact captured bytes")
    );
    assert!(
        matches!(&outcome.stages[1], StageOutcome::Failed { reason } if reason == "native failed after fetch")
    );
    service.revoke().unwrap();
    assert_eq!(result, request(&route, read(&service)));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(service.provider_invocations(), 2);
}
#[test]
fn absent_admission_empty_cache_and_unavailable_provider_never_fallback() {
    let (temp, route, service, id) = fixture(&[StageKind::Hydration]);
    request(
        &route,
        q::Command::Inspect(Box::new(q::CommandInspect { snapshot_id: id })),
    );
    let r::Body::Preparation(p) = request(&route, prepare(&service)).body else {
        panic!()
    };
    assert_eq!(p.status, r::BodyPreparationStatus::NeedsPreparation);
    assert!(!temp.path().join("preview/.vibe").exists());
    let calls = Arc::new(AtomicUsize::new(0));
    service
        .admit(Arc::new(Provider(calls.clone())), 60_000)
        .unwrap();
    let r::Body::Preparation(p) = request(&route, prepare(&service)).body else {
        panic!()
    };
    assert_eq!(p.status, r::BodyPreparationStatus::Unavailable);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(service.provider_invocations(), 1);
}
#[test]
fn project_cache_destination_is_refused() {
    let (temp, route, service, _) = fixture(&[StageKind::Fetch]);
    let mut plan = service.plan().clone();
    plan.cache = route.root().to_string_lossy().into();
    assert!(PreparationService::open(route.root(), plan.clone(), "host".into()).is_err());
    let foreign = temp.path().join("foreign-project");
    std::fs::create_dir(&foreign).unwrap();
    let misbound = Arc::new(PreparationService::open(&foreign, plan, "host".into()).unwrap());
    assert!(route.register_preparation(misbound).is_err());
    assert!(!route.root().join(".vibe").exists());
}

#[test]
fn refused_grant_and_revocation_before_prepare_leave_provider_counter_zero() {
    let (_temp, route, service, id) = fixture(&[StageKind::Fetch]);
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(service.admit(Arc::new(Provider(calls.clone())), 0).is_err());
    request(&route, prepare(&service));
    service
        .admit(Arc::new(Provider(calls.clone())), 60_000)
        .unwrap();
    request(
        &route,
        q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.clone(),
        })),
    );
    request(
        &route,
        q::Command::Compare(Box::new(q::CommandCompare {
            snapshot_id: id.clone(),
            other_snapshot_id: id,
        })),
    );
    service.revoke().unwrap();
    let r::Body::Preparation(body) = request(&route, prepare(&service)).body else {
        panic!()
    };
    assert_eq!(body.status, r::BodyPreparationStatus::NeedsPreparation);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(service.provider_invocations(), 0);
    assert!(service.read().unwrap().is_none());
}
