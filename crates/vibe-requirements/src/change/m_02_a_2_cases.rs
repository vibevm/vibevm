//! Shared intent snapshots and adoption/personal perturbations, no scheduler.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#BINDING-PLANNING-INDEPENDENCE");
use super::m_02_a_1_cases::Fixture;
use crate::{RequirementsQuery, query};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
const ORIGIN: &str = "spec://org.fixture/reusable-intent/FEAT-001#DELIVERY";
fn shared(f: &Fixture) -> Vec<(String, Vec<u8>)> {
    let mut rows = vec![];
    for (label, root, files) in [
        (
            "origin",
            &f.origin,
            vec!["vibe.toml", "vibevm/vibespecs/FEAT-001-native-intent.md"],
        ),
        (
            "alpha",
            &f.alpha,
            vec!["vibe.toml", "change.toml", "vibevm/vibespecs/CHANGE-001.md"],
        ),
        (
            "beta",
            &f.beta,
            vec!["vibe.toml", "change.toml", "vibevm/vibespecs/CHANGE-001.md"],
        ),
    ] {
        for path in files {
            rows.push((
                format!("{label}/{path}"),
                fs::read(root.join(path)).unwrap(),
            ));
        }
    }
    rows
}
fn snapshot(name: &str, before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)]) {
    assert_eq!(before, after);
    let hashes = before
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":format!("{:x}",Sha256::digest(b))}))
        .collect::<Vec<_>>();
    println!(
        "M02_A2_SHARED {name} {}",
        json!({"before":before,"after":after,"leaf_hashes":hashes})
    );
}
fn all_bytes(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, rows: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, rows);
            } else {
                rows.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut rows = std::collections::BTreeMap::new();
    walk(root, root, &mut rows);
    rows
}
fn requirements(root: &Path) -> vibe_wire::generated::requirements_report::RequirementsReport {
    let r = query(
        &RequirementsQuery::default(),
        &crate::tests_query::ctx(root),
        None,
    )
    .unwrap();
    vibe_wire::behaviour::requirements_report::validate(&r).unwrap();
    r
}
fn assert_no_scheduler(value: &toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                assert!(
                    !["owner", "current", "done"].contains(&key.as_str()),
                    "scheduler key {key} in shared binding"
                );
                assert_no_scheduler(value);
            }
        }
        toml::Value::Array(items) => {
            for item in items {
                assert_no_scheduler(item);
            }
        }
        _ => {}
    }
}
#[test]
fn shared_binding_has_no_scheduler_state_and_plan_perturbation_preserves_identity() {
    let f = Fixture::new();
    for root in [&f.alpha, &f.beta] {
        let b: toml::Value =
            toml::from_str(&fs::read_to_string(root.join("change.toml")).unwrap()).unwrap();
        assert_no_scheduler(&b);
    }
    let before = shared(&f);
    let plan = f.owned.path().join("private-plan.toml");
    fs::write(
        &plan,
        include_str!("../../tests/fixtures/change/personal/plan-before.toml"),
    )
    .unwrap();
    let old_plan = fs::read(&plan).unwrap();
    let old_report = requirements(&f.alpha);
    fs::write(
        &plan,
        include_str!("../../tests/fixtures/change/personal/plan-after.toml"),
    )
    .unwrap();
    let new_plan = fs::read(&plan).unwrap();
    assert_ne!(old_plan, new_plan);
    let new_report = requirements(&f.alpha);
    assert_eq!(
        serde_json::to_value(&old_report).unwrap(),
        serde_json::to_value(&new_report).unwrap()
    );
    snapshot("private-plan", &before, &shared(&f));
    f.assert_origin_leaves_unchanged();
    println!(
        "M02_A2_PLAN {}",
        json!({"before":old_plan,"after":new_plan,"requirements_observation_unchanged":true})
    );
}
#[test]
fn adoption_perturbation_changes_observation_not_shared_intent() {
    use vibe_wire::generated::requirements_report::AdoptionObservationPresence as A;
    let f = Fixture::new();
    let before = shared(&f);
    let old = requirements(&f.alpha);
    let registry_before = all_bytes(&f.alpha);
    let origin_body =
        fs::read_to_string(f.origin.join("vibevm/vibespecs/FEAT-001-native-intent.md")).unwrap();
    let terminal_before = terminal(
        &origin_body,
        "spec://org.fixture/reusable-intent/FEAT-001",
        "DELIVERY",
        &progress_core::evidence::NoEvidence,
    );
    assert_eq!(
        old.rows
            .iter()
            .find(|r| r.address == ORIGIN)
            .unwrap()
            .adoption
            .presence,
        A::Absent
    );
    let mut registry = vibe_facts::Registry::load(&f.alpha).unwrap();
    registry
        .upsert(
            &f.alpha,
            vibe_facts::FactEntry {
                address: ORIGIN.into(),
                origin: vibe_facts::FactOrigin::Package,
                package: Some("org.fixture/reusable-intent".into()),
                status: Some(vibe_facts::FactStatus::parse("impl/done").unwrap()),
                comment: Some("Owned A2 adoption perturbation".into()),
            },
        )
        .unwrap();
    let new = requirements(&f.alpha);
    let old_row = old.rows.iter().find(|r| r.address == ORIGIN).unwrap();
    let new_row = new.rows.iter().find(|r| r.address == ORIGIN).unwrap();
    assert_eq!(new_row.adoption.presence, A::Recorded);
    assert_ne!(
        old.observation.observation_id,
        new.observation.observation_id
    );
    assert_eq!(
        serde_json::to_value(&old_row.authoring).unwrap(),
        serde_json::to_value(&new_row.authoring).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            old.rows
                .iter()
                .find(|r| r.address.ends_with("#ALPHA-ACCEPTANCE"))
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(
            new.rows
                .iter()
                .find(|r| r.address.ends_with("#ALPHA-ACCEPTANCE"))
                .unwrap()
        )
        .unwrap()
    );
    let registry_after = all_bytes(&f.alpha);
    let changed = registry_after
        .iter()
        .filter(|(path, bytes)| registry_before.get(*path) != Some(*bytes))
        .map(|(path, bytes)| json!({"path":path,"before":registry_before.get(path),"after":bytes}))
        .collect::<Vec<_>>();
    assert_eq!(changed.len(), 1);
    assert!(
        registry_before
            .keys()
            .all(|path| registry_after.contains_key(path))
    );
    let terminal_after = terminal(
        &origin_body,
        "spec://org.fixture/reusable-intent/FEAT-001",
        "DELIVERY",
        &progress_core::evidence::NoEvidence,
    );
    assert_eq!(terminal_before, terminal_after);
    assert!(terminal_after.artifacts.iter().all(|a| a.state
        == progress_core::terminal::ArtifactState::Unavailable
        && a.count.is_none()));
    dump_terminal("adoption-before-no-provider", &terminal_before);
    dump_terminal("adoption-after-no-provider", &terminal_after);
    println!(
        "M02_A2_REGISTRY_DELTA {}",
        json!({"changed_files":changed,"shared_intent_unchanged":true})
    );
    snapshot("adoption", &before, &shared(&f));
    f.assert_origin_leaves_unchanged();
    println!(
        "M02_A2_ADOPTION {}",
        json!({"before":old,"after":new,"shared_intent_unchanged":true})
    );
}

struct SnapshotProvider {
    address: String,
    evidence: progress_core::evidence::Evidence,
}
impl progress_core::evidence::EvidenceProvider for SnapshotProvider {
    fn evidence_for(&self, address: &str) -> Option<progress_core::evidence::Evidence> {
        (address == self.address).then(|| self.evidence.clone())
    }
}
fn terminal(
    body: &str,
    document: &str,
    id: &str,
    provider: &dyn progress_core::evidence::EvidenceProvider,
) -> progress_core::terminal::TerminalObservation {
    let parsed = progress_core::parse::parse_document(document, body);
    assert_eq!(parsed.error_count(), 0);
    let fact = parsed
        .blocks
        .iter()
        .flat_map(|b| &b.facts)
        .find(|f| f.id.as_deref() == Some(id))
        .unwrap();
    let address = progress_core::terminal::fact_address(&parsed, fact).unwrap();
    let marker = progress_core::terminal::fact_marker(&parsed, fact).unwrap();
    progress_core::terminal::resolve_terminal(
        &address,
        marker,
        fact.requirements.as_ref(),
        provider,
    )
}
fn dump_terminal(name: &str, observation: &progress_core::terminal::TerminalObservation) {
    println!(
        "M02_A2_TERMINAL {name} {}",
        json!({"outcome":observation.outcome,"artifacts":observation.artifacts})
    );
}
#[test]
fn work_coverage_and_personal_done_cannot_satisfy_missing_implementation() {
    use progress_core::terminal::{ArtifactState, TerminalOutcome};
    let f = Fixture::new();
    let before = shared(&f);
    let plan: toml::Value = toml::from_str(include_str!(
        "../../tests/fixtures/change/personal/plan-after.toml"
    ))
    .unwrap();
    assert!(
        plan["work"]
            .as_array()
            .unwrap()
            .iter()
            .all(|w| w["done"].as_bool() == Some(true))
    );
    fs::write(
        f.owned.path().join("private-plan.toml"),
        include_str!("../../tests/fixtures/change/personal/plan-after.toml"),
    )
    .unwrap();
    let provider = SnapshotProvider {
        address: ORIGIN.into(),
        evidence: serde_json::from_str(include_str!(
            "../../tests/fixtures/change/evidence/work-refs-only.json"
        ))
        .unwrap(),
    };
    assert_eq!(provider.evidence.implements, 0);
    assert_eq!(provider.evidence.verifies, 0);
    assert!(!provider.evidence.refs.is_empty());
    let body =
        fs::read_to_string(f.origin.join("vibevm/vibespecs/FEAT-001-native-intent.md")).unwrap();
    let missing = terminal(
        &body,
        "spec://org.fixture/reusable-intent/FEAT-001",
        "DELIVERY",
        &provider,
    );
    assert_eq!(missing.outcome, TerminalOutcome::Pending);
    assert_eq!(missing.artifacts.len(), 2);
    assert!(
        missing
            .artifacts
            .iter()
            .all(|a| a.state == ArtifactState::Missing
                && a.count == Some(0)
                && a.locators.is_none())
    );
    let unavailable = terminal(
        &body,
        "spec://org.fixture/reusable-intent/FEAT-001",
        "DELIVERY",
        &progress_core::evidence::NoEvidence,
    );
    assert_eq!(unavailable.outcome, TerminalOutcome::Pending);
    assert!(
        unavailable
            .artifacts
            .iter()
            .all(|a| a.state == ArtifactState::Unavailable && a.count.is_none())
    );
    snapshot("work-and-done", &before, &shared(&f));
    f.assert_origin_leaves_unchanged();
    dump_terminal("work-ref-known-zero-missing", &missing);
    dump_terminal("no-provider-unavailable", &unavailable);
    println!(
        "M02_A2_WORK {}",
        json!({"personal_plan":plan,"untyped_legacy_refs":provider.evidence.refs,"typed_work_api":null,"implementation_count":0,"verification_count":0})
    );
}
#[test]
fn existing_artifacts_can_be_observed_without_work_item() {
    use progress_core::terminal::{ArtifactState, TerminalOutcome};
    let f = Fixture::new();
    let before = shared(&f);
    let provider = SnapshotProvider {
        address: ORIGIN.into(),
        evidence: serde_json::from_str(include_str!(
            "../../tests/fixtures/change/evidence/known-artifacts.json"
        ))
        .unwrap(),
    };
    assert!(
        f.owned
            .path()
            .join("private-plan.toml")
            .try_exists()
            .unwrap()
            == false
    );
    let body =
        fs::read_to_string(f.origin.join("vibevm/vibespecs/FEAT-001-native-intent.md")).unwrap();
    let observed = terminal(
        &body,
        "spec://org.fixture/reusable-intent/FEAT-001",
        "DELIVERY",
        &provider,
    );
    assert_eq!(observed.outcome, TerminalOutcome::Terminal);
    assert!(
        observed
            .artifacts
            .iter()
            .all(|a| a.state == ArtifactState::Satisfied
                && a.count == Some(1)
                && a.locators.is_none())
    );
    // The current local authoring marker stays test/plan: artifact existence
    // does not rewrite it or invent a work item/acceptance runtime.
    let local_address = "spec://org.fixture/alpha/CHANGE-001#ALPHA-ACCEPTANCE";
    let local_provider = SnapshotProvider {
        address: local_address.into(),
        evidence: provider.evidence.clone(),
    };
    let local_body = fs::read_to_string(f.alpha.join("vibevm/vibespecs/CHANGE-001.md")).unwrap();
    let local = terminal(
        &local_body,
        "spec://org.fixture/alpha/CHANGE-001",
        "ALPHA-ACCEPTANCE",
        &local_provider,
    );
    assert_eq!(local.outcome, TerminalOutcome::Pending);
    assert!(
        local
            .artifacts
            .iter()
            .all(|a| a.state == ArtifactState::Satisfied)
    );
    let refinement = terminal(
        &local_body,
        "spec://org.fixture/alpha/CHANGE-001",
        "ALPHA-REFINEMENT",
        &progress_core::evidence::NoEvidence,
    );
    assert_eq!(refinement.outcome, TerminalOutcome::Terminal);
    assert_eq!(
        refinement.artifacts[0].locators,
        Some(vec![
            "spec://org.fixture/alpha/CHANGE-001#ALPHA-REFINEMENT".into()
        ])
    );
    snapshot("valid-artifacts-no-work", &before, &shared(&f));
    f.assert_origin_leaves_unchanged();
    dump_terminal("provider-supplied-known-artifacts", &observed);
    dump_terminal("local-plan-despite-artifacts", &local);
    dump_terminal("self-carried-specification", &refinement);
    println!(
        "M02_A2_EVIDENCE {}",
        json!({"provider_snapshot":provider.evidence,"work_items":0,"validity_boundary":"provider-supplied counts; not a new artifact validator; legacy locators remain unavailable"})
    );
}
