//! The agent handler's red matrix.
//!
//! Every guard is proven by a counterexample, and the counterexamples share
//! one shape: the fake backend counts its paid calls, so "refused before
//! spend" is an assertion (`calls() == 0`) rather than a claim, and "refused
//! before any write" is proven by reading the project tree back.

#[cfg(test)]
mod contract;
#[cfg(test)]
mod plan;
#[cfg(test)]
mod prior;
#[cfg(test)]
mod resolver;
#[cfg(test)]
pub(crate) mod support;

use std::fs;

use support::{
    PROMPT, PROVIDER_ROOT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, context, row,
};

use super::{AgentBackend, AgentError, NoAgentBackend, PreparedAgent, execute, prepare};

type Reply = vibe_wire::generated::lifecycle::e1::reply::Reply;

const KEY: &str = "org.demo/tools#produce";

/// The exact two-step the runner performs: credential-free preparation first
/// (whose bytes the fingerprint binds), then the paid half with those same
/// bytes. No test resolves twice, because the product never does.
fn prepared_run(
    project: &std::path::Path,
    config_toml: &str,
    prompt: &str,
    backend: &dyn AgentBackend,
) -> Result<(PreparedAgent, Reply), AgentError> {
    let row = row(config_toml, prompt);
    let context = context(project, &row);
    let prepared = prepare(backend, &row, &context)?.expect("an agent row prepares");
    let reply = execute(backend, KEY, &context, &prepared)?;
    Ok((prepared, reply))
}

fn run(
    config_toml: &str,
    prompt: &str,
    backend: &RecordingBackend,
) -> (tempfile::TempDir, Result<Reply, AgentError>) {
    let project = tempfile::tempdir().unwrap();
    let reply = prepared_run(project.path(), config_toml, prompt, backend).map(|(_, reply)| reply);
    (project, reply)
}

fn tree(root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if entry.file_type().is_file() {
            found.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    found.sort();
    found
}

#[test]
fn a_complete_multi_output_result_is_validated_then_written_in_order() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    let reply = reply.expect("the happy path completes");

    assert_eq!(backend.calls(), 1, "exactly one paid call per execution");
    assert_eq!(
        reply
            .artifacts
            .iter()
            .map(|artifact| artifact.id.as_str())
            .collect::<Vec<_>>(),
        ["docs/guide.md", "docs/reference.md"],
        "artifacts follow declaration order",
    );
    assert!(
        reply
            .artifacts
            .iter()
            .all(|artifact| artifact.kind == "file")
    );
    assert!(reply.tasks.is_empty(), "CLI mode delegates nothing");
    assert_eq!(tree(project.path()), ["docs/guide.md", "docs/reference.md"]);
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "# Guide\n"
    );

    let message = reply.message.expect("the reply narrates the run");
    assert!(
        message.contains("the set was not one transaction"),
        "the message must not imply cross-file atomicity: {message}"
    );
    assert!(
        message.contains("usage prompt=11 completion=7 total=18"),
        "provider-independent counters are reported: {message}"
    );
}

#[test]
fn the_prompt_request_pins_the_executing_provider_instance() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (_project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    reply.expect("the happy path completes");
    let resolved = backend.resolved.lock().unwrap();
    assert_eq!(resolved.len(), 1, "resolved exactly once, never twice");
    assert_eq!(resolved[0].address, PROMPT);
    assert_eq!(resolved[0].provider_group, "org.demo");
    assert_eq!(resolved[0].provider_name, "tools");
    assert_eq!(
        resolved[0].provider_root,
        std::path::PathBuf::from(PROVIDER_ROOT),
        "the selected slot itself, carried by the registry row — not a coordinate the \
         resolver would then re-search and answer with the freshest installed version",
    );
}

/// The prompt BYTES, not the address, are what the fingerprint binds, so an
/// edited prompt document produces different material and the next run is a
/// real run. This is the counterexample a host-prompt edit needs.
#[test]
fn edited_prompt_bytes_change_the_fingerprint_material() {
    let project = tempfile::tempdir().unwrap();
    let before = RecordingBackend::answering_prompt("Write the guide.", TWO_OUTPUTS_RESULT);
    let after = RecordingBackend::answering_prompt("Write the guide, twice.", TWO_OUTPUTS_RESULT);
    let (first, _) = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &before).unwrap();
    let (second, _) = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &after).unwrap();
    assert_eq!(
        first.fingerprint_material().0,
        second.fingerprint_material().0,
        "the address alone is unchanged, which is exactly why it cannot be the material",
    );
    assert_ne!(
        first.fingerprint_material().1,
        second.fingerprint_material().1,
        "the resolved bytes must differ, or an edited prompt would fresh-skip",
    );
}

/// The freshness probe is credential-free and is not satisfied by the record
/// alone: the declared outputs must still be there, and still acceptable.
#[test]
fn the_freshness_probe_refuses_deleted_emptied_and_mismatched_outputs() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    let (prepared, _) = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend).unwrap();
    let root = vibe_core::machine_json_path(project.path());
    let recorded = prepared.contract().planned_state_rows(&root);
    let probe = |rows: &[vibe_wire::generated::lifecycle_state::StateArtifact]| {
        super::probe_outputs(project.path(), prepared.contract(), rows)
    };
    assert!(probe(&recorded), "an intact result is fresh");

    fs::write(project.path().join("docs/guide.md"), "").unwrap();
    assert!(!probe(&recorded), "an emptied output is not fresh");

    fs::write(project.path().join("docs/guide.md"), "restored").unwrap();
    assert!(probe(&recorded));
    fs::remove_file(project.path().join("docs/reference.md")).unwrap();
    assert!(!probe(&recorded), "a deleted output is not fresh");
    fs::write(project.path().join("docs/reference.md"), "back").unwrap();
    assert!(probe(&recorded));

    // Row-level tampering: every field of every row, plus the set itself.
    // Comparing ids alone would let each of these hydrate the envelope with an
    // artifact this run never produced.
    let mutate = |mutation: fn(&mut Vec<vibe_wire::generated::lifecycle_state::StateArtifact>)| {
        let mut rows = recorded.clone();
        mutation(&mut rows);
        rows
    };
    for (label, rows) in [
        (
            "path repointed outside the project",
            mutate(|rows| rows[0].path = "/elsewhere/guide.md".into()),
        ),
        (
            "path repointed inside the project",
            mutate(|rows| {
                rows[0].path = format!(
                    "{}/docs/other.md",
                    vibe_core::machine_json_path(std::path::Path::new("")).trim_end_matches('/')
                )
            }),
        ),
        (
            "kind tampered",
            mutate(|rows| rows[0].kind = "directory".into()),
        ),
        (
            "id tampered",
            mutate(|rows| rows[0].id = "docs/other.md".into()),
        ),
        ("order swapped", mutate(|rows| rows.swap(0, 1))),
        (
            "row missing",
            mutate(|rows| {
                rows.pop();
            }),
        ),
        (
            "extra row",
            mutate(|rows| {
                let extra = rows[0].clone();
                rows.push(extra);
            }),
        ),
    ] {
        assert!(
            !probe(&rows),
            "`{label}` must force a rerun, never hydrate a forged artifact",
        );
    }
}

#[test]
fn a_prompt_naming_another_package_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (project, reply) = run(
        TWO_OUTPUTS,
        "spec://org.other/secrets/common/PROMPT-001#root",
        &backend,
    );
    assert!(matches!(reply, Err(AgentError::PromptProvider { .. })));
    assert_eq!(backend.calls(), 0, "a foreign address costs nothing");
    assert!(
        backend.resolved.lock().unwrap().is_empty(),
        "and never reaches the resolver either",
    );
    assert!(tree(project.path()).is_empty());
}

#[test]
fn an_unresolvable_prompt_refuses_before_the_provider_call() {
    let backend = RecordingBackend::refusing_prompt("document `common/PROMPT-001` not found");
    let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    assert!(matches!(reply, Err(AgentError::PromptUnresolved { .. })));
    assert_eq!(backend.calls(), 0);
    assert!(tree(project.path()).is_empty());
}

#[test]
fn a_malformed_prompt_address_refuses_before_anything_else() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for address in [
        "common/PROMPT-001#root",
        "spec://demo/common/PROMPT-001#root",
        "spec://org.demo/tools@1.0.0/common/PROMPT-001#root",
    ] {
        let (_project, reply) = run(TWO_OUTPUTS, address, &backend);
        assert!(
            matches!(reply, Err(AgentError::PromptAddress { .. })),
            "`{address}` must be refused as an address",
        );
    }
    assert_eq!(backend.calls(), 0);
}

#[test]
fn a_missing_provider_fails_with_remediation_and_never_skips() {
    let backend = RecordingBackend::refusing_provider(
        "no LLM provider is configured: configure user `[llm]` or run under an agent host",
    );
    let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    let error = reply.expect_err("a selected agent contribution is never skipped");
    assert!(matches!(error, AgentError::Provider { .. }));
    let rendered = error.to_string();
    assert!(
        rendered.contains("configure user `[llm]`") && rendered.contains("agent host"),
        "the remediation must survive to the surface: {rendered}"
    );
    assert!(tree(project.path()).is_empty());
}

#[test]
fn the_default_backend_refuses_with_the_same_remediation() {
    let project = tempfile::tempdir().unwrap();
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &NoAgentBackend)
        .expect_err("no backend is a failure, never a skip");
    let rendered = error.to_string();
    assert!(rendered.contains("configure user `[llm]`"), "{rendered}");
    assert!(tree(project.path()).is_empty());
}

/// Each row is one mutation of the provider's answer. None of them may leave
/// a byte behind: the complete result is validated before any of it applies.
#[test]
fn a_result_that_is_not_the_exact_contract_writes_nothing() {
    for (label, text) in [
        (
            "missing row",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"}]}"#,
        ),
        (
            "extra row",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"docs/reference.md","content":"y"},{"path":"docs/extra.md","content":"z"}]}"#,
        ),
        (
            "reordered",
            r#"{"outputs":[{"path":"docs/reference.md","content":"y"},{"path":"docs/guide.md","content":"x"}]}"#,
        ),
        (
            "duplicated",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"docs/guide.md","content":"x"}]}"#,
        ),
        (
            "undeclared path",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"../escape.md","content":"y"}]}"#,
        ),
        (
            "empty content",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"docs/reference.md","content":""}]}"#,
        ),
        ("empty set", r#"{"outputs":[]}"#),
        ("not json", "Sure! Here are your files."),
        (
            "fenced",
            "```json\n{\"outputs\":[{\"path\":\"docs/guide.md\",\"content\":\"x\"}]}\n```",
        ),
        (
            "two documents",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"docs/reference.md","content":"y"}]} {"outputs":[]}"#,
        ),
        (
            "trailing prose",
            r#"{"outputs":[{"path":"docs/guide.md","content":"x"},{"path":"docs/reference.md","content":"y"}]} Hope this helps!"#,
        ),
        (
            "wrong member type",
            r#"{"outputs":[{"path":"docs/guide.md","content":["x"]},{"path":"docs/reference.md","content":"y"}]}"#,
        ),
    ] {
        let backend = RecordingBackend::answering(text);
        let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
        assert!(
            matches!(reply, Err(AgentError::Result { .. })),
            "`{label}` must be refused as a result, got {reply:?}",
        );
        assert_eq!(backend.calls(), 1, "`{label}` still paid for one call");
        assert!(
            tree(project.path()).is_empty(),
            "`{label}` mutated the project",
        );
    }
}

#[test]
fn an_ancestor_that_is_not_a_directory_refuses_without_outside_mutation() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    // `docs` is occupied by a regular file: the declared output cannot be
    // created under it, and the occupant must survive untouched.
    fs::write(project.path().join("docs"), "occupied").unwrap();
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect_err("a non-directory ancestor refuses");
    // The credential-free preflight sees it first, so this now costs nothing.
    assert!(
        matches!(error, AgentError::Preflight { .. }),
        "expected a preflight refusal, got {error}"
    );
    assert_eq!(backend.calls(), 0, "refused before spend");
    assert_eq!(
        fs::read_to_string(project.path().join("docs")).unwrap(),
        "occupied",
        "the pre-existing file was not replaced",
    );
}

/// Canary: neither a hostile provider's file bodies nor an unbounded path may
/// ride out through a refusal. Only the offending path is quoted, truncated.
#[test]
fn a_refusal_quotes_no_provider_content_and_bounds_the_echoed_path() {
    const BODY_CANARY: &str = "provider-body-canary-must-never-appear";
    // Two rows, so the row-count guard passes and the per-row path comparison
    // is the branch that refuses — the one branch that quotes provider bytes.
    let hostile = format!(
        r#"{{"outputs":[{{"path":"{}","content":"{BODY_CANARY}"}},{{"path":"docs/reference.md","content":"{BODY_CANARY}"}}]}}"#,
        "z".repeat(4096),
    );
    let backend = RecordingBackend::answering(&hostile);
    let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    let error = reply.expect_err("a hostile result is refused");
    let rendered = format!("{error} / {error:?}");
    assert!(
        !rendered.contains(BODY_CANARY),
        "provider content leaked into a diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("… (truncated)") && rendered.len() < 1024,
        "the echoed path must be bounded: {} chars",
        rendered.len(),
    );
    assert!(tree(project.path()).is_empty());
}

/// The link oracle: `docs` is a link/reparse point aimed outside the project.
/// A naive `create_dir_all` + write would follow it and mutate a directory the
/// operator never declared, so the refusal is asserted together with the
/// untouched outside tree.
fn refuses_linked_ancestor(link: impl FnOnce(&std::path::Path, &std::path::Path)) {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let outside = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    link(outside.path(), &project.path().join("docs"));
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect_err("a linked ancestor refuses");
    // The preflight walks existing ancestors no-follow, so a junction is
    // refused for free — before the provider, not at the write.
    assert!(
        matches!(error, AgentError::Preflight { .. }),
        "expected a preflight refusal, got {error}",
    );
    assert_eq!(backend.calls(), 0, "refused before spend");
    assert!(
        tree(outside.path()).is_empty(),
        "the refusal must not mutate anything outside the project",
    );
}

#[test]
#[cfg(unix)]
fn a_symlinked_ancestor_refuses_without_outside_mutation() {
    refuses_linked_ancestor(|target, link| {
        std::os::unix::fs::symlink(target, link).unwrap();
    });
}

/// A junction, not a symlink: Windows grants junction creation without the
/// Developer-Mode privilege a symlink needs, so this oracle runs on ordinary
/// hosts rather than being skipped where it matters most.
#[test]
#[cfg(windows)]
fn a_junctioned_ancestor_refuses_without_outside_mutation() {
    refuses_linked_ancestor(|target, link| {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .expect("mklink is available on Windows");
        assert!(
            status.status.success(),
            "mklink /J failed: {}",
            String::from_utf8_lossy(&status.stderr),
        );
    });
}

#[test]
fn an_existing_declared_output_is_replaced_and_re_accepted() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "stale").unwrap();
    prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect("replacing an existing declared output is ordinary");
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "# Guide\n"
    );
    assert_eq!(
        tree(project.path()),
        ["docs/guide.md", "docs/reference.md"],
        "no staging file survives the replacement",
    );
}
