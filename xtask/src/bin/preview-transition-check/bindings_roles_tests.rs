use super::*;
use std::path::PathBuf;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap()
}
#[test]
fn authoritative_native_accounting_join_and_retained_census() {
    let root = root();
    let census: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../formats/corpora/change-manifest/e1/retained-preview.json"
    ))
    .unwrap();
    let rows = census["records"].as_array().unwrap();
    assert_eq!(rows.len(), 71);
    for row in rows {
        let id = row["change_id"].as_str().unwrap();
        let bytes = bindings::native_record(&root, None, id).unwrap();
        assert_eq!(capture::sha256(&bytes), row["sha256"].as_str().unwrap());
        assert_eq!(
            bytes,
            fs::read(root.join(row["path"].as_str().unwrap())).unwrap()
        );
    }
    let request: CheckRequest = serde_json::from_value(serde_json::json!({"schema":1,"transition":root.join("docs/releases/developer-preview-1-to-2/transition.json"),"mode":"bindings","stage":"roles","format":"json"})).unwrap();
    let report = run(&request);
    assert_eq!(
        report.verdict,
        CheckReportVerdict::Passed,
        "{:?}",
        report.findings
    );
    assert_eq!(
        report
            .checked_claims
            .iter()
            .filter(|s| s.starts_with("native-release-impact:"))
            .count(),
        4
    );
}
#[test]
fn wrong_id_digest_path_epoch_and_normative_evidence_refuse() {
    let source = root();
    let scratch = tempfile::tempdir().unwrap();
    fs::copy(source.join("vibe.toml"), scratch.path().join("vibe.toml")).unwrap();
    let id = "preview-accounting-foundation";
    let relative = Path::new("custom/specs/changes").join(id);
    let directory = scratch.path().join(&relative);
    fs::create_dir_all(&directory).unwrap();
    let original = source.join("vibevm/vibespecs/changes").join(id);
    let manifest = fs::read_to_string(original.join("change.toml")).unwrap();
    let bytes = fs::read(original.join("release-impact.json")).unwrap();
    fs::write(directory.join("change.toml"), &manifest).unwrap();
    fs::write(directory.join("release-impact.json"), &bytes).unwrap();
    assert_eq!(
        bindings::native_record(scratch.path(), Some(Path::new("custom/specs")), id).unwrap(),
        bytes
    );
    assert!(
        bindings::bound_record(
            scratch.path(),
            Some(Path::new("custom/specs")),
            id,
            &"0".repeat(64)
        )
        .is_err()
    );
    for text in [
        manifest.replace("normative = false", "normative = true"),
        manifest.replace(
            "path = \"release-impact.json\"",
            "path = \"../release-impact.json\"",
        ),
        manifest.replace(id, "replacement-id"),
    ] {
        fs::write(directory.join("change.toml"), text).unwrap();
        assert!(
            bindings::native_record(scratch.path(), Some(Path::new("custom/specs")), id).is_err()
        );
    }
    fs::write(directory.join("change.toml"), manifest).unwrap();
    let normative =
        include_str!("../../../../formats/corpora/change-manifest/e1/valid/representative.toml")
            .replace("chg-0123456789abcdef0123456789abcdef", id);
    let normative = format!(
        "{normative}\n[[documents]]\nrole='release-impact'\npath='release-impact.json'\nnormative=false\ninput=true\n"
    );
    fs::write(directory.join("change.toml"), &normative).unwrap();
    assert!(bindings::native_record(scratch.path(), Some(Path::new("custom/specs")), id).is_err());
    fs::write(directory.join("intent.md"), "intent").unwrap();
    fs::write(directory.join("acceptance.md"), "acceptance").unwrap();
    assert!(bindings::native_record(scratch.path(), Some(Path::new("custom/specs")), id).is_ok());
    for (field, value) in [
        ("change_id", serde_json::json!("replacement-id")),
        ("schema", serde_json::json!(99)),
    ] {
        let mut record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        record[field] = value;
        fs::write(
            directory.join("release-impact.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        assert!(
            bindings::native_record(scratch.path(), Some(Path::new("custom/specs")), id).is_err()
        );
    }
}
#[test]
fn linked_proof_stays_pending_despite_roles_and_producer_flags() {
    let request: CheckRequest = serde_json::from_value(serde_json::json!({"schema":1,"transition":root().join("docs/releases/developer-preview-1-to-2/transition.json"),"mode":"bindings","stage":"linked","format":"json"})).unwrap();
    let mut transition: Transition =
        serde_json::from_slice(&fs::read(&request.transition).unwrap()).unwrap();
    transition.proof.capture_validated = true;
    transition.proof.contributions_verified = true;
    transition.proof.documentation_verified = true;
    transition.proof.migration_rehearsed = true;
    transition.proof.final_product_panel_passed = true;
    let mut report = empty_report(report_mode(&request.mode), report_stage(&request.stage));
    bindings::check(&request, &transition, &mut report).unwrap();
    assert_eq!(report.verdict, CheckReportVerdict::Pending);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == "bindings-linked-owner-unavailable")
    );
    assert!(
        report
            .checked_claims
            .iter()
            .any(|s| s.starts_with("native-release-impact:"))
    );
}

// Both consumers must use the saved owner's limits and retained observations.
fn bounded_fixture() -> (tempfile::TempDir, PathBuf, String, Vec<u8>) {
    let source = root();
    let scratch = tempfile::tempdir().unwrap();
    fs::copy(source.join("vibe.toml"), scratch.path().join("vibe.toml")).unwrap();
    let id = "preview-accounting-foundation";
    let original = source.join("vibevm/vibespecs/changes").join(id);
    let directory = scratch.path().join("custom/specs/changes").join(id);
    fs::create_dir_all(&directory).unwrap();
    let manifest = fs::read_to_string(original.join("change.toml")).unwrap();
    let bytes = fs::read(original.join("release-impact.json")).unwrap();
    fs::write(directory.join("change.toml"), &manifest).unwrap();
    fs::write(directory.join("release-impact.json"), &bytes).unwrap();
    (scratch, directory, manifest, bytes)
}
fn same_saved_bytes(root: &Path, bytes: &[u8]) {
    let id = "preview-accounting-foundation";
    let saved = vibe_requirements::change::read_saved_change(
        root,
        ".",
        Some(Path::new("custom/specs")),
        id,
    )
    .unwrap();
    assert_eq!(saved.release_impacts().unwrap().len(), 1);
    let path = saved.manifest.directory().join("release-impact.json");
    assert_eq!(
        saved
            .input_observations
            .iter()
            .find(|(p, _)| *p == path)
            .unwrap()
            .1
            .bytes,
        bytes
    );
    assert_eq!(
        bindings::native_record(root, Some(Path::new("custom/specs")), id).unwrap(),
        bytes
    );
}
fn same_saved_refusal(root: &Path, boundary: &'static str) {
    let id = "preview-accounting-foundation";
    let error = vibe_requirements::change::read_saved_change(
        root,
        ".",
        Some(Path::new("custom/specs")),
        id,
    )
    .unwrap_err();
    assert!(
        matches!(error, vibe_requirements::change::SavedChangeError::Unavailable(b) if b == boundary)
    );
    let error = bindings::native_record(root, Some(Path::new("custom/specs")), id).unwrap_err();
    assert_eq!(error.code, "native-saved");
}
#[test]
fn saved_manifest_limit_is_shared_with_accounting() {
    let (scratch, directory, mut manifest, bytes) = bounded_fixture();
    manifest.push_str("\n#");
    manifest.extend(std::iter::repeat_n('x', 64 * 1024 - manifest.len()));
    fs::write(directory.join("change.toml"), &manifest).unwrap();
    same_saved_bytes(scratch.path(), &bytes);
    manifest.push('x');
    fs::write(directory.join("change.toml"), manifest).unwrap();
    same_saved_refusal(scratch.path(), "change manifest");
}
#[test]
fn saved_role_limit_is_shared_for_evidence_and_ordinary_inputs() {
    let (scratch, directory, manifest, bytes) = bounded_fixture();
    let id = "preview-accounting-foundation";
    let mut record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    record["documentation_impact"]["reason"] = serde_json::json!("x".repeat(16 * 1024 * 1024));
    let oversized = serde_json::to_vec(&record).unwrap();
    // This is semantically valid evidence, refused specifically by saved admission.
    vibe_wire::generated::preview::e1::release_impact::ReleaseImpact::read_for_change(
        &oversized,
        id,
        "custom/specs/changes/preview-accounting-foundation/release-impact.json",
    )
    .unwrap();
    fs::write(directory.join("release-impact.json"), oversized).unwrap();
    same_saved_refusal(scratch.path(), "held role input");
    fs::write(directory.join("release-impact.json"), &bytes).unwrap();
    let ordinary =
        include_str!("../../../../formats/corpora/change-manifest/e1/valid/representative.toml")
            .replace("chg-0123456789abcdef0123456789abcdef", id);
    let ordinary = format!(
        "{ordinary}\n[[documents]]\nrole='release-impact'\npath='release-impact.json'\nnormative=false\ninput=true\n"
    );
    fs::write(directory.join("change.toml"), ordinary).unwrap();
    fs::write(directory.join("intent.md"), "intent").unwrap();
    fs::write(directory.join("acceptance.md"), "acceptance").unwrap();
    same_saved_bytes(scratch.path(), &bytes);
    fs::write(
        directory.join("intent.md"),
        vec![b'x'; 16 * 1024 * 1024 + 1],
    )
    .unwrap();
    same_saved_refusal(scratch.path(), "held role input");
    // Restore the small evidence-only fixture to prove it remains readable.
    fs::write(directory.join("change.toml"), manifest).unwrap();
    same_saved_bytes(scratch.path(), &bytes);
}
