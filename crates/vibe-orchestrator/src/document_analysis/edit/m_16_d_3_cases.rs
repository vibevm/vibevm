use super::*;
use std::{fs, time::Duration};
fn fixture() -> (
    tempfile::TempDir,
    Arc<DocumentAnalysisHost>,
    Arc<DocumentEditAuthority>,
    ResourceId,
    ResourceId,
) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("a.md"), b"before-a").unwrap();
    fs::write(root.path().join("b.md"), b"before-b").unwrap();
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "a.md".into(),
                role: ResourceRole::Opaque,
            },
            ResourceDeclaration {
                path: "b.md".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 4096,
            capture_bytes: 65536,
            open_buffers: 8,
        },
    )
    .unwrap();
    let a = host.resource("a.md").unwrap();
    let b = host.resource("b.md").unwrap();
    let authority = DocumentEditAuthority::new(host.clone(), vec![a, b]).unwrap();
    (root, host, authority, a, b)
}
fn replace(id: ResourceId, bytes: &[u8]) -> DocumentReplacement {
    DocumentReplacement {
        resource: id,
        bytes: Arc::from(bytes),
    }
}
#[test]
fn clean_acknowledged_saved_edit_commits() {
    let (root, host, authority, id, second) = fixture();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    a.open(
        1,
        id,
        1,
        ByteEncoding::Utf8,
        Arc::from(b"before-a".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    b.open(
        1,
        id,
        9,
        ByteEncoding::Utf8,
        Arc::from(b"before-a".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let capture = a.freeze().unwrap();
    let plan = authority
        .prepare(&capture, vec![replace(id, b"after-a")])
        .unwrap();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    assert_eq!(
        barrier.apply().unwrap_err(),
        EditError::AcknowledgementRequired
    );
    drop(barrier);
    let plan = authority
        .prepare(&capture, vec![replace(id, b"after-a")])
        .unwrap();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    a.acknowledge_edit(&barrier, true).unwrap();
    b.acknowledge_edit(&barrier, true).unwrap();
    assert_eq!(
        a.change(
            1,
            id,
            1,
            2,
            ByteEncoding::Utf8,
            Arc::from(b"late".as_slice()),
            BufferIntent::Replace
        ),
        Err(DocumentAnalysisError::BarrierHeld)
    );
    assert_eq!(
        a.open(
            1,
            second,
            1,
            ByteEncoding::Utf8,
            Arc::from(b"before-b".as_slice()),
            BufferIntent::Move { target: id }
        ),
        Err(DocumentAnalysisError::BarrierHeld)
    );
    let outcome = barrier.apply().unwrap();
    assert_eq!(outcome.state, EditOutcomeState::Completed);
    assert_eq!(outcome.verified, vec![id]);
    assert!(outcome.uncertain.is_empty());
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"after-a");
    assert_eq!(fs::read(root.path().join("b.md")).unwrap(), b"before-b");
    let record = journal::load(&authority, &outcome.plan_id)
        .unwrap()
        .unwrap();
    assert_eq!(record.phase, journal::Phase::Completed);
    assert_eq!(record.files[0].before, b"before-a");
    assert_eq!(record.files[0].after, b"after-a");
    assert_eq!(barrier.apply().unwrap_err(), EditError::AlreadyApplied);
}

#[test]
fn buffer_proposal_save_requires_fresh_capture() {
    let (root, host, authority, id, _) = fixture();
    let a = host.client().unwrap();
    a.open(
        1,
        id,
        1,
        ByteEncoding::Utf8,
        Arc::from(b"before-a".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let base = a.freeze().unwrap();
    let proposal = base
        .propose_buffer_edit(id, Arc::from(b"editor-a".as_slice()))
        .unwrap();
    a.accept_buffer_proposal(&proposal, 2).unwrap();
    assert_eq!(
        a.accept_buffer_proposal(&proposal, 3),
        Err(DocumentAnalysisError::StaleVersion)
    );
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"before-a");
    assert!(!root.path().join(".vibe").exists());
    let dirty = a.freeze().unwrap();
    assert_eq!(
        authority
            .prepare(&dirty, vec![replace(id, b"service-a")])
            .unwrap_err(),
        EditError::SaveRequired
    );
    // Explicit editor save is fixture action, not proposal acceptance/apply.
    fs::write(root.path().join("a.md"), b"editor-a").unwrap();
    assert_eq!(
        authority
            .prepare(&dirty, vec![replace(id, b"service-a")])
            .unwrap_err(),
        EditError::SaveRequired
    );
    let saved = a.freeze().unwrap();
    let plan = authority
        .prepare(&saved, vec![replace(id, b"service-a")])
        .unwrap();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    a.acknowledge_edit(&barrier, true).unwrap();
    assert_eq!(barrier.apply().unwrap().state, EditOutcomeState::Completed);
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"service-a");
}
#[test]
fn late_buffer_change_is_stale_and_private() {
    let (root, host, authority, id, _) = fixture();
    let a = host.client().unwrap();
    let b = host.client().unwrap();
    b.open(
        1,
        id,
        4,
        ByteEncoding::Utf8,
        Arc::from(b"before-a".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let capture = a.freeze().unwrap();
    let plan = authority
        .prepare(&capture, vec![replace(id, b"after-a")])
        .unwrap();
    b.change(
        1,
        id,
        4,
        5,
        ByteEncoding::Utf8,
        Arc::from(b"SECRET-other-client".as_slice()),
        BufferIntent::Replace,
    )
    .unwrap();
    let error = plan.barrier(Duration::from_secs(10)).unwrap_err();
    assert_eq!(error, EditError::Stale);
    assert!(!format!("{error:?} {error}").contains("SECRET"));
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"before-a");
    assert!(!root.path().join(".vibe").exists());
    assert_eq!(
        authority
            .prepare(&capture, vec![replace(id, b"after-a")])
            .unwrap_err(),
        EditError::SaveRequired
    );
}
#[test]
fn refused_expired_disconnected_and_new_participant_stop() {
    for scenario in ["refused", "expired", "disconnected", "new"] {
        let (root, host, authority, id, _) = fixture();
        let a = host.client().unwrap();
        let mut b = Some(host.client().unwrap());
        let capture = a.freeze().unwrap();
        let plan = authority
            .prepare(&capture, vec![replace(id, b"after-a")])
            .unwrap();
        if scenario == "new" {
            let _new = host.client().unwrap();
            assert_eq!(
                plan.barrier(Duration::from_secs(10)).unwrap_err(),
                EditError::Stale
            );
        } else {
            let ttl = if scenario == "expired" {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(10)
            };
            let barrier = plan.barrier(ttl).unwrap();
            a.acknowledge_edit(&barrier, true).unwrap();
            let expected = match scenario {
                "refused" => {
                    assert_eq!(
                        b.as_ref().unwrap().acknowledge_edit(&barrier, false),
                        Err(EditError::Refused)
                    );
                    EditError::Refused
                }
                "expired" => {
                    b.as_ref()
                        .unwrap()
                        .acknowledge_edit(&barrier, true)
                        .unwrap();
                    std::thread::sleep(Duration::from_millis(1100));
                    EditError::Expired
                }
                "disconnected" => {
                    b.as_ref()
                        .unwrap()
                        .acknowledge_edit(&barrier, true)
                        .unwrap();
                    drop(b.take());
                    EditError::Disconnected
                }
                _ => unreachable!(),
            };
            assert_eq!(barrier.apply().unwrap_err(), expected);
            assert!(host.client().is_ok());
        }
        assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"before-a");
        assert!(!root.path().join(".vibe").exists());
    }
}
#[test]
fn disk_drift_and_editor_double_apply_refuse() {
    for bytes in [b"third-state".as_slice(), b"after-a".as_slice()] {
        let (root, host, authority, id, _) = fixture();
        let a = host.client().unwrap();
        let capture = a.freeze().unwrap();
        let barrier = authority
            .prepare(&capture, vec![replace(id, b"after-a")])
            .unwrap()
            .barrier(Duration::from_secs(10))
            .unwrap();
        a.acknowledge_edit(&barrier, true).unwrap();
        fs::write(root.path().join("a.md"), bytes).unwrap();
        assert_eq!(barrier.apply().unwrap_err(), EditError::Stale);
        assert_eq!(fs::read(root.path().join("a.md")).unwrap(), bytes);
        assert!(host.client().is_ok());
    }
    let (root, host, authority, id, _) = fixture();
    let a = host.client().unwrap();
    let barrier = authority
        .prepare(&a.freeze().unwrap(), vec![replace(id, b"after-a")])
        .unwrap()
        .barrier(Duration::from_secs(10))
        .unwrap();
    a.acknowledge_edit(&barrier, true).unwrap();
    let lease = vibe_lifecycle::LifecycleLease::acquire(root.path()).unwrap();
    assert_eq!(barrier.apply().unwrap_err(), EditError::Busy);
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"before-a");
    drop(lease);
    assert!(host.client().is_ok());
}
#[test]
fn journal_contract_rejects_unknown_epoch_field_and_poisoned_images() {
    let (_root, host, authority, id, _) = fixture();
    let a = host.client().unwrap();
    let plan = authority
        .prepare(&a.freeze().unwrap(), vec![replace(id, b"after-a")])
        .unwrap();
    let journal = journal::from_plan(&plan).unwrap();
    let raw = serde_json::to_vec(&journal).unwrap();
    assert_eq!(journal::decode(&authority, &raw).unwrap(), journal);
    let mut unknown = serde_json::to_value(&journal).unwrap();
    unknown["foreign"] = serde_json::json!(true);
    assert_eq!(
        journal::decode(&authority, &serde_json::to_vec(&unknown).unwrap()).unwrap_err(),
        EditError::JournalShape
    );
    let mut epoch = journal.clone();
    epoch.schema = 99;
    assert_eq!(
        journal::decode(&authority, &serde_json::to_vec(&epoch).unwrap()).unwrap_err(),
        EditError::JournalEpoch
    );
    let mut poisoned = journal.clone();
    poisoned.files[0].after = b"poison".to_vec();
    assert_eq!(
        journal::decode(&authority, &serde_json::to_vec(&poisoned).unwrap()).unwrap_err(),
        EditError::JournalIntegrity
    );
    let mut coordinated = poisoned.clone();
    coordinated.files[0].after_sha256 = journal::raw_hash(&coordinated.files[0].after);
    assert_eq!(
        journal::decode(&authority, &serde_json::to_vec(&coordinated).unwrap()).unwrap_err(),
        EditError::JournalIntegrity
    );
    let corpus = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/document-edit-journal/e1/prepared.json"
    ));
    let value: vibe_wire::generated::document_edit_journal::DocumentEditJournal =
        serde_json::from_slice(corpus).unwrap();
    assert_eq!(
        serde_json::from_slice::<vibe_wire::generated::document_edit_journal::DocumentEditJournal>(
            &serde_json::to_vec(&value).unwrap()
        )
        .unwrap(),
        value
    );
}
#[test]
fn post_write_partial_cold_recovery_and_third_state() {
    if let Some(path) = std::env::var_os("VIBE_M16_D3_CHILD_ROOT") {
        let root = std::path::PathBuf::from(path);
        let id = std::env::var("VIBE_M16_D3_CHILD_ID").unwrap();
        let host = DocumentAnalysisHost::new(
            &root,
            "vibe.toml",
            vec![
                ResourceDeclaration {
                    path: "vibe.toml".into(),
                    role: ResourceRole::Manifest,
                },
                ResourceDeclaration {
                    path: "a.md".into(),
                    role: ResourceRole::Opaque,
                },
                ResourceDeclaration {
                    path: "b.md".into(),
                    role: ResourceRole::Opaque,
                },
            ],
            AnalysisLimits {
                resource_bytes: 4096,
                capture_bytes: 65536,
                open_buffers: 8,
            },
        )
        .unwrap();
        let a = host.resource("a.md").unwrap();
        let b = host.resource("b.md").unwrap();
        let authority = DocumentEditAuthority::new(host.clone(), vec![a, b]).unwrap();
        let client = host.client().unwrap();
        let capture = client.freeze().unwrap();
        let mode = std::env::var("VIBE_M16_D3_CHILD_MODE").unwrap();
        if mode == "third" {
            assert_eq!(
                authority.prepare_recovery(&capture, &id).unwrap_err(),
                EditError::ThirdState
            );
            assert_eq!(fs::read(root.join("b.md")).unwrap(), b"third-b");
            return;
        }
        let plan = authority.prepare_recovery(&capture, &id).unwrap();
        let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
        client.acknowledge_edit(&barrier, true).unwrap();
        let outcome = barrier.apply().unwrap();
        assert_eq!(outcome.state, EditOutcomeState::Completed);
        assert!(outcome.pending.is_empty());
        assert!(outcome.uncertain.is_empty());
        assert_eq!(fs::read(root.join("a.md")).unwrap(), b"after-a");
        assert_eq!(fs::read(root.join("b.md")).unwrap(), b"after-b");
        return;
    }
    let (root, host, authority, a_id, b_id) = fixture();
    let client = host.client().unwrap();
    let barrier = authority
        .prepare(
            &client.freeze().unwrap(),
            vec![replace(a_id, b"after-a"), replace(b_id, b"after-b")],
        )
        .unwrap()
        .barrier(Duration::from_secs(10))
        .unwrap();
    client.acknowledge_edit(&barrier, true).unwrap();
    vibe_safefs::fail_after_publish(Some("a.md"));
    let outcome = barrier.apply().unwrap();
    vibe_safefs::fail_after_publish(None);
    assert_eq!(outcome.state, EditOutcomeState::Partial);
    assert_eq!(outcome.uncertain, vec![a_id]);
    assert_eq!(outcome.pending, vec![b_id]);
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"after-a");
    assert_eq!(fs::read(root.path().join("b.md")).unwrap(), b"before-b");
    let id = outcome.plan_id;
    drop(barrier);
    drop(client);
    drop(authority);
    drop(host);
    let child = |mode: &str| {
        let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","document_analysis::edit::m_16_d_3_cases::post_write_partial_cold_recovery_and_third_state","--nocapture"])
            .env("VIBE_M16_D3_CHILD_ROOT",root.path()).env("VIBE_M16_D3_CHILD_ID",&id).env("VIBE_M16_D3_CHILD_MODE",mode)
            .env("HOME",root.path().join("isolated-home")).env("XDG_CONFIG_HOME",root.path().join("isolated-config")).env("XDG_CACHE_HOME",root.path().join("isolated-cache")).env("XDG_STATE_HOME",root.path().join("isolated-state")).output().unwrap();
        println!(
            "cold child {mode}: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success());
    };
    fs::write(root.path().join("b.md"), b"third-b").unwrap();
    child("third");
    fs::write(root.path().join("b.md"), b"before-b").unwrap();
    child("complete");
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"after-a");
    assert_eq!(fs::read(root.path().join("b.md")).unwrap(), b"after-b");
}

#[test]
fn participant_disconnect_after_landed_write_stops_second() {
    fn arm(peer: std::rc::Rc<std::cell::RefCell<Option<DocumentClient>>>) {
        vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
            if name == "a.md" {
                drop(peer.borrow_mut().take());
            } else {
                arm(peer.clone());
            }
        })));
    }
    for single in [false, true] {
        let (root, host, authority, a_id, b_id) = fixture();
        let a = host.client().unwrap();
        let b = host.client().unwrap();
        let barrier = authority
            .prepare(
                &a.freeze().unwrap(),
                if single {
                    vec![replace(a_id, b"after-a")]
                } else {
                    vec![replace(a_id, b"after-a"), replace(b_id, b"after-b")]
                },
            )
            .unwrap()
            .barrier(Duration::from_secs(10))
            .unwrap();
        a.acknowledge_edit(&barrier, true).unwrap();
        b.acknowledge_edit(&barrier, true).unwrap();
        let peer = std::rc::Rc::new(std::cell::RefCell::new(Some(b)));
        arm(peer.clone());
        let outcome = barrier.apply().unwrap();
        vibe_safefs::arm_before_publish_verify(None);
        assert!(peer.borrow().is_none());
        assert_eq!(outcome.state, EditOutcomeState::Partial);
        assert_eq!(outcome.reason, Some(EditError::Disconnected));
        assert_eq!(outcome.verified, vec![a_id]);
        assert_eq!(outcome.pending, if single { vec![] } else { vec![b_id] });
        assert_eq!(fs::read(root.path().join("a.md")).unwrap(), b"after-a");
        assert_eq!(fs::read(root.path().join("b.md")).unwrap(), b"before-b");
        let record = journal::load(&authority, &outcome.plan_id)
            .unwrap()
            .unwrap();
        assert_eq!(record.phase, journal::Phase::Recovery);
        assert_eq!(record.files[0].state, journal::FileState::Verified);
        assert!(host.client().is_ok());
    }
}
