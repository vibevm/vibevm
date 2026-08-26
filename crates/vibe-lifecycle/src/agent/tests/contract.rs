//! The declared-contract half of the red matrix.
//!
//! Split from the parent so neither file outgrows the 600-line budget, and
//! because these cases share one shape: every refusal is lexical, happens at
//! preparation, and is asserted together with a paid-call counter of zero.

use std::fs;

use super::super::AgentError;
use super::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, context, row};
use super::{prepared_run, run, tree};

/// Each row is one mutation of the exact contract grammar.
#[test]
fn an_unknown_or_missing_output_contract_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for (label, config) in [
        ("absent", ""),
        ("empty", "outputs = []"),
        ("not an array", "outputs = \"docs/guide.md\""),
        (
            "unknown acceptance",
            r#"outputs = [{ path = "a.md", kind = "file", accept = "exists" }]"#,
        ),
        (
            "unknown kind",
            r#"outputs = [{ path = "a", kind = "directory", accept = "non-empty file" }]"#,
        ),
        (
            "unknown key",
            r#"outputs = [{ path = "a.md", kind = "file", accept = "non-empty file", mode = "755" }]"#,
        ),
        (
            "missing accept",
            r#"outputs = [{ path = "a.md", kind = "file" }]"#,
        ),
        (
            "duplicate path",
            r#"outputs = [
                 { path = "a.md", kind = "file", accept = "non-empty file" },
                 { path = "a.md", kind = "file", accept = "non-empty file" },
               ]"#,
        ),
    ] {
        let (project, reply) = run(config, PROMPT, &backend);
        assert!(
            matches!(reply, Err(AgentError::Contract { .. })),
            "`{label}` must be refused as a contract",
        );
        assert!(tree(project.path()).is_empty(), "`{label}` wrote something");
    }
    assert_eq!(
        backend.calls(),
        0,
        "no contract red case reached a provider"
    );
}

#[test]
fn an_unsafe_declared_path_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for path in [
        "../escape.md",
        "docs/../../escape.md",
        "/etc/passwd",
        "C:/Windows/system32/x.md",
        "docs\\guide.md",
        "./docs/guide.md",
        "docs//guide.md",
        "",
        // The shared portable law, reached through the contract rather than
        // restated beside it: an alternate data stream, a device spelling with
        // and without an extension, a superscript port, a Win32-stripped
        // trailing dot/space, and this crate's reserved staging prefix.
        "docs/guide.md:ads",
        "docs/guide.md:$DATA",
        "docs/CON",
        "docs/COM1.json",
        "docs/LPT\u{b9}.log",
        "NUL",
        "docs/trailing.",
        "docs/trailing ",
        "docs/.vibe-stage-1234-0",
    ] {
        let config = format!(
            r#"outputs = [{{ path = "{}", kind = "file", accept = "non-empty file" }}]"#,
            path.replace('\\', "\\\\"),
        );
        let (project, reply) = run(&config, PROMPT, &backend);
        assert!(
            matches!(reply, Err(AgentError::Contract { .. })),
            "`{path}` must be refused as a declared path, got {reply:?}",
        );
        assert!(tree(project.path()).is_empty());
    }
    // A control character cannot be written raw in TOML, but its escape can:
    // the refusal must come from the contract law, not from the TOML parser
    // happening to be strict about one spelling. The raw string below
    // carries the six characters of a TOML escape, which TOML decodes to
    // U+0007 before the contract ever sees it.
    let escaped_control =
        r#"outputs = [{ path = "docs/bell\u0007.md", kind = "file", accept = "non-empty file" }]"#;
    let (project, reply) = run(escaped_control, PROMPT, &backend);
    assert!(
        matches!(reply, Err(AgentError::Contract { .. })),
        "an escaped control character must still be refused, got {reply:?}",
    );
    assert!(tree(project.path()).is_empty());
    assert_eq!(backend.calls(), 0);
}

/// Two declared rows that are one physical file, or a file that must also be a
/// directory. Every one of these is refused lexically, before the paid call —
/// at write time the first row would already be on disk.
#[test]
fn output_output_physical_overlap_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for (label, rows) in [
        ("exact duplicate", &["docs/a.md", "docs/a.md"][..]),
        ("case-folded alias", &["docs/a.md", "Docs/A.MD"][..]),
        (
            "non-ascii case fold",
            &["docs/\u{c4}pfel.md", "docs/\u{e4}pfel.md"][..],
        ),
        ("prefix overlap, file first", &["docs", "docs/a.md"][..]),
        ("prefix overlap, dir first", &["docs/a.md", "docs"][..]),
        (
            "deep prefix overlap",
            &["docs/nested/a.md", "docs/nested"][..],
        ),
    ] {
        let declared = rows
            .iter()
            .map(|path| {
                format!(r#"{{ path = "{path}", kind = "file", accept = "non-empty file" }}"#)
            })
            .collect::<Vec<_>>()
            .join(",\n  ");
        let config = format!("outputs = [\n  {declared}\n]");
        let (project, reply) = run(&config, PROMPT, &backend);
        assert!(
            matches!(reply, Err(AgentError::Contract { .. })),
            "`{label}` must be refused as a contract, got {reply:?}",
        );
        assert!(tree(project.path()).is_empty(), "`{label}` wrote something");
    }
    assert_eq!(backend.calls(), 0, "no overlap red case reached a provider");
}

/// Per-file atomicity is real and the set is not a transaction, so when a
/// later row fails the rows that ARE on disk must be named. The failure is
/// injected *after* row 2's rename, which is the only way to reach the
/// possibly-published branch without racing the filesystem.
#[test]
fn a_later_failure_names_applied_possibly_applied_and_created_directories() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    vibe_safefs::fail_after_publish(Some("docs/reference.md"));
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect_err("an injected post-publication failure fails the execution");
    vibe_safefs::fail_after_publish(None);

    let rendered = error.to_string();
    assert!(
        rendered.contains("docs/guide.md ARE already applied and were not rolled back"),
        "the applied prefix must be named: {rendered}"
    );
    assert!(
        rendered.contains("docs/reference.md was already renamed into place and MAY hold"),
        "the failing row is past the rename, so it must be named as possibly applied: {rendered}"
    );
    assert!(
        rendered.contains("created the directory") && rendered.contains("docs"),
        "an empty directory this run created is observable state: {rendered}"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "# Guide\n",
        "the applied row really is on disk — the message is not a guess",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/reference.md")).unwrap(),
        "# Reference\n",
        "and the possibly-applied row really did land, which is why the claim is hedged",
    );
}

/// The same law on the FIRST row: nothing was applied, the row itself may be
/// on disk, and the directory this run created is still named. "Project
/// unchanged" would be false here even though no output was ever *verified*.
#[test]
fn a_first_row_failure_still_names_the_directory_it_created() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    vibe_safefs::fail_after_publish(Some("docs/guide.md"));
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect_err("an injected post-publication failure fails the execution");
    vibe_safefs::fail_after_publish(None);

    let rendered = error.to_string();
    assert!(
        rendered.contains("No earlier declared output was applied"),
        "{rendered}"
    );
    assert!(
        rendered.contains("docs/guide.md was already renamed into place and MAY hold"),
        "{rendered}"
    );
    assert!(
        rendered.contains("created the directory"),
        "the created directory must not be hidden behind `nothing was written`: {rendered}"
    );
    assert!(project.path().join("docs").is_dir());
}

/// A parent that cannot be created — because something else already occupies
/// the name — is caught by the credential-free preflight, so it costs nothing
/// and writes nothing.
#[test]
fn an_occupied_parent_refuses_at_preflight_with_zero_calls() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("docs"), "occupied").unwrap();
    let error = prepared_run(project.path(), TWO_OUTPUTS, PROMPT, &backend)
        .expect_err("an occupied parent refuses");
    assert!(
        matches!(error, AgentError::Preflight { .. }),
        "expected a preflight refusal, got {error}"
    );
    assert_eq!(backend.calls(), 0, "preflight runs before any spend");
    assert_eq!(
        fs::read_to_string(project.path().join("docs")).unwrap(),
        "occupied",
        "the occupant is untouched",
    );
}

/// The generic artifact law, applied to the plan before a token is spent.
/// Each row is a set the ordinary reply validator would have refused *after*
/// the provider was paid.
#[test]
fn an_invalid_artifact_plan_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let over_cap = (0..=1024)
        .map(|index| {
            format!(r#"{{ path = "docs/{index}.md", kind = "file", accept = "non-empty file" }}"#)
        })
        .collect::<Vec<_>>()
        .join(",\n  ");
    let long_name = format!("{}.md", "x".repeat(300));
    for (label, config) in [
        ("1025 outputs", format!("outputs = [\n  {over_cap}\n]")),
        (
            "over-long id/path",
            format!(
                r#"outputs = [{{ path = "docs/{long_name}", kind = "file", accept = "non-empty file" }}]"#
            ),
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        let error = prepared_run(project.path(), &config, PROMPT, &backend)
            .expect_err("an invalid plan refuses");
        assert!(
            matches!(error, AgentError::PlannedArtifacts { .. }),
            "`{label}` must be refused as a plan, got {error}"
        );
        assert!(tree(project.path()).is_empty(), "`{label}` wrote something");
    }
    assert_eq!(backend.calls(), 0, "no plan red case reached a provider");
}

/// A planned row that collides with an artifact an earlier phase already
/// produced. Only the shared generic law knows about prior artifacts, so this
/// is the case a contract-local duplicate check could never catch.
#[test]
fn a_plan_colliding_with_a_prior_phase_artifact_refuses_before_the_call() {
    use vibe_wire::generated::lifecycle::e1::context::Artifact;

    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    let row = row(TWO_OUTPUTS, PROMPT);
    let mut context = context(project.path(), &row);
    context.artifacts.push(Artifact {
        id: "docs/guide.md".into(),
        kind: "file".into(),
        path: format!("{}/docs/guide.md", context.project.root),
        phase: "build".into(),
    });
    let error = super::prepare(&backend, &row, &context).expect_err("a colliding plan refuses");
    assert!(
        matches!(error, AgentError::PlannedArtifacts { .. }),
        "expected a plan refusal, got {error}"
    );
    assert!(
        error
            .to_string()
            .contains("already produced in phase `build`"),
        "{error}"
    );
    assert_eq!(backend.calls(), 0);
    assert!(tree(project.path()).is_empty());
}

/// `#use` and `#source` are composition this handler does not perform. Calling
/// the result a full closure while dropping them would put text in front of a
/// paid model that the author believed had been assembled.
#[test]
fn unsupported_prompt_composition_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT)
        .with_unsupported(&["#use spec://org.demo/tools/common/OTHER"]);
    let (project, reply) = run(TWO_OUTPUTS, PROMPT, &backend);
    let error = reply.expect_err("unsupported composition refuses");
    assert!(
        matches!(error, AgentError::PromptComposition { .. }),
        "got {error}"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("#use spec://org.demo/tools/common/OTHER"),
        "{rendered}"
    );
    assert!(
        rendered.contains("one addressed section plus recursive `#embed` expansion"),
        "the remediation must name what IS supported: {rendered}"
    );
    assert_eq!(backend.calls(), 0, "refused before spend");
    assert!(tree(project.path()).is_empty());
}
