use super::*;
use crate::document_analysis::*;
use std::{fs, sync::Arc};
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
fn fixture() -> (tempfile::TempDir, Arc<DocumentAnalysisHost>) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("docs/custom/changes").join(ID);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    let text = format!(
        "schema=1\n[change]\nid='{ID}'\nselected_node='.'\nslug='local'\ntitle='Local intent'\n[[origins]]\npackage='feat:org.fixture/reusable-intent@=1.0.0'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.fixture/reusable-intent/FEAT-001#DELIVERY'\nsha256='sha256:{}'\n",
        "a".repeat(64),
        "b".repeat(64)
    );
    let mut text = text;
    for (role, path, normative, input) in [
        ("intent", "intent.md", true, true),
        ("refinement", "refinement.md", true, true),
        ("acceptance", "acceptance.md", true, true),
        ("work", "work.toml", false, true),
        ("closure", "closure.md", false, false),
    ] {
        text.push_str(&format!(
            "[[documents]]\nrole='{role}'\npath='{path}'\nnormative={normative}\ninput={input}\n"
        ));
    }
    fs::write(dir.join("change.toml"), text).unwrap();
    for (path, raw) in [
        ("intent.md", "Shared local intent\n"),
        ("refinement.md", "Saved refinement\n"),
        ("acceptance.md", "Independent acceptance\n"),
        (
            "work.toml",
            "schema=1\n[[items]]\nid='work-a'\ntouches=['src/product.rs']\n[[items]]\nid='work-b'\ntouches=['src/other.rs']\n",
        ),
        ("closure.md", "Derived closure is not input\n"),
    ] {
        fs::write(dir.join(path), raw).unwrap();
    }
    let mut declarations = vec![ResourceDeclaration {
        path: "vibe.toml".into(),
        role: ResourceRole::Manifest,
    }];
    for path in [
        "change.toml",
        "intent.md",
        "refinement.md",
        "acceptance.md",
        "work.toml",
        "closure.md",
    ] {
        declarations.push(ResourceDeclaration {
            path: format!("docs/custom/changes/{ID}/{path}"),
            role: ResourceRole::Opaque,
        });
    }
    let host = DocumentAnalysisHost::new(
        root.path(),
        "vibe.toml",
        declarations,
        AnalysisLimits {
            resource_bytes: 1048576,
            capture_bytes: 8388608,
            open_buffers: 8,
        },
    )
    .unwrap();
    (root, host)
}
fn resource(host: &DocumentAnalysisHost, path: &str) -> ResourceId {
    host.resource(&format!("docs/custom/changes/{ID}/{path}"))
        .unwrap()
}
fn view(capture: &AnalysisCapture, node: ResourceId) -> CapturedChangeView<'_> {
    analyze_change(capture, node, Some(Path::new("docs/custom")), ID)
}
fn inventory(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut pending = vec![root.to_path_buf()];
    let mut rows = vec![];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                pending.push(p.clone());
                rows.push((p.strip_prefix(root).unwrap().into(), None));
            } else {
                rows.push((
                    p.strip_prefix(root).unwrap().into(),
                    Some(fs::read(&p).unwrap()),
                ));
            }
        }
    }
    rows.sort();
    rows
}
#[test]
fn captured_selected_plan_is_saved_read_only_and_exposes_unavailable_carriage() {
    let (root, host) = fixture();
    let client = host.client().unwrap();
    let capture = client.freeze().unwrap();
    let node = host.resource("vibe.toml").unwrap();
    let v = view(&capture, node);
    let before = inventory(root.path());
    let plan = plan_change_full_pass(&v, "work-a").unwrap();
    assert_eq!(plan.capture_id(), capture.id());
    assert_eq!(plan.selection().work(), "work-a");
    assert_eq!(
        plan.selection().intent(),
        &v.analysis().unwrap().identities.as_ref().unwrap().intent
    );
    assert_eq!(
        plan.selection().operations(),
        [
            vibe_core::manifest::change::ChangeOperationRole::Production,
            vibe_core::manifest::change::ChangeOperationRole::CurrentSourceVerification
        ]
    );
    assert_eq!(
        plan.execution_carriage(),
        "unavailable:m16-change-full-pass"
    );
    assert_eq!(
        plan.unbound_inputs(),
        ["resolution", "declaration", "target"]
    );
    assert_eq!(before, inventory(root.path()));
    assert!(!root.path().join(".vibe").exists());
    client
        .open(
            1,
            resource(&host, "refinement.md"),
            1,
            ByteEncoding::Utf8,
            Arc::from(b"dirty".as_slice()),
            BufferIntent::Replace,
        )
        .unwrap();
    let dirty = client.freeze().unwrap();
    assert!(matches!(
        plan_change_full_pass(&view(&dirty, node), "work-a"),
        Err(FullPassPreparationError::SaveRequired)
    ));
    assert_eq!(before, inventory(root.path()));
}

#[test]
fn exact_selected_request_parks_then_explicitly_continues_to_verification() {
    hosted_case(false);
}
#[test]
fn failed_verification_stops_without_production_retry_or_work_successor() {
    hosted_case(true);
}
fn hosted_case(fail: bool) {
    use super::full_pass_fixture as hosted;
    let (root, _) = fixture();
    hosted::declarations(root.path(), ID);
    // The actual public query preparation selects immutable change + Work,
    // while the existing host owns lifecycle authority separately.
    let selected =
        query::CapturedChangeQuery::prepare(root.path(), Some(Path::new("docs/custom")), ID)
            .unwrap()
            .full_pass_plan("work-a")
            .unwrap();
    let retained = selected.selection().clone();
    let host = Arc::new(hosted::Host::default());
    let (park, original) = hosted::run(root.path(), host.clone());
    assert!(matches!(park, crate::PhaseOutcome::Parked(_)), "{park:?}");
    assert_eq!(original.requested, "full-pass");
    let state = vibe_lifecycle::LifecycleStateStore::peek(root.path())
        .unwrap()
        .unwrap();
    let producer = &state.execution["org.demo/consumer#produce"];
    let tasks = producer.tasks.clone();
    assert_eq!(tasks.len(), 1);
    let task = fs::read(root.path().join(&tasks[0])).unwrap();
    assert!(!task.is_empty());
    assert!(!root.path().join("calls").exists());
    assert!(!root.path().join("src/product.rs").exists());
    assert!(producer.completion_basis.is_none());
    let fingerprint = producer.fingerprint.clone();
    // Explicit owned fake-host fulfillment after park, not lifecycle replay.
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(root.path().join("src/product.rs"), hosted::SOURCE).unwrap();
    if fail {
        fs::write(root.path().join("FAIL-VERIFY"), b"owned failure injection").unwrap();
    }
    let (outcome, resumed) = hosted::run(root.path(), host.clone());
    assert_eq!(original.run_id, resumed.run_id);
    assert_eq!(original.started, resumed.started);
    assert_eq!(original.chain, resumed.chain);
    if fail {
        let crate::PhaseOutcome::Failed { original, .. } = outcome else {
            panic!("verification did not fail: {outcome:?}")
        };
        assert!(format!("{original:#}").contains("selected verification failure"));
    } else {
        assert!(
            matches!(outcome, crate::PhaseOutcome::Completed(_)),
            "{outcome:?}"
        );
    }
    assert_eq!(
        fs::read_to_string(root.path().join("calls")).unwrap(),
        "compile\nobserve\n"
    );
    assert_eq!(
        fs::read(root.path().join("evidence/current-source")).unwrap(),
        hosted::SOURCE
    );
    assert_eq!(
        fs::read(root.path().join("src/product.rs")).unwrap(),
        hosted::SOURCE
    );
    assert!(
        fs::read_to_string(root.path().join("evidence/stdout"))
            .unwrap()
            .contains("1 passed")
    );
    assert!(!root.path().join(&tasks[0]).exists());
    let state = vibe_lifecycle::LifecycleStateStore::peek(root.path())
        .unwrap()
        .unwrap();
    let producer = &state.execution["org.demo/consumer#produce"];
    assert_eq!(producer.fingerprint, fingerprint);
    assert!(
        producer.completion_basis.is_none(),
        "B240 must not be reconstructed"
    );
    assert!(producer.tasks.is_empty());
    assert_eq!(producer.artifacts.len(), 1);
    assert_eq!(
        producer.artifacts[0].measured_run_id.as_deref(),
        Some(original.run_id.as_str())
    );
    assert!(producer.artifacts[0].witness.is_some());
    assert_eq!(selected.selection(), &retained);
    assert_eq!(selected.selection().work(), "work-a");
    assert!(host.paid.lock().unwrap().is_empty());
    let resolved = host.resolved.lock().unwrap();
    assert_eq!(
        resolved.len(),
        3,
        "park, resume and current declaration reconciliation"
    );
    assert!(
        resolved
            .iter()
            .all(|r| r.address == "spec://org.demo/consumer/PROMPT#root")
    );
    use sha2::{Digest, Sha256};
    eprintln!(
        "A2_HOSTED_EVIDENCE {}",
        serde_json::json!({"failed_verify":fail,"original_run":original.run_id,"resumed_run":resumed.run_id,"requested":original.requested,"chain":original.chain,"selected_work":selected.selection().work(),"intent":selected.selection().intent().digest(),"contribution":selected.selection().contribution().digest(),"task":tasks[0],"task_sha256":format!("{:x}",Sha256::digest(&task)),"fingerprint":fingerprint,"artifact_run":producer.artifacts[0].measured_run_id,"hosted_basis":producer.completion_basis,"calls":"compile\nobserve\n","source_sha256":format!("{:x}",Sha256::digest(hosted::SOURCE)),"prompt_resolutions":resolved.len(),"paid_calls":0,"carriage":selected.execution_carriage()})
    );
}
