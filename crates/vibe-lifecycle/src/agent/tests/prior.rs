//! The pre-spend physical gate sees the prior phases too.
//!
//! The portable identity key already refuses a planned row that collides with
//! an earlier phase's artifact — `Docs/A.md` against `docs/a.md`, NFC against
//! NFD, `Maße` against `MASSE`. What no key can model is an alias the *host*
//! invents: a Win32 8.3 short spelling, a Unix bind mount, a case-insensitive
//! volume mounted inside a case-sensitive one, an alias a future filesystem
//! adds. Those exist only in the OS's answer, so the OS has to be asked — and
//! asked **before** the provider is paid, not after a canonicalisation that
//! only notices once the earlier artifact has already been overwritten.
//!
//! No host can produce every alias in that class, so the identity the OS
//! reports is injected here. That is the point of the seam: the branch is
//! reachable on every machine and in every profile, and the case that proves
//! prior rows are genuinely in the comparison set turns red the moment they
//! are taken out of it.

use std::fs;

use vibe_wire::generated::lifecycle::e1::context::Artifact;

use super::super::AgentError;
use super::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, context, row};
use super::tree;

/// One artifact recorded by an earlier phase of this same run.
fn prior(root: &str, relative: &str) -> Artifact {
    Artifact {
        id: relative.to_string(),
        kind: "file".into(),
        path: format!("{root}/{relative}"),
        phase: "build".into(),
    }
}

/// The declared output and the prior artifact share **nothing** lexically, so
/// the free key passes them; the host reports one identity, so the OS refuses
/// them. Zero provider calls, zero mutation, and the earlier artifact's bytes
/// are still exactly what the earlier phase wrote.
#[test]
fn an_os_alias_to_a_prior_artifact_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/earlier.md"), "phase one").unwrap();
    fs::write(project.path().join("docs/guide.md"), "stale").unwrap();

    let row = row(TWO_OUTPUTS, PROMPT);
    let mut context = context(project.path(), &row);
    context
        .artifacts
        .push(prior(&context.project.root, "docs/earlier.md"));

    vibe_safefs::arm_identity_alias(Some(Box::new(|relative| {
        matches!(relative, "docs/earlier.md" | "docs/guide.md").then_some(1)
    })));
    let outcome = super::prepare(&backend, &row, &context);
    vibe_safefs::arm_identity_alias(None);

    let error = outcome.expect_err("an alias to a prior artifact refuses");
    assert!(
        matches!(error, AgentError::Preflight { .. }),
        "expected a physical refusal, got {error}"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("an earlier phase already produced")
            && rendered.contains("docs/earlier.md"),
        "the refusal must name the artifact at risk: {rendered}"
    );
    assert_eq!(backend.calls(), 0, "refused before any spend");
    assert_eq!(
        fs::read_to_string(project.path().join("docs/earlier.md")).unwrap(),
        "phase one",
        "and before any mutation",
    );
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "stale",
    );
}

/// The discrimination: the same run with the same injected alias, minus the
/// prior row, prepares normally. This is the case that goes red if prior
/// artifacts are ever dropped from the comparison set — the alias alone is not
/// what refuses, the alias *to a recorded artifact* is.
#[test]
fn the_same_alias_without_the_prior_row_does_not_refuse() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/earlier.md"), "phase one").unwrap();
    fs::write(project.path().join("docs/guide.md"), "stale").unwrap();

    let row = row(TWO_OUTPUTS, PROMPT);
    let context = context(project.path(), &row);
    assert!(
        context.artifacts.is_empty(),
        "the only difference from the case above",
    );

    vibe_safefs::arm_identity_alias(Some(Box::new(|relative| {
        matches!(relative, "docs/earlier.md" | "docs/guide.md").then_some(1)
    })));
    let outcome = super::prepare(&backend, &row, &context);
    vibe_safefs::arm_identity_alias(None);

    assert!(
        outcome.is_ok(),
        "with nothing recorded there is nothing to protect: {:?}",
        outcome.err().map(|error| error.to_string()),
    );
}

/// A prior row that is not below this project is **skipped**, never opened.
/// Reaching outside the project through the project capability is the one
/// thing the capability exists to prevent, and a recorded row is
/// handler-supplied text like any other.
#[test]
fn a_prior_artifact_outside_the_project_is_skipped_not_opened() {
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("elsewhere.md"), "not ours").unwrap();
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let project = tempfile::tempdir().unwrap();

    let row = row(TWO_OUTPUTS, PROMPT);
    let mut context = context(project.path(), &row);
    context.artifacts.push(Artifact {
        id: "elsewhere".into(),
        kind: "file".into(),
        path: format!(
            "{}/elsewhere.md",
            vibe_core::machine_json_path(outside.path())
        ),
        phase: "build".into(),
    });

    super::prepare(&backend, &row, &context).expect("an outside row is simply not our business");
    assert_eq!(
        fs::read_to_string(outside.path().join("elsewhere.md")).unwrap(),
        "not ours",
    );
}

/// A prior row that claims to be inside this project but is malformed refuses,
/// rather than being quietly skipped. An artifact we cannot locate is one we
/// cannot prove a declared output is not about to destroy — and the planned
/// rows are judged by exactly these rules, so the prior rows are too.
#[test]
fn a_malformed_prior_artifact_inside_the_project_refuses() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for (label, relative) in [
        ("dot-dot", "docs/../../escape.md"),
        ("empty", ""),
        ("current", "."),
    ] {
        let project = tempfile::tempdir().unwrap();
        let row = row(TWO_OUTPUTS, PROMPT);
        let mut context = context(project.path(), &row);
        let path = format!("{}/{relative}", context.project.root);
        context.artifacts.push(Artifact {
            id: "earlier".into(),
            kind: "file".into(),
            path,
            phase: "build".into(),
        });

        let error = super::prepare(&backend, &row, &context)
            .expect_err("a malformed recorded artifact refuses");
        assert!(
            matches!(error, AgentError::Preflight { .. }),
            "`{label}` expected a physical refusal, got {error}"
        );
        assert!(
            error.to_string().contains("cannot be checked against"),
            "`{label}`: {error}"
        );
        assert!(tree(project.path()).is_empty(), "`{label}` wrote something");
    }
    assert_eq!(backend.calls(), 0, "no malformed row reached a provider");
}

/// The complete classification, as one table, because the interesting property
/// is the **boundary** between the answers and not any single row.
///
/// Two of these five would have been silently waived by a prefix test alone:
///
/// - a **raw relative** spelling shares no prefix with the root, so a prefix
///   test calls it "outside" and skips it. It is not outside; it is
///   unlocatable, and the file it means may be exactly the one a declared
///   output is about to overwrite;
/// - the **root itself** strips to an empty remainder, which the same test also
///   reads as "outside". It names no file below the project at all.
///
/// Both must refuse. Only a well-formed absolute path that genuinely lives
/// elsewhere may be skipped.
#[test]
fn every_prior_path_class_is_answered_distinctly() {
    use crate::artifacts::eligible_relative;

    let root = if cfg!(windows) {
        "C:/abs/project"
    } else {
        "/abs/project"
    };
    let elsewhere = if cfg!(windows) {
        "C:/abs/other/note.md"
    } else {
        "/abs/other/note.md"
    };
    let sibling = format!("{root}-two/note.md");

    // 1. Valid, inside: the only class the OS is asked about.
    assert_eq!(
        eligible_relative("a", &format!("{root}/docs/guide.md"), root),
        Ok(Some("docs/guide.md")),
    );

    // 2. Truly outside, absolute and well-formed: skipped, never opened.
    assert_eq!(eligible_relative("a", elsewhere, root), Ok(None));
    // A sibling whose name merely starts with the root's is outside too.
    assert_eq!(eligible_relative("a", &sibling, root), Ok(None));

    // 3. Raw relative: malformed, NOT "outside".
    for relative in ["docs/guide.md", "guide.md", "./guide.md", "../guide.md"] {
        let outcome = eligible_relative("a", relative, root);
        assert!(
            outcome
                .as_ref()
                .is_err_and(|reason| reason.contains("has a relative path")),
            "`{relative}` must refuse as malformed, not skip as outside: {outcome:?}",
        );
    }

    // 4. The root exactly, with and without a trailing separator: names no
    //    output file, so it is not a row this law can judge.
    for exact in [root.to_string(), format!("{root}/")] {
        let outcome = eligible_relative("a", &exact, root);
        assert!(
            outcome
                .as_ref()
                .is_err_and(|reason| reason.contains("names the selected project root itself")),
            "`{exact}` must refuse: {outcome:?}",
        );
    }

    // 5. Malformed inside.
    for (label, path) in [
        ("dot-dot", format!("{root}/docs/../../escape.md")),
        ("current", format!("{root}/.")),
        ("backslash", format!("{root}/docs\\guide.md")),
    ] {
        assert!(
            eligible_relative("a", &path, root).is_err(),
            "`{label}` must refuse",
        );
    }
}

/// The same classification through the whole handler: a relative prior row and
/// a root-exact prior row each refuse before a token is spent, and neither is
/// waived as "somewhere else".
#[test]
fn an_unlocatable_prior_artifact_refuses_before_the_provider_call() {
    let backend = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    for (label, path, expected) in [
        ("raw relative", "docs/earlier.md", "has a relative path"),
        ("bare name", "earlier.md", "has a relative path"),
        ("dot relative", "./earlier.md", "has a relative path"),
        ("root exact", "", "names the selected project root itself"),
        ("root slash", "/", "names the selected project root itself"),
    ] {
        let project = tempfile::tempdir().unwrap();
        fs::create_dir_all(project.path().join("docs")).unwrap();
        fs::write(project.path().join("docs/earlier.md"), "phase one").unwrap();

        let row = row(TWO_OUTPUTS, PROMPT);
        let mut context = context(project.path(), &row);
        // `root exact` and `root slash` spell the root itself; the rest are
        // stored exactly as given, with no root prefix at all.
        let stored = if path.is_empty() || path == "/" {
            format!("{}{path}", context.project.root)
        } else {
            path.to_string()
        };
        context.artifacts.push(Artifact {
            id: "earlier".into(),
            kind: "file".into(),
            path: stored,
            phase: "build".into(),
        });

        let error = super::prepare(&backend, &row, &context)
            .expect_err("an unlocatable recorded artifact refuses");
        assert!(
            matches!(error, AgentError::Preflight { .. }),
            "`{label}` expected a physical refusal, got {error}"
        );
        let rendered = error.to_string();
        assert!(
            rendered.contains("cannot be checked against") && rendered.contains(expected),
            "`{label}`: {rendered}"
        );
        assert_eq!(
            tree(project.path()),
            ["docs/earlier.md"],
            "`{label}` wrote something beyond the seeded artifact",
        );
        assert_eq!(
            fs::read_to_string(project.path().join("docs/earlier.md")).unwrap(),
            "phase one",
            "`{label}` mutated the recorded artifact",
        );
    }
    assert_eq!(backend.calls(), 0, "no unlocatable row reached a provider");
}
