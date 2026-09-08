//! Relational admission for the stateless native build-provider wire.

use crate::generated::native::e1::{build_reply as reply, build_request as request};
use std::fmt;

mod admission;
use admission::*;

pub const ENVELOPE_EPOCH: u32 = 1;
pub const REPLY_CAP_BYTES: usize = 24 * 1024 * 1024;
const COLLECTION_CAP: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOperation {
    Plan,
    Fingerprint,
    Apply,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBuildError {
    law: &'static str,
    message: &'static str,
}

impl NativeBuildError {
    pub const fn law(&self) -> &'static str {
        self.law
    }
}

impl fmt::Display for NativeBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native build {}: {}", self.law, self.message)
    }
}

impl std::error::Error for NativeBuildError {}

#[derive(Debug, Clone, Copy, Default)]
pub struct RetainedBuild<'a> {
    pub target: Option<&'a request::BuildTarget>,
    pub authority: Option<&'a request::BuildAuthority>,
    pub plan: Option<&'a request::BuildPlan>,
    pub fingerprint: Option<&'a request::BuildFingerprint>,
    pub staging: Option<&'a request::StagingAuthority>,
    pub staged: Option<&'a [request::StagedOutput]>,
}

pub fn decode_reply(raw: &[u8]) -> Result<reply::BuildReply, NativeBuildError> {
    if raw.len() > REPLY_CAP_BYTES {
        return Err(fault("reply-cap", "reply exceeds the fixed byte cap"));
    }
    serde_json::from_slice(raw).map_err(|_| fault("reply-json", "reply is not strict JSON"))
}

pub fn validate_request(
    value: &request::BuildRequest,
    expected_provider: &str,
    expected_mechanism: &str,
    expected_target: &str,
    retained: RetainedBuild<'_>,
) -> Result<BuildOperation, NativeBuildError> {
    validate_expected(expected_provider, expected_mechanism, expected_target)?;
    let operation = match value {
        request::BuildRequest::Plan(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.target.is_none()
                    && retained.authority.is_none()
                    && retained.plan.is_none()
                    && retained.fingerprint.is_none()
                    && retained.staging.is_none()
                    && retained.staged.is_none(),
            )?;
            BuildOperation::Plan
        }
        request::BuildRequest::Fingerprint(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.target == Some(&v.target)
                    && retained.authority == Some(&v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint.is_none()
                    && retained.staging.is_none()
                    && retained.staged.is_none(),
            )?;
            request_plan(&v.plan, &v.target)?;
            BuildOperation::Fingerprint
        }
        request::BuildRequest::Apply(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.target == Some(&v.target)
                    && retained.authority == Some(&v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint == Some(&v.fingerprint)
                    && retained.staging.is_none()
                    && retained.staged.is_none(),
            )?;
            request_plan(&v.plan, &v.target)?;
            request_fingerprint(&v.fingerprint)?;
            staging(&v.staging, &v.authority)?;
            BuildOperation::Apply
        }
        request::BuildRequest::Verify(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.target == Some(&v.target)
                    && retained.authority == Some(&v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint == Some(&v.fingerprint)
                    && retained.staging == Some(&v.staging)
                    && retained.staged == Some(&v.staged),
            )?;
            request_plan(&v.plan, &v.target)?;
            request_fingerprint(&v.fingerprint)?;
            staging(&v.staging, &v.authority)?;
            staged(&v.staged, &v.plan)?;
            BuildOperation::Verify
        }
    };
    Ok(operation)
}

pub fn validate_reply(
    expected: BuildOperation,
    value: &reply::BuildReply,
) -> Result<(), NativeBuildError> {
    let (operation, envelope, protocol) = reply_head(value);
    epoch(envelope)?;
    epoch(protocol)?;
    if operation != expected {
        return Err(fault(
            "reply-operation",
            "reply operation differs from retained request",
        ));
    }
    match value {
        reply::BuildReply::Plan(value) => match &value.result {
            reply::PlanResult::Ok(ok) => reply_plan(&ok.plan),
            reply::PlanResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::BuildReply::Fingerprint(value) => match &value.result {
            reply::FingerprintResult::Ok(ok) => reply_fingerprint(&ok.fingerprint),
            reply::FingerprintResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::BuildReply::Apply(value) => match &value.result {
            reply::ApplyResult::Ok(ok) => {
                diagnostic(&ok.evidence, "evidence", true)?;
                reply_staged(&ok.staged)
            }
            reply::ApplyResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::BuildReply::Verify(value) => match &value.result {
            reply::VerifyResult::Ok(ok) => {
                diagnostic(&ok.evidence, "evidence", true)?;
                verified(&ok.verified)
            }
            reply::VerifyResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
    }
}

pub fn validate_exchange(
    request: &request::BuildRequest,
    expected_provider: &str,
    expected_mechanism: &str,
    expected_target: &str,
    retained: RetainedBuild<'_>,
    reply: &reply::BuildReply,
) -> Result<(), NativeBuildError> {
    let operation = validate_request(
        request,
        expected_provider,
        expected_mechanism,
        expected_target,
        retained,
    )?;
    validate_reply(operation, reply)?;
    match (request, reply) {
        (request::BuildRequest::Plan(request), reply::BuildReply::Plan(reply)) => {
            if let reply::PlanResult::Ok(ok) = &reply.result {
                exact_plan_outputs(&request.target.outputs, &ok.plan.outputs)?;
            }
        }
        (request::BuildRequest::Apply(request), reply::BuildReply::Apply(reply)) => {
            if let reply::ApplyResult::Ok(ok) = &reply.result {
                exact_staged_outputs(&request.plan, &ok.staged)?;
            }
        }
        (request::BuildRequest::Verify(request), reply::BuildReply::Verify(reply)) => {
            if let reply::VerifyResult::Ok(ok) = &reply.result {
                exact_verified_outputs(&request.staged, &ok.verified)?;
            }
        }
        (request::BuildRequest::Fingerprint(_), reply::BuildReply::Fingerprint(_)) => {}
        _ => unreachable!("reply operation was validated above"),
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::{Value, json};

    use super::*;

    const PROVIDER: &str = "org.example/tools#builder";
    const MECHANISM: &str = "build:cargo";
    const TARGET: &str = "demo-build";

    fn digest() -> String {
        "a".repeat(64)
    }
    fn target() -> Value {
        json!({
            "id": TARGET, "workdir": ".", "config_toml": "release = true\n",
            "inputs": [{"kind":"path","path_relative":"Cargo.toml"}, {"kind":"artifact","artifact":"generated"}],
            "outputs": [
                {"id":"demo.exe","kind":"executable","select_toml":"bin = \"demo\"\n"},
                {"id":"demo.map","kind":"file"}
            ]
        })
    }
    fn authority() -> Value {
        json!({"project_root_absolute":"C:/workspace","build_root_absolute":"C:/workspace/target/vibe-build","build_root_relative":"target/vibe-build","offline":true})
    }
    fn plan() -> Value {
        json!({"summary":"build two outputs","outputs":[
            {"id":"demo.exe","kind":"executable","shape":"file","path_relative":"demo.exe"},
            {"id":"demo.map","kind":"file","shape":"file","path_relative":"demo.map"}
        ]})
    }
    fn fingerprint() -> Value {
        json!({"digest":digest(),"summary":"toolchain exact"})
    }
    fn staging() -> Value {
        json!({"root_absolute":"C:/workspace/target/vibe-build/stage","root_relative":"target/vibe-build/stage"})
    }
    fn staged() -> Value {
        json!([
            {"id":"demo.exe","kind":"executable","shape":"file","path_relative":"demo.exe","fresh":false},
            {"id":"demo.map","kind":"file","shape":"file","path_relative":"demo.map","fresh":false}
        ])
    }
    pub(super) fn request_value(operation: &str) -> Value {
        let mut value = match operation {
            "plan" => json!({}),
            "fingerprint" => json!({"plan":plan()}),
            "apply" => json!({"plan":plan(),"fingerprint":fingerprint(),"staging":staging()}),
            "verify" => {
                json!({"plan":plan(),"fingerprint":fingerprint(),"staging":staging(),"staged":staged()})
            }
            _ => unreachable!(),
        };
        let object = value.as_object_mut().unwrap();
        object.insert("operation".into(), operation.into());
        object.insert("envelope".into(), 1.into());
        object.insert("protocol".into(), 1.into());
        object.insert(
            "identity".into(),
            json!({"provider":PROVIDER,"mechanism":MECHANISM,"target":TARGET}),
        );
        object.insert("target".into(), target());
        object.insert("authority".into(), authority());
        value
    }
    pub(super) fn reply_value(operation: &str) -> Value {
        let result = match operation {
            "plan" => json!({"status":"ok","plan":{"summary":"build two outputs","outputs":[
                {"id":"demo.exe","kind":"executable","shape":"file","path_relative":"demo.exe"},
                {"id":"demo.map","kind":"file","shape":"file","path_relative":"demo.map"}]}}),
            "fingerprint" => json!({"status":"ok","fingerprint":fingerprint()}),
            "apply" => json!({"status":"ok","staged":[
                {"id":"demo.exe","path_relative":"demo.exe","fresh":false},
                {"id":"demo.map","path_relative":"demo.map","fresh":false}],"evidence":"built"}),
            "verify" => json!({"status":"ok","verified":[
                {"id":"demo.exe","path_relative":"demo.exe","digest":digest(),"bytes":"7"},
                {"id":"demo.map","path_relative":"demo.map","digest":digest(),"bytes":"2"}],"evidence":"verified"}),
            _ => unreachable!(),
        };
        json!({"operation":operation,"envelope":1,"protocol":1,"result":result})
    }
    fn request(operation: &str) -> request::BuildRequest {
        serde_json::from_value(request_value(operation)).unwrap()
    }
    fn reply(operation: &str) -> reply::BuildReply {
        serde_json::from_value(reply_value(operation)).unwrap()
    }
    struct Accepted {
        target: request::BuildTarget,
        authority: request::BuildAuthority,
        plan: request::BuildPlan,
        fingerprint: request::BuildFingerprint,
        staging: request::StagingAuthority,
        staged: Vec<request::StagedOutput>,
    }
    fn accepted() -> Accepted {
        Accepted {
            target: serde_json::from_value(target()).unwrap(),
            authority: serde_json::from_value(authority()).unwrap(),
            plan: serde_json::from_value(plan()).unwrap(),
            fingerprint: serde_json::from_value(fingerprint()).unwrap(),
            staging: serde_json::from_value(staging()).unwrap(),
            staged: serde_json::from_value(staged()).unwrap(),
        }
    }
    fn retained<'a>(operation: &str, accepted: &'a Accepted) -> RetainedBuild<'a> {
        match operation {
            "plan" => RetainedBuild::default(),
            "fingerprint" => RetainedBuild {
                target: Some(&accepted.target),
                authority: Some(&accepted.authority),
                plan: Some(&accepted.plan),
                ..RetainedBuild::default()
            },
            "apply" => RetainedBuild {
                target: Some(&accepted.target),
                authority: Some(&accepted.authority),
                plan: Some(&accepted.plan),
                fingerprint: Some(&accepted.fingerprint),
                staging: None,
                staged: None,
            },
            "verify" => RetainedBuild {
                target: Some(&accepted.target),
                authority: Some(&accepted.authority),
                plan: Some(&accepted.plan),
                fingerprint: Some(&accepted.fingerprint),
                staging: Some(&accepted.staging),
                staged: Some(&accepted.staged),
            },
            _ => unreachable!(),
        }
    }

    fn assert_chain_refusal(value: Value, operation: &str, accepted: &Accepted) {
        let request = serde_json::from_value(value).unwrap();
        assert_eq!(
            validate_request(
                &request,
                PROVIDER,
                MECHANISM,
                TARGET,
                retained(operation, accepted),
            )
            .unwrap_err()
            .law(),
            "accepted-chain"
        );
    }

    #[test]
    fn all_four_stateless_exchanges_preserve_the_accepted_chain() {
        let accepted = accepted();
        for (name, operation) in [
            ("plan", BuildOperation::Plan),
            ("fingerprint", BuildOperation::Fingerprint),
            ("apply", BuildOperation::Apply),
            ("verify", BuildOperation::Verify),
        ] {
            let request = request(name);
            let retained = retained(name, &accepted);
            assert_eq!(
                validate_request(&request, PROVIDER, MECHANISM, TARGET, retained).unwrap(),
                operation
            );
            validate_exchange(
                &request,
                PROVIDER,
                MECHANISM,
                TARGET,
                retained,
                &reply(name),
            )
            .unwrap();
        }
    }

    #[test]
    fn identity_config_path_set_and_chain_faults_are_typed() {
        let accepted = accepted();
        for (mut value, expected) in [
            (
                {
                    let mut v = request_value("plan");
                    v["identity"]["provider"] = "other".into();
                    v
                },
                "exact-identity",
            ),
            (
                {
                    let mut v = request_value("plan");
                    v["target"]["config_toml"] = "z=1\na=2\n".into();
                    v
                },
                "canonical-config",
            ),
            (
                {
                    let mut v = request_value("plan");
                    v["authority"]["build_root_absolute"] = "C:/elsewhere".into();
                    v
                },
                "paths",
            ),
            (
                {
                    let mut v = request_value("plan");
                    let row = v["target"]["outputs"][0].clone();
                    v["target"]["outputs"].as_array_mut().unwrap().push(row);
                    v
                },
                "declared-sets",
            ),
        ] {
            let request: request::BuildRequest = serde_json::from_value(value.take()).unwrap();
            assert_eq!(
                validate_request(
                    &request,
                    PROVIDER,
                    MECHANISM,
                    TARGET,
                    RetainedBuild::default()
                )
                .unwrap_err()
                .law(),
                expected
            );
        }
        let apply = request("apply");
        assert_eq!(
            validate_request(
                &apply,
                PROVIDER,
                MECHANISM,
                TARGET,
                RetainedBuild {
                    fingerprint: None,
                    ..retained("apply", &accepted)
                }
            )
            .unwrap_err()
            .law(),
            "accepted-chain"
        );
        let verify = request("verify");
        let mut wrong_staged = accepted.staged.clone();
        wrong_staged.reverse();
        assert_eq!(
            validate_request(
                &verify,
                PROVIDER,
                MECHANISM,
                TARGET,
                RetainedBuild {
                    staged: Some(&wrong_staged),
                    ..retained("verify", &accepted)
                }
            )
            .unwrap_err()
            .law(),
            "accepted-chain"
        );

        for pointer in [
            "/target/config_toml",
            "/target/inputs/0/path_relative",
            "/target/outputs/0/select_toml",
            "/authority/offline",
        ] {
            let mut drift = request_value("fingerprint");
            *drift.pointer_mut(pointer).unwrap() = match pointer {
                "/target/config_toml" => "release = false\n".into(),
                "/target/inputs/0/path_relative" => "src/lib.rs".into(),
                "/target/outputs/0/select_toml" => "bin = \"other\"\n".into(),
                _ => false.into(),
            };
            assert_chain_refusal(drift, "fingerprint", &accepted);
        }
        let mut root = request_value("apply");
        root["authority"]["project_root_absolute"] = "C:/other".into();
        root["authority"]["build_root_absolute"] = "C:/other/target/vibe-build".into();
        assert_chain_refusal(root, "apply", &accepted);
        let mut stage = request_value("verify");
        stage["staging"]["root_absolute"] = "C:/workspace/target/other".into();
        stage["staging"]["root_relative"] = "target/other".into();
        assert_chain_refusal(stage, "verify", &accepted);
    }

    #[test]
    fn reply_caps_digests_shapes_and_exact_order_refuse_without_echo() {
        assert_eq!(
            decode_reply(&vec![b'x'; REPLY_CAP_BYTES + 1])
                .unwrap_err()
                .law(),
            "reply-cap"
        );
        for (operation, mutate, expected) in [
            ("plan", ("shape", "pipe"), "plan-outputs"),
            ("fingerprint", ("digest", "BAD"), "fingerprint"),
            ("verify", ("bytes", "01"), "verification-rows"),
        ] {
            let mut value = reply_value(operation);
            match operation {
                "plan" => value["result"]["plan"]["outputs"][0][mutate.0] = mutate.1.into(),
                "fingerprint" => value["result"]["fingerprint"][mutate.0] = mutate.1.into(),
                "verify" => value["result"]["verified"][0][mutate.0] = mutate.1.into(),
                _ => unreachable!(),
            }
            assert_eq!(
                validate_reply(
                    match operation {
                        "plan" => BuildOperation::Plan,
                        "fingerprint" => BuildOperation::Fingerprint,
                        _ => BuildOperation::Verify,
                    },
                    &serde_json::from_value(value).unwrap()
                )
                .unwrap_err()
                .law(),
                expected
            );
        }
        let accepted = accepted();
        let request = request("apply");
        let mut wrong = reply_value("apply");
        wrong["result"]["staged"].as_array_mut().unwrap().reverse();
        assert_eq!(
            validate_exchange(
                &request,
                PROVIDER,
                MECHANISM,
                TARGET,
                retained("apply", &accepted),
                &serde_json::from_value(wrong).unwrap()
            )
            .unwrap_err()
            .law(),
            "staged-outputs"
        );
        let mut fail = reply_value("apply");
        fail["result"] = json!({"status":"fail","message":"bad\nbody"});
        let error = validate_reply(
            BuildOperation::Apply,
            &serde_json::from_value(fail).unwrap(),
        )
        .unwrap_err();
        assert_eq!(error.law(), "fail-message");
        assert!(error.to_string().len() < 160);
    }
}
