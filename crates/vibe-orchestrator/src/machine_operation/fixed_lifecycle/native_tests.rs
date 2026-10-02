//! Actual isolated fixed-owner route cases; no mock executor or second journal.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
use super::{FixedLifecycleRoute, FixedLifecycleRouteError, contract::*};
use crate::machine_operation::{input_decision as d, lifecycle_view as v};
use crate::machine_submission::{FixedLifecycleDecisionHost, HostPolicyFacts};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
mod cases;
const WAIT: Duration = Duration::from_secs(20);
pub(super) struct Fixture {
    pub(super) dir: tempfile::TempDir,
    pub(super) scope: SubmissionScope,
    pub(super) ns: SubmissionNamespace,
}
pub(super) fn facts(approve: bool) -> HostPolicyFacts {
    HostPolicyFacts {
        approve_captured_script: approve,
        allow_detach: true,
        allow_cancel_pre_dispatch: true,
        allow_cancel_deferred_boundary: true,
    }
}
impl Fixture {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vibe.toml"),"[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
        let root = dir.path().to_str().unwrap().replace('\'', "'\\''");
        fs::write(dir.path().join("run.sh"),format!("#!/bin/sh\nset -eu\ncd '{root}'\nprintf 'effect\\n' >> counter\nprintf 'native-output\\n'\ntouch entered\ni=0\nwhile [ ! -f release ]; do i=$((i+1)); if [ \"$i\" -gt 400 ]; then exit 70; fi; sleep 0.05; done\n")).unwrap();
        let scope = SubmissionScope {
            context: "mcp-native".into(),
            target: "root".into(),
            principal: "trusted-host".into(),
        };
        let lease = LifecycleLease::acquire(dir.path()).unwrap();
        let ns = SubmissionReservations::issue_namespace(&lease, &scope, 120_000).unwrap();
        drop(lease);
        Self { dir, scope, ns }
    }
    pub(super) fn host(&self, approve: bool) -> Arc<FixedLifecycleDecisionHost> {
        Arc::new(
            FixedLifecycleDecisionHost::from_host(
                self.dir.path().into(),
                self.ns.clone(),
                self.scope.clone(),
                TargetOs::Linux,
                facts(approve),
            )
            .unwrap(),
        )
    }
    pub(super) fn release(&self) {
        fs::write(self.dir.path().join("release"), "go").unwrap();
    }
    pub(super) fn count(&self) {
        assert_eq!(
            fs::read(self.dir.path().join("counter")).unwrap(),
            b"effect\n"
        );
    }
    pub(super) fn no_provider(&self) {
        assert!(!self.dir.path().join("counter").exists());
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::write(self.dir.path().join("release"), "cleanup");
    }
}
pub(super) fn until(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + WAIT;
    while !f() {
        assert!(Instant::now() < end, "native deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn request(action: Action) -> LifecycleActionRequest {
    LifecycleActionRequest { schema: 1, action }
}
fn bytes(request: &LifecycleActionRequest) -> Vec<u8> {
    serde_json::to_vec(request).unwrap()
}
fn call(
    route: &FixedLifecycleRoute,
    root: &std::path::Path,
    action: Action,
) -> Result<LifecycleActionReply, FixedLifecycleRouteError> {
    FixedLifecycleRoute::dispatch(Some(route), &bytes(&request(action)), root)
}
fn prepare(control: RequestedOptionsControl) -> Action {
    Action::Prepare(Box::new(ActionPrepare {
        phase: "validate".into(),
        requested_options: RequestedOptions {
            force: false,
            trace: false,
            deploy_profile: None,
            control,
        },
    }))
}
fn answer(q: &d::InputDecisionMessage) -> Action {
    Action::Answer(Box::new(ActionAnswer {
        message_json: serde_json::to_string(q).unwrap(),
    }))
}
fn question(reply: LifecycleActionReply) -> d::InputDecisionMessage {
    let Reply::Question(q) = reply.reply else {
        panic!("question");
    };
    serde_json::from_str(&q.payload_json).unwrap()
}
fn submit(reference: &str, key: &str) -> Action {
    Action::Submit(Box::new(ActionSubmit {
        accepted_ref: reference.into(),
        key: key.into(),
    }))
}
fn inspect(key: &str) -> Action {
    Action::Inspect(Box::new(ActionInspect { key: key.into() }))
}
fn row(reply: LifecycleActionReply) -> v::LifecycleSubmissionView {
    let text = match reply.reply {
        Reply::View(v) => v.payload_json,
        Reply::Submitted(v) => v.payload_json,
        _ => panic!("view"),
    };
    serde_json::from_str(&text).unwrap()
}
fn approve(
    route: &FixedLifecycleRoute,
    root: &std::path::Path,
    control: RequestedOptionsControl,
) -> String {
    let mut q = question(call(route, root, prepare(control)).unwrap());
    if let d::Body::SelectionQuestion(choices) = &q.body {
        assert!(choices.choices.iter().any(|c| c.id == "none"));
        q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        q = question(call(route, root, answer(&q)).unwrap());
    }
    let mut wrong = q.clone();
    wrong.plan_id = Some("foreign-plan".into());
    wrong.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        call(route, root, answer(&wrong)).unwrap_err().error.code,
        LifecycleActionErrorCode::DecisionRefused
    );
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let id = q.question_id.clone().unwrap();
    for _ in 0..2 {
        let Reply::Validated(v) = call(route, root, answer(&q)).unwrap().reply else {
            panic!("validated");
        };
        assert_eq!(v.accepted_ref, id);
        assert_eq!(Some(v.plan_id), q.plan_id);
    }
    id
}
