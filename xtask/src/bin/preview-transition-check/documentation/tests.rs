//! Isolated immutable source fixtures for bounded documentation subclaims.
use super::*;
use serde_json::{Value, json};
use vibe_wire::generated::preview::e1::check_request::CheckRequestFormat;
use vibe_wire::generated::preview::e1::transition::{
    DocumentationCaptureRef, PendingCapture, PendingCaptureState, SourceDigestRef,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#DOCUMENTATION-INPUT-CHECK");

const XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"guide\">Guide</title><p>Text.</p></spec>";
struct Docs {
    product: tests::Fixture,
    capture: Value,
    map: Value,
}
impl Docs {
    fn new() -> Self {
        Self::source(XML, false, false)
    }
    fn source(xml: &str, source_link: bool, build_link: bool) -> Self {
        let product = tests::Fixture::new();
        fs::create_dir(product.root.join("docs")).unwrap();
        fs::write(product.root.join("docs/guide.xml"), xml).unwrap();
        fs::write(product.root.join("docs/asset.txt"), b"asset").unwrap();
        fs::write(product.root.join("build.toml"), b"recipe").unwrap();
        #[cfg(unix)]
        {
            if source_link {
                std::os::unix::fs::symlink("guide.xml", product.root.join("docs/link.xml"))
                    .unwrap();
            }
            if build_link {
                fs::remove_file(product.root.join("build.toml")).unwrap();
                std::os::unix::fs::symlink("docs/guide.xml", product.root.join("build.toml"))
                    .unwrap();
            }
        }
        product.git(&["add", "docs", "build.toml"]);
        product.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "docs",
        ]);
        fs::write(product.release.join("docs-raw.json"), b"raw docs").unwrap();
        let file = |path: &str, bytes: &[u8]| json!({"path":path,"sha256":capture::sha256(bytes),"git_blob_sha1":product.git(&["rev-parse", &format!("HEAD:{path}")])});
        let mut guide = file("docs/guide.xml", xml.as_bytes());
        guide["path"] = json!("guide.xml");
        let mut asset = file("docs/asset.txt", b"asset");
        asset["path"] = json!("asset.txt");
        let build = file(
            "build.toml",
            if build_link {
                b"docs/guide.xml"
            } else {
                b"recipe"
            },
        );
        let capture = json!({"schema":1,"capture_id":"docs old opaque ID","source_record":"docs-raw.json","source_record_sha256":capture::sha256(b"raw docs"),
            "source_subject":{"kind":"git","repository_key":"documentation","repository_identity":"urn:fixture:declared","commit_sha1":product.git(&["rev-parse","HEAD"]),"tree_sha1":product.git(&["rev-parse","HEAD^{tree}"])},
            "source_root_components":["docs"],"source_profile":"urn:vibevm:doc:xml","product_baseline_capture_id":"before",
            "files":[asset,guide],"pages":[{"path":"guide.xml","source_kind":"authored","generated_from":[]}],"exclusions":[],
            "build_inputs":[{"purpose":"recipe","path":"build.toml","sha256":build["sha256"],"git_blob_sha1":build["git_blob_sha1"]}],"toolchains":[]});
        let map = json!({"schema":1,"documentation_capture_id":"docs old opaque ID","product_capture_id":"before","topics":[{"topic_id":"guide-topic","change_ids":[],"claim_anchors":["product-claim"],"disposition":"existing","source_kind":"authored","source_page":"guide.xml","proof":{"state":"pending","reason":"Review pending","subject_ids":[],"evidence":[]}}]});
        Self {
            product,
            capture,
            map,
        }
    }
    fn selected(&mut self) -> CheckRequest {
        let bytes = serde_json::to_vec(&self.capture).unwrap();
        fs::write(self.product.release.join("docs.json"), &bytes).unwrap();
        self.product.transition.documentation = serde_json::from_value(json!({"state":"captured","capture_id":self.capture["capture_id"],"source":"docs.json","capture_file_sha256":capture::sha256(&bytes)})).unwrap();
        let bytes = serde_json::to_vec(&self.map).unwrap();
        fs::write(self.product.release.join("map.json"), &bytes).unwrap();
        self.product.transition.documentation_map = Some(SourceDigestRef {
            source: "map.json".into(),
            sha256: capture::sha256(&bytes),
        });
        self.request()
    }
    fn request(&self) -> CheckRequest {
        let path = self.product.release.join("transition.json");
        fs::write(&path, serde_json::to_vec(&self.product.transition).unwrap()).unwrap();
        CheckRequest {
            schema: 1,
            transition: path.to_str().unwrap().into(),
            mode: CheckRequestMode::Documentation,
            stage: Some(CheckRequestStage::Intake),
            format: CheckRequestFormat::Json,
            repository_roots: Some(std::collections::BTreeMap::from([(
                "documentation".into(),
                self.product.root.to_str().unwrap().into(),
            )])),
        }
    }
    fn report(&mut self) -> CheckReport {
        run(&self.selected())
    }
}
fn has(report: &CheckReport, code: &str) -> bool {
    report.findings.iter().any(|finding| finding.code == code)
}
fn failed(report: CheckReport, code: &str) {
    assert!(
        matches!(report.verdict, CheckReportVerdict::Failed),
        "{report:?}"
    );
    assert!(has(&report, code), "{report:?}");
}

#[test]
fn verified_bytes_census_anchors_membership_stay_pending_after_worktree_edit() {
    let mut docs = Docs::new();
    let mut product_alias: vibe_wire::generated::preview::e1::capture::Capture =
        serde_json::from_slice(&fs::read(docs.product.release.join("before.json")).unwrap())
            .unwrap();
    product_alias.capture_id = "map target separate from baseline".into();
    docs.product.transition.captures = Some(vec![
        docs.product.write_capture(&product_alias, "alias.json"),
    ]);
    docs.map["product_capture_id"] = json!("map target separate from baseline");
    fs::write(
        docs.product.root.join("docs/guide.xml"),
        b"broken mutable XML",
    )
    .unwrap();
    let report = docs.report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Pending),
        "{report:?}"
    );
    for claim in [
        "documentation-file-census:docs old opaque ID",
        "documentation-build-input-bytes:docs old opaque ID:build.toml",
        "documentation-xml-page-census:docs old opaque ID",
        "documentation-declared-page-membership:docs old opaque ID:guide-topic",
    ] {
        assert!(
            report.checked_claims.iter().any(|found| found == claim),
            "{report:?}"
        );
    }
    assert!(
        report
            .subject_ids
            .iter()
            .any(|id| id == "documentation-anchor:docs old opaque ID:guide.xml#guide")
    );
    assert!(has(&report, "documentation-topic-denominator-missing"));
    assert!(has(&report, "documentation-anchor-bindings-missing"));
}
#[test]
fn missing_key_and_parent_directory_never_substitute_a_repository() {
    let mut docs = Docs::new();
    let mut request = docs.selected();
    request.repository_roots = None;
    let report = run(&request);
    assert!(has(&report, "documentation-repository-root-missing"));
    assert!(
        !report
            .checked_claims
            .iter()
            .any(|claim| claim.starts_with("documentation-git-tree"))
    );
    request.repository_roots = Some(std::collections::BTreeMap::from([(
        "documentation".into(),
        docs.product.root.join("docs").to_str().unwrap().into(),
    )]));
    failed(run(&request), "documentation-repository-selection");
}
#[test]
fn immutable_identity_and_complete_file_page_censuses_refuse_contradictions() {
    for (case, code) in [
        (0, "documentation-git-tree"),
        (1, "documentation-file-digest"),
        (2, "documentation-file-census"),
        (3, "documentation-page-census"),
        (4, "documentation-map-capture"),
        (5, "documentation-map-page"),
    ] {
        let mut docs = Docs::new();
        match case {
            0 => docs.capture["source_subject"]["tree_sha1"] = json!("0".repeat(40)),
            1 => docs.capture["files"][0]["sha256"] = json!("0".repeat(64)),
            2 => {
                docs.capture["files"].as_array_mut().unwrap().remove(0);
            }
            3 => docs.capture["pages"] = json!([]),
            4 => docs.map["documentation_capture_id"] = json!("foreign"),
            _ => docs.map["topics"][0]["source_page"] = json!("missing.xml"),
        };
        failed(docs.report(), code);
    }
}
#[test]
fn retained_raw_map_capture_bytes_and_identity_conflicts_refuse() {
    for (case, code) in [
        (0, "documentation-capture-digest"),
        (1, "documentation-raw-digest"),
        (2, "documentation-map-digest"),
        (3, "documentation-capture-binding"),
        (4, "documentation-capture-conflict"),
    ] {
        let mut docs = Docs::new();
        docs.selected();
        match case {
            0 => {
                fs::write(docs.product.release.join("docs.json"), b"changed").unwrap();
            }
            1 => {
                fs::write(docs.product.release.join("docs-raw.json"), b"changed").unwrap();
            }
            2 => {
                fs::write(docs.product.release.join("map.json"), b"changed").unwrap();
            }
            3 => docs.product.transition.documentation.capture_id = Some("foreign".into()),
            _ => {
                docs.product.transition.documentation_base = Some(DocumentationCaptureRef {
                    capture_id: "docs old opaque ID".into(),
                    source: "other.json".into(),
                    capture_file_sha256: "0".repeat(64),
                })
            }
        }
        failed(run(&docs.request()), code);
    }
}
#[test]
fn malformed_xml_reports_fixed_reason_and_line_without_fragments() {
    let mut docs = Docs::source(
        "<spec xmlns=\"https://vibevm.org/spec/1\">\n<SECRET-FRAGMENT",
        false,
        false,
    );
    let report = docs.report();
    assert!(has(&report, "documentation-xml"), "{report:?}");
    let encoded = serde_json::to_string(&report).unwrap();
    assert!(!encoded.contains("SECRET-FRAGMENT"));
    assert!(encoded.contains("source line"));
}
#[cfg(unix)]
#[test]
fn symlink_source_and_outside_root_build_input_are_unsupported_not_regular_blobs() {
    for (source, build, code) in [
        (true, false, "documentation-git-entry-unsupported"),
        (false, true, "documentation-build-entry-unsupported"),
    ] {
        let mut docs = Docs::source(XML, source, build);
        let report = docs.report();
        assert!(
            matches!(report.verdict, CheckReportVerdict::Pending),
            "{report:?}"
        );
        assert!(has(&report, code), "{report:?}");
        assert!(
            !report
                .checked_claims
                .iter()
                .any(|claim| claim.starts_with("documentation-xml-page:"))
        );
        assert!(
            !report
                .checked_claims
                .iter()
                .any(|claim| claim.starts_with("documentation-build-input-bytes:"))
        );
    }
}
#[test]
fn unsupported_profiles_and_content_recipes_cannot_close_intake() {
    let mut docs = Docs::new();
    docs.capture["source_profile"] = json!("urn:fixture:foreign");
    let report = docs.report();
    assert!(has(&report, "documentation-profile-unsupported"));
    assert!(matches!(report.verdict, CheckReportVerdict::Pending));
    docs.capture["source_subject"] = json!({"kind":"content","repository_key":"documentation","repository_identity":"urn:fixture:declared","tree_sha256":"0".repeat(64),"recipe_id":"urn:fixture:recipe"});
    // Existing registered content shape owns the recipe field names.
    let report = docs.report();
    assert!(
        has(&report, "documentation-content-recipe-unsupported"),
        "{report:?}"
    );
}
#[test]
fn unresolved_product_has_due_state_and_checked_or_empty_topics_never_pass() {
    let mut docs = Docs::new();
    docs.map["product_capture_id"] = json!("unknown");
    let report = docs.report();
    assert!(has(&report, "documentation-product-capture-missing"));
    assert!(matches!(report.verdict, CheckReportVerdict::Pending));
    let reference = docs.product.transition.dp1.clone();
    docs.product.transition.dp2 = PendingCapture {
        state: PendingCaptureState::Candidate,
        capture_id: Some(reference.capture_id),
        source: Some(reference.source),
        capture_file_sha256: Some(reference.capture_file_sha256),
        commit_sha1: Some(reference.commit_sha1),
        git_tree_sha1: Some(reference.git_tree_sha1),
    };
    failed(docs.report(), "documentation-product-capture-missing");
    let mut docs = Docs::new();
    docs.map["topics"][0]["proof"] = json!({"state":"checked","reason":"","subject_ids":["before","docs old opaque ID"],"evidence":["producer says yes"]});
    docs.selected();
    docs.product.transition.documentation.state =
        vibe_wire::generated::preview::e1::transition::PendingDocumentationState::Verified;
    docs.product.transition.proof.documentation_verified = true;
    let report = run(&docs.request());
    assert!(
        matches!(report.verdict, CheckReportVerdict::Pending),
        "{report:?}"
    );
    let mut docs = Docs::new();
    docs.map["topics"] = json!([]);
    assert!(matches!(docs.report().verdict, CheckReportVerdict::Pending));
}

#[cfg(unix)]
#[test]
fn gitlink_and_unsafe_retained_locations_are_not_admitted() {
    let mut docs = Docs::new();
    let commit = docs.product.git(&["rev-parse", "HEAD"]);
    docs.product.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{commit},docs/submodule"),
    ]);
    docs.product.git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--quiet",
        "-m",
        "gitlink",
    ]);
    docs.capture["source_subject"]["commit_sha1"] = json!(docs.product.git(&["rev-parse", "HEAD"]));
    docs.capture["source_subject"]["tree_sha1"] =
        json!(docs.product.git(&["rev-parse", "HEAD^{tree}"]));
    let report = docs.report();
    assert!(
        has(&report, "documentation-git-entry-unsupported"),
        "{report:?}"
    );
    assert!(matches!(report.verdict, CheckReportVerdict::Pending));
    let mut docs = Docs::new();
    docs.selected();
    std::os::unix::fs::symlink(
        docs.product.release.join("docs.json"),
        docs.product.release.join("linked.json"),
    )
    .unwrap();
    docs.product.transition.documentation.source = Some("linked.json".into());
    failed(run(&docs.request()), "evidence-read-refused");
    docs.product.transition.documentation.source = Some("../docs.json".into());
    let report = run(&docs.request());
    assert!(matches!(report.verdict, CheckReportVerdict::Failed));
}
