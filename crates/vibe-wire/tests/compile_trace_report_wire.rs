//! The shared compile-trace member across the FOUR command-report roots
//! (`cli-install-report`, `cli-lifecycle-report`, `cli-update-report`,
//! `cli-reinstall-report`; PROP-054 `##OBS-TRACE`, R3.4 §5.4 of the
//! implementation architecture).
//!
//! Three facts are pinned here. ROUND-TRIP: disabled old JSON omits
//! `trace` and still parses — and re-serialises without inventing the
//! member — while one authored `unavailable` and one active value ride
//! every root byte-identically. TYPE IDENTITY: every root's
//! `trace.timings` is the SAME generated `TimingRow` the trace index
//! aggregates, proven at compile time (shared-module re-exports, not
//! same-shaped per-module types). SCHEMA PARITY: the four roots spell
//! the member identically and pull the one shared fragment, and the
//! fragment's `x-relational-laws` label set equals the implemented-law
//! list of the hand-written validator
//! (`behaviour::compile_trace_report`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use vibe_wire::behaviour::compile_trace_report::{IMPLEMENTED_LAWS, validate};
use vibe_wire::generated::compiler_trace_index::e1::index::TimingRow as IndexTimingRow;
use vibe_wire::generated::shared::{CompileTraceReport, TraceReportStatus};
use vibe_wire::generated::{
    install_report::InstallReport, lifecycle_report::LifecycleReport,
    reinstall_report::ReinstallReport, update_report::UpdateReport,
};

const RUN_ID: &str = "00112233445566778899aabbccddeeff";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> serde_json::Value {
    let path = repo_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} readable: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative} parses: {e}"))
}

/// The unavailable trace: requested, never opened, honest about why.
fn unavailable_trace() -> serde_json::Value {
    serde_json::json!({
        "status": "unavailable",
        "run_id": RUN_ID,
        "finalised": false,
        "budget_exhausted": false,
        "events": "0",
        "snapshots": "0",
        "snapshot_bytes": "0",
        "timings": [],
        "warnings": ["trace open refused: .vibe/trace is not writable"],
    })
}

/// An active (parked, running) trace: pathed, counted, two timing rows.
fn running_trace() -> serde_json::Value {
    serde_json::json!({
        "status": "running",
        "run_id": RUN_ID,
        "run_path": format!("C:/work/demo/.vibe/trace/{RUN_ID}"),
        "finalised": false,
        "budget_exhausted": false,
        "events": "12",
        "snapshots": "11",
        "snapshot_bytes": "1048576",
        "timings": [
            {
                "pass": "parse",
                "invocations": 8,
                "pass_total": {"micros": 2100, "saturated": false},
                "verify_total": {"micros": 80, "saturated": false},
                "encode_total": {"micros": 1500, "saturated": false}
            },
            {
                "pass": "emit:static-xml",
                "invocations": 4,
                "pass_total": {"micros": 4200, "saturated": false},
                "verify_total": {"micros": 40, "saturated": false},
                "encode_total": {"micros": 640, "saturated": false}
            }
        ],
        "warnings": []
    })
}

/// One pre-R3.4 document for each root — exactly the member set the
/// old writers emitted, no `trace` anywhere.
fn old_documents() -> Vec<(&'static str, serde_json::Value)> {
    let slot_facts = serde_json::json!({
        "complete": true,
        "unchanged": false,
        "materialised": ["vibedeps/org.demo.tools/0.1.0"],
        "skipped": [],
        "pruned": [],
        "nodes_regenerated": ["."],
    });
    let merge = |command: &str, extra: serde_json::Value| {
        let mut base = serde_json::json!({
            "ok": true,
            "command": command,
            "project": "C:/work/demo",
        });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base.as_object_mut()
            .unwrap()
            .extend(slot_facts.as_object().unwrap().clone());
        base
    };
    vec![
        // The install root omits empty `hooks`/`notices` (`x-empty:
        // "omit"`) — the old writer's exact bytes.
        ("install", merge("install", serde_json::json!({}))),
        (
            "update",
            merge(
                "update",
                serde_json::json!({
                    "scope": "all",
                    "packages": [],
                    "packages_resolved": 1,
                    "version_bumps": [],
                    "hooks": [],
                }),
            ),
        ),
        (
            "reinstall",
            merge(
                "reinstall",
                serde_json::json!({"forced": false, "hooks": []}),
            ),
        ),
        (
            "lifecycle",
            read_json("formats/corpora/lifecycle/e1/report.json"),
        ),
    ]
}

/// The typed roots: parsing and re-serialising one JSON value through
/// each root proves the four shapes agree on the member.
fn round_trip<T: serde::de::DeserializeOwned + serde::Serialize>(
    label: &str,
    authored: serde_json::Value,
) -> serde_json::Value {
    let value: T =
        serde_json::from_value(authored.clone()).unwrap_or_else(|e| panic!("{label} parses: {e}"));
    let rendered = serde_json::to_value(&value).unwrap();
    assert_eq!(rendered, authored, "{label} loses data on round-trip");
    rendered
}

/// Disabled old JSON omits `trace`, still parses, and re-serialises
/// without inventing the member — for every one of the four roots.
#[test]
fn every_old_report_object_without_trace_round_trips_unchanged() {
    let install = round_trip::<InstallReport>("install-old", old_documents()[0].1.clone());
    let update = round_trip::<UpdateReport>("update-old", old_documents()[1].1.clone());
    let reinstall = round_trip::<ReinstallReport>("reinstall-old", old_documents()[2].1.clone());
    let lifecycle = round_trip::<LifecycleReport>("lifecycle-corpus", old_documents()[3].1.clone());
    for (label, rendered) in [
        ("install", install),
        ("update", update),
        ("reinstall", reinstall),
        ("lifecycle", lifecycle),
    ] {
        assert!(
            rendered.get("trace").is_none(),
            "{label}: an absent trace member round-trips as absent, never as `null`"
        );
    }
}

/// One authored unavailable value and one active value ride all four
/// generated roots byte-identically, and validate green through the
/// shared behaviour cell.
#[test]
fn one_unavailable_and_one_active_trace_round_trip_through_all_four_roots() {
    for trace in [unavailable_trace(), running_trace()] {
        let expected: CompileTraceReport =
            serde_json::from_value(trace.clone()).expect("the shared member parses");
        validate(&expected).unwrap_or_else(|e| panic!("authored trace violates a law: {e}"));
        assert_eq!(serde_json::to_value(&expected).unwrap(), trace);

        let mut install = old_documents()[0].1.clone();
        install["trace"] = trace.clone();
        let rendered = round_trip::<InstallReport>("install+trace", install);
        assert_eq!(rendered["trace"], trace);

        let mut update = old_documents()[1].1.clone();
        update["trace"] = trace.clone();
        assert_eq!(
            round_trip::<UpdateReport>("update+trace", update)["trace"],
            trace
        );

        let mut reinstall = old_documents()[2].1.clone();
        reinstall["trace"] = trace.clone();
        assert_eq!(
            round_trip::<ReinstallReport>("reinstall+trace", reinstall)["trace"],
            trace
        );

        let mut lifecycle = old_documents()[3].1.clone();
        lifecycle["trace"] = trace.clone();
        let rendered = round_trip::<LifecycleReport>("lifecycle+trace", lifecycle);
        assert_eq!(rendered["trace"], trace);
        // The typed member agrees with the wire: the running value is
        // pathed and unfinalised, the unavailable one silent-counted.
        let report: LifecycleReport = serde_json::from_value(rendered).unwrap();
        let member = report.trace.as_ref().unwrap();
        assert_eq!(member.run_id, RUN_ID);
        match &member.status {
            TraceReportStatus::Unavailable => {
                assert!(member.run_path.is_none());
                assert_eq!(member.events, "0");
                assert_eq!(member.warnings.len(), 1);
            }
            TraceReportStatus::Running => {
                assert_eq!(
                    member.run_path.as_deref(),
                    Some("C:/work/demo/.vibe/trace/00112233445566778899aabbccddeeff")
                );
                assert!(!member.finalised);
                assert_eq!(member.timings.len(), 2);
                assert_eq!(member.timings[1].pass, "emit:static-xml");
            }
            other => panic!(
                "the authored fixture is unavailable or running, not {other:?}",
                other = &other
            ),
        }
    }
}

/// COMPILE-TIME identity proof: every root's `trace.timings` is the
/// SAME `TimingRow` type the trace index's `aggregates` carry — the
/// shared module's one type, re-exported, never a per-module twin that
/// a converter would have to bridge.
fn rows_of_the_index_type(rows: &[IndexTimingRow]) -> usize {
    rows.len()
}

/// COMPILE-TIME member proof: all four roots spell the member
/// `trace: Option<CompileTraceReport>` over the one shared type.
fn trace_member_of_the_shared_type(
    trace: Option<CompileTraceReport>,
) -> Option<CompileTraceReport> {
    trace
}

#[test]
fn the_four_roots_share_the_index_timing_row_and_one_trace_type() {
    for trace in [unavailable_trace(), running_trace()] {
        let install: InstallReport = serde_json::from_value({
            let mut doc = old_documents()[0].1.clone();
            doc["trace"] = trace.clone();
            doc
        })
        .unwrap();
        let update: UpdateReport = serde_json::from_value({
            let mut doc = old_documents()[1].1.clone();
            doc["trace"] = trace.clone();
            doc
        })
        .unwrap();
        let reinstall: ReinstallReport = serde_json::from_value({
            let mut doc = old_documents()[2].1.clone();
            doc["trace"] = trace.clone();
            doc
        })
        .unwrap();
        let lifecycle: LifecycleReport = serde_json::from_value({
            let mut doc = old_documents()[3].1.clone();
            doc["trace"] = trace.clone();
            doc
        })
        .unwrap();

        for (label, member) in [
            ("install", install.trace.as_ref()),
            ("update", update.trace.as_ref()),
            ("reinstall", reinstall.trace.as_ref()),
            ("lifecycle", lifecycle.trace.as_ref()),
        ] {
            let member = member.unwrap_or_else(|| panic!("{label}: the fixture embeds a trace"));
            // Would not compile against a per-module TimingRow twin.
            assert_eq!(
                rows_of_the_index_type(&member.timings),
                member.timings.len()
            );
        }

        // Would not compile if any root spelled the member over another
        // type than the shared fragment's.
        trace_member_of_the_shared_type(install.trace.clone());
        trace_member_of_the_shared_type(update.trace.clone());
        trace_member_of_the_shared_type(reinstall.trace.clone());
        trace_member_of_the_shared_type(lifecycle.trace.clone());
    }
}

/// The four schemas spell the member identically and pull the ONE
/// shared fragment — schema-side parity, so a copy-paste drift in one
/// root is red before any Rust is generated.
#[test]
fn the_four_schema_roots_spell_the_trace_member_identically() {
    for path in [
        "schemas/install_report.jtd.json",
        "schemas/lifecycle_report.jtd.json",
        "schemas/update_report.jtd.json",
        "schemas/reinstall_report.jtd.json",
    ] {
        let schema = read_json(path);
        let pulled: BTreeSet<String> = schema["metadata"]["x-vocabularies"]
            .as_array()
            .unwrap_or_else(|| panic!("{path}: the root declares its vocabularies"))
            .iter()
            .map(|value| value.as_str().expect("a vocabulary name").to_string())
            .collect();
        assert!(
            pulled.contains("compile_trace_report"),
            "{path}: the root pulls the shared trace fragment"
        );
        // The lifecycle root pulls one more — R7.5's verification
        // evidence member — and it is the ONLY root that may: the
        // trace member's own parity is what this assertion defends,
        // not the size of the vocabulary list.
        let expected: BTreeSet<String> = if path == "schemas/lifecycle_report.jtd.json" {
            ["compile_trace_report", "verification_evidence"]
                .into_iter()
                .map(str::to_string)
                .collect()
        } else {
            ["compile_trace_report"]
                .into_iter()
                .map(str::to_string)
                .collect()
        };
        assert_eq!(pulled, expected, "{path}: unexpected vocabulary closure");
        let member = &schema["optionalProperties"]["trace"];
        assert_eq!(member["ref"], "compile_trace_report", "{path}");
        assert_eq!(
            member["metadata"]["x-default"],
            serde_json::Value::Null,
            "{path}"
        );
    }
    // The fragment itself is shared from the vocabulary home, and the
    // index schema pulls the duration/timing-row fragments it used to
    // define locally — the move that makes the types one.
    let vocabularies = read_json("formats/vocabularies.json");
    for fragment in [
        "duration",
        "timing_row",
        "trace_report_status",
        "compile_trace_report",
    ] {
        assert!(
            vocabularies[fragment].is_object(),
            "the {fragment} fragment lives in the vocabulary home"
        );
    }
    let index = read_json("schemas/compiler_trace_index/e1/index.jtd.json");
    assert_eq!(
        index["metadata"]["x-vocabularies"],
        serde_json::json!(["timestamp", "duration", "timing_row"]),
        "the index schema pulls the shared duration/timing-row fragments"
    );
    assert!(
        index["definitions"].get("duration").is_none()
            && index["definitions"].get("timing_row").is_none(),
        "the index schema no longer carries its own duration/timing_row definitions"
    );
}

/// The fragment's documented law labels and diagnostic cap are exactly
/// what the hand-written validator implements — an undocumented law and
/// an unimplemented label are both red, and the warnings cap is the
/// SAME cap the trace index carries (one diagnostic budget, not two).
#[test]
fn law_labels_and_diagnostic_cap_match_the_vocabulary_fragment() {
    let fragment = read_json("formats/vocabularies.json")["compile_trace_report"].clone();
    let documented: BTreeSet<String> = fragment["metadata"]["x-relational-laws"]
        .as_array()
        .expect("x-relational-laws is an array")
        .iter()
        .map(|law| {
            law.as_str()
                .expect("every law is a string")
                .split_once(':')
                .expect("every law is `label: sentence`")
                .0
                .to_string()
        })
        .collect();
    let implemented: BTreeSet<&str> = IMPLEMENTED_LAWS.iter().copied().collect();
    let undocumented: Vec<&str> = implemented
        .iter()
        .filter(|law| !documented.contains(**law))
        .copied()
        .collect();
    let unimplemented: Vec<&String> = documented
        .iter()
        .filter(|law| !implemented.contains(law.as_str()))
        .collect();
    assert!(
        undocumented.is_empty() && unimplemented.is_empty(),
        "law parity drift:\n  implemented but undocumented: {undocumented:?}\n  \
         documented but unimplemented: {unimplemented:?}"
    );
    assert_eq!(
        fragment["metadata"]["x-diagnostic-cap-bytes"].as_u64(),
        Some(vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES as u64),
        "the report's warning cap is the trace index's diagnostic cap — one budget"
    );
}
