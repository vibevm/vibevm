use super::*;
use crate::compiler::builtin::ArtifactCompileError;
use crate::compiler::ir::ArtifactInputType;

fn plan(first: &str, second: &str) -> ArtifactPlan {
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibedeps",
        vec![
            ArtifactInput::normal("org.demo/first", "boot/first.md", spec(first)).unwrap(),
            ArtifactInput::normal("org.demo/second", "boot/second.md", spec(second)).unwrap(),
        ],
    )
    .unwrap()
}

fn attributed(error: ArtifactCompileError, index: usize) -> CompileError {
    let ArtifactCompileError::Input { input, source } = error else {
        panic!("expected attributed error, got {error:?}")
    };
    assert_eq!(input.index, index);
    assert_eq!(input.kind, ArtifactInputType::Normal);
    let ArtifactCompileError::Compile(error) = *source else {
        panic!("expected concrete CompileError source")
    };
    error
}

#[test]
fn unresolved_use_keeps_first_normal_plan_order_and_typed_source() {
    let first = "spec://org.demo/first/boot/entry#root";
    let second = "spec://org.demo/second/boot/entry#root";
    let source = CountingSource::with(&[
        (
            first,
            "# First {#root}\n#use spec://org.missing/z/boot/x#root\n",
        ),
        (
            second,
            "# Second {#root}\n#use spec://org.missing/a/boot/x#root\n",
        ),
    ]);
    let error = attributed(
        compile_artifact(plan(first, second), &source).unwrap_err(),
        0,
    );
    assert!(matches!(error, CompileError::UseGraph(_)));
    assert_eq!(
        error.to_string(),
        "cannot resolve use spec://org.missing/z/boot/x#root: missing spec://org.missing/z/boot/x#root"
    );
}

#[test]
fn unresolved_source_is_attributed_to_the_second_normal_root() {
    let first = "spec://org.demo/first/boot/entry#root";
    let second = "spec://org.demo/second/boot/entry#root";
    let source = CountingSource::with(&[
        (first, "# First {#root}\n"),
        (
            second,
            "# Second {#root}\n#source spec://org.missing/source/impl/doc#root\n",
        ),
    ]);
    let error = attributed(
        compile_artifact(plan(first, second), &source).unwrap_err(),
        1,
    );
    assert!(matches!(error, CompileError::Unresolved { .. }));
    assert_eq!(
        error.to_string(),
        "cannot load spec://org.missing/source/impl/doc#root: missing spec://org.missing/source/impl/doc#root"
    );
}

#[test]
fn unresolved_embed_is_attributed_to_the_second_normal_root() {
    let first = "spec://org.demo/first/boot/entry#root";
    let second = "spec://org.demo/second/boot/entry#root";
    let source = CountingSource::with(&[
        (first, "# First {#root}\n"),
        (
            second,
            "# Second {#root}\n#embed spec://org.missing/embed/boot/doc#root\n",
        ),
    ]);
    let error = attributed(
        compile_artifact(plan(first, second), &source).unwrap_err(),
        1,
    );
    assert!(matches!(error, CompileError::Embed(_)));
    assert_eq!(
        error.to_string(),
        "cannot resolve embed spec://org.missing/embed/boot/doc#root: missing spec://org.missing/embed/boot/doc#root"
    );
}

#[test]
fn ambiguous_link_keeps_root_attribution_and_sorted_candidates() {
    let first = "spec://org.demo/first/boot/entry#root";
    let second = "spec://org.demo/second/boot/entry#root";
    let source = CountingSource::with(&[
        (
            first,
            "# First {#root}\nSee (#SHARED).\n#use spec://org.z/z/boot/z#root\n#use spec://org.b/b/boot/b#root\n",
        ),
        (second, "# Second {#root}\n"),
        (
            "spec://org.z/z/boot/z#root",
            "# Z {#root}\n##SHARED z rule\n",
        ),
        (
            "spec://org.b/b/boot/b#root",
            "# B {#root}\n##SHARED b rule\n",
        ),
    ]);
    let error = attributed(
        compile_artifact(plan(first, second), &source).unwrap_err(),
        0,
    );
    assert!(matches!(
        error,
        CompileError::AmbiguousShortLink { ref candidates, .. }
            if candidates == &[
                "org-b--b--SHARED (org.b/b)".to_string(),
                "org-z--z--SHARED (org.z/z)".to_string(),
            ]
    ));
    assert_eq!(
        error.to_string(),
        "ambiguous short link `SHARED`: defined by org-b--b--SHARED (org.b/b), org-z--z--SHARED (org.z/z)"
    );
}
