use super::*;
use std::time::Duration;
use vibe_workspace::derivatives::{RebaseHandoff, RebaseReview, capture_retained};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

fn host_at(
    root: &Path,
) -> (
    Arc<DocumentAnalysisHost>,
    Arc<DocumentEditAuthority>,
    ResourceId,
) {
    let host = DocumentAnalysisHost::new(
        root,
        "vibe.toml",
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "authored.toml".into(),
                role: ResourceRole::Opaque,
            },
        ],
        AnalysisLimits {
            resource_bytes: 1 << 20,
            capture_bytes: 4 << 20,
            open_buffers: 8,
        },
    )
    .unwrap();
    let id = host.resource("authored.toml").unwrap();
    let authority = DocumentEditAuthority::new(host.clone(), vec![id]).unwrap();
    (host, authority, id)
}
struct Scenario {
    f: Fixture,
    host: Arc<DocumentAnalysisHost>,
    authority: Arc<DocumentEditAuthority>,
    id: ResourceId,
    handoff: Option<RebaseHandoff>,
}
impl Scenario {
    fn new(family: &str) -> Self {
        let f = Fixture::new(family);
        let old = capture_retained(&f.base, f.binding.base()).unwrap();
        let local = capture_retained(&f.local, f.binding.local()).unwrap();
        let mut next = f.binding.base().clone();
        next.files[0].bytes.extend_from_slice(b"\n");
        f.base
            .write_atomic(&next.files[0].path, &next.files[0].bytes)
            .unwrap();
        let new = capture_retained(&f.base, &next).unwrap();
        let binding = DerivationBinding::new(
            next.clone(),
            f.binding.local().clone(),
            next.observation_id().unwrap(),
            DerivationRecipe::ReplaceSelectedInventory,
        )
        .unwrap();
        let text = toml::to_string(&binding).unwrap();
        let handoff = RebaseReview::compare(&f.text, Some(&old), &new, &local)
            .unwrap()
            .value
            .select(&[&text], &new, &local)
            .unwrap()
            .value;
        let root = f._home.path().join("authored");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("vibe.toml"),
            "[project]\nname='demo'\nversion='1.0.0'\n",
        )
        .unwrap();
        fs::write(root.join("authored.toml"), &f.text).unwrap();
        let (host, authority, id) = host_at(&root);
        Self {
            f,
            host,
            authority,
            id,
            handoff: Some(handoff),
        }
    }
    fn plan(&mut self, client: &DocumentClient) -> Result<DerivativeRebasePlan, EditError> {
        self.authority.prepare_derivative_rebase(
            &client.freeze().unwrap(),
            self.id,
            self.handoff.take().unwrap(),
            &self.f.output,
            "result.json",
            None,
        )
    }
}
fn apply_family(family: &str) {
    let mut s = Scenario::new(family);
    let client = s.host.client().unwrap();
    let peer = s.host.client().unwrap();
    let plan = s.plan(&client).unwrap();
    let id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    peer.acknowledge_edit(barrier.edit_barrier(), true).unwrap();
    let result = barrier.apply(&s.f.base, &s.f.local, &s.f.output).unwrap();
    assert_eq!(result.authored, DerivativeEffect::Completed);
    assert_eq!(result.result, DerivativeEffect::Completed);
    let record = derivative_journal::load(&s.authority, &id).unwrap();
    let previous = derivative_journal::result(&record.previous_result).unwrap();
    assert_eq!(previous.binding_toml, s.f.text);
    let desired = derivative_journal::result(&record.desired_result).unwrap();
    assert_eq!(
        desired.checked_binding().unwrap().local().files,
        s.f.expected
    );
    assert_ne!(previous.provenance_identity, desired.provenance_identity);
    let before =
        s.f.output
            .read_file_snapshot_bounded("result.json", 1 << 20)
            .unwrap();
    drop(barrier);
    let recovery = s
        .authority
        .recover_derivative_rebase(
            &client.freeze().unwrap(),
            &id,
            &s.f.base,
            &s.f.local,
            &s.f.output,
        )
        .unwrap()
        .barrier(Duration::from_secs(10))
        .unwrap();
    client
        .acknowledge_edit(recovery.edit_barrier(), true)
        .unwrap();
    peer.acknowledge_edit(recovery.edit_barrier(), true)
        .unwrap();
    let replay = recovery.apply(&s.f.base, &s.f.local, &s.f.output).unwrap();
    assert_eq!(replay.result, DerivativeEffect::Completed);
    assert_eq!(
        before,
        s.f.output
            .read_file_snapshot_bounded("result.json", 1 << 20)
            .unwrap()
    );
    assert!(!s.f._home.path().join("DO_NOT_EXECUTE").exists());
}
#[test]
fn commonmark_guarded_rebase_replay() {
    apply_family("commonmark")
}
#[test]
fn xml_data_guarded_rebase_replay() {
    apply_family("xml-data")
}
#[test]
fn vibe_spec_guarded_rebase_replay() {
    apply_family("vibe-spec")
}
#[test]
fn opaque_guarded_rebase_replay() {
    apply_family("opaque-file")
}
#[test]
fn tree_guarded_rebase_replay() {
    apply_family("nested-tree")
}
#[test]
fn dirty_concurrent_refused_expired_disconnected_and_stale_base() {
    for mode in [
        "dirty",
        "concurrent",
        "refused",
        "expired",
        "disconnected",
        "disk",
        "base",
    ] {
        let mut s = Scenario::new("opaque-file");
        let client = s.host.client().unwrap();
        let mut peer = Some(s.host.client().unwrap());
        if mode == "dirty" {
            peer.as_ref()
                .unwrap()
                .open(
                    1,
                    s.id,
                    1,
                    ByteEncoding::Utf8,
                    Arc::from(b"dirty".as_slice()),
                    BufferIntent::Replace,
                )
                .unwrap();
            assert_eq!(s.plan(&client).unwrap_err(), EditError::SaveRequired);
            continue;
        }
        let plan = s.plan(&client).unwrap();
        if mode == "concurrent" {
            peer.as_ref()
                .unwrap()
                .open(
                    1,
                    s.id,
                    1,
                    ByteEncoding::Utf8,
                    Arc::from(b"dirty".as_slice()),
                    BufferIntent::Replace,
                )
                .unwrap();
            assert_eq!(
                plan.barrier(Duration::from_secs(10)).unwrap_err(),
                EditError::Stale
            );
            continue;
        }
        let ttl = if mode == "expired" {
            Duration::ZERO
        } else {
            Duration::from_secs(10)
        };
        let barrier = plan.barrier(ttl).unwrap();
        if mode != "expired" {
            client
                .acknowledge_edit(barrier.edit_barrier(), true)
                .unwrap();
        }
        let expected = match mode {
            "refused" => {
                assert_eq!(
                    peer.as_ref()
                        .unwrap()
                        .acknowledge_edit(barrier.edit_barrier(), false),
                    Err(EditError::Refused)
                );
                EditError::Refused
            }
            "expired" => EditError::Expired,
            _ => {
                peer.as_ref()
                    .unwrap()
                    .acknowledge_edit(barrier.edit_barrier(), true)
                    .unwrap();
                match mode {
                    "disconnected" => {
                        drop(peer.take());
                        EditError::Disconnected
                    }
                    "disk" => {
                        fs::write(s.host.root().join("authored.toml"), b"third").unwrap();
                        EditError::Stale
                    }
                    _ => {
                        s.f.base
                            .write_atomic(&s.f.binding.base().files[0].path, b"drift")
                            .unwrap();
                        EditError::Stale
                    }
                }
            }
        };
        assert_eq!(
            barrier
                .apply(&s.f.base, &s.f.local, &s.f.output)
                .unwrap_err(),
            expected
        );
        assert!(!s.f.output.root_path().join("result.json").exists());
        if mode != "disk" {
            assert_eq!(
                fs::read_to_string(s.host.root().join("authored.toml")).unwrap(),
                s.f.text
            )
        }
    }
}
#[test]
fn publication_faults_preserve_before_and_truthful_effects() {
    for (target, after) in [
        ("authored.toml", false),
        ("authored.toml", true),
        ("result.json", false),
        ("result.json", true),
    ] {
        let mut s = Scenario::new("nested-tree");
        let client = s.host.client().unwrap();
        let plan = s.plan(&client).unwrap();
        let id = plan.id().to_owned();
        let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
        client
            .acknowledge_edit(barrier.edit_barrier(), true)
            .unwrap();
        if after {
            vibe_safefs::fail_after_publish(Some(target))
        } else {
            vibe_safefs::fail_before_publish(Some(target))
        }
        let result = barrier.apply(&s.f.base, &s.f.local, &s.f.output);
        vibe_safefs::fail_after_publish(None);
        vibe_safefs::fail_before_publish(None);
        let result = result.unwrap();
        assert_eq!(
            result.authored,
            if target == "result.json" {
                DerivativeEffect::Completed
            } else if after {
                DerivativeEffect::Unknown
            } else {
                DerivativeEffect::Pending
            }
        );
        assert_eq!(
            result.result,
            if target == "result.json" && after {
                DerivativeEffect::Unknown
            } else {
                DerivativeEffect::Pending
            }
        );
        let retained = derivative_journal::load(&s.authority, &id).unwrap();
        assert_eq!(
            derivative_journal::result(&retained.previous_result)
                .unwrap()
                .binding_toml,
            s.f.text
        );
        capture_retained(&s.f.local, s.f.binding.local()).unwrap();
        drop(barrier);
        let recovered = s
            .authority
            .recover_derivative_rebase(
                &client.freeze().unwrap(),
                &id,
                &s.f.base,
                &s.f.local,
                &s.f.output,
            )
            .unwrap()
            .barrier(Duration::from_secs(10))
            .unwrap();
        client
            .acknowledge_edit(recovered.edit_barrier(), true)
            .unwrap();
        assert_eq!(
            recovered
                .apply(&s.f.base, &s.f.local, &s.f.output)
                .unwrap()
                .result,
            DerivativeEffect::Completed
        );
    }
}
#[test]
fn cold_recovery_and_unknown_third_state() {
    if let Some(home) = std::env::var_os("VIBE_M10C2_HOME") {
        let home = std::path::PathBuf::from(home);
        let (_host, authority, _) = host_at(&home.join("authored"));
        let client = authority.host.client().unwrap();
        let id = std::env::var("VIBE_M10C2_ID").unwrap();
        let base = Project::open(&home.join("base")).unwrap();
        let local = Project::open(&home.join("local")).unwrap();
        let output = Project::open(&home.join("output")).unwrap();
        let plan = authority.recover_derivative_rebase(
            &client.freeze().unwrap(),
            &id,
            &base,
            &local,
            &output,
        );
        if std::env::var("VIBE_M10C2_MODE").unwrap() == "third" {
            assert_eq!(plan.unwrap_err(), EditError::ThirdState);
            return;
        }
        let barrier = plan.unwrap().barrier(Duration::from_secs(10)).unwrap();
        client
            .acknowledge_edit(barrier.edit_barrier(), true)
            .unwrap();
        assert_eq!(
            barrier.apply(&base, &local, &output).unwrap().result,
            DerivativeEffect::Completed
        );
        return;
    }
    let mut s = Scenario::new("opaque-file");
    let client = s.host.client().unwrap();
    let plan = s.plan(&client).unwrap();
    let id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    vibe_safefs::fail_after_publish(Some("authored.toml"));
    let result = barrier.apply(&s.f.base, &s.f.local, &s.f.output);
    vibe_safefs::fail_after_publish(None);
    assert_eq!(result.unwrap().authored, DerivativeEffect::Unknown);
    drop(barrier);
    drop(client);
    drop(s.authority);
    drop(s.host);
    let run = |mode: &str| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "document_analysis::edit::m_10_c_2_cases::cold_recovery_and_unknown_third_state",
                "--nocapture",
            ])
            .env("VIBE_M10C2_HOME", s.f._home.path())
            .env("VIBE_M10C2_ID", &id)
            .env("VIBE_M10C2_MODE", mode)
            .output()
            .unwrap();
        println!(
            "cold {mode}: {}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success());
    };
    let target = s.f._home.path().join("authored/authored.toml");
    let desired = fs::read(&target).unwrap();
    fs::write(&target, b"third").unwrap();
    run("third");
    assert_eq!(fs::read(&target).unwrap(), b"third");
    fs::write(&target, desired).unwrap();
    run("complete");
    run("complete");
}
#[test]
fn registered_carrier_roundtrip_and_unknown_fields_refuse() {
    use vibe_wire::generated::derivative_rebase_journal::DerivativeRebaseJournal;
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/derivative-rebase-journal/e1/shape.json"
    ));
    let value: DerivativeRebaseJournal = serde_json::from_str(raw).unwrap();
    assert_eq!(
        serde_json::from_slice::<DerivativeRebaseJournal>(&serde_json::to_vec(&value).unwrap())
            .unwrap(),
        value
    );
    let mut unknown: serde_json::Value = serde_json::from_str(raw).unwrap();
    unknown["foreign"] = true.into();
    assert!(serde_json::from_value::<DerivativeRebaseJournal>(unknown).is_err());
}

#[test]
fn interrupted_before_edit_journal_and_foreign_result_refuse() {
    let mut s = Scenario::new("opaque-file");
    let client = s.host.client().unwrap();
    let plan = s.plan(&client).unwrap();
    let id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    let edit_name = format!("{}.json", barrier.edit_barrier().plan.id);
    vibe_safefs::fail_before_publish(Some(&edit_name));
    let failed = barrier.apply(&s.f.base, &s.f.local, &s.f.output);
    vibe_safefs::fail_before_publish(None);
    assert_eq!(failed.unwrap_err(), EditError::RecoveryRequired);
    assert_eq!(
        fs::read_to_string(s.host.root().join("authored.toml")).unwrap(),
        s.f.text
    );
    assert_eq!(
        s.authority
            .inspect_derivative_rebase(&id, &s.f.output)
            .unwrap()
            .authored,
        DerivativeEffect::Pending
    );
    drop(barrier);
    s.f.output.write_atomic("result.json", b"FOREIGN").unwrap();
    assert_eq!(
        s.authority
            .recover_derivative_rebase(
                &client.freeze().unwrap(),
                &id,
                &s.f.base,
                &s.f.local,
                &s.f.output
            )
            .unwrap_err(),
        EditError::ThirdState
    );
    assert_eq!(
        s.f.output
            .read_file_bounded("result.json", 64)
            .unwrap()
            .unwrap(),
        b"FOREIGN"
    );
    // Explicit fixture removal of foreign test data, never recovery cleanup.
    s.f.output.remove_file("result.json").unwrap();
    let recovery = s
        .authority
        .recover_derivative_rebase(
            &client.freeze().unwrap(),
            &id,
            &s.f.base,
            &s.f.local,
            &s.f.output,
        )
        .unwrap()
        .barrier(Duration::from_secs(10))
        .unwrap();
    client
        .acknowledge_edit(recovery.edit_barrier(), true)
        .unwrap();
    assert_eq!(
        recovery
            .apply(&s.f.base, &s.f.local, &s.f.output)
            .unwrap()
            .result,
        DerivativeEffect::Completed
    );
}
#[test]
fn repeated_rebase_retains_complete_prior_chain() {
    let mut s = Scenario::new("xml-data");
    let client = s.host.client().unwrap();
    let plan = s.plan(&client).unwrap();
    let first_id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    assert_eq!(
        barrier
            .apply(&s.f.base, &s.f.local, &s.f.output)
            .unwrap()
            .result,
        DerivativeEffect::Completed
    );
    drop(barrier);
    let first = derivative_journal::load(&s.authority, &first_id).unwrap();
    let prior = derivative_journal::result(&first.desired_result).unwrap();
    let binding = prior.checked_binding().unwrap();
    let old = capture_retained(&s.f.base, binding.base()).unwrap();
    let local = capture_retained(&s.f.local, binding.local()).unwrap();
    let mut next = binding.base().clone();
    next.files[0].bytes.extend_from_slice(b"\n");
    s.f.base
        .write_atomic(&next.files[0].path, &next.files[0].bytes)
        .unwrap();
    let new = capture_retained(&s.f.base, &next).unwrap();
    let next_binding = DerivationBinding::new(
        next.clone(),
        binding.local().clone(),
        next.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let text = toml::to_string(&next_binding).unwrap();
    let handoff = RebaseReview::compare(&prior.binding_toml, Some(&old), &new, &local)
        .unwrap()
        .value
        .select(&[&text], &new, &local)
        .unwrap()
        .value;
    let plan = s
        .authority
        .prepare_derivative_rebase(
            &client.freeze().unwrap(),
            s.id,
            handoff,
            &s.f.output,
            "second.json",
            Some(&first_id),
        )
        .unwrap();
    let second_id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    assert_eq!(
        barrier
            .apply(&s.f.base, &s.f.local, &s.f.output)
            .unwrap()
            .result,
        DerivativeEffect::Completed
    );
    let second = s
        .authority
        .inspect_derivative_rebase_intent(&second_id)
        .unwrap();
    assert_eq!(second.prior, Some(first_id.clone()));
    assert_eq!(second.previous_result, first.desired_result);
    assert_eq!(
        derivative_journal::load(&s.authority, &first_id).unwrap(),
        first
    );
}

#[test]
fn failed_replay_keeps_already_completed_result_visible() {
    let mut s = Scenario::new("opaque-file");
    let client = s.host.client().unwrap();
    let plan = s.plan(&client).unwrap();
    let id = plan.id().to_owned();
    let barrier = plan.barrier(Duration::from_secs(10)).unwrap();
    client
        .acknowledge_edit(barrier.edit_barrier(), true)
        .unwrap();
    assert_eq!(
        barrier
            .apply(&s.f.base, &s.f.local, &s.f.output)
            .unwrap()
            .result,
        DerivativeEffect::Completed
    );
    drop(barrier);
    let recovery = s
        .authority
        .recover_derivative_rebase(
            &client.freeze().unwrap(),
            &id,
            &s.f.base,
            &s.f.local,
            &s.f.output,
        )
        .unwrap()
        .barrier(Duration::from_secs(10))
        .unwrap();
    client
        .acknowledge_edit(recovery.edit_barrier(), true)
        .unwrap();
    let edit_name = format!("{}.json", recovery.edit_barrier().plan.id);
    // Fail the next edit-journal checkpoint after the initial recovery checkpoint.
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |_, name| {
        assert_eq!(name, edit_name);
        vibe_safefs::fail_before_publish(Some(&format!(".vibe/document-edits/{edit_name}")));
    })));
    let outcome = recovery.apply(&s.f.base, &s.f.local, &s.f.output);
    vibe_safefs::arm_before_publish_verify(None);
    vibe_safefs::fail_before_publish(None);
    let outcome = outcome.unwrap();
    assert!(outcome.reason.is_some());
    assert_eq!(outcome.result, DerivativeEffect::Completed);
    assert_ne!(outcome.authored, DerivativeEffect::Completed);
}
