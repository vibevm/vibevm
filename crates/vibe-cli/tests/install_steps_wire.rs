//! Wire-parity oracle for the `install-steps` format
//! (`schemas/install_steps.jtd.json` →
//! `vibe_wire::generated::install_steps`, corpus
//! `formats/corpora/install-steps/e1/`).
//!
//! Same law as this repository's other wire-parity oracles: the writer and
//! the generated reader are meant to meet on ONE wire. Here the authored
//! corpus carries the documents a reader has to survive — a row that ran
//! with its log, a row a condition skipped with its reason and no start,
//! and a failed row whose reason names the retry and whose log hit the
//! capture bound — and three halves prove the meeting:
//!
//! 1. every corpus document round-trips through the generated reader with
//!    nothing lost, so a member the schema forgot is caught here;
//! 2. a required member with the wrong shape is refused rather than
//!    coerced;
//! 3. an unknown member is read and ignored, because the registry rules
//!    this format `foreign_parsers = "many"` and a client's parser may be
//!    older than the build that wrote the file.
//!
//! The WRITER's half is `cli_install_slot_steps.rs`, which reads back the
//! record a real install produced: its timestamps make byte-equality with
//! a committed corpus impossible, so the authored document pins the shape
//! and the install pins the content.

use std::path::PathBuf;

use serde_json::Value;
use vibe_wire::generated::install_steps::{InstallSteps, Outcome};

/// The corpus home the format registry names for `install-steps`.
fn corpus(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/install-steps/e1")
        .join(name)
}

fn document(name: &str) -> String {
    std::fs::read_to_string(corpus(name))
        .unwrap_or_else(|error| panic!("corpus `{name}` is readable: {error}"))
        .replace("\r\n", "\n")
}

/// Nothing is lost on the way through, and the three rows say what they
/// are meant to say.
#[test]
fn the_corpus_survives_the_generated_reader() {
    let text = document("steps.json");
    let authored: Value = serde_json::from_str(&text).expect("the corpus is JSON");
    let typed: InstallSteps = serde_json::from_str(&text).expect("the corpus is a record");
    let back = serde_json::to_value(&typed).expect("the record serialises");
    assert_eq!(
        authored, back,
        "the record lost something on the way through"
    );

    assert_eq!(typed.protocol, "vibe-install-steps/1");
    assert_eq!(typed.contributions.len(), 3);
    let ran = &typed.contributions[0];
    assert_eq!(ran.outcome, Outcome::Ok);
    assert_eq!(ran.log.as_deref(), Some("logs/build.log"));
    let skipped = &typed.contributions[1];
    assert_eq!(skipped.outcome, Outcome::Skip);
    assert!(
        skipped.started.is_none(),
        "a skip is decided, never started",
    );
    assert!(
        skipped.reason.is_some(),
        "a skip never says `skip` without saying why",
    );
    let failed = &typed.contributions[2];
    assert_eq!(failed.outcome, Outcome::Fail);
    assert_eq!(failed.log_truncated, Some(true));
    assert!(
        failed
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("vibe reinstall org.example/hooked")),
        "a failure's reason carries the retry the law requires: {failed:#?}",
    );
}

/// A required member with the wrong shape fails at the reader instead of
/// being coerced into something plausible.
#[test]
fn a_broken_shape_is_refused_rather_than_coerced() {
    let mut value: Value = serde_json::from_str(&document("steps.json")).expect("JSON");
    value["contributions"][0]["id"] = Value::from(7);
    assert!(
        serde_json::from_value::<InstallSteps>(value).is_err(),
        "a numeric id must be refused",
    );
}

/// An outcome outside the closed vocabulary is refused: the whole point of
/// closing it is that a reader never has to guess what a word means.
#[test]
fn an_outcome_outside_the_closed_vocabulary_is_refused() {
    let mut value: Value = serde_json::from_str(&document("steps.json")).expect("JSON");
    value["contributions"][0]["outcome"] = Value::from("probably-fine");
    assert!(
        serde_json::from_value::<InstallSteps>(value).is_err(),
        "an unknown outcome must be refused, never defaulted",
    );
}

/// An unknown member is READ AND IGNORED: the registry rules this format
/// `foreign_parsers = "many"`, and permissiveness is that reader's forward
/// compatibility.
#[test]
fn an_unknown_member_is_tolerated_because_the_registry_says_so() {
    let mut value: Value = serde_json::from_str(&document("steps.json")).expect("JSON");
    value["invented_by_a_later_build"] = Value::from("hello");
    value["contributions"][0]["also_invented"] = Value::from(1);
    let typed: InstallSteps = serde_json::from_value(value).expect("tolerated");
    assert_eq!(typed.contributions[0].id, "build");
}
