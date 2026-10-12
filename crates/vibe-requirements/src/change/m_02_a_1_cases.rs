//! Native origin and independent local authored bindings over incumbent facts.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#ORIGIN-LOCAL-IDENTITY");
use crate::{RequirementsQuery, query};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use vibe_core::{
    ContentHash,
    manifest::{CURRENT_SCHEMA_VERSION, Manifest, SpecFormat},
};
use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, FactStatusState, RequiredArtifactKind as K, RequirementRow,
    RequirementSourceKind, RequirementsReport, SourceResultState,
};
const ORIGIN_MANIFEST: &str = include_str!("../../tests/fixtures/change/origin/vibe.toml");
const ORIGIN_DOC: &str =
    include_str!("../../tests/fixtures/change/origin/vibevm/vibespecs/FEAT-001-native-intent.md");
const ORIGIN_PATH: &str = "vibevm/vibespecs/FEAT-001-native-intent.md";
const LOCAL_PATH: &str = "vibevm/vibespecs/CHANGE-001.md";
const ORIGIN_ADDRESS: &str = "spec://org.fixture/reusable-intent/FEAT-001#DELIVERY";
fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn write(root: &Path, relative: &str, body: &str) {
    let p = root.join(relative);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}
struct Fixture {
    owned: tempfile::TempDir,
    origin: PathBuf,
    alpha: PathBuf,
    beta: PathBuf,
    alpha_slot: PathBuf,
    beta_slot: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::tempdir().unwrap();
        let origin = owned.path().join("origin");
        let alpha = owned.path().join("alpha");
        let beta = owned.path().join("beta");
        write(&origin, "vibe.toml", ORIGIN_MANIFEST);
        write(&origin, ORIGIN_PATH, ORIGIN_DOC);
        for (root, manifest, document, binding) in [
            (
                &alpha,
                include_str!("../../tests/fixtures/change/alpha/vibe.toml"),
                include_str!("../../tests/fixtures/change/alpha/vibevm/vibespecs/CHANGE-001.md"),
                include_str!("../../tests/fixtures/change/alpha/change.toml"),
            ),
            (
                &beta,
                include_str!("../../tests/fixtures/change/beta/vibe.toml"),
                include_str!("../../tests/fixtures/change/beta/vibevm/vibespecs/CHANGE-001.md"),
                include_str!("../../tests/fixtures/change/beta/change.toml"),
            ),
        ] {
            write(root, "vibe.toml", manifest);
            write(root, LOCAL_PATH, document);
            write(root, "change.toml", binding);
            let group = "org.fixture".parse().unwrap();
            let version = "1.0.0".parse().unwrap();
            // Record/lock fixture identity only, never authenticated package proof.
            let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
            vibe_workspace::vibedeps::materialise_with_spec_format(
                root,
                &group,
                "reusable-intent",
                &version,
                &origin,
                vibe_workspace::vibedeps::CopyMode::Copy,
                SpecFormat::Mixed,
                &hash,
            )
            .unwrap();
            write(
                root,
                "vibe.lock",
                &format!(
                    "[meta]\ngenerated_by='fixture'\ngenerated_at='2026-10-12T00:00:00Z'\nschema_version={CURRENT_SCHEMA_VERSION}\n[[package]]\nkind='feat'\ngroup='org.fixture'\nname='reusable-intent'\nversion='1.0.0'\nsource_url='fixture://owned-origin'\ncontent_hash='{hash}'\n"
                ),
            );
        }
        let slot = |root: &Path| {
            vibe_workspace::vibedeps::slot_abs_path(
                root,
                &"org.fixture".parse().unwrap(),
                "reusable-intent",
                &"1.0.0".parse().unwrap(),
            )
        };
        let alpha_slot = slot(&alpha);
        let beta_slot = slot(&beta);
        Self {
            owned,
            origin,
            alpha,
            beta,
            alpha_slot,
            beta_slot,
        }
    }
    fn assert_origin_leaves_unchanged(&self) {
        for (path, bytes) in [
            ("vibe.toml", ORIGIN_MANIFEST.as_bytes()),
            (ORIGIN_PATH, ORIGIN_DOC.as_bytes()),
        ] {
            for (role, root) in [
                ("origin", &self.origin),
                ("alpha-slot", &self.alpha_slot),
                ("beta-slot", &self.beta_slot),
            ] {
                let actual = fs::read(root.join(path)).unwrap();
                assert_eq!(actual, bytes);
                println!(
                    "M02_A1_LEAF {}",
                    json!({"role":role,"path":path,"sha256":sha(&actual),"bytes":actual.len()})
                );
            }
        }
    }
}
fn report(root: &Path) -> RequirementsReport {
    query(
        &RequirementsQuery::default(),
        &crate::tests_query::ctx(root),
        None,
    )
    .unwrap()
}
fn binding(root: &Path) -> toml::Value {
    toml::from_str(&fs::read_to_string(root.join("change.toml")).unwrap()).unwrap()
}
fn row<'a>(report: &'a RequirementsReport, address: &str) -> &'a RequirementRow {
    report
        .rows
        .iter()
        .find(|r| r.address == address)
        .unwrap_or_else(|| panic!("missing {address}: {report:?}"))
}
fn dump(name: &str, report: &RequirementsReport) {
    vibe_wire::behaviour::requirements_report::validate(report).unwrap();
    println!(
        "M02_A1_REPORT {name} {}",
        serde_json::to_string(report).unwrap()
    );
}
#[test]
fn native_origin_observation_preserves_artifact_requirements() {
    let package = Manifest::parse_str(ORIGIN_MANIFEST)
        .unwrap()
        .package
        .unwrap();
    assert_eq!(package.kind, vibe_core::PackageKind::Feat);
    let observed =
        vibe_facts::observe_declared_document(&package, ORIGIN_PATH, Some(ORIGIN_DOC.as_bytes()))
            .unwrap();
    let vibe_facts::DeclaredDocumentObservation::Facts { provenance, facts } = observed else {
        panic!("native origin not classified as facts: {observed:?}")
    };
    assert_eq!(provenance.reader, Some("vibe-specdoc:markdown/spec"));
    let delivery = facts.iter().find(|f| f.address == ORIGIN_ADDRESS).unwrap();
    assert!(delivery.requirements.is_some());
    let f = Fixture::new();
    let r = report(&f.alpha);
    assert_eq!(
        row(&r, ORIGIN_ADDRESS).authoring.requires,
        Some(vec![K::Implementation, K::Verification])
    );
    assert_eq!(
        row(&r, "spec://org.fixture/reusable-intent/FEAT-001#FAILURE")
            .authoring
            .requires,
        Some(vec![K::Specification, K::Implementation, K::Verification])
    );
    f.assert_origin_leaves_unchanged();
    dump("native-origin", &r);
    println!(
        "M02_A1_ORIGIN {}",
        json!({"package":"feat:org.fixture/reusable-intent@=1.0.0","document_sha256":sha(ORIGIN_DOC.as_bytes()),"manifest_sha256":sha(ORIGIN_MANIFEST.as_bytes()),"facts":facts.iter().map(|f|&f.address).collect::<Vec<_>>()})
    );
}
#[test]
fn two_projects_reuse_exact_origin_with_distinct_local_intent() {
    let f = Fixture::new();
    let a = binding(&f.alpha);
    let b = binding(&f.beta);
    assert_eq!(a["origin"], b["origin"]);
    assert_eq!(
        a["origin"]["package"].as_str(),
        Some("feat:org.fixture/reusable-intent@=1.0.0")
    );
    assert_eq!(
        a["origin"]["document_sha256"].as_str(),
        Some(sha(ORIGIN_DOC.as_bytes()).as_str())
    );
    assert_eq!(
        a["origin"]["manifest_sha256"].as_str(),
        Some(sha(ORIGIN_MANIFEST.as_bytes()).as_str())
    );
    assert_ne!(a["local"], b["local"]);
    assert_ne!(
        fs::read(f.alpha.join(LOCAL_PATH)).unwrap(),
        fs::read(f.beta.join(LOCAL_PATH)).unwrap()
    );
    let alpha = report(&f.alpha);
    let beta = report(&f.beta);
    let source = |r: &RequirementsReport| {
        r.sources
            .iter()
            .find(|s| s.source.package == "org.fixture/reusable-intent")
            .unwrap()
            .clone()
    };
    assert_eq!(source(&alpha).state, SourceResultState::Available);
    assert_eq!(source(&alpha).digest, source(&beta).digest);
    for (name, binding, r) in [("alpha", &a, &alpha), ("beta", &b, &beta)] {
        for address in binding["origin"]["facts"].as_array().unwrap() {
            let item = row(r, address.as_str().unwrap());
            assert_eq!(item.source.kind, RequirementSourceKind::Package);
            assert_eq!(item.source.package, "org.fixture/reusable-intent");
        }
        for field in ["refinements", "acceptance"] {
            for address in binding["local"][field].as_array().unwrap() {
                let item = row(r, address.as_str().unwrap());
                assert_eq!(item.source.kind, RequirementSourceKind::Host);
                assert_eq!(item.source.package, format!("org.fixture/{name}"));
            }
        }
        assert_eq!(
            row(r, binding["local"]["acceptance"][0].as_str().unwrap())
                .authoring
                .status
                .as_ref()
                .unwrap()
                .state,
            FactStatusState::Plan
        );
        dump(name, r);
    }
    f.assert_origin_leaves_unchanged();
    println!(
        "M02_A1_BINDINGS {}",
        json!({"alpha":a,"beta":b,"origin_exact_bytes_equal":true,"temporary_owned_root":f.owned.path()})
    );
}
#[test]
fn copied_package_status_cannot_replace_missing_selected_local_acceptance() {
    let f = Fixture::new();
    let selected = binding(&f.alpha)["local"]["acceptance"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let body = fs::read_to_string(f.alpha.join(LOCAL_PATH)).unwrap();
    let mut changed = body
        .lines()
        .filter(|line| !line.starts_with("@fact:ALPHA-ACCEPTANCE"))
        .collect::<Vec<_>>()
        .join("\n");
    changed.push_str("\n\n");
    changed.push_str(
        ORIGIN_DOC
            .lines()
            .find(|line| line.starts_with("@fact:DELIVERY"))
            .unwrap(),
    );
    changed.push('\n');
    fs::write(f.alpha.join(LOCAL_PATH), changed).unwrap();
    let r = report(&f.alpha);
    assert!(r.rows.iter().all(|r| r.address != selected));
    let copied = row(&r, "spec://org.fixture/alpha/CHANGE-001#DELIVERY");
    assert_eq!(copied.source.kind, RequirementSourceKind::Host);
    assert_eq!(
        copied.authoring.status.as_ref().unwrap().state,
        FactStatusState::Done
    );
    assert_ne!(copied.address, selected);
    assert_eq!(
        row(&r, ORIGIN_ADDRESS).source.kind,
        RequirementSourceKind::Package
    );
    f.assert_origin_leaves_unchanged();
    dump("copied-status-selected-local-missing", &r);
    println!(
        "M02_A1_NEGATIVE {}",
        json!({"selected_local_acceptance":selected,"selected_local_fact_present":false,"copied_status_address":copied.address,"runtime_acceptance_verdict":null})
    );
    // Even copying the done marker onto the selected local ID remains an
    // authoring observation, not implementation/verification evidence.
    let copied_selected = ORIGIN_DOC
        .lines()
        .find(|line| line.starts_with("@fact:DELIVERY"))
        .unwrap()
        .replacen("@fact:DELIVERY", "@fact:ALPHA-ACCEPTANCE", 1);
    let body = fs::read_to_string(f.alpha.join(LOCAL_PATH)).unwrap();
    fs::write(
        f.alpha.join(LOCAL_PATH),
        format!("{body}\n{copied_selected}\n"),
    )
    .unwrap();
    let copied_report = report(&f.alpha);
    let copied_selected_row = row(&copied_report, &selected);
    assert_eq!(
        copied_selected_row.authoring.status.as_ref().unwrap().state,
        FactStatusState::Done
    );
    assert_eq!(
        copied_selected_row.authoring.requires,
        Some(vec![K::Implementation, K::Verification])
    );
    let encoded = serde_json::to_value(copied_selected_row).unwrap();
    for field in ["terminal", "accepted", "fulfilled"] {
        assert!(encoded.get(field).is_none());
    }
    f.assert_origin_leaves_unchanged();
    dump(
        "copied-selected-done-is-metadata-not-terminal-proof",
        &copied_report,
    );
}
#[test]
fn implicit_adoption_is_not_local_acceptance() {
    let f = Fixture::new();
    for (name, root) in [("alpha", &f.alpha), ("beta", &f.beta)] {
        let r = report(root);
        let origin = row(&r, ORIGIN_ADDRESS);
        assert_eq!(
            origin.authoring.status.as_ref().unwrap().state,
            FactStatusState::Done
        );
        assert_eq!(
            origin.adoption.presence,
            AdoptionObservationPresence::Absent
        );
        let local = row(
            &r,
            binding(root)["local"]["acceptance"][0].as_str().unwrap(),
        );
        assert_eq!(
            local.adoption.presence,
            AdoptionObservationPresence::NotApplicable
        );
        assert_eq!(
            local.authoring.status.as_ref().unwrap().state,
            FactStatusState::Plan
        );
        assert_eq!(
            local.authoring.requires,
            Some(vec![K::Implementation, K::Verification])
        );
        let encoded = serde_json::to_value(local).unwrap();
        for forbidden in ["terminal", "fulfilled", "accepted"] {
            assert!(encoded.get(forbidden).is_none());
        }
        dump(&format!("{name}-no-implicit-adoption"), &r);
    }
    f.assert_origin_leaves_unchanged();
}
