use super::*;
use std::io::{self, Cursor, Write};

use serde_json::{Value, json};
use vibe_orchestrator::machine_operation::PurePlannerAdmission;
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        operation_envelope::Body as EnvelopeBody,
        session_frame::{BodyRequest, Capabilities, ContextKind, SchemaOffer},
    },
    migration::e1::{plan_error::PlanError, plan_report::PlanReport},
};

use crate::ServerContext;
use crate::machine_session::state::{TrustedContextKind, TrustedContextSource};

fn runner(root: &str) -> MachineSessionRunner {
    MachineSessionRunner::new(
        MachineSession::new(vec![TrustedContextSource {
            kind: TrustedContextKind::Workspace,
            context: ServerContext::with_store_root(root, format!("{root}-store")),
            target_roots: vec![format!("{root}/member").into()],
        }])
        .unwrap(),
    )
    .unwrap()
}
fn offer(id: &str) -> SessionFrame {
    let mut frame: SessionFrame = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/machine-session-frame/e1/valid/initialize-offer.json"
    )))
    .unwrap();
    frame.session_id = None;
    frame.request_id = Some(id.to_owned());
    let Body::InitializeOffer(body) = &mut frame.body else {
        unreachable!()
    };
    body.api_versions = vec!["1".to_owned()];
    body.position_encodings = vec!["raw-byte-offsets".to_owned()];
    body.schema_offers = [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MigrationPlanRequest,
        FormatId::MigrationPlanReport,
        FormatId::MigrationPlanError,
    ]
    .into_iter()
    .map(|format| SchemaOffer {
        format_id: format.id().to_owned(),
        epoch: u16::try_from(format.epoch()).unwrap(),
    })
    .collect();
    body.requested_capabilities = Capabilities {
        streaming: true,
        replay: true,
        control: true,
    };
    frame
}
fn domain_request() -> Value {
    json!({
        "schema":1,
        "source_text":"# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\nmaterialization='snapshot' # keep\r\n",
        "source_profile":"materialization-legacy-absent-r1",
        "target_profile":"materialization-current-absent-r1",
        "recipe":"snapshot-to-copy-r1"
    })
}
fn envelope(domain: Value) -> OperationEnvelope {
    serde_json::from_value(json!({
        "schema":1,
        "operation_id":"rust:vibe-workspace:migration_planning",
        "domain_owner":"rust:vibe-workspace:migration_planning",
        "source_id":"pure-migration-transport-adapter",
        "evidence_id":null,
        "body":{"stage":"input","format_id":"migration-plan-request","payload_json":domain.to_string()}
    }))
    .unwrap()
}
fn request(
    session_id: &str,
    request_id: &str,
    context_id: &str,
    target_id: Option<String>,
    domain: Value,
) -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id: Some(session_id.to_owned()),
        request_id: Some(request_id.to_owned()),
        body: Body::Request(Box::new(BodyRequest {
            context_id: context_id.to_owned(),
            target_id,
            envelope_json: serde_json::to_string(&envelope(domain)).unwrap(),
        })),
    }
}
fn send(runner: &mut MachineSessionRunner, frame: &SessionFrame) -> SessionFrame {
    let mut input = Vec::new();
    write_frame(&mut input, frame).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        runner.step(&mut Cursor::new(input), &mut output).unwrap(),
        RunnerStep::Replied
    );
    let mut output = Cursor::new(output);
    let response = read_frame(&mut output).unwrap().unwrap();
    assert!(read_frame(&mut output).unwrap().is_none());
    response
}
fn initialized(runner: &mut MachineSessionRunner, id: &str) -> (String, String, String) {
    let reply = send(runner, &offer(id));
    let session_id = reply.session_id.unwrap();
    let Body::Initialized(body) = reply.body else {
        panic!("negotiation failed")
    };
    assert!(!body.actual_capabilities.streaming);
    assert!(!body.actual_capabilities.replay);
    assert!(!body.actual_capabilities.control);
    let workspace = body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Workspace)
        .unwrap();
    (
        session_id,
        workspace.context_id.clone(),
        workspace.target_ids[0].clone(),
    )
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-MCP-COEXISTENCE",
    r = 1
)]
fn two_fake_clients_publish_same_direct_plan_under_distinct_contexts() {
    let mut a = runner("/owned-a");
    let mut b = runner("/owned-b");
    let (a_session, a_context, _) = initialized(&mut a, "init-a");
    let (b_session, b_context, _) = initialized(&mut b, "init-b");
    assert_ne!(a_session, b_session);
    assert_ne!(a_context, b_context);
    let original = envelope(domain_request());
    let response = send(
        &mut a,
        &request(&a_session, "plan-a", &a_context, None, domain_request()),
    );
    let Body::Complete(body) = response.body else {
        panic!("pure plan missing completion")
    };
    assert_eq!(body.outcome.effect_state, OutcomeEffectState::NoEffects);
    assert_eq!(body.outcome.completeness, OutcomeCompleteness::Complete);
    let published: OperationEnvelope =
        serde_json::from_str(body.envelope_json.as_deref().unwrap()).unwrap();
    let PurePlannerExchange::Published {
        envelope: direct, ..
    } = MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted)
        .unwrap()
        .exchange_pure_planner(original)
    else {
        panic!("one-shot library route unavailable")
    };
    assert_eq!(published, direct);
    let EnvelopeBody::Result(row) = published.body else {
        panic!("normal plan became error")
    };
    let _: PlanReport = serde_json::from_str(row.payload_json.as_deref().unwrap()).unwrap();
    let other = send(
        &mut b,
        &request(&b_session, "plan-b", &b_context, None, domain_request()),
    );
    assert!(matches!(other.body, Body::Complete(_)));
    let cross = send(
        &mut b,
        &request(&b_session, "cross", &a_context, None, domain_request()),
    );
    let Body::Refusal(row) = cross.body else {
        panic!("foreign context was admitted")
    };
    assert_eq!(row.reason, BodyRefusalReason::UnknownContext);
    assert!(row.envelope_json.is_none());
    let mut output = Vec::new();
    assert_eq!(
        a.step(&mut Cursor::new(Vec::<u8>::new()), &mut output)
            .unwrap(),
        RunnerStep::Closed
    );
    assert!(output.is_empty());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn wrong_optional_target_and_domain_refusal_keep_truthful_channels() {
    let mut runner = runner("/owned");
    let (session_id, context_id, target_id) = initialized(&mut runner, "init");
    let target = send(
        &mut runner,
        &request(
            &session_id,
            "target",
            &context_id,
            Some(target_id),
            domain_request(),
        ),
    );
    let Body::Refusal(row) = target.body else {
        panic!("planner accepted a target binding")
    };
    assert_eq!(row.reason, BodyRefusalReason::InvalidRequest);
    assert!(row.envelope_json.is_none());
    let mut wrong = domain_request();
    wrong["source_profile"] = Value::Null;
    let preflight = send(
        &mut runner,
        &request(&session_id, "optional", &context_id, None, wrong),
    );
    let Body::Refusal(row) = preflight.body else {
        panic!("invalid optional became complete")
    };
    assert_eq!(row.reason, BodyRefusalReason::InvalidRequest);
    assert_eq!(row.outcome.effect_state, OutcomeEffectState::NoEffects);
    let envelope: OperationEnvelope =
        serde_json::from_str(row.envelope_json.as_deref().unwrap()).unwrap();
    let EnvelopeBody::Error(error) = envelope.body else {
        panic!("preflight lost Error")
    };
    let _: PlanError = serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
    let mut refused = domain_request();
    refused["source_profile"] = json!("unknown-r2");
    let completion = send(
        &mut runner,
        &request(&session_id, "refused", &context_id, None, refused),
    );
    let Body::Complete(row) = completion.body else {
        panic!("domain refusal was not terminal")
    };
    assert_eq!(row.outcome.effect_state, OutcomeEffectState::NoEffects);
    assert_eq!(row.outcome.completeness, OutcomeCompleteness::Complete);
    let envelope: OperationEnvelope =
        serde_json::from_str(row.envelope_json.as_deref().unwrap()).unwrap();
    let EnvelopeBody::Error(error) = envelope.body else {
        panic!("domain refusal lost Error")
    };
    let report: PlanReport = serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
    assert!(matches!(
        report.outcome,
        vibe_wire::generated::migration::e1::plan_report::Outcome::Refused(_)
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn body_path_and_over_limit_byte_frame_stop_before_dispatch() {
    let mut malformed_runner = runner("/owned");
    let mut raw = serde_json::to_value(offer("path")).unwrap();
    raw["body"]["project_root"] = json!("/not-authorized");
    let body = serde_json::to_vec(&raw).unwrap();
    let mut bytes = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    bytes.extend(body);
    let mut output = Vec::new();
    assert!(matches!(
        malformed_runner.step(&mut Cursor::new(bytes), &mut output),
        Err(RunnerError::Frame(FrameCodecError::InvalidShape))
    ));
    assert!(output.is_empty());
    let mut request_runner = runner("/owned");
    let (session_id, context_id, _) = initialized(&mut request_runner, "init-request-path");
    let mut raw = serde_json::to_value(request(
        &session_id,
        "request-path",
        &context_id,
        None,
        domain_request(),
    ))
    .unwrap();
    raw["body"]["project_root"] = json!("/not-authorized");
    let body = serde_json::to_vec(&raw).unwrap();
    let mut bytes = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
    bytes.extend(body);
    assert!(matches!(
        request_runner.step(&mut Cursor::new(bytes), &mut output),
        Err(RunnerError::Frame(FrameCodecError::InvalidShape))
    ));
    assert!(output.is_empty());
    let mut oversized_runner = runner("/owned");
    let prefix = u32::try_from(super::super::framing::MAX_FRAME_BYTES + 1)
        .unwrap()
        .to_be_bytes();
    assert!(matches!(
        oversized_runner.step(&mut Cursor::new(prefix), &mut output),
        Err(RunnerError::Frame(FrameCodecError::OverLimit { .. }))
    ));
    assert!(output.is_empty());
}

struct BrokenWriter;
impl Write for BrokenWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "test sink closed",
        ))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
    r = 1
)]
fn output_failure_retains_response_and_closes_before_next_request() {
    let mut runner = runner("/owned");
    let mut input = Vec::new();
    write_frame(&mut input, &offer("init-output-failure")).unwrap();
    let Err(RunnerError::Output { error, response }) =
        runner.step(&mut Cursor::new(input), &mut BrokenWriter)
    else {
        panic!("broken output did not preserve the response")
    };
    assert!(matches!(error, FrameCodecError::Io(_)));
    assert!(matches!(response.body, Body::Initialized(_)));
    let mut later = Cursor::new(vec![0, 0, 0, 0]);
    let mut output = Vec::new();
    assert_eq!(
        runner.step(&mut later, &mut output).unwrap(),
        RunnerStep::Closed
    );
    assert_eq!(later.position(), 0, "closed session read another frame");
    assert!(output.is_empty());
}
