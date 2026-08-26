//! Two declared names, one physical file — caught before anything is written.

use std::fs;

use crate::Project;

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

#[test]
fn distinct_existing_files_pass_the_set_preflight() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/a.md"), "a").unwrap();
    fs::write(dir.path().join("docs/b.md"), "b").unwrap();
    project
        .preflight_set(&["docs/a.md".into(), "docs/b.md".into()])
        .expect("two distinct files are two outputs");
}

#[test]
fn missing_paths_are_legal_in_the_set_preflight() {
    let (_dir, project) = project();
    project
        .preflight_set(&["docs/a.md".into(), "docs/nested/b.md".into()])
        .expect("nothing exists yet, so nothing can collide yet");
}

/// A hard link is two names for one file. Lexically the rows differ;
/// physically the second write would destroy the first. Two independent laws
/// catch it — the per-row link-count check fires first, the set-level identity
/// check would catch it even if link counting were unavailable — so the
/// assertion is on the refusal and its reason, not on which layer got there.
#[test]
fn two_names_of_one_hard_linked_file_refuse_before_any_write() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/a.md"), "shared").unwrap();
    if fs::hard_link(dir.path().join("docs/a.md"), dir.path().join("docs/b.md")).is_err() {
        // Some filesystems refuse hard links; the case is then unreachable
        // here and covered by the case-fold oracle below.
        return;
    }
    let error = project
        .preflight_set(&["docs/a.md".into(), "docs/b.md".into()])
        .expect_err("two names of one file are not two outputs");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("the same physical file") || rendered.contains("hard link"),
        "{rendered}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("docs/a.md")).unwrap(),
        "shared",
        "the refusal wrote nothing",
    );
}

/// A hard link between a declared output and something an **earlier phase**
/// produced. Nothing in the declared set collides; the collision is only
/// visible once the prior artifact is in the comparison set.
#[test]
fn a_declared_output_hard_linked_to_a_prior_artifact_refuses() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/earlier.md"), "phase one").unwrap();
    if fs::hard_link(
        dir.path().join("docs/earlier.md"),
        dir.path().join("docs/new.md"),
    )
    .is_err()
    {
        return;
    }
    let error = project
        .preflight_set_against(&["docs/new.md".into()], &["docs/earlier.md".into()])
        .expect_err("a declared output that is a prior artifact is refused");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("an earlier phase already produced")
            || rendered.contains("hard link")
            || rendered.contains("the same physical file"),
        "{rendered}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("docs/earlier.md")).unwrap(),
        "phase one",
        "the refusal wrote nothing",
    );
}

/// The alias class the portable key **cannot** model: an 8.3 short spelling, a
/// bind mount, a case-insensitive volume mounted inside a case-sensitive one.
/// The injected identity makes two ordinary, lexically unrelated files report
/// one OS identity, so the branch is reachable on every host — which is the
/// whole point, since no single host can produce every alias this must catch.
#[test]
fn an_os_alias_the_key_cannot_model_refuses_against_a_prior_artifact() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/earlier.md"), "phase one").unwrap();
    fs::write(dir.path().join("docs/quite-different.md"), "unrelated").unwrap();

    // Lexically these share nothing: the free first gate passes them.
    assert_ne!(
        crate::path_identity_key("docs/earlier.md"),
        crate::path_identity_key("docs/quite-different.md"),
        "the portable key really does call these two different files",
    );

    crate::arm_identity_alias(Some(Box::new(|relative| {
        matches!(relative, "docs/earlier.md" | "docs/quite-different.md").then_some(7)
    })));
    let outcome = project.preflight_set_against(
        &["docs/quite-different.md".into()],
        &["docs/earlier.md".into()],
    );
    crate::arm_identity_alias(None);

    let error = outcome.expect_err("one physical file is not two artifacts");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("an earlier phase already produced")
            && rendered.contains("docs/earlier.md"),
        "the refusal must name the artifact at risk: {rendered}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("docs/earlier.md")).unwrap(),
        "phase one",
        "and nothing was written",
    );
}

/// The injection is inert outside its group, so it cannot manufacture a
/// refusal for unrelated rows — the previous case is a real discrimination.
#[test]
fn an_unaliased_prior_artifact_still_passes() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/earlier.md"), "phase one").unwrap();
    fs::write(dir.path().join("docs/quite-different.md"), "unrelated").unwrap();

    crate::arm_identity_alias(Some(Box::new(|relative| {
        (relative == "docs/somewhere-else.md").then_some(7)
    })));
    let outcome = project.preflight_set_against(
        &["docs/quite-different.md".into()],
        &["docs/earlier.md".into()],
    );
    crate::arm_identity_alias(None);

    outcome.expect("two genuinely distinct files remain two files");
}

/// A prior row is compared, never judged as a destination: an earlier phase may
/// have produced something this contract would refuse to *write*, and that is
/// not a reason to refuse the run.
#[test]
fn a_prior_artifact_is_compared_not_preflighted() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs/occupied.md")).unwrap();
    project
        .preflight_set_against(&["docs/new.md".into()], &["docs/occupied.md".into()])
        .expect("a prior row that is a directory is simply not the declared file");
}

/// On a case-folding volume the two spellings open the same file, so the OS
/// identity matches and the set refuses. On a case-sensitive one they really
/// are two files and it passes — the oracle asserts the platform's own answer
/// rather than a guess about it.
#[test]
fn case_folded_spellings_agree_with_the_filesystem() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/a.md"), "a").unwrap();
    let folds = fs::read_to_string(dir.path().join("docs/A.md")).is_ok();
    let outcome = project.preflight_set(&["docs/a.md".into(), "docs/A.md".into()]);
    assert_eq!(
        outcome.is_err(),
        folds,
        "the set preflight must agree with the volume's own case behaviour",
    );
}
