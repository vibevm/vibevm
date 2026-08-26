//! The plan returns *its own* prevalidated identities — in every profile.
//!
//! The defect this closes survives a casual reading. `apply` used to take
//! `planned: &[ReplyArtifact]` from its caller and only `debug_assert` that the
//! length matched. Same-length rows belonging to a *different* contract would
//! then be accepted in release: the reply would name ids, kinds and paths for
//! files this run never wrote, and a later contribution reading that envelope
//! would treat them as produced artifacts. Length agreement is not identity
//! agreement, and the one guard that noticed compiled out of the shipped
//! binary.
//!
//! The repair is structural — the rows are bound inside `ResultPlan` at parse
//! time and `apply` has no parameter through which anything else can arrive —
//! so these cases hold the behavioural half: what comes back is the bound set,
//! asserted field by field, under an attribute that keeps the assertion alive
//! where `debug_assert` died.

use std::fs;

use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;

use super::super::AgentError;
use super::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT};
use super::{prepared_run, tree};

/// A second contract of the **same arity** whose rows differ in every field a
/// caller could have confused — exactly the shape the old signature took
/// without complaint.
const TWO_FOREIGN_OUTPUTS: &str = r#"
outputs = [
  { path = "reports/first.md", kind = "file", accept = "non-empty file" },
  { path = "reports/second.md", kind = "file", accept = "non-empty file" },
]
"#;

fn artifacts(reply: &super::Reply) -> Vec<ReplyArtifact> {
    reply.artifacts.clone()
}

/// What the run returns is the plan's own row set: the ids, kinds and paths a
/// pre-spend contract validated, matched one-for-one against the files that
/// actually exist. No field of the same-arity foreign contract can appear,
/// because no foreign row was ever bound.
#[test]
fn a_run_returns_the_identities_its_own_contract_planned() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    let (prepared, reply) =
        prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend).expect("the run succeeds");

    let returned = artifacts(&reply);
    assert_eq!(
        returned, prepared.planned,
        "the reply carries exactly the prevalidated rows the contract planned",
    );
    assert_eq!(
        tree(project.path()),
        ["docs/guide.md", "docs/reference.md"],
        "and those rows are the files on disk",
    );
    for row in &returned {
        assert_eq!(row.kind, "file");
        assert!(
            fs::read_to_string(&row.path).is_ok_and(|body| !body.is_empty()),
            "`{}` names a real non-empty file",
            row.path,
        );
        assert!(
            !row.id.starts_with("reports/") && !row.path.contains("/reports/"),
            "no foreign-contract identity may surface: `{}`",
            row.id,
        );
    }
}

/// The two contracts really are same-arity and fully disjoint, so the previous
/// case is a genuine discrimination rather than a tautology — and a run of the
/// foreign contract returns only *its* identities.
#[test]
fn a_same_arity_foreign_contract_returns_only_its_own_rows() {
    const FOREIGN_RESULT: &str = r##"{"outputs":[
  {"path":"reports/first.md","content":"# First\n"},
  {"path":"reports/second.md","content":"# Second\n"}
]}"##;
    let backend = RecordingBackend::answering(FOREIGN_RESULT);
    let project = tempfile::tempdir().unwrap();
    let (prepared, reply) = prepared_run(project.path(), TWO_FOREIGN_OUTPUTS, PROMPT, &backend)
        .expect("the run succeeds");

    let returned = artifacts(&reply);
    assert_eq!(returned.len(), 2, "same arity as the other contract");
    assert_eq!(returned, prepared.planned);
    for row in &returned {
        assert!(
            row.id.starts_with("reports/"),
            "`{}` belongs to this contract",
            row.id,
        );
    }
    assert_eq!(
        tree(project.path()),
        ["reports/first.md", "reports/second.md"],
    );
}

/// The binding must hold in the profile the old `debug_assert` did not exist
/// in. `debug_assertions` is off in a release test build, so this case is the
/// one that would have passed vacuously before and fails now if the
/// caller-supplied parameter ever returns.
#[test]
fn the_binding_holds_with_debug_assertions_compiled_out() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    let (prepared, reply) =
        prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend).expect("the run succeeds");

    // Stated, not assumed: whichever profile this compiled under, the rows are
    // compared in full rather than by length.
    let returned = artifacts(&reply);
    assert_eq!(returned.len(), prepared.planned.len());
    for (returned, planned) in returned.iter().zip(&prepared.planned) {
        assert_eq!(returned.id, planned.id);
        assert_eq!(returned.kind, planned.kind);
        assert_eq!(returned.path, planned.path);
    }
    assert!(
        !cfg!(debug_assertions) || returned == prepared.planned,
        "the full-equality assertion is the guard; a length check is not",
    );
}

/// A provider that answers with the *other* contract's paths is refused at
/// parse, before a byte is written — the failure mode the old signature let a
/// caller reproduce from the inside.
#[test]
fn a_result_naming_another_contracts_paths_is_refused_before_any_write() {
    const FOREIGN_RESULT: &str = r##"{"outputs":[
  {"path":"reports/first.md","content":"# First\n"},
  {"path":"reports/second.md","content":"# Second\n"}
]}"##;
    let backend = RecordingBackend::answering(FOREIGN_RESULT);
    let project = tempfile::tempdir().unwrap();

    let outcome = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend);

    assert!(
        matches!(outcome, Err(AgentError::Result { .. })),
        "a foreign path at a declared position is a contract break",
    );
    assert!(
        tree(project.path()).is_empty(),
        "and nothing was written before the refusal",
    );
}
