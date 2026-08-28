//! The fences the requirements root must never fail: no verdict
//! vocabulary, no prose-carrying member, and a source-body canary that
//! cannot survive re-emission.
//!
//! Split from `requirements_report_wire.rs` at the 600-line budget,
//! along the seam between «does the wire say what it should» (there)
//! and «can the wire say what it must not» (here).

use std::collections::BTreeSet;

use vibe_wire::behaviour::requirements_report::validate;
use vibe_wire::generated::requirements_report::RequirementsReport;

#[path = "wire_support/mod.rs"]
mod support;
use support::{read_json, repo_root};

/// The four verdict words `##REQUIREMENT-OBSERVATION-AXES` bars. The
/// relation VERB `verifies` is not one of them: an edge saying a
/// symbol claims to verify a fact is an observation, and the past
/// participle would be the verdict.
const FORBIDDEN_VERDICT_WORDS: &[&str] = &["unmet", "met", "fulfilled", "verified"];

/// The member names that would turn a metadata answer into a content
/// answer. `##FACT-QUERY-CONTRACT`: no fact prose, code body, prompt,
/// recommendation, ranking or next task.
const FORBIDDEN_CONTENT_MEMBERS: &[&str] = &[
    "text",
    "body",
    "prose",
    "content",
    "prompt",
    "recommendation",
    "ranking",
    "rank",
    "score",
    "next_task",
    "suggestion",
];

/// The canary a real source body would carry. It is authored NOWHERE
/// in the corpus tree on purpose: the assertion is its absence from
/// every emitted byte, which is what «this report ships no prose»
/// means operationally (`Q3` of the architecture's matrix).
const SOURCE_BODY_CANARY: &str = "CANARY-fact-prose-must-never-ship";

fn corpus_dir() -> std::path::PathBuf {
    repo_root().join("formats/corpora/requirements/e1")
}

/// Parse one corpus through the generated root, prove the bytes
/// survive, and prove the value satisfies every relational law.
fn corpus(name: &str) -> RequirementsReport {
    let authored = read_json(&format!("formats/corpora/requirements/e1/{name}"));
    let report: RequirementsReport =
        serde_json::from_value(authored.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        authored,
        "{name} loses data on generated round-trip"
    );
    validate(&report).unwrap_or_else(|e| panic!("{name} violates a relational law: {e}"));
    report
}

/// Every `properties`/`optionalProperties` key a JTD document
/// declares, at any depth.
fn declared_members(value: &serde_json::Value, names: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, field) in fields {
                if (key == "properties" || key == "optionalProperties")
                    && let Some(members) = field.as_object()
                {
                    names.extend(members.keys().cloned());
                }
                declared_members(field, names);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                declared_members(item, names);
            }
        }
        _ => {}
    }
}

#[test]
fn the_requirements_wire_carries_no_verdict_and_no_prose() {
    let schema = read_json("schemas/requirements_report.jtd.json");
    let mut members = BTreeSet::new();
    declared_members(&schema, &mut members);
    for word in FORBIDDEN_VERDICT_WORDS
        .iter()
        .chain(FORBIDDEN_CONTENT_MEMBERS)
    {
        assert!(
            !members.contains(*word),
            "the requirements schema declares a `{word}` member"
        );
    }
    let generated = std::fs::read_to_string(
        repo_root().join("crates/vibe-wire/src/generated/requirements_report/mod.rs"),
    )
    .unwrap();
    for word in FORBIDDEN_VERDICT_WORDS
        .iter()
        .chain(FORBIDDEN_CONTENT_MEMBERS)
    {
        assert!(
            !generated.contains(&format!("pub {word}:")),
            "the generated requirements root declares a `{word}` field"
        );
    }
}

/// The source-body canary, made to BITE. The generated reader is
/// permissive (`foreign_parsers = "many"`), so a `body` member CAN
/// arrive on the wire — from a future writer, a hand-edited file, or a
/// proxy that decided to be helpful. The property that matters is not
/// that such a document is refused, it is that this root cannot
/// PROPAGATE it: the member has nowhere to land in the generated type,
/// so canonical re-emission drops it on the floor. That is what «this
/// report ships no prose» means operationally (`Q3`).
#[test]
fn an_injected_source_body_cannot_survive_re_emission() {
    let mut poisoned = read_json("formats/corpora/requirements/e1/report_base.json");
    // One canary at the root, one on a fact row, and one inside the
    // row's own nested observation — the three places a prose leak
    // would plausibly be introduced.
    poisoned["body"] = serde_json::json!(SOURCE_BODY_CANARY);
    poisoned["rows"][0]["body"] = serde_json::json!(SOURCE_BODY_CANARY);
    poisoned["rows"][0]["authoring"]["text"] = serde_json::json!(SOURCE_BODY_CANARY);
    let raw = serde_json::to_string(&poisoned).unwrap();
    assert!(
        raw.contains(SOURCE_BODY_CANARY),
        "the poisoned document really does carry the canary before parsing"
    );

    let report: RequirementsReport =
        serde_json::from_value(poisoned).expect("the permissive reader accepts the unknown member");
    validate(&report).expect("an unknown member changes none of the relational laws");
    let rendered = serde_json::to_string(&report).unwrap();
    assert!(
        !rendered.contains(SOURCE_BODY_CANARY),
        "the canary survived re-emission; this root would then be a prose carrier"
    );
    for word in FORBIDDEN_VERDICT_WORDS
        .iter()
        .chain(FORBIDDEN_CONTENT_MEMBERS)
    {
        assert!(
            !rendered.contains(&format!("\"{word}\":")),
            "re-emission produced a `{word}` member"
        );
    }
    // The generated type itself has nowhere to put one — the reason
    // the drop above is structural rather than incidental.
    let generated = std::fs::read_to_string(
        repo_root().join("crates/vibe-wire/src/generated/requirements_report/mod.rs"),
    )
    .unwrap();
    assert!(!generated.contains("pub body:"));
}

/// …and no AUTHORED byte carries one either: the corpora are the
/// golden documents a wire-diff judges against, so a canary sneaking
/// into one would make the fence above vacuous.
#[test]
fn no_authored_requirements_byte_carries_a_source_body() {
    let mut checked = 0;
    for entry in std::fs::read_dir(corpus_dir()).expect("the corpus home is readable") {
        let path = entry.expect("a corpus entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read_to_string(&path).expect("a corpus file is readable");
        assert!(
            !bytes.contains(SOURCE_BODY_CANARY),
            "{name} carries the source-body canary"
        );
        let rendered = serde_json::to_string(&corpus(&name)).unwrap();
        for word in FORBIDDEN_CONTENT_MEMBERS {
            assert!(
                !rendered.contains(&format!("\"{word}\":")),
                "{name} emits a `{word}` member"
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked, 4,
        "base, relations, partial and truncated corpora are all checked"
    );
}
