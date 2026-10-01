use std::cell::{Cell, RefCell};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use super::super::{
    CallAction, LANDED_FORMAT, Landed, PENDING_FORMAT, Pending, decompose, suffix,
    verified_transition,
};
use super::*;
use crate::Token;
use crate::github_release::{GithubGitObject, GithubGitRef, GithubRelease, GithubReleaseAsset};
use crate::publication_approval::ApprovalHome;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::remote_observation::{
    ObservationError, RemoteSnapshot, normalize_fixed,
};
use crate::publication_full_plan::session::{ReadyRun, SessionOpen};
use crate::publication_full_plan::{
    prepare,
    tests::{expected, scope},
};
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::{Project, StreamOutcome};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedTag, PublicationObservedTagPresent,
};

struct Fixture {
    tiny: TinyPayloadWorld,
    _root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    home: ApprovalHome,
    expected: crate::publication_effect_plan::PublicationEffectPlanExpectations<'static>,
    remote: Rc<RefCell<RemoteSnapshot>>,
}

impl Fixture {
    fn new() -> Self {
        let tiny = TinyPayloadWorld::new();
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let expected = expected();
        let output = Project::open(tiny.output.path()).unwrap();
        assert!(matches!(
            prepare::prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "asset-driver-key",
                &tiny.subject,
                &tiny.plan,
                &expected,
                prepare::trusted(
                    tiny.settings.path(),
                    &lease,
                    &scope,
                    &tiny.subject,
                    &tiny.plan,
                    &output,
                    tiny.finals()
                ),
            )
            .unwrap(),
            crate::publication_full_plan::Inspection::JournalReady { .. }
        ));
        let home = tiny.approval_home();
        let release = GithubRelease {
            id: 700,
            tag_name: "v1.1.0".into(),
            target_commitish: tiny.subject.source.commit.clone(),
            name: Some("vibevm 1.1.0".into()),
            body: Some(match &tiny.plan.partitions[1] {
                vibe_wire::generated::publication_effect_plan::PublicationPartition::Release(
                    step,
                ) => step.desired_body.clone(),
                _ => panic!(),
            }),
            draft: true,
            prerelease: false,
            html_url: String::new(),
            upload_url: String::new(),
        };
        let tag = GithubGitRef {
            reference: "refs/tags/v1.1.0".into(),
            object: GithubGitObject {
                sha: tiny.subject.source.commit.clone(),
                kind: "commit".into(),
                url: String::new(),
            },
        };
        let remote = Rc::new(RefCell::new(RemoteSnapshot {
            latest: crate::github_release::LatestReleaseObservation::Present {
                release_id: 101,
                tag: "v1.0.1".into(),
            },
            tag: Some(tag),
            release: Some(release),
            assets: Some(Vec::new()),
            streamed_digests: Vec::new(),
        }));
        Self {
            tiny,
            _root: root,
            lease,
            namespace,
            scope,
            home,
            expected,
            remote,
        }
    }

    fn prepare_prefix(&self) {
        let run_id = match self.open() {
            SessionOpen::Ready(run) => run.run_id,
            _ => panic!(),
        };
        let schedule = decompose(&self.tiny.subject, &self.tiny.plan, &self.expected).unwrap();
        let mut before = CallObservation {
            generation_ref: self.tiny.plan.remote_generation_ref.clone(),
            tag: self.tiny.plan.observed_tag.clone(),
            release: self.tiny.plan.observed_release.clone(),
            latest_release: self.tiny.plan.observed_latest_release.clone(),
        };
        for call in &schedule.calls {
            let pending = Pending {
                format: PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
                before: before.clone(),
            };
            journal::publish(
                &self.lease,
                &run_id,
                &suffix(call.ordinal, "pending"),
                &pending,
            )
            .unwrap();
            let after = match &call.action {
                CallAction::CreateTag { tag, commit } => CallObservation {
                    generation_ref: "tag-created-generation".into(),
                    tag: PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                        reference: format!("refs/tags/{tag}"),
                        object_kind: "commit".into(),
                        object_sha: commit.clone(),
                    })),
                    release: before.release.clone(),
                    latest_release: before.latest_release.clone(),
                },
                CallAction::CreateDraft { .. } => {
                    let observed = normalize_fixed(
                        self.remote.borrow().clone(),
                        &self.tiny.subject.source.commit,
                    )
                    .unwrap();
                    CallObservation {
                        generation_ref: observed.generation_ref,
                        tag: observed.tag,
                        release: observed.release,
                        latest_release: observed.observed_latest_release,
                    }
                }
                _ => panic!("new-draft fixture only"),
            };
            let receipt = if matches!(call.action, CallAction::CreateDraft { .. }) {
                "700"
            } else {
                "tag-created"
            };
            assert!(verified_transition(call, &before, &after, receipt));
            let landed = Landed {
                format: LANDED_FORMAT.into(),
                run_id: run_id.clone(),
                pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
                receipt_id: receipt.into(),
                after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
                after: after.clone(),
            };
            journal::publish(
                &self.lease,
                &run_id,
                &suffix(call.ordinal, "landed"),
                &landed,
            )
            .unwrap();
            before = after;
        }
    }

    fn open(&self) -> SessionOpen<'_> {
        ReadyRun::open(
            &self.lease,
            &self.namespace,
            &self.scope,
            "asset-driver-key",
            &self.tiny.subject,
            &self.tiny.plan,
            &self.expected,
            &self.home,
        )
        .unwrap()
    }
}

struct FakeReader {
    state: Rc<RefCell<RemoteSnapshot>>,
    reads: usize,
    drift_on_read: Option<usize>,
}
impl RemoteReadPort for FakeReader {
    fn read_complete(&mut self) -> std::result::Result<RemoteSnapshot, ObservationError> {
        self.reads += 1;
        let mut row = self.state.borrow().clone();
        if self.drift_on_read == Some(self.reads) {
            row.release.as_mut().unwrap().body = Some("foreign".into());
        }
        Ok(row)
    }
}

struct FakeTransport {
    state: Rc<RefCell<RemoteSnapshot>>,
    writes: RefCell<Vec<usize>>,
    lost_upload_at: Cell<Option<usize>>,
    lost_rename_at: Cell<Option<usize>>,
    fail_candidate_at: Cell<Option<usize>>,
}
impl FakeTransport {
    fn new(state: Rc<RefCell<RemoteSnapshot>>) -> Self {
        Self {
            state,
            writes: RefCell::new(Vec::new()),
            lost_upload_at: Cell::new(None),
            lost_rename_at: Cell::new(None),
            fail_candidate_at: Cell::new(None),
        }
    }
}
impl AssetCallTransport for FakeTransport {
    fn upload(
        &self,
        release_id: u64,
        call: &PlannedAssetCall,
        mut held: VaultAssetStream,
    ) -> std::result::Result<u64, String> {
        assert_eq!(release_id, 700);
        assert_eq!(held.name, call.canonical_name);
        let mut bytes = Vec::new();
        held.reader.read_to_end(&mut bytes).unwrap();
        assert!(matches!(
            held.completion.outcome(),
            StreamOutcome::Complete { .. }
        ));
        assert_eq!(bytes.len() as u64, call.size_bytes);
        self.writes.borrow_mut().push(call.ordinal);
        let id = 701 + call.asset_index as u64;
        self.state
            .borrow_mut()
            .assets
            .as_mut()
            .unwrap()
            .push(GithubReleaseAsset {
                id,
                name: call.temporary_name.clone(),
                size: call.size_bytes,
                digest: Some(call.sha256.clone()),
                browser_download_url: String::new(),
                content_type: call.media_type.clone(),
            });
        if self.lost_upload_at.get() == Some(call.ordinal)
            || self.fail_candidate_at.get() == Some(call.ordinal)
        {
            return Err("fake POST or candidate GET uncertain".into());
        }
        Ok(id)
    }
    fn rename(&self, asset_id: u64, call: &PlannedAssetCall) -> std::result::Result<u64, String> {
        self.writes.borrow_mut().push(call.ordinal);
        let mut state = self.state.borrow_mut();
        let row = state
            .assets
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row.id == asset_id)
            .unwrap();
        assert_eq!(row.name, call.temporary_name);
        row.name = call.canonical_name.clone();
        if self.lost_rename_at.get() == Some(call.ordinal) {
            return Err("fake PATCH uncertain".into());
        }
        Ok(asset_id)
    }
}

fn one_step(
    fixture: &Fixture,
    reader: &mut FakeReader,
    transport: &FakeTransport,
) -> AssetInspection {
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!("sealed session absent")
    };
    drive_one(&session, reader, transport).unwrap()
}

#[test]
fn first_middle_last_pairs_land_one_call_at_a_time_and_stop_at_cleanup() {
    let fixture = Fixture::new();
    fixture.prepare_prefix();
    let mut reader = FakeReader {
        state: fixture.remote.clone(),
        reads: 0,
        drift_on_read: None,
    };
    let transport = FakeTransport::new(fixture.remote.clone());
    for index in 0..38 {
        let status = one_step(&fixture, &mut reader, &transport);
        if index < 37 {
            assert!(
                matches!(status, AssetInspection::Next { call, .. } if call.ordinal == index + 3)
            );
        } else {
            assert_eq!(status, AssetInspection::Unavailable { landed: 38 });
        }
    }
    assert_eq!(transport.writes.borrow().len(), 38);
    assert_eq!(fixture.remote.borrow().assets.as_ref().unwrap().len(), 19);
    assert_eq!(
        one_step(&fixture, &mut reader, &transport),
        AssetInspection::Unavailable { landed: 38 }
    );
    assert_eq!(transport.writes.borrow().len(), 38);
}

#[test]
fn later_lost_upload_or_rename_leaves_pending_and_never_replays() {
    for lost_upload in [true, false] {
        let fixture = Fixture::new();
        fixture.prepare_prefix();
        let mut reader = FakeReader {
            state: fixture.remote.clone(),
            reads: 0,
            drift_on_read: None,
        };
        let transport = FakeTransport::new(fixture.remote.clone());
        let stop = if lost_upload { 22 } else { 23 };
        for _ in 2..stop {
            assert!(matches!(
                one_step(&fixture, &mut reader, &transport),
                AssetInspection::Next { .. }
            ));
        }
        if lost_upload {
            transport.lost_upload_at.set(Some(stop));
        } else {
            transport.lost_rename_at.set(Some(stop));
        }
        assert_eq!(
            one_step(&fixture, &mut reader, &transport),
            AssetInspection::Uncertain { ordinal: stop }
        );
        let count = transport.writes.borrow().len();
        let SessionOpen::Uncertain { .. } = fixture.open() else {
            panic!("pending must block attach")
        };
        assert_eq!(transport.writes.borrow().len(), count);
    }
}

#[test]
fn candidate_failure_and_generation_drift_refuse_without_next_call() {
    let fixture = Fixture::new();
    fixture.prepare_prefix();
    let mut reader = FakeReader {
        state: fixture.remote.clone(),
        reads: 0,
        drift_on_read: None,
    };
    let transport = FakeTransport::new(fixture.remote.clone());
    transport.fail_candidate_at.set(Some(2));
    assert_eq!(
        one_step(&fixture, &mut reader, &transport),
        AssetInspection::Uncertain { ordinal: 2 }
    );
    assert_eq!(transport.writes.borrow().as_slice(), &[2]);

    let fixture = Fixture::new();
    fixture.prepare_prefix();
    let mut reader = FakeReader {
        state: fixture.remote.clone(),
        reads: 0,
        drift_on_read: None,
    };
    let transport = FakeTransport::new(fixture.remote.clone());
    fixture.remote.borrow_mut().release.as_mut().unwrap().body = Some("foreign".into());
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!()
    };
    assert!(matches!(
        drive_one(&session, &mut reader, &transport),
        Err(Error::Refused(_))
    ));
    assert!(transport.writes.borrow().is_empty());

    let fixture = Fixture::new();
    fixture.prepare_prefix();
    let mut reader = FakeReader {
        state: fixture.remote.clone(),
        reads: 0,
        drift_on_read: None,
    };
    let transport = FakeTransport::new(fixture.remote.clone());
    fixture
        .remote
        .borrow_mut()
        .release
        .as_mut()
        .unwrap()
        .html_url = "https://foreign.example".into();
    let SessionOpen::Ready(session) = fixture.open() else {
        panic!()
    };
    assert!(matches!(
        drive_one(&session, &mut reader, &transport),
        Err(Error::Refused(_))
    ));
    assert!(
        transport.writes.borrow().is_empty(),
        "generation-only drift called transport"
    );
}

#[test]
fn post_effect_second_read_drift_keeps_only_pending() {
    let fixture = Fixture::new();
    fixture.prepare_prefix();
    let mut reader = FakeReader {
        state: fixture.remote.clone(),
        reads: 0,
        drift_on_read: Some(4),
    };
    let transport = FakeTransport::new(fixture.remote.clone());
    assert_eq!(
        one_step(&fixture, &mut reader, &transport),
        AssetInspection::Uncertain { ordinal: 2 }
    );
    let run_id = match fixture.open() {
        SessionOpen::Uncertain { run_id } => run_id,
        _ => panic!("post-effect drift reopened effect"),
    };
    assert!(
        journal::read_bytes(&fixture.lease, &run_id, &assets::asset_suffix(2, "pending"))
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&fixture.lease, &run_id, &assets::asset_suffix(2, "landed"))
            .unwrap()
            .is_none()
    );
    assert_eq!(transport.writes.borrow().as_slice(), &[2]);
}

fn read_http(socket: &mut std::net::TcpStream) -> (String, String, Vec<u8>) {
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(socket.try_clone().unwrap());
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap().to_owned();
    let path = parts.next().unwrap().to_owned();
    let mut size = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        if let Some((key, value)) = line.split_once(':')
            && key.eq_ignore_ascii_case("content-length")
        {
            size = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0; size];
    reader.read_exact(&mut body).unwrap();
    (method, path, body)
}

#[test]
fn production_transport_uses_candidate_id_for_one_get_without_reposting() {
    for fail_get in [false, true] {
        let tiny = TinyPayloadWorld::new();
        let call = assets::decompose_asset_calls(
            &tiny.subject,
            &tiny.plan,
            &expected(),
            "00000000000000000000000000000001",
        )
        .unwrap()
        .remove(0);
        let output = Project::open(tiny.output.path()).unwrap();
        let (reader, completion) = output
            .open_verified_stream_bounded(
                &call.canonical_name,
                &call.sha256[7..],
                call.size_bytes,
                call.size_bytes,
            )
            .unwrap();
        let held = VaultAssetStream {
            name: call.canonical_name.clone(),
            media_type: call.media_type.clone(),
            size_bytes: call.size_bytes,
            sha256: call.sha256.clone(),
            reader,
            completion,
        };
        let bytes = format!("fixture payload: {}\n", call.canonical_name).into_bytes();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let temporary_name = call.temporary_name.clone();
        let server = thread::spawn(move || {
            let (mut post, _) = listener.accept().unwrap();
            let (method, path, body) = read_http(&mut post);
            assert_eq!(method, "POST");
            assert!(path.contains("/releases/700/assets?name="));
            assert_eq!(body, bytes);
            let response = format!(
                "{{\"id\":701,\"name\":\"{temporary_name}\",\"size\":{},\"digest\":null}}",
                body.len(),
            );
            write!(
                post,
                "HTTP/1.1 201 Created\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            )
            .unwrap();
            drop(post);
            let (mut get, _) = listener.accept().unwrap();
            let (method, path, _) = read_http(&mut get);
            assert_eq!(method, "GET");
            assert!(path.ends_with("/releases/assets/701"));
            if fail_get {
                write!(
                    get,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
            } else {
                write!(
                    get,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                get.write_all(&body).unwrap();
            }
        });
        let client = GithubReleaseClient::with_endpoints_and_timeouts(
            Token::from_explicit("fixture-token"),
            "vibevm",
            "vibevm",
            &base,
            &base,
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .unwrap();
        let result = AssetCallTransport::upload(&client, 700, &call, held);
        server.join().unwrap();
        if fail_get {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), 701);
        }
    }
}
