//! Owned source-delivery fixtures; syscall observers consume explicit query intervals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-QUERY-PURITY-EVIDENCE");
use super::*;
use std::fs;
use vibe_core::manifest::SpecFormat;
fn inventory(root: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
    fn walk(root: &Path, at: &Path, rows: &mut Vec<(PathBuf, String, Vec<u8>)>) {
        let m = fs::symlink_metadata(at).unwrap();
        let value = if m.is_file() {
            format!("file:{:x}", Sha256::digest(fs::read(at).unwrap()))
        } else {
            format!("{:?}", m.file_type())
        };
        let bytes = if m.is_file() {
            fs::read(at).unwrap()
        } else {
            Vec::new()
        };
        rows.push((at.strip_prefix(root).unwrap().to_path_buf(), value, bytes));
        if m.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(at)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            entries.sort();
            for entry in entries {
                walk(root, &entry, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let (root, host) = super::m_01_d_2_cases::installed(SpecFormat::Markdown);
    fs::create_dir_all(root.path().join("home/.vibe/cache")).unwrap();
    fs::write(
        root.path().join("home/.vibe/config.toml"),
        "# E2 unchanged empty configuration\n",
    )
    .unwrap();
    (root, host)
}
fn request() -> Value {
    json!({"schema":1,"address":"spec://org.demo/dep/DOC#ONE","mode":"read","format":"md","children":true})
}
fn measured<T>(name: &str, query: impl FnOnce() -> T) -> T {
    eprintln!("E2_QUERY_BEGIN {name}");
    let result = query();
    eprintln!("E2_QUERY_END {name}");
    result
}
#[test]
fn prepared_offline_query_repeats_without_state_drift() {
    let (root, host) = fixture();
    let service = SourceQueryService::prepare_host(&host).unwrap();
    let request = wire::read_request(&request()).unwrap();
    let before = inventory(root.path());
    let a = measured("prepared-repeat-a", || service.query(&request).unwrap());
    let b = measured("prepared-repeat-b", || service.query(&request).unwrap());
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    let after = inventory(root.path());
    assert_eq!(after, before);
    println!("E2_SNAPSHOT {}", json!({"before":before,"after":after}));
}
#[test]
fn missing_capture_is_unavailable_without_recreation() {
    let (root, host) = fixture();
    let service = SourceQueryService::prepare_host(&host).unwrap();
    let slot = crate::vibedeps::slot_abs_path(
        &host,
        &"org.demo".parse().unwrap(),
        "dep",
        &"1.0.0".parse().unwrap(),
    );
    let output = slot.join("vibevm/vibespecs/DOC.md");
    fs::remove_file(&output).unwrap();
    let before = inventory(root.path());
    let error = measured("missing-prepared", || {
        service.query(&wire::read_request(&request()).unwrap())
    })
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UnavailableWorld);
    assert_eq!(error.boundary, "selected output");
    assert!(!output.exists());
    let after = inventory(root.path());
    assert_eq!(after, before);
    println!("E2_SNAPSHOT {}", json!({"before":before,"after":after}));
}
#[test]
fn payload_tamper_refuses_without_repair() {
    let (root, host) = fixture();
    let slot = crate::vibedeps::slot_abs_path(
        &host,
        &"org.demo".parse().unwrap(),
        "dep",
        &"1.0.0".parse().unwrap(),
    );
    fs::write(slot.join("vibevm/vibespecs/DOC.md"), "POISONED_PAYLOAD").unwrap();
    let before = inventory(root.path());
    let error = wire::read_error(
        &measured("payload-tamper", || {
            SourceQueryService::execute_json(&host, &request())
        })
        .into_value(),
    )
    .unwrap();
    assert_eq!(error.code, ErrorCode::UnavailableWorld);
    assert_eq!(error.boundary, "selected descriptor/output map");
    let after = inventory(root.path());
    assert_eq!(after, before);
    println!("E2_SNAPSHOT {}", json!({"before":before,"after":after}));
}
#[test]
fn changed_expectation_refuses_and_local_strength_stays_explicit() {
    let (root, host) = fixture();
    let before = inventory(root.path());
    let result = wire::read_result(
        &measured("local-strength", || {
            SourceQueryService::execute_json(&host, &request())
        })
        .into_value(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(result.observation.strength).unwrap(),
        json!("record-consistent")
    );
    for (field, prefix, boundary) in [
        ("expected_document_hash", "sha256:", "document expectation"),
        ("expected_source_hash", "sha256:", "source expectation"),
        (
            "expected_resolution_hash",
            "sha256-query-resolution/1:",
            "resolution expectation",
        ),
    ] {
        let mut input = request();
        input[field] = json!(format!("{prefix}{}", "b".repeat(64)));
        let error = wire::read_error(
            &measured(field, || SourceQueryService::execute_json(&host, &input)).into_value(),
        )
        .unwrap();
        assert_eq!(error.code, ErrorCode::StaleExpectation);
        assert_eq!(error.boundary, boundary);
    }
    let after = inventory(root.path());
    assert_eq!(after, before);
    println!("E2_SNAPSHOT {}", json!({"before":before,"after":after}));
}
