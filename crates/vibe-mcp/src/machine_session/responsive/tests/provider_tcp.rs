use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vibe_core::manifest::TargetOs;
use vibe_lifecycle::{LifecycleLease, Phase};
use vibe_lifecycle::{SubmissionReservations, SubmissionScope};
use vibe_orchestrator::{
    DefaultLifecycleRequest,
    machine_submission::{HostOwnedFixedOperation, Readiness},
};
use vibe_wire::generated::{format_id::FormatId, lifecycle::e1::context::RunAgentMode};

struct Fixture {
    dir: tempfile::TempDir,
    host: HostOwnedFixedOperation,
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while !condition() {
        assert!(Instant::now() < deadline, "finite oracle deadline");
        std::thread::sleep(Duration::from_millis(2));
    }
}
impl Fixture {
    fn new(flood: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        // Exact incumbent one-script manifest from lifecycle_adapter.rs.
        std::fs::write(dir.path().join("vibe.toml"), "[project]\nname = 'demo'\nversion = '1.1.0'\n[[extension]]\nid = 'once'\npoint = 'phase:validate'\nhandler = { kind = 'script', base = 'run' }\n").unwrap();
        let tail = if flood {
            "i=0; while [ \"$i\" -lt 800 ]; do printf '%04096d' 0; i=$((i + 1)); done\n"
        } else {
            ""
        };
        std::fs::write(dir.path().join("run.sh"), format!(
            "#!/bin/sh\nset -eu\nprintf 'effect\\n' >> counter\nprintf '\\377A\\000'\nprintf '\\376B\\000' >&2\nprintf entered > entered\ni=0\nwhile [ ! -f release ]; do i=$((i + 1)); if [ \"$i\" -ge 240 ]; then printf timeout > finished; exit 70; fi; sleep 0.05; done\n{tail}printf finished > finished\n"
        )).unwrap();
        let scope = SubmissionScope {
            context: "tcp-host".into(),
            target: "root".into(),
            principal: "owner".into(),
        };
        let namespace = {
            let lease = LifecycleLease::acquire(dir.path()).unwrap();
            SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap()
        };
        let host = HostOwnedFixedOperation::start(
            dir.path().into(),
            namespace,
            scope,
            "tcp-once".into(),
            DefaultLifecycleRequest {
                requested: Phase::Validate,
                force: false,
                agent_mode: RunAgentMode::Cli,
                assume_yes: true,
                trace_flag: false,
                install_inputs: Default::default(),
                policy: Default::default(),
            },
            TargetOs::Linux,
            "2026-09-30T00:00:05Z".parse().unwrap(),
        )
        .unwrap();
        let fixture = Self { dir, host };
        until(|| match fixture.host.wait_ready(Duration::ZERO).unwrap() {
            Readiness::Ready(_) => true,
            Readiness::Starting => false,
            other => panic!("actual admission failed: {other:?}"),
        });
        until(|| fixture.path("entered").is_file());
        fixture
    }
    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
    fn release(&self) {
        std::fs::write(self.path("release"), b"release").unwrap();
    }
    fn finish(&self) {
        self.release();
        until(|| {
            matches!(
                self.host.inspect().unwrap(),
                FixedOperationStatus::ResultAvailable { .. }
            )
        });
        assert_eq!(std::fs::read(self.path("counter")).unwrap(), b"effect\n");
    }
    fn session(&self, controlled: bool, blocked: bool) -> (ResponsiveSession, Client) {
        let (port, client) = pair(blocked);
        (
            ResponsiveSession::new(
                self.host.attach().unwrap(),
                controlled.then(|| self.host.grant_control()),
                port,
            )
            .unwrap(),
            client,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.path("release"), b"cleanup");
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            if matches!(
                self.host.inspect(),
                Ok(FixedOperationStatus::ResultAvailable { .. }
                    | FixedOperationStatus::RecoveryRequired { .. })
            ) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
struct Client {
    socket: TcpStream,
    decoder: super::super::super::incremental::Decoder,
}
fn pair(blocked: bool) -> (TcpFramePort, Client) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server.set_nonblocking(true).unwrap();
    if blocked {
        // Establish actual kernel backpressure before handing over the
        // connected socket. These tests never decode the padding peer.
        let padding = [0u8; 65536];
        let mut count = 0;
        for _ in 0..5 {
            loop {
                match server.write(&padding) {
                    Ok(n) => {
                        assert!(n > 0);
                        count += n;
                        assert!(count < 64 * 1024 * 1024);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("backpressure setup: {e}"),
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(count > 0);
    }
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    client
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    (
        TcpFramePort::new(server).unwrap(),
        Client {
            socket: client,
            decoder: super::super::super::incremental::Decoder::new(),
        },
    )
}
fn encoded(value: &frame::SessionFrame) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend(body);
    bytes
}
impl Client {
    fn send(&mut self, value: &frame::SessionFrame) {
        self.socket.write_all(&encoded(value)).unwrap();
    }
    fn response(&mut self, session: &mut ResponsiveSession, id: &str) -> frame::SessionFrame {
        self.socket.set_nonblocking(true).unwrap();
        let mut result = None;
        until(|| {
            session.poll().unwrap();
            match self.decoder.poll_read(&mut self.socket).unwrap() {
                ReadPoll::Frame(value) if value.request_id.as_deref() == Some(id) => {
                    result = Some(value);
                    true
                }
                ReadPoll::Pending | ReadPoll::Frame(_) => false,
                other => panic!("unexpected transport: {other:?}"),
            }
        });
        self.socket.set_nonblocking(false).unwrap();
        result.unwrap()
    }
}
fn offer() -> frame::SessionFrame {
    frame::SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some("init".into()),
        body: frame::Body::InitializeOffer(Box::new(frame::BodyInitializeOffer {
            api_versions: vec!["1".into()],
            schema_offers: [
                FormatId::MachineSessionFrame,
                FormatId::MachineOperationStreamMessage,
            ]
            .into_iter()
            .map(|f| frame::SchemaOffer {
                format_id: f.id().into(),
                epoch: 1,
            })
            .collect(),
            position_encodings: vec!["raw-byte-offsets".into()],
            requested_capabilities: frame::Capabilities {
                streaming: true,
                replay: true,
                control: true,
            },
        })),
    }
}
struct Assigned {
    session: String,
    context: String,
    target: Option<String>,
}
fn initialized(session: &mut ResponsiveSession, client: &mut Client, control: bool) -> Assigned {
    client.send(&offer());
    let reply = client.response(session, "init");
    let frame::Body::Initialized(body) = reply.body else {
        panic!("initialization")
    };
    assert!(body.actual_capabilities.streaming && body.actual_capabilities.replay);
    assert_eq!(body.actual_capabilities.control, control);
    assert_eq!(body.selected_schemas.len(), 2);
    assert_eq!(body.position_encoding, "raw-byte-offsets");
    assert_eq!(body.contexts.len(), 1);
    let context = &body.contexts[0];
    assert_ne!(context.context_id, session.reader.summary().context_id());
    Assigned {
        session: reply.session_id.unwrap(),
        context: context.context_id.clone(),
        target: context.target_ids.first().cloned(),
    }
}
fn replay() -> stream::Body {
    stream::Body::ReplayRequest(Box::new(stream::BodyReplayRequest {
        after_sequence: "0".into(),
        max_events: 64,
        max_bytes: 65536,
    }))
}
fn cancel(id: &str) -> stream::Body {
    stream::Body::ControlRequest(Box::new(stream::BodyControlRequest {
        action: stream::BodyControlRequestAction::Cancel,
        after_sequence: "0".into(),
        request_id: id.into(),
    }))
}
fn request(
    session: &ResponsiveSession,
    ids: &Assigned,
    id: &str,
    body: stream::Body,
) -> frame::SessionFrame {
    frame::SessionFrame {
        schema: 1,
        session_id: Some(ids.session.clone()),
        request_id: Some(id.into()),
        body: frame::Body::Request(Box::new(frame::BodyRequest {
            context_id: ids.context.clone(),
            target_id: ids.target.clone(),
            envelope_json: envelope(&session.stream_request(body)).unwrap(),
        })),
    }
}
fn inner(value: frame::SessionFrame) -> stream::OperationStreamMessage {
    let frame::Body::Complete(body) = value.body else {
        panic!("typed complete: {value:?}")
    };
    assert_eq!(
        body.outcome.effect_state,
        frame::OutcomeEffectState::Uncertain
    );
    serde_json::from_str(&body.envelope_json.unwrap()).unwrap()
}
fn control_status(value: frame::SessionFrame) -> stream::BodyControlReplyStatus {
    let stream::Body::ControlReply(body) = inner(value).body else {
        panic!("control reply")
    };
    body.status
}

mod cases;
