//! Finite fixture joins; these cases do not claim production callable coverage.
use super::*;
use serde_json::{Value, json};
use std::process::Command;
use vibe_wire::generated::machine::e1::operation_descriptor::OperationDescriptor;
use vibe_wire::generated::preview::e1::release_impact::ReleaseImpact;

struct Fixture {
    temp: tempfile::TempDir,
    transition: Transition,
    record: ReleaseImpact,
    inventory: Value,
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(out.status.success(), "{:?}", out);
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
impl Fixture {
    fn new() -> Self {
        Self::with_descriptor(None)
    }
    fn with_descriptor(native: Option<OperationDescriptor>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-q"]);
        write(root, "src/shape_fixture.rs", b"source-owner-before");
        write(
            root,
            "formats/breaks/001.md",
            b"Fixture break; manual authored conversion only.",
        );
        let mut inventory: Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/capability-coverage/e1/valid/mapped.json"
        ))
        .unwrap();
        inventory["sources"][0]["sha256"] = json!(capture::sha256(b"source-owner-before"));
        if let Some(row) = &native {
            inventory["sources"][0]["source_id"] = json!(row.source_id);
            for surface in inventory["surfaces"].as_array_mut().unwrap() {
                surface["source_ids"] = json!([row.source_id]);
            }
            inventory["models"][0]["source_ids"] = json!([row.source_id]);
            inventory["planned_families"][0]["source_ids"] = json!([row.source_id]);
            inventory["surfaces"][1]["operation_id"] = json!(row.operation_id);
        }
        let before = serde_json::to_vec(&inventory).unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "before"]);
        let before_commit = git(root, &["rev-parse", "HEAD"]);
        let before_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
        inventory["surfaces"][1]["effects"] = json!(["changed-declaration"]);
        inventory["sources"][0]["sha256"] = json!(capture::sha256(b"source-owner-after"));
        write(root, "src/shape_fixture.rs", b"source-owner-after");
        let mut descriptor: Value = serde_json::from_str(include_str!("../../../../formats/corpora/machine-operation-descriptor/e1/valid/source-bound-migration-shape.json")).unwrap();
        descriptor["operation_id"] = json!("operation:fixture-read");
        descriptor["capability_id"] = json!("strict-text-only-migration-planning");
        descriptor["source_id"] = json!("source:shape-fixture");
        if let Some(row) = native {
            descriptor = serde_json::to_value(row).unwrap();
        }
        write(
            root,
            "owner/descriptor.json",
            &serde_json::to_vec(&descriptor).unwrap(),
        );
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "after"]);
        let after_commit = git(root, &["rev-parse", "HEAD"]);
        let after_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
        write(root, "release/before.json", &before);
        write(
            root,
            "release/after.json",
            &serde_json::to_vec(&inventory).unwrap(),
        );
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/preview-transition/e1/valid/captured.json"
        ))
        .unwrap();
        value["dp1"] = json!({"capture_id":"before","source":"before-capture.json","capture_file_sha256":"0".repeat(64),"commit_sha1":before_commit,"git_tree_sha1":before_tree});
        value["captures"] = json!([{"capture_id":"after","source":"after-capture.json","capture_file_sha256":"1".repeat(64),"commit_sha1":after_commit,"git_tree_sha1":after_tree}]);
        value["change_records"] = json!(["fixture-change"]);
        value["contribution_scope"] = json!({"before_capture_id":"before","after_capture_id":"after","record_digests":[{"change_id":"fixture-change","sha256":"2".repeat(64)}]});
        value["domain_bindings"] = json!({"before_capture_id":"before","after_capture_id":"after",
            "before_inventory":{"source":"before.json","sha256":capture::sha256(&before)},
            "after_inventory":{"source":"after.json","sha256":capture::sha256(&serde_json::to_vec(&inventory).unwrap())},
            "effects":[{"change_id":"fixture-change","interface_id":"interface.fixture","capability_id":"strict-text-only-migration-planning","audience":"author","platform":"fixture","surface_id":"surface:fixture-read","topic_id":"topic.fixture","reason":"Explicit bounded fixture action classification","action":"manual","data_class":"authored",
                "format_breaks":[{"capture_id":"after","path":"formats/breaks/001.md","sha256":capture::sha256(&fs::read(root.join("formats/breaks/001.md")).unwrap())}],
                "migration_descriptor":{"capture_id":"after","path":"owner/descriptor.json","sha256":capture::sha256(&serde_json::to_vec(&descriptor).unwrap())}}]});
        let transition: Transition = serde_json::from_value(value).unwrap();
        let mut record: Value = serde_json::from_str(include_str!(
            "../../../../formats/corpora/release-impact/e1/valid/internal-refactor.json"
        ))
        .unwrap();
        record["change_id"] = json!("fixture-change");
        record["before_capture_id"] = json!("before");
        record["after_capture_id"] = json!("after");
        record["effects"] = json!([{"kind":"changed","interface_id":"interface.fixture","capability_id":"strict-text-only-migration-planning","audience":"author","platform":"fixture","before":"before","after":"after","contract_anchors":[]}]);
        record["user_impact"] =
            json!({"state":"impact","reason":"fixture","migration_action":"manual"});
        record["documentation_impact"] =
            json!({"state":"impact","reason":"fixture","documentation_action":"update"});
        Self {
            temp,
            transition,
            record: serde_json::from_value(record).unwrap(),
            inventory,
        }
    }
    fn run(&self) -> Result<CheckReport, CheckFailure> {
        let mut report = empty_report(CheckReportMode::Bindings, Some(CheckReportStage::Linked));
        report.verdict = CheckReportVerdict::Passed;
        domain_bindings::check(
            &self.temp.path().join("release/transition.json"),
            self.temp.path(),
            &self.transition,
            std::slice::from_ref(&self.record),
            &mut report,
        )?;
        Ok(report)
    }
    fn inventory(&mut self) {
        let bytes = serde_json::to_vec(&self.inventory).unwrap();
        write(self.temp.path(), "release/after.json", &bytes);
        self.transition
            .domain_bindings
            .as_mut()
            .unwrap()
            .after_inventory
            .sha256 = capture::sha256(&bytes);
    }
    fn link(&mut self) -> &mut vibe_wire::generated::preview::e1::transition::DomainEffect {
        &mut self.transition.domain_bindings.as_mut().unwrap().effects[0]
    }
    fn action(&mut self, action: &str, class: &str) {
        self.record.user_impact.migration_action = serde_json::from_value(json!(action)).unwrap();
        self.link().action = serde_json::from_value(json!(action)).unwrap();
        self.link().data_class = serde_json::from_value(json!(class)).unwrap();
    }
}
#[test]
fn break_descriptor_and_capability_join_one_impact() {
    let f = Fixture::new();
    let report = f.run().unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Passed);
    assert!(
        report
            .checked_claims
            .iter()
            .any(|c| c.starts_with("domain-descriptor:"))
    );
    assert!(
        report
            .checked_claims
            .iter()
            .any(|c| c == "canonical-surface-delta:1")
    );
}
#[test]
fn new_capability_without_break_has_documentation_obligation() {
    let mut f = Fixture::new();
    // A second new surface is missed even though the existing impact is valid.
    let mut new = f.inventory["surfaces"][1].clone();
    new["surface_id"] = json!("surface:new");
    new["name"] = json!("new");
    new["aliases"] = json!([]);
    f.inventory["surfaces"].as_array_mut().unwrap().push(new);
    f.inventory();
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("missing a release impact")
    );
    let mut effect = f.record.effects[0].clone();
    effect.interface_id = "interface.new".into();
    effect.kind = serde_json::from_value(json!("introduced")).unwrap();
    f.record.effects.push(effect);
    let mut link = f.link().clone();
    link.interface_id = "interface.new".into();
    link.surface_id = "surface:new".into();
    link.format_breaks.clear();
    link.migration_descriptor = None;
    f.transition
        .domain_bindings
        .as_mut()
        .unwrap()
        .effects
        .push(link);
    assert!(
        f.run()
            .unwrap()
            .checked_claims
            .contains(&"documentation-obligation:surface:new:topic.fixture".into())
    );
    f.record.documentation_impact.state = serde_json::from_value(json!("no-impact")).unwrap();
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("documentation obligation")
    );
}
#[test]
fn prose_cannot_supply_automatic_converter_and_metadata_cannot_admit_it() {
    let mut f = Fixture::new();
    f.action("automated", "authored");
    let report = f.run().unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Pending);
    assert!(
        report
            .findings
            .iter()
            .any(|v| v.code == "bindings-automatic-relationship-unselected"
                && v.state == FindingState::Unsupported)
    );
    f.link().migration_descriptor = None;
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("no exact domain descriptor")
    );
}
#[test]
fn stale_dangling_and_cross_capture_references_refuse() {
    let mut f = Fixture::new();
    f.link().migration_descriptor.as_mut().unwrap().sha256 = "0".repeat(64);
    assert!(f.run().unwrap_err().reason.contains("stale"));
    f.link().migration_descriptor.as_mut().unwrap().path = "owner/missing.json".into();
    assert!(f.run().unwrap_err().reason.contains("missing"));
    f.link().migration_descriptor.as_mut().unwrap().capture_id = "foreign".into();
    assert!(f.run().unwrap_err().reason.contains("captured reference"));
}
#[test]
fn stale_selected_source_refuses_without_global_recensus() {
    let mut f = Fixture::new();
    f.inventory["sources"][0]["sha256"] = json!("0".repeat(64));
    f.inventory();
    assert!(f.run().unwrap_err().reason.contains("stale"));
}
#[test]
fn schema_only_exposure_is_reference_consistency_not_operational_coverage() {
    let mut f = Fixture::new();
    f.inventory["surfaces"][1]["implementation"] = json!("planned");
    f.inventory["surfaces"][1]["external_access"] = json!("unimplemented");
    f.inventory();
    let report = f.run().unwrap();
    assert!(
        report
            .findings
            .iter()
            .any(|v| v.code == "bindings-operational-coverage-unverified")
    );
    assert!(
        !report
            .checked_claims
            .iter()
            .any(|v| v.contains("callable") || v.contains("operational-coverage"))
    );
}
#[test]
fn five_actions_remain_distinct_and_derived_conversion_refuses() {
    for (action, class) in [
        ("none", "none"),
        ("manual", "authored"),
        ("rebuild", "derived"),
        ("unsupported", "authored"),
    ] {
        let mut f = Fixture::new();
        f.action(action, class);
        assert!(
            f.run()
                .unwrap()
                .checked_claims
                .iter()
                .any(|v| v.ends_with(&format!(":{action}:{class}")))
        );
    }
    let mut f = Fixture::new();
    f.action("automated", "derived");
    assert!(f.run().unwrap_err().reason.contains("ownership"));
}
#[test]
fn second_journal_and_unknown_wire_fields_are_refused() {
    let f = Fixture::new();
    let mut value = serde_json::to_value(&f.transition).unwrap();
    value["domain_bindings"]["effects"][0]["journal"] = json!([]);
    assert!(serde_json::from_value::<Transition>(value).is_err());
    let mut f = Fixture::new();
    let source = f.link().migration_descriptor.as_mut().unwrap();
    source.path = "formats/breaks/001.md".into(); // prose cannot decode as descriptor
    // Avoid duplicate-reference refusal to exercise the descriptor reader itself.
    let source = source.clone();
    f.link().format_breaks.clear();
    f.link().migration_descriptor.as_mut().unwrap().sha256 =
        capture::sha256(&fs::read(f.temp.path().join(source.path)).unwrap());
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("not the registered machine descriptor")
    );
}

#[test]
fn missing_selection_is_pending_and_supplied_identity_mismatch_refuses() {
    let mut f = Fixture::new();
    f.transition.domain_bindings = None;
    let report = f.run().unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Pending);
    assert!(
        report
            .findings
            .iter()
            .any(|v| v.code == "bindings-domain-selection-missing")
    );
    let mut f = Fixture::new();
    f.record.effects[0].capability_id = "other-capability".into();
    f.link().capability_id = "other-capability".into();
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("Descriptor operation/capability/source")
    );
}

#[test]
fn catalogue_metadata_does_not_invent_behavioral_change() {
    let mut f = Fixture::new();
    f.record.effects[0].kind = serde_json::from_value(json!("unchanged")).unwrap();
    f.record.documentation_impact.state = serde_json::from_value(json!("no-impact")).unwrap();
    assert_eq!(f.run().unwrap().verdict, CheckReportVerdict::Passed);
    // Conversely, the same declared surface can retain an independently
    // reviewed semantic change without a wire/catalogue shape delta.
    let before = fs::read(f.temp.path().join("release/before.json")).unwrap();
    f.inventory["surfaces"] = serde_json::from_slice::<Value>(&before).unwrap()["surfaces"].clone();
    f.inventory();
    f.record.effects[0].kind = serde_json::from_value(json!("changed")).unwrap();
    f.record.documentation_impact.state = serde_json::from_value(json!("impact")).unwrap();
    assert_eq!(f.run().unwrap().verdict, CheckReportVerdict::Passed);
}

// Capture deliberately changed descriptor bytes in the isolated fixture only.
// The production repository and its retained historical captures are untouched.
impl Fixture {
    fn replace_descriptor(&mut self, value: Value) {
        let bytes = serde_json::to_vec(&value).unwrap();
        write(self.temp.path(), "owner/descriptor.json", &bytes);
        git(self.temp.path(), &["add", "owner/descriptor.json"]);
        git(
            self.temp.path(),
            &["commit", "-qm", "controlled descriptor"],
        );
        self.transition.captures.as_mut().unwrap()[0].commit_sha1 =
            git(self.temp.path(), &["rev-parse", "HEAD"]);
        self.transition.captures.as_mut().unwrap()[0].git_tree_sha1 =
            git(self.temp.path(), &["rev-parse", "HEAD^{tree}"]);
        self.link().migration_descriptor.as_mut().unwrap().sha256 = capture::sha256(&bytes);
    }
}

#[test]
fn actual_metadata_owner_rejects_shape_valid_lies_before_claiming_descriptor() {
    use vibe_orchestrator::machine_operation::{MachineOperationService, PurePlannerAdmission};
    let owner =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Unavailable).unwrap();
    for (field, value) in [
        ("availability", json!("available")),
        ("authorization", json!("admitted")),
        ("reason", json!(" ")),
        ("implementation_evidence_id", json!(" ")),
    ] {
        let mut f = Fixture::new();
        let mut row: Value =
            serde_json::from_slice(&fs::read(f.temp.path().join("owner/descriptor.json")).unwrap())
                .unwrap();
        row[field] = value;
        // Shape decoding succeeds; semantic owner must refuse.
        serde_json::from_value::<OperationDescriptor>(row.clone()).unwrap();
        f.replace_descriptor(row);
        assert!(f.run().is_err(), "{field}");
    }
    let mut f = Fixture::with_descriptor(Some(owner.descriptors()[0].clone()));
    let mut row = serde_json::to_value(&owner.descriptors()[0]).unwrap();
    row["domain_owner"] = json!("foreign:owner");
    f.replace_descriptor(row);
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("accepted source and formats")
    );
}

#[test]
fn native_saved_evidence_and_actual_pure_planner_join_controlled_selected_scope() {
    use vibe_orchestrator::machine_operation::{MachineOperationService, PurePlannerAdmission};
    use vibe_wire::generated::migration::e1::plan_report::Outcome;
    use vibe_workspace::migration_planning::{
        MigrationPlanningResponse, decode_migration_request, validate_report_for_request,
    };
    // Explicit test admission invokes only the existing text-only route. No file
    // migration, application journal or host grant is constructed.
    let service =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted).unwrap();
    let descriptor = service.descriptors()[0].clone();
    let mut f = Fixture::with_descriptor(Some(descriptor.clone()));
    let source = "[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nmaterialization='snapshot'\n";
    let request = decode_migration_request(json!({
        "schema":1, "source_text":source,
        "source_profile":vibe_core::manifest::LEGACY_ABSENT,
        "target_profile":vibe_core::manifest::CURRENT_ABSENT,
        "recipe":vibe_core::manifest::SNAPSHOT_TO_COPY
    }))
    .unwrap();
    let response = service
        .plan_migration(descriptor.operation_id.as_deref().unwrap(), request.clone())
        .unwrap();
    let MigrationPlanningResponse::Report(report) = response else {
        panic!("native report required")
    };
    validate_report_for_request(&request, &report).unwrap();
    let Outcome::Planned(planned) = &report.outcome else {
        panic!("native automatic text plan required")
    };
    assert_eq!(planned.plan.before, source);
    assert_ne!(planned.plan.before_identity, planned.plan.after_identity);
    assert!(planned.plan.after.contains("materialization='copy'"));
    // Exact replay refuses even a shape-valid report from a different request.
    let mut foreign = request.clone();
    foreign.source_text.push_str("# another subject\n");
    assert!(validate_report_for_request(&foreign, &report).is_err());
    let mut tampered = report.clone();
    let Outcome::Planned(row) = &mut tampered.outcome else {
        unreachable!()
    };
    row.plan.recipe = "copied prose".into();
    assert!(validate_report_for_request(&request, &tampered).is_err());
    let mut no_op = request.clone();
    no_op.source_text = planned.plan.after.clone();
    no_op.source_profile = Some(vibe_core::manifest::CURRENT_ABSENT.into());
    no_op.recipe = Some(vibe_core::manifest::IDENTITY.into());
    let MigrationPlanningResponse::Report(no_op_report) = service
        .plan_migration(descriptor.operation_id.as_deref().unwrap(), no_op.clone())
        .unwrap()
    else {
        panic!("report")
    };
    validate_report_for_request(&no_op, &no_op_report).unwrap();
    assert!(matches!(no_op_report.outcome, Outcome::NoOp(_)));
    let mut unsupported = request.clone();
    unsupported.recipe = Some("unknown-recipe".into());
    let MigrationPlanningResponse::Report(refused) = service
        .plan_migration(
            descriptor.operation_id.as_deref().unwrap(),
            unsupported.clone(),
        )
        .unwrap()
    else {
        panic!("report")
    };
    validate_report_for_request(&unsupported, &refused).unwrap();
    assert!(matches!(refused.outcome, Outcome::Refused(_)));

    f.action("automated", "authored");
    // Save actual API output as explicitly selected controlled evidence; the native
    // saved-role owner supplies the exact impact used by the domain checker.
    let bytes = serde_json::to_vec(&report).unwrap();
    write(f.temp.path(), "proof/native-plan.json", &bytes);
    f.record.evidence[0].source = "proof/native-plan.json".into();
    f.record.evidence[0].sha256 = Some(capture::sha256(&bytes));
    write(
        f.temp.path(),
        "vibe.toml",
        b"[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n",
    );
    write(f.temp.path(), "specs/changes/fixture-change/change.toml", b"schema=1\norigins=[]\n[change]\nid='fixture-change'\nselected_node='.'\nslug='fixture-change'\ntitle='Controlled native planner linkage'\n[[documents]]\nrole='release-impact'\npath='release-impact.json'\nnormative=false\ninput=true\n");
    let record_bytes = serde_json::to_vec(&f.record).unwrap();
    write(
        f.temp.path(),
        "specs/changes/fixture-change/release-impact.json",
        &record_bytes,
    );
    let native = bindings::bound_record(
        f.temp.path(),
        Some(Path::new("specs")),
        "fixture-change",
        &capture::sha256(&record_bytes),
    )
    .unwrap();
    assert_eq!(native, record_bytes);
    f.record = serde_json::from_slice(&native).unwrap();
    let selected = &f.record.evidence[0];
    let retained = fs::read(f.temp.path().join(&selected.source)).unwrap();
    assert_eq!(
        selected.sha256.as_deref(),
        Some(capture::sha256(&retained).as_str())
    );
    let retained_report = serde_json::from_slice(&retained).unwrap();
    validate_report_for_request(&request, &retained_report).unwrap();
    assert_eq!(retained_report, report);
    let captured_descriptor: OperationDescriptor =
        serde_json::from_slice(&fs::read(f.temp.path().join("owner/descriptor.json")).unwrap())
            .unwrap();
    assert_eq!(captured_descriptor, descriptor);
    let joined = f.run().unwrap();
    assert!(
        joined
            .checked_claims
            .iter()
            .any(|c| c.starts_with("domain-descriptor:"))
    );
    // Automated text planning is proven for these exact controlled inputs, but
    // the supplement has no request/report relationship selector. No inference
    // from a prose digest or descriptor upgrades it to an applied conversion.
    f.action("automated", "authored");
    assert_eq!(f.run().unwrap().verdict, CheckReportVerdict::Pending);
    for (action, class) in [
        ("manual", "authored"),
        ("rebuild", "derived"),
        ("none", "none"),
        ("unsupported", "authored"),
    ] {
        f.action(action, class);
        assert!(
            f.run()
                .unwrap()
                .checked_claims
                .iter()
                .any(|c| c.ends_with(&format!(":{action}:{class}")))
        );
    }
    println!(
        "CONTROLLED_NATIVE_LINKAGE {}",
        json!({
            "meaning":"Explicitly controlled test selection, not authenticated production registration or applied migration",
            "descriptor":descriptor,"request":request,"actual_report":retained_report,
            "native_record_sha256":capture::sha256(&native),
            "selected_native_evidence":f.record.evidence[0],
            "binding_verdict":joined.verdict,"binding_checked_claims":joined.checked_claims
        })
    );
    let unavailable =
        MachineOperationService::pure_text_planner(PurePlannerAdmission::Unavailable).unwrap();
    assert!(
        unavailable
            .plan_migration(descriptor.operation_id.as_deref().unwrap(), request)
            .is_err()
    );
}

// Real retained identities and native saved-role bytes, with only retrieval
// locations isolated for refusal cases. No canonical ID is renamed to join.
struct SelectedEnvelope {
    temp: tempfile::TempDir,
    transition: Transition,
    record: ReleaseImpact,
    root: std::path::PathBuf,
}
impl SelectedEnvelope {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let release = root.join("docs/releases/developer-preview-1-to-2");
        let transition: Transition = serde_json::from_slice(
            &fs::read(release.join("bind2-envelope-transition.json")).unwrap(),
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let binding = transition.domain_bindings.as_ref().unwrap();
        for reference in [
            &binding.before_inventory,
            &binding.after_inventory,
            &binding.effects[0].native_planning.as_ref().unwrap().request,
            &binding.effects[0].native_planning.as_ref().unwrap().report,
        ] {
            write(
                temp.path(),
                &reference.source,
                &fs::read(release.join(&reference.source)).unwrap(),
            );
        }
        let digest = &transition
            .contribution_scope
            .as_ref()
            .unwrap()
            .record_digests
            .iter()
            .find(|x| x.change_id == "pure-machine-service-and-envelope-tool")
            .unwrap()
            .sha256;
        let bytes = bindings::bound_record(
            &root,
            None,
            "pure-machine-service-and-envelope-tool",
            digest,
        )
        .unwrap();
        Self {
            temp,
            transition,
            record: serde_json::from_slice(&bytes).unwrap(),
            root,
        }
    }
    fn run(&self) -> Result<CheckReport, CheckFailure> {
        let mut report = empty_report(CheckReportMode::Bindings, Some(CheckReportStage::Linked));
        report.verdict = CheckReportVerdict::Passed;
        domain_bindings::check(
            &self.temp.path().join("transition.json"),
            &self.root,
            &self.transition,
            std::slice::from_ref(&self.record),
            &mut report,
        )?;
        Ok(report)
    }
    fn retained_pair(&mut self, request: Value, report: Value) {
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let report_bytes = serde_json::to_vec(&report).unwrap();
        for link in &mut self.transition.domain_bindings.as_mut().unwrap().effects {
            let p = link.native_planning.as_mut().unwrap();
            write(self.temp.path(), &p.request.source, &request_bytes);
            write(self.temp.path(), &p.report.source, &report_bytes);
            p.request.sha256 = capture::sha256(&request_bytes);
            p.report.sha256 = capture::sha256(&report_bytes);
        }
    }
    fn pair(&self) -> (Value, Value) {
        let p = self.transition.domain_bindings.as_ref().unwrap().effects[0]
            .native_planning
            .as_ref()
            .unwrap();
        (
            serde_json::from_slice(&fs::read(self.temp.path().join(&p.request.source)).unwrap())
                .unwrap(),
            serde_json::from_slice(&fs::read(self.temp.path().join(&p.report.source)).unwrap())
                .unwrap(),
        )
    }
}

#[test]
fn real_selected_envelope_and_rust_effects_replay_through_production_reader() {
    let f = SelectedEnvelope::new();
    let report = f.run().unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Passed);
    assert_eq!(
        report
            .checked_claims
            .iter()
            .filter(|c| c.starts_with("current-native-planning-replay:") && c.ends_with(":planned"))
            .count(),
        2
    );
    assert_eq!(report.checked_claims.iter().filter(|c| c.starts_with("reviewed-planner-mapping:") && c.contains("operation.mcp.plan_manifest_migration_envelope:rust:vibe-workspace:migration_planning:pure-migration-transport-adapter:strict-text-only-migration-planning")).count(), 2);
    assert!(
        report
            .checked_claims
            .iter()
            .any(|c| c.contains("pure-machine-planner-service"))
    );
    assert!(
        report
            .checked_claims
            .iter()
            .any(|c| c.contains("pure-machine-planner-transport"))
    );
    assert!(
        report
            .checked_claims
            .contains(&"canonical-surface-delta:1".into())
    );
}

#[test]
fn production_native_selection_refuses_stale_invalid_and_foreign_bytes() {
    let mut f = SelectedEnvelope::new();
    f.transition.domain_bindings.as_mut().unwrap().effects[0]
        .native_planning
        .as_mut()
        .unwrap()
        .request
        .sha256 = "0".repeat(64);
    assert!(f.run().unwrap_err().reason.contains("digest"));
    let mut f = SelectedEnvelope::new();
    let (mut request, report) = f.pair();
    request["source_text"] = json!("foreign source");
    f.retained_pair(request, report);
    assert!(f.run().is_err());
    let mut f = SelectedEnvelope::new();
    let (mut request, report) = f.pair();
    request["recipe"] = Value::Null;
    f.retained_pair(request, report);
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("request owner refused")
    );
    let mut f = SelectedEnvelope::new();
    let (request, mut report) = f.pair();
    report["journal"] = json!([]);
    f.retained_pair(request, report);
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("registered native report")
    );
    let mut f = SelectedEnvelope::new();
    f.transition.domain_bindings.as_mut().unwrap().effects[0]
        .native_planning
        .as_mut()
        .unwrap()
        .domain_owner = "foreign:owner".into();
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("foreign native planner")
    );
    let mut f = SelectedEnvelope::new();
    f.transition
        .domain_bindings
        .as_mut()
        .unwrap()
        .effects
        .remove(0);
    assert!(
        f.run()
            .unwrap_err()
            .reason
            .contains("no canonical surface join")
    );
}

#[test]
fn production_native_outcomes_remain_distinct_and_planning_does_not_prove_apply() {
    use vibe_workspace::migration_planning::{
        MigrationPlanningResponse, decode_migration_request, plan_migration_request,
    };
    for outcome in ["no-op", "needs-input", "refused"] {
        let mut f = SelectedEnvelope::new();
        let (mut request, report) = f.pair();
        match outcome {
            "no-op" => {
                request["source_text"] = report["outcome"]["plan"]["after"].clone();
                request["source_profile"] = json!(vibe_core::manifest::CURRENT_ABSENT);
                request["recipe"] = json!(vibe_core::manifest::IDENTITY);
            }
            "needs-input" => {
                request.as_object_mut().unwrap().remove("recipe");
            }
            _ => request["recipe"] = json!("unimplemented-recipe"),
        }
        let native = decode_migration_request(request.clone()).unwrap();
        let MigrationPlanningResponse::Report(actual) = plan_migration_request(native) else {
            panic!("report")
        };
        f.retained_pair(request, serde_json::to_value(actual).unwrap());
        let result = f.run().unwrap();
        assert_eq!(result.verdict, CheckReportVerdict::Passed);
        assert_eq!(
            result
                .checked_claims
                .iter()
                .filter(|c| c.starts_with("current-native-planning-replay:")
                    && c.ends_with(&format!(":{outcome}")))
                .count(),
            2
        );
        assert!(
            result.checked_claims.iter().any(
                |c| c.starts_with("automatic-file-application:") && c.ends_with(":unsupported")
            )
        );
    }
}

#[test]
fn selected_native_relationship_is_required_and_retained_bytes_are_bounded() {
    let mut f = SelectedEnvelope::new();
    f.transition.domain_bindings.as_mut().unwrap().effects[0].native_planning = None;
    let report = f.run().unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Pending);
    assert!(
        report
            .findings
            .iter()
            .any(|x| x.code == "bindings-native-planning-selection-missing")
    );
    let mut f = SelectedEnvelope::new();
    let plan = f.transition.domain_bindings.as_mut().unwrap().effects[0]
        .native_planning
        .as_mut()
        .unwrap();
    plan.report.sha256 = "0".repeat(64);
    assert!(f.run().unwrap_err().reason.contains("digest"));
    let f = SelectedEnvelope::new();
    let p = f.transition.domain_bindings.as_ref().unwrap().effects[0]
        .native_planning
        .as_ref()
        .unwrap();
    write(
        f.temp.path(),
        &p.report.source,
        &vec![b' '; 4 * 1024 * 1024 + 1],
    );
    assert!(f.run().unwrap_err().reason.contains("exceeds 4 MiB"));
}
