//! Isolated immutable Git cases for finite affected-source review claims.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-081#AFFECTED-SOURCE-REVIEW");
use super::{tests::Fixture, *};
use serde_json::{Value, json};
use vibe_wire::generated::preview::e1::release_impact::ReleaseImpact;
use vibe_wire::generated::preview::e1::transition::SourceDigestRef;

const URI: &str = "spec://org.example/fixture/common/PROP-001#RULE";
const XML: &[u8] = b"<spec xmlns=\"https://vibevm.org/spec/1\"><section id=\"RULE\" title=\"Rule\"><p>Bound rule.</p></section></spec>";
fn specs() -> String {
    vibe_core::layout::current_specs_root()
        .to_str()
        .unwrap()
        .into()
}
fn doc() -> String {
    format!("{}/common/PROP-001-law.xml", specs())
}
fn identity(f: &Fixture, commit: &str, path: &str) -> Value {
    let oid = f.git(&["rev-parse", &format!("{commit}:{path}")]);
    let bytes =
        capture::git(&f.root, &["cat-file", "blob", &oid]).unwrap_or_else(|e| panic!("{}", e.code));
    json!({"git_blob_sha1":oid,"sha256":capture::sha256(&bytes)})
}
fn local(seed: impl FnOnce(&Fixture)) -> (Fixture, ReleaseImpact, Value) {
    let mut f = Fixture::new();
    fs::write(
        f.root.join("vibe.toml"),
        b"[project]\nname = 'fixture'\ngroup = 'org.example'\nversion = '1.0.0'\n",
    )
    .unwrap();
    fs::create_dir_all(f.root.join(doc()).parent().unwrap()).unwrap();
    fs::write(f.root.join(doc()), XML).unwrap();
    seed(&f);
    f.git(&["add", "--", "vibe.toml", &specs()]);
    f.commit();
    f.transition.dp1 = f.snapshot("before");
    let subject = f.commit_product("cli.rs", b"enum Cli { Install, Check }\n");
    f.after(f.snapshot("after"));
    let mut record = f.impact("change-local", &subject, "before", "after", &["cli.rs"]);
    let effect = json!({"interface_id":"cli","capability_id":"cap-cli","kind":"changed","audience":"developer","platform":"Linux","before":"Install only","after":"Install and Check","contract_anchors":[URI]});
    record.effects = vec![serde_json::from_value(effect.clone()).unwrap()];
    let mut interface = effect;
    interface["source_paths"] = json!(["cli.rs"]);
    interface["topic_ids"] = json!(["cli-topic"]);
    let after = f.transition.dp2.commit_sha1.as_ref().unwrap();
    let manifest = identity(&f, after, "vibe.toml");
    let document = identity(&f, after, &doc());
    fs::write(
        f.release.join("source-review.md"),
        b"independent fixture review\n",
    )
    .unwrap();
    let review = json!({"schema":1,"review_id":"finite-review","before_capture_id":"before","after_capture_id":"after",
        "source_record":"source-review.md","source_record_sha256":capture::sha256(b"independent fixture review\n"),
        "paths":[{"path":"cli.rs","before":identity(&f,&f.transition.dp1.commit_sha1,"cli.rs"),"after":identity(&f,after,"cli.rs"),"interface_ids":["cli"]}],
        "interfaces":[interface],"topics":[{"topic_id":"cli-topic","obligation":"Explain Check."}],
        "contract_sources":[{"group":"org.example","name":"fixture","spec_root_components":specs().split('/').collect::<Vec<_>>(),
            "source_profile":"urn:vibevm:spec:xml","manifest_path":"vibe.toml","manifest_sha256":manifest["sha256"],
            "documents":[{"path":doc(),"git_blob_sha1":document["git_blob_sha1"],"sha256":document["sha256"]}]}]});
    (f, record, review)
}
fn bind(f: &mut Fixture, record: &ReleaseImpact, review: &Value) {
    f.select(std::slice::from_ref(record));
    let bytes = serde_json::to_vec(review).unwrap();
    fs::write(f.release.join("review.json"), &bytes).unwrap();
    f.transition
        .contribution_scope
        .as_mut()
        .unwrap()
        .affected_source_review = Some(SourceDigestRef {
        source: "review.json".into(),
        sha256: capture::sha256(&bytes),
    });
}
fn has(report: &CheckReport, code: &str) -> bool {
    report.findings.iter().any(|f| f.code == code)
}
fn failed(report: &CheckReport, code: &str) {
    assert!(
        matches!(report.verdict, CheckReportVerdict::Failed),
        "{report:?}"
    );
    assert!(has(report, code), "{report:?}");
}
fn claim(report: &CheckReport, prefix: &str) -> bool {
    report.checked_claims.iter().any(|c| c.starts_with(prefix))
}
fn anchor(review: &mut Value, record: &mut ReleaseImpact, uri: &str) {
    review["interfaces"][0]["contract_anchors"] = json!([uri]);
    record.effects[0].contract_anchors = vec![uri.into()];
}

#[test]
fn finite_bytes_paths_nodes_and_membership_do_not_grant_central_acceptance() {
    let (mut f, r, v) = local(|_| {});
    bind(&mut f, &r, &v);
    f.transition.proof.contributions_verified = true;
    let report = f.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    for prefix in [
        "affected-review-bytes:",
        "affected-review-product-path-census:",
        "affected-contract-anchor:",
        "affected-interface-current-evidence-membership:",
        "affected-topic-obligation-membership:",
    ] {
        assert!(claim(&report, prefix), "{prefix}: {report:?}");
    }
    assert!(has(&report, "affected-review-central-adequacy-later"));
    fs::write(f.root.join(doc()), b"mutable worktree is not captured XML").unwrap();
    assert_eq!(
        serde_json::to_vec(&report).unwrap(),
        serde_json::to_vec(&f.contribution_report()).unwrap()
    );
}

#[test]
fn exact_review_digest_source_and_capture_scope_refuse_contradictions() {
    for defect in ["digest", "source", "scope"] {
        let (mut f, r, mut v) = local(|_| {});
        if defect == "scope" {
            v["after_capture_id"] = json!("other");
        }
        bind(&mut f, &r, &v);
        let code = match defect {
            "digest" => {
                f.transition
                    .contribution_scope
                    .as_mut()
                    .unwrap()
                    .affected_source_review
                    .as_mut()
                    .unwrap()
                    .sha256 = "0".repeat(64);
                "affected-review-digest"
            }
            "source" => {
                fs::write(f.release.join("source-review.md"), b"replaced").unwrap();
                "affected-review-source-digest"
            }
            _ => "affected-review-scope",
        };
        failed(&f.contribution_report(), code);
    }
}

#[test]
fn omitted_invented_paths_and_endpoint_identity_or_absence_refuse() {
    for defect in ["omitted", "invented", "sha", "absence"] {
        let (mut f, r, mut v) = local(|_| {});
        let code = match defect {
            "omitted" => {
                v["paths"] = json!([]);
                v["interfaces"] = json!([]);
                v["topics"] = json!([]);
                "affected-review-path-census"
            }
            "invented" => {
                v["paths"].as_array_mut().unwrap().push(json!({"path":"invented.rs","interface_ids":[],"user_no_impact_reason":"No user surface.","documentation_no_impact_reason":"No documented surface."}));
                "affected-review-path-census"
            }
            "sha" => {
                v["paths"][0]["before"]["sha256"] = json!("0".repeat(64));
                "affected-review-endpoint-identity"
            }
            _ => {
                v["paths"][0].as_object_mut().unwrap().remove("before");
                "affected-review-endpoint-absence"
            }
        };
        bind(&mut f, &r, &v);
        failed(&f.contribution_report(), code);
    }
}

#[test]
fn captured_contract_authority_node_binding_and_alias_collision_refuse() {
    for defect in ["authority", "anchor", "unbound", "collision"] {
        let (mut f, mut r, mut v) = local(|f| {
            if defect == "collision" {
                fs::write(
                    f.root.join(format!("{}/common/PROP-001-alias.md", specs())),
                    b"alias",
                )
                .unwrap();
            }
        });
        let code = match defect {
            "authority" => {
                v["contract_sources"][0]["name"] = json!("other");
                "affected-contract-authority"
            }
            "anchor" => {
                anchor(
                    &mut v,
                    &mut r,
                    "spec://org.example/fixture/common/PROP-001#MISSING",
                );
                "affected-contract-anchor-missing"
            }
            "unbound" => {
                v["contract_sources"][0]["documents"] = json!([]);
                "affected-contract-document-unbound"
            }
            _ => "affected-contract-document-collision",
        };
        bind(&mut f, &r, &v);
        failed(&f.contribution_report(), code);
    }
}

#[test]
fn unsupported_profile_or_authority_never_hides_endpoint_or_document_digests() {
    for defect in ["profile", "authority", "pin"] {
        let (mut f, mut r, mut v) = local(|_| {});
        if defect == "profile" {
            v["contract_sources"][0]["source_profile"] = json!("urn:foreign:spec");
        } else if defect == "authority" {
            anchor(
                &mut v,
                &mut r,
                "spec://org.other/foreign/common/PROP-001#RULE",
            );
        } else {
            anchor(
                &mut v,
                &mut r,
                "spec://org.example/fixture@1.0.0/common/PROP-001#RULE",
            );
        }
        bind(&mut f, &r, &v);
        let report = f.contribution_report();
        assert!(
            matches!(report.verdict, CheckReportVerdict::Pending),
            "{report:?}"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| matches!(f.state, FindingState::Unsupported))
        );
        v["paths"][0]["after"]["sha256"] = json!("0".repeat(64));
        bind(&mut f, &r, &v);
        failed(
            &f.contribution_report(),
            "affected-review-endpoint-identity",
        );
        v["paths"][0]["after"] =
            identity(&f, f.transition.dp2.commit_sha1.as_ref().unwrap(), "cli.rs");
        v["contract_sources"][0]["documents"][0]["sha256"] = json!("0".repeat(64));
        bind(&mut f, &r, &v);
        failed(
            &f.contribution_report(),
            "affected-contract-source-identity",
        );
    }
}

#[test]
fn effect_scope_anchor_comparable_kind_and_missing_current_interface_refuse() {
    for defect in ["cap", "anchor", "kind", "missing"] {
        let (mut f, mut r, mut v) = local(|_| {});
        let code = match defect {
            "cap" => {
                r.effects[0].capability_id = "other-cap".into();
                "affected-effect-scope"
            }
            "anchor" => {
                r.effects[0].contract_anchors =
                    vec!["spec://org.example/fixture/common/PROP-001#OTHER".into()];
                "affected-effect-anchor"
            }
            "kind" => {
                v["interfaces"][0]["kind"] = json!("introduced");
                "affected-effect-comparable-declaration"
            }
            _ => {
                r.effects.clear();
                "affected-interface-current-coverage"
            }
        };
        bind(&mut f, &r, &v);
        failed(&f.contribution_report(), code);
    }
}

#[test]
fn intermediate_effect_descriptions_remain_limited_membership_not_global_net() {
    let (mut f, mut r, v) = local(|_| {});
    let intermediate = f.snapshot("intermediate");
    f.transition.captures = Some(vec![intermediate]);
    r.after_capture_id = "intermediate".into();
    r.effects[0].before = "Earlier contribution description".into();
    bind(&mut f, &r, &v);
    let report = f.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert!(has(&report, "affected-effect-semantic-comparison-pending"));
    assert!(!claim(&report, "affected-effect-comparable-declarations:"));
    assert!(claim(&report, "affected-effect-declared-membership:"));
}

#[cfg(unix)]
#[test]
fn unrelated_nonregular_contract_does_not_mask_known_target_contradictions() {
    for defect in ["anchor", "collision", "unbound", "valid"] {
        let (mut f, mut r, mut v) = local(|f| {
            std::os::unix::fs::symlink(
                "outside",
                f.root
                    .join(format!("{}/common/PROP-999-unsupported.xml", specs())),
            )
            .unwrap();
            if defect == "collision" {
                fs::write(
                    f.root.join(format!("{}/common/PROP-001-alias.md", specs())),
                    b"alias",
                )
                .unwrap();
            }
        });
        if defect == "anchor" {
            anchor(
                &mut v,
                &mut r,
                "spec://org.example/fixture/common/PROP-001#MISSING",
            );
        }
        if defect == "unbound" {
            v["contract_sources"][0]["documents"] = json!([]);
        }
        bind(&mut f, &r, &v);
        let report = f.contribution_report();
        if defect == "valid" {
            assert!(
                matches!(report.verdict, CheckReportVerdict::Pending),
                "{report:?}"
            );
            assert!(claim(&report, "affected-contract-anchor:"));
        } else {
            failed(
                &report,
                match defect {
                    "anchor" => "affected-contract-anchor-missing",
                    "collision" => "affected-contract-document-collision",
                    _ => "affected-contract-document-unbound",
                },
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn unknown_nonutf8_candidate_prevents_false_missing_but_not_known_node_refusal() {
    use std::os::unix::ffi::OsStringExt;
    for target in ["unknown", "missing-node", "known"] {
        let (mut f, mut r, mut v) = local(|f| {
            let name = std::ffi::OsString::from_vec(b"PROP-999-\xff.xml".to_vec());
            fs::write(f.root.join(specs()).join("common").join(name), XML).unwrap();
        });
        if target == "unknown" {
            anchor(
                &mut v,
                &mut r,
                "spec://org.example/fixture/common/PROP-999#RULE",
            );
        }
        if target == "missing-node" {
            anchor(
                &mut v,
                &mut r,
                "spec://org.example/fixture/common/PROP-001#MISSING",
            );
        }
        bind(&mut f, &r, &v);
        let report = f.contribution_report();
        if target == "missing-node" {
            failed(&report, "affected-contract-anchor-missing");
        } else {
            assert!(
                matches!(report.verdict, CheckReportVerdict::Pending),
                "{report:?}"
            );
            assert!(!claim(&report, "affected-contract-anchor:"));
            if target == "unknown" {
                assert!(has(
                    &report,
                    "affected-contract-target-inventory-unsupported"
                ));
            } else {
                assert!(claim(&report, "affected-contract-known-node:"));
            }
        }
    }
}

fn unmapped(path: &str, before: Value, after: Value) -> Value {
    let mut value = json!({"path":path,"interface_ids":[],"user_no_impact_reason":"Internal storage only.","documentation_no_impact_reason":"No documentation claim."});
    if !before.is_null() {
        value["before"] = before;
    }
    if !after.is_null() {
        value["after"] = after;
    }
    value
}

#[test]
fn transient_paths_are_reviewed_even_when_absent_at_both_endpoints() {
    let (mut f, mut r, mut v) = local(|_| {});
    let add = f.commit_product("transient.rs", b"temporary product implementation\n");
    f.git(&["rm", "--quiet", "transient.rs"]);
    let delete = f.commit();
    r.implementation_subjects
        .push(serde_json::from_value(json!({"kind":"commit","identity":add})).unwrap());
    r.implementation_subjects
        .push(serde_json::from_value(json!({"kind":"commit","identity":delete})).unwrap());
    r.affected_paths = Some(vec!["cli.rs".into(), "transient.rs".into()]);
    f.after(f.snapshot("after"));
    v["paths"]
        .as_array_mut()
        .unwrap()
        .push(unmapped("transient.rs", Value::Null, Value::Null));
    bind(&mut f, &r, &v);
    let report = f.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert!(claim(
        &report,
        "affected-path-endpoint-absence:finite-review:before:transient.rs"
    ));
    assert!(claim(
        &report,
        "affected-path-endpoint-absence:finite-review:after:transient.rs"
    ));
    v["paths"].as_array_mut().unwrap().pop();
    bind(&mut f, &r, &v);
    failed(&f.contribution_report(), "affected-review-path-census");
}

#[test]
fn mixed_internal_path_reasons_do_not_force_whole_record_no_impact() {
    let (mut f, mut r, mut v) = local(|_| {});
    let commit = f.commit_product("internal.rs", b"private cache layout\n");
    r.implementation_subjects
        .push(serde_json::from_value(json!({"kind":"commit","identity":commit})).unwrap());
    r.affected_paths = Some(vec!["cli.rs".into(), "internal.rs".into()]);
    let mut json = serde_json::to_value(&r).unwrap();
    json["documentation_impact"] = json!({"state":"impact","documentation_action":"update","reason":"Explain the public Check command."});
    r = serde_json::from_value(json).unwrap();
    f.after(f.snapshot("after"));
    v["paths"].as_array_mut().unwrap().push(unmapped(
        "internal.rs",
        Value::Null,
        identity(
            &f,
            f.transition.dp2.commit_sha1.as_ref().unwrap(),
            "internal.rs",
        ),
    ));
    bind(&mut f, &r, &v);
    let report = f.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Passed),
        "{report:?}"
    );
    assert!(claim(&report, "affected-interface-path-membership:"));
}

#[cfg(unix)]
#[test]
fn observed_symlink_is_not_absent_or_a_verified_regular_blob() {
    let (mut f, mut r, mut v) = local(|_| {});
    std::os::unix::fs::symlink("cli.rs", f.root.join("link.rs")).unwrap();
    f.git(&["add", "--", "link.rs"]);
    let subject = f.commit();
    r.implementation_subjects
        .push(serde_json::from_value(json!({"kind":"commit","identity":subject})).unwrap());
    r.affected_paths = Some(vec!["cli.rs".into(), "link.rs".into()]);
    f.after(f.snapshot("after"));
    v["paths"]
        .as_array_mut()
        .unwrap()
        .push(unmapped("link.rs", Value::Null, Value::Null));
    bind(&mut f, &r, &v);
    failed(&f.contribution_report(), "affected-review-endpoint-absence");
    v["paths"][1]["after"] = identity(
        &f,
        f.transition.dp2.commit_sha1.as_ref().unwrap(),
        "link.rs",
    );
    bind(&mut f, &r, &v);
    let report = f.contribution_report();
    assert!(
        matches!(report.verdict, CheckReportVerdict::Pending),
        "{report:?}"
    );
    assert!(!claim(
        &report,
        "affected-path-endpoint-bytes:finite-review:after:link.rs"
    ));
}
