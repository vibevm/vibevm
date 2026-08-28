//! The vocabulary/back-edge fence for the external reference process — split
//! out of `pdsa_reference.rs` for the 600-line file budget, and nothing else.
//!
//! The textual half is deliberately narrow: `pdsa` is not an English word, and
//! `Study`/`Act` are checked only in enum-variant position. `Plan` and `Do` are
//! NOT grepped — `install_plan`, `lifecycle_plan`, the PROP-043
//! `FactStatusState::Plan` and ordinary prose all spell them legitimately, so a
//! grep for them would be a false-positive machine rather than a fence. What a
//! new phase, verdict or back-edge would actually have to enter is a CLOSED
//! vocabulary, so those are pinned exhaustively instead: the `match`es below
//! stop compiling when a variant is added, and the four words are proven
//! unparseable at every closed vocabulary the process touches.
//!
//! The runtime half of "no back-edge" lives in the two scenario cells: each
//! counts its external invocations and requires exactly two.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use vibe_core::lifecycle::{DEFAULT_PHASES, Phase};
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;
use vibe_wire::generated::lifecycle_tasks::LifecycleTasksStatus;
use vibe_wire::generated::shared::EvidenceStatus;

/// No PDSA vocabulary and no widened lifecycle vocabulary entered the product.
#[test]
fn pdsa_the_product_never_grew_the_vocabulary() {
    const FORBIDDEN: [&str; 4] = ["plan", "do", "study", "act"];

    let sources = production_sources();
    assert!(
        sources.len() > 100,
        "the fence found the product: {}",
        sources.len()
    );
    for (path, body) in &sources {
        assert!(
            !body.to_ascii_lowercase().contains("pdsa"),
            "the product must not learn the word: {}",
            path.display()
        );
        for line in body.lines() {
            let trimmed = line.trim();
            for word in ["Study", "Act"] {
                assert!(
                    trimmed != format!("{word},")
                        && !trimmed.starts_with(&format!("{word}("))
                        && !trimmed.starts_with(&format!("{word} {{")),
                    "no such variant may exist: {} — {line}",
                    path.display()
                );
            }
        }
    }

    // The phase vocabulary is exactly nine words, in exactly this order.
    assert_eq!(
        DEFAULT_PHASES.map(Phase::as_str),
        [
            "validate", "install", "generate", "build", "test", "create", "verify", "package",
            "deploy",
        ]
    );
    for word in FORBIDDEN {
        assert!(
            word.parse::<Phase>().is_err(),
            "`{word}` is not a lifecycle phase"
        );
        assert!(
            serde_json::from_value::<EvidenceStatus>(json!(word)).is_err(),
            "`{word}` is not an evidence outcome"
        );
        assert!(
            serde_json::from_value::<ExecutionRecordStatus>(json!(word)).is_err(),
            "`{word}` is not an execution record status"
        );
        assert!(
            serde_json::from_value::<LifecycleTasksStatus>(json!(word)).is_err(),
            "`{word}` is not a mailbox status"
        );
    }

    // Exhaustive by construction: a widened vocabulary fails to compile here
    // before it can ever fail a runtime assertion.
    fn evidence(status: &EvidenceStatus) -> &'static str {
        match status {
            EvidenceStatus::Matched => "matched",
            EvidenceStatus::Missing => "missing",
            EvidenceStatus::Stale => "stale",
            EvidenceStatus::Unavailable => "unavailable",
            EvidenceStatus::Unstable => "unstable",
        }
    }
    fn execution(status: &ExecutionRecordStatus) -> &'static str {
        match status {
            ExecutionRecordStatus::Delegated => "delegated",
            ExecutionRecordStatus::Fail => "fail",
            ExecutionRecordStatus::Fresh => "fresh",
            ExecutionRecordStatus::Ok => "ok",
            ExecutionRecordStatus::Skip => "skip",
        }
    }
    fn mailbox(status: &LifecycleTasksStatus) -> &'static str {
        match status {
            LifecycleTasksStatus::Absent => "absent",
            LifecycleTasksStatus::Idle => "idle",
            LifecycleTasksStatus::Parked => "parked",
        }
    }
    for status in [
        EvidenceStatus::Matched,
        EvidenceStatus::Missing,
        EvidenceStatus::Stale,
        EvidenceStatus::Unavailable,
        EvidenceStatus::Unstable,
    ] {
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!(evidence(&status))
        );
    }
    for status in [
        ExecutionRecordStatus::Delegated,
        ExecutionRecordStatus::Fail,
        ExecutionRecordStatus::Fresh,
        ExecutionRecordStatus::Ok,
        ExecutionRecordStatus::Skip,
    ] {
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!(execution(&status))
        );
    }
    for status in [
        LifecycleTasksStatus::Absent,
        LifecycleTasksStatus::Idle,
        LifecycleTasksStatus::Parked,
    ] {
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!(mailbox(&status))
        );
    }
}

/// Every production Rust source: `crates/*/src/**/*.rs`. Test targets live in
/// `crates/*/tests/`, so this walk structurally cannot read the cells that call
/// it — the fence needs no exclusion list to avoid its own vocabulary.
fn production_sources() -> Vec<(PathBuf, String)> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the repository root is two levels above this crate")
        .to_path_buf();
    let mut sources = Vec::new();
    let crates = fs::read_dir(repo.join("crates")).expect("the product crates");
    for entry in crates {
        let src = entry.unwrap().path().join("src");
        if src.is_dir() {
            collect_rust(&src, &mut sources);
        }
    }
    sources
}

fn collect_rust(at: &Path, into: &mut Vec<(PathBuf, String)>) {
    for entry in fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let body = fs::read_to_string(&path).unwrap();
            into.push((path, body));
        }
    }
}
