//! Real owner-to-envelope fixtures; no mock owner or second journal.
use super::contract::{err, rep};
use super::*;
use serde_json::{Value, json};
use vibe_wire::generated::{format_id::FormatId, machine::e1::operation_envelope as e};

struct Fixture {
    root: tempfile::TempDir,
    project: Arc<Project>,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source.bin"), b"copy once").unwrap();
        let project = Arc::new(Project::open(root.path()).unwrap());
        let scope = SubmissionScope {
            context: "adapter-context".into(),
            target: "launcher".into(),
            principal: "adapter-principal".into(),
        };
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        Self {
            root,
            project,
            namespace,
            scope,
        }
    }
    fn host(&self) -> Arc<FixedPackageHost> {
        Arc::new(
            FixedPackageHost::from_trusted_host(
                self.project.clone(),
                self.namespace.clone(),
                self.scope.clone(),
            )
            .unwrap(),
        )
    }
    fn route(&self) -> FixedPackageRoute {
        FixedPackageRoute::new(self.host())
    }
    fn output(&self) -> std::path::PathBuf {
        self.root
            .path()
            .join("target/vibe-package/launcher/launcher.bin")
    }
    fn exchange(&self, route: &FixedPackageRoute, action: &str, key: &str) -> e::OperationEnvelope {
        FixedPackageRoute::exchange_local(Some(route), &frame(action, key), self.root.path())
            .unwrap()
    }
}
fn frame(action: &str, key: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"input","format_id":FormatId::MachineFixedPackageActionRequest.id(),
        "payload_json":serde_json::to_string(&json!({"schema":1,"action":{"kind":action,"key":key}})).unwrap()}})).unwrap()
}
fn roundtrip(outer: &e::OperationEnvelope) {
    let b = serde_json::to_vec(outer).unwrap();
    assert_eq!(
        serde_json::from_slice::<e::OperationEnvelope>(&b).unwrap(),
        *outer
    );
    assert_eq!(outer.operation_id.as_deref(), Some(OPERATION_ID));
    assert_eq!(outer.domain_owner.as_deref(), Some(DOMAIN_OWNER));
    assert!(outer.source_id.is_none() && outer.evidence_id.is_none());
}
fn reply(outer: &e::OperationEnvelope) -> rep::FixedPackageActionReply {
    roundtrip(outer);
    let e::Body::Result(body) = &outer.body else {
        panic!("expected result: {outer:?}")
    };
    assert_eq!(
        body.format_id.as_deref(),
        Some(FormatId::MachineFixedPackageActionReply.id())
    );
    let reply: rep::FixedPackageActionReply =
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(reply.schema, 1);
    assert_eq!(body.provenance_id.as_deref(), contract::run(&reply.reply));
    reply
}
fn error(outer: &e::OperationEnvelope) -> err::FixedPackageActionError {
    roundtrip(outer);
    let e::Body::Error(body) = &outer.body else {
        panic!("expected error: {outer:?}")
    };
    assert_eq!(
        body.format_id.as_deref(),
        Some(FormatId::MachineFixedPackageActionError.id())
    );
    let error: err::FixedPackageActionError =
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(error.schema, 1);
    assert_eq!(body.provenance_id, error.known_run_id);
    error
}
fn completed(outer: &e::OperationEnvelope) -> String {
    let rep::Reply::Completed(r) = reply(outer).reply else {
        panic!("expected completed")
    };
    let e::Body::Result(b) = &outer.body else {
        unreachable!()
    };
    assert_eq!(b.effect_state, e::BodyResultEffectState::Completed);
    assert_eq!(b.completeness, e::BodyResultCompleteness::Complete);
    assert_eq!(r.artifact_record, ".vibe/state/artifacts/launcher.bin.json");
    r.run_id
}

mod negative;
mod positive;
mod publication;
