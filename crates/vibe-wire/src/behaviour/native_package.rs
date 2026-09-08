//! Relational admission for the stateless native package-provider wire.

use crate::generated::native::e1::{package_reply as reply, package_request as request};
use std::fmt;

mod admission;
use admission::*;

pub const ENVELOPE_EPOCH: u32 = 1;
pub const REPLY_CAP_BYTES: usize = 24 * 1024 * 1024;
const COLLECTION_CAP: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageOperation {
    Plan,
    Fingerprint,
    Apply,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePackageError {
    law: &'static str,
    message: &'static str,
}

impl NativePackageError {
    pub const fn law(&self) -> &'static str {
        self.law
    }
}

impl fmt::Display for NativePackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native package {}: {}", self.law, self.message)
    }
}

impl std::error::Error for NativePackageError {}

#[derive(Debug, Clone, Copy, Default)]
pub struct RetainedPackage<'a> {
    pub target: Option<&'a request::PackageTarget>,
    pub inputs: Option<&'a [request::ResolvedInput]>,
    pub authority: Option<&'a request::PackageAuthority>,
    pub plan: Option<&'a request::PackagePlan>,
    pub fingerprint: Option<&'a request::PackageFingerprint>,
    pub staging: Option<&'a request::StagingAuthority>,
    pub staged: Option<&'a [request::StagedOutput]>,
}

pub fn decode_reply(raw: &[u8]) -> Result<reply::PackageReply, NativePackageError> {
    if raw.len() > REPLY_CAP_BYTES {
        return Err(fault("reply-cap", "reply exceeds the fixed byte cap"));
    }
    serde_json::from_slice(raw).map_err(|_| fault("reply-json", "reply is not strict JSON"))
}

fn diagnostic(value: &str, law: &'static str, nonblank: bool) -> Result<(), NativePackageError> {
    if value.len() > crate::behaviour::native_mechanism::DIAGNOSTIC_CAP_BYTES
        || value.chars().any(char::is_control)
        || (nonblank && value.trim().is_empty())
    {
        Err(fault(law, "text violates its bounded control-free law"))
    } else {
        Ok(())
    }
}

const fn fault(law: &'static str, message: &'static str) -> NativePackageError {
    NativePackageError { law, message }
}

pub fn validate_request(
    value: &request::PackageRequest,
    expected_provider: &str,
    expected_mechanism: &str,
    expected_target: &str,
    retained: RetainedPackage<'_>,
) -> Result<PackageOperation, NativePackageError> {
    validate_expected(expected_provider, expected_mechanism, expected_target)?;
    let operation = match value {
        request::PackageRequest::Plan(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.inputs,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(retained.is_empty())?;
            PackageOperation::Plan
        }
        request::PackageRequest::Fingerprint(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.inputs,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.prefix(&v.target, &v.inputs, &v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint.is_none()
                    && retained.staging.is_none()
                    && retained.staged.is_none(),
            )?;
            request_plan(&v.plan, &v.target)?;
            PackageOperation::Fingerprint
        }
        request::PackageRequest::Apply(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.inputs,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.prefix(&v.target, &v.inputs, &v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint == Some(&v.fingerprint)
                    && retained.staging.is_none()
                    && retained.staged.is_none(),
            )?;
            request_plan(&v.plan, &v.target)?;
            request_fingerprint(&v.fingerprint, v.inputs.len())?;
            staging(&v.staging, &v.authority)?;
            PackageOperation::Apply
        }
        request::PackageRequest::Verify(v) => {
            base(
                v.envelope,
                v.protocol,
                &v.identity,
                &v.target,
                &v.inputs,
                &v.authority,
                expected_provider,
                expected_mechanism,
                expected_target,
            )?;
            require_chain(
                retained.prefix(&v.target, &v.inputs, &v.authority)
                    && retained.plan == Some(&v.plan)
                    && retained.fingerprint == Some(&v.fingerprint)
                    && retained.staging == Some(&v.staging)
                    && retained.staged == Some(&v.staged),
            )?;
            request_plan(&v.plan, &v.target)?;
            request_fingerprint(&v.fingerprint, v.inputs.len())?;
            staging(&v.staging, &v.authority)?;
            staged(&v.staged, &v.plan)?;
            PackageOperation::Verify
        }
    };
    Ok(operation)
}

impl RetainedPackage<'_> {
    fn is_empty(self) -> bool {
        self.target.is_none()
            && self.inputs.is_none()
            && self.authority.is_none()
            && self.plan.is_none()
            && self.fingerprint.is_none()
            && self.staging.is_none()
            && self.staged.is_none()
    }

    fn prefix(
        self,
        target: &request::PackageTarget,
        inputs: &[request::ResolvedInput],
        authority: &request::PackageAuthority,
    ) -> bool {
        self.target == Some(target)
            && self.inputs == Some(inputs)
            && self.authority == Some(authority)
    }
}

pub fn validate_reply(
    expected: PackageOperation,
    value: &reply::PackageReply,
) -> Result<(), NativePackageError> {
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
        reply::PackageReply::Plan(v) => match &v.result {
            reply::PlanResult::Ok(ok) => reply_plan(&ok.plan),
            reply::PlanResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::PackageReply::Fingerprint(v) => match &v.result {
            reply::FingerprintResult::Ok(ok) => reply_fingerprint(&ok.fingerprint),
            reply::FingerprintResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::PackageReply::Apply(v) => match &v.result {
            reply::ApplyResult::Ok(ok) => {
                diagnostic(&ok.evidence, "evidence", true)?;
                reply_staged(&ok.staged)
            }
            reply::ApplyResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
        reply::PackageReply::Verify(v) => match &v.result {
            reply::VerifyResult::Ok(ok) => {
                diagnostic(&ok.evidence, "evidence", true)?;
                verified(&ok.verified)
            }
            reply::VerifyResult::Fail(fail) => diagnostic(&fail.message, "fail-message", true),
        },
    }
}

pub fn validate_exchange(
    request: &request::PackageRequest,
    expected_provider: &str,
    expected_mechanism: &str,
    expected_target: &str,
    retained: RetainedPackage<'_>,
    reply: &reply::PackageReply,
) -> Result<(), NativePackageError> {
    let operation = validate_request(
        request,
        expected_provider,
        expected_mechanism,
        expected_target,
        retained,
    )?;
    validate_reply(operation, reply)?;
    match (request, reply) {
        (request::PackageRequest::Plan(request), reply::PackageReply::Plan(reply)) => {
            if let reply::PlanResult::Ok(ok) = &reply.result {
                exact_plan_outputs(&request.target.outputs, &ok.plan.outputs)?;
            }
        }
        (
            request::PackageRequest::Fingerprint(request),
            reply::PackageReply::Fingerprint(reply),
        ) => {
            if let reply::FingerprintResult::Ok(ok) = &reply.result {
                exact_fingerprint(&ok.fingerprint, request.inputs.len())?;
            }
        }
        (request::PackageRequest::Apply(request), reply::PackageReply::Apply(reply)) => {
            if let reply::ApplyResult::Ok(ok) = &reply.result {
                exact_staged_outputs(&request.plan, &ok.staged)?;
            }
        }
        (request::PackageRequest::Verify(request), reply::PackageReply::Verify(reply)) => {
            if let reply::VerifyResult::Ok(ok) = &reply.result {
                exact_verified_outputs(&request.staged, &ok.verified)?;
            }
        }
        _ => unreachable!("reply operation was validated above"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const PROVIDER: &str = "org.example/tools#packager";
    const MECHANISM: &str = "package:archive";
    const TARGET: &str = "bundle";
    fn digest() -> String {
        "a".repeat(64)
    }
    fn target() -> Value {
        json!({"id":TARGET,"config_toml":"level = 1\n","outputs":[
        {"id":"bundle","kind":"archive"},{"id":"plugin","kind":"agent-plugin"}]})
    }
    fn inputs() -> Value {
        json!([
            {"name":"binary","reference":"artifact:binary","path_absolute":"C:/work/artifacts/bin","path_relative":"artifacts/bin","digest":digest(),"bytes":"7","shape":"file","origin":{"kind":"artifact-record","recorded_kind":"executable"}},
            {"name":"README.md","reference":"path:README.md","path_absolute":"C:/work/README.md","path_relative":"README.md","digest":digest(),"bytes":"3","shape":"file","origin":{"kind":"workspace-path"}}
        ])
    }
    fn authority() -> Value {
        json!({"project_root_absolute":"C:/work","package_root_absolute":"C:/work/target/packages","package_root_relative":"target/packages","output_root_absolute":"C:/work/target/packages/bundle","output_root_relative":"target/packages/bundle"})
    }
    fn plan() -> Value {
        json!({"summary":"two outputs","outputs":[
        {"id":"bundle","kind":"archive","shape":"file","path_relative":"bundle.tar","media_type":"application/x-tar"},
        {"id":"plugin","kind":"agent-plugin","shape":"directory","path_relative":"."}]})
    }
    fn fingerprint() -> Value {
        json!({"digest":digest(),"counted_inputs":"2"})
    }
    fn staging() -> Value {
        json!({"root_absolute":"C:/work/target/packages/stage","root_relative":"target/packages/stage"})
    }
    fn staged() -> Value {
        json!([
        {"id":"bundle","kind":"archive","shape":"file","path_relative":"bundle.tar","media_type":"application/x-tar"},
        {"id":"plugin","kind":"agent-plugin","shape":"directory","path_relative":"."}])
    }
    fn request_value(op: &str) -> Value {
        let mut v = match op {
            "plan" => json!({}),
            "fingerprint" => json!({"plan":plan()}),
            "apply" => json!({"plan":plan(),"fingerprint":fingerprint(),"staging":staging()}),
            "verify" => {
                json!({"plan":plan(),"fingerprint":fingerprint(),"staging":staging(),"staged":staged()})
            }
            _ => unreachable!(),
        };
        let o = v.as_object_mut().unwrap();
        o.insert("operation".into(), op.into());
        o.insert("envelope".into(), 1.into());
        o.insert("protocol".into(), 1.into());
        o.insert(
            "identity".into(),
            json!({"provider":PROVIDER,"mechanism":MECHANISM,"target":TARGET}),
        );
        o.insert("target".into(), target());
        o.insert("inputs".into(), inputs());
        o.insert("authority".into(), authority());
        v
    }
    fn reply_value(op: &str) -> Value {
        let result = match op {
            "plan" => json!({"status":"ok","plan":plan()}),
            "fingerprint" => json!({"status":"ok","fingerprint":fingerprint()}),
            "apply" => {
                json!({"status":"ok","staged":[{"id":"bundle","path_relative":"bundle.tar"},{"id":"plugin","path_relative":"."}],"evidence":"packed"})
            }
            "verify" => {
                json!({"status":"ok","verified":[{"id":"bundle","path_relative":"bundle.tar","digest":digest(),"bytes":"7","files":"1"},{"id":"plugin","path_relative":".","digest":digest(),"bytes":"9","files":"2"}],"evidence":"verified"})
            }
            _ => unreachable!(),
        };
        json!({"operation":op,"envelope":1,"protocol":1,"result":result})
    }
    fn request(op: &str) -> request::PackageRequest {
        serde_json::from_value(request_value(op)).unwrap()
    }
    fn reply(op: &str) -> reply::PackageReply {
        serde_json::from_value(reply_value(op)).unwrap()
    }

    struct Accepted {
        target: request::PackageTarget,
        inputs: Vec<request::ResolvedInput>,
        authority: request::PackageAuthority,
        plan: request::PackagePlan,
        fingerprint: request::PackageFingerprint,
        staging: request::StagingAuthority,
        staged: Vec<request::StagedOutput>,
    }
    fn accepted() -> Accepted {
        Accepted {
            target: serde_json::from_value(target()).unwrap(),
            inputs: serde_json::from_value(inputs()).unwrap(),
            authority: serde_json::from_value(authority()).unwrap(),
            plan: serde_json::from_value(plan()).unwrap(),
            fingerprint: serde_json::from_value(fingerprint()).unwrap(),
            staging: serde_json::from_value(staging()).unwrap(),
            staged: serde_json::from_value(staged()).unwrap(),
        }
    }
    fn retained<'a>(op: &str, a: &'a Accepted) -> RetainedPackage<'a> {
        let prefix = RetainedPackage {
            target: Some(&a.target),
            inputs: Some(&a.inputs),
            authority: Some(&a.authority),
            ..Default::default()
        };
        match op {
            "plan" => Default::default(),
            "fingerprint" => RetainedPackage {
                plan: Some(&a.plan),
                ..prefix
            },
            "apply" => RetainedPackage {
                plan: Some(&a.plan),
                fingerprint: Some(&a.fingerprint),
                ..prefix
            },
            "verify" => RetainedPackage {
                plan: Some(&a.plan),
                fingerprint: Some(&a.fingerprint),
                staging: Some(&a.staging),
                staged: Some(&a.staged),
                ..prefix
            },
            _ => unreachable!(),
        }
    }

    fn request_fault(value: Value, op: &str, accepted: &Accepted, law: &str) {
        let request = serde_json::from_value(value).unwrap();
        let error = validate_request(
            &request,
            PROVIDER,
            MECHANISM,
            TARGET,
            retained(op, accepted),
        )
        .unwrap_err();
        assert_eq!(error.law(), law);
    }

    fn exchange_fault(value: Value, op: &str, accepted: &Accepted, law: &str) {
        let reply = serde_json::from_value(value).unwrap();
        let error = validate_exchange(
            &request(op),
            PROVIDER,
            MECHANISM,
            TARGET,
            retained(op, accepted),
            &reply,
        )
        .unwrap_err();
        assert_eq!(error.law(), law);
    }

    #[test]
    fn all_four_exchanges_and_origins_are_exact() {
        let a = accepted();
        for (name, operation) in [
            ("plan", PackageOperation::Plan),
            ("fingerprint", PackageOperation::Fingerprint),
            ("apply", PackageOperation::Apply),
            ("verify", PackageOperation::Verify),
        ] {
            let req = request(name);
            let retained = retained(name, &a);
            assert_eq!(
                validate_request(&req, PROVIDER, MECHANISM, TARGET, retained).unwrap(),
                operation
            );
            validate_exchange(&req, PROVIDER, MECHANISM, TARGET, retained, &reply(name)).unwrap();
        }
        let mut bad = request_value("plan");
        bad["inputs"][0]["origin"] = json!({"kind":"workspace-path","recorded_kind":"file"});
        let decoded: request::PackageRequest = serde_json::from_value(bad).unwrap();
        let canonical = serde_json::to_value(decoded).unwrap();
        assert!(
            canonical["inputs"][0]["origin"]
                .get("recorded_kind")
                .is_none()
        );
        let mut wrong_shape = request_value("plan");
        wrong_shape["inputs"][0]["shape"] = "directory".into();
        let req = serde_json::from_value(wrong_shape).unwrap();
        assert_eq!(
            validate_request(&req, PROVIDER, MECHANISM, TARGET, Default::default())
                .unwrap_err()
                .law(),
            "resolved-inputs"
        );
        assert_eq!(
            validate_request(
                &request("plan"),
                PROVIDER,
                MECHANISM,
                TARGET,
                RetainedPackage {
                    target: Some(&a.target),
                    ..Default::default()
                }
            )
            .unwrap_err()
            .law(),
            "accepted-chain"
        );
    }

    #[test]
    fn retained_chain_refuses_every_engine_fact_drift() {
        let a = accepted();
        for pointer in [
            "/target/config_toml",
            "/inputs/0/digest",
            "/inputs/0/bytes",
            "/inputs/0/origin/recorded_kind",
        ] {
            let mut v = request_value("fingerprint");
            *v.pointer_mut(pointer).unwrap() = match pointer {
                "/target/config_toml" => "level = 2\n".into(),
                "/inputs/0/digest" => "b".repeat(64).into(),
                "/inputs/0/bytes" => "8".into(),
                _ => "file".into(),
            };
            request_fault(v, "fingerprint", &a, "accepted-chain");
        }
        let mut path = request_value("fingerprint");
        path["inputs"][0]["path_relative"] = "other/bin".into();
        path["inputs"][0]["path_absolute"] = "C:/work/other/bin".into();
        request_fault(path, "fingerprint", &a, "accepted-chain");
        let mut roots = request_value("fingerprint");
        roots["authority"]["package_root_relative"] = "other".into();
        roots["authority"]["package_root_absolute"] = "C:/work/other".into();
        roots["authority"]["output_root_relative"] = "other/bundle".into();
        roots["authority"]["output_root_absolute"] = "C:/work/other/bundle".into();
        request_fault(roots, "fingerprint", &a, "accepted-chain");
        let mut v = request_value("verify");
        v["staging"]["root_relative"] = "target/packages/other".into();
        v["staging"]["root_absolute"] = "C:/work/target/packages/other".into();
        request_fault(v, "verify", &a, "accepted-chain");
    }

    #[test]
    fn strict_reply_permissive_request_and_exact_descriptors() {
        assert_eq!(
            decode_reply(&vec![b'x'; REPLY_CAP_BYTES + 1])
                .unwrap_err()
                .law(),
            "reply-cap"
        );
        let mut permissive = request_value("plan");
        permissive["future"] = true.into();
        permissive["inputs"][0]["future"] = true.into();
        assert!(serde_json::from_value::<request::PackageRequest>(permissive).is_ok());
        let mut strict = reply_value("plan");
        strict["result"]["plan"]["outputs"][0]["future"] = true.into();
        assert!(decode_reply(&serde_json::to_vec(&strict).unwrap()).is_err());
        let a = accepted();
        for (field, value) in [("kind", "file"), ("shape", "directory")] {
            let mut bad = reply_value("plan");
            bad["result"]["plan"]["outputs"][0][field] = value.into();
            exchange_fault(bad, "plan", &a, "plan-outputs");
        }
        let mut media = reply_value("plan");
        media["result"]["plan"]["outputs"][1]["media_type"] = "application/x-dir".into();
        exchange_fault(media, "plan", &a, "media-type");
        let mut counted = reply_value("fingerprint");
        counted["result"]["fingerprint"]["counted_inputs"] = "1".into();
        exchange_fault(counted, "fingerprint", &a, "fingerprint");
        let mut staged_reply = reply_value("apply");
        staged_reply["result"]["staged"]
            .as_array_mut()
            .unwrap()
            .reverse();
        exchange_fault(staged_reply, "apply", &a, "staged-outputs");
        let mut verified_reply = reply_value("verify");
        verified_reply["result"]["verified"]
            .as_array_mut()
            .unwrap()
            .reverse();
        exchange_fault(verified_reply, "verify", &a, "verification-rows");
        let mut file_count = reply_value("verify");
        file_count["result"]["verified"][0]["files"] = "2".into();
        exchange_fault(file_count, "verify", &a, "verification-rows");
    }
}
