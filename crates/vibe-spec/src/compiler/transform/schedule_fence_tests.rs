//! The per-cell fence CLAIMS: which surfaces each transform cell admits, and
//! the mutation fixtures proving the classifier really sees every spelling a
//! future edit could introduce.
//!
//! The rule families and the AST classifier they run on live in
//! [`super::fence_families`]; this cell only applies them. The module-tree
//! claim — that every production cell is declared AND classified under some
//! family — is `transform_cells_fence_tests`, because it makes a different
//! statement: not "this cell obeys its family" but "this cell HAS one".

use super::fence_families::{
    MINIFY_RULES, PLAN_CARRIER_RULES, SELECTOR_RULES, WRAPPER_RULES, offenders,
};

/// The wrapper cell admits exactly its mandated surfaces.
#[test]
fn the_schedule_cell_renders_names_and_holds_one_behavior_channel() {
    let found = offenders(include_str!("schedule.rs"), &WRAPPER_RULES);
    assert!(
        found.is_empty(),
        "schedule.rs is fenced: {found:#?} — the wrapper cell renders pass names and holds Arc<dyn …>, nothing else"
    );
}

/// The selector-admission cell admits exactly its two mandated surfaces.
#[test]
fn the_selector_admission_cell_touches_the_kernel_selector_and_nothing_else() {
    let found = offenders(include_str!("selector_admission.rs"), &SELECTOR_RULES);
    assert!(
        found.is_empty(),
        "selector_admission.rs is fenced: {found:#?} — it evaluates the kernel selector and holds no behavior channel"
    );
    // And the permission is genuinely narrow: the SAME source refuses under
    // the wrapper family, so the exception is a named cell's, not a general
    // loosening of the wrapper law.
    assert!(
        !offenders(include_str!("selector_admission.rs"), &WRAPPER_RULES).is_empty(),
        "the admission cell must be exactly what the wrapper cell may not be"
    );
}

/// The R4.2 binding cell admits exactly its one mandated surface — the EMIT
/// cell's framing — and the permission is narrow in both directions.
///
/// The second half is the load-bearing one. `vibe_specdoc`'s generated-comment
/// codec is banned HERE even though the segmenter must recognise a
/// codec-encoded marker, because the codec is the emit cell's to spell: the
/// binding asks `framing::hoisted_origin_in_comment` instead. The header cell,
/// which IS allowed to name the codec, therefore refuses under this family —
/// which is what makes "one framing grammar, one codec call site" a fence
/// rather than a comment.
#[test]
fn the_minify_binding_cell_reads_the_emit_framing_and_nothing_else() {
    let found = offenders(include_str!("xml_minify_binding.rs"), &MINIFY_RULES);
    assert!(
        found.is_empty(),
        "xml_minify_binding.rs is fenced: {found:#?} — it reads the emit cell's framing and holds no behavior channel"
    );
    assert!(
        !offenders(include_str!("header.rs"), &MINIFY_RULES).is_empty(),
        "the codec permission belongs to the header cell alone: its own source \
         must refuse under the binding's family"
    );
    for source in [
        "use vibe_specdoc::decode_generated_xml_comment;",
        "use vibe_specdoc::encode_generated_xml_comment as encode;",
        "fn f(c: &str) { vibe_specdoc::decode_generated_xml_comment(c).ok(); }",
    ] {
        assert!(
            !offenders(source, &MINIFY_RULES).is_empty(),
            "a second codec call site refuses under every spelling: `{source}`"
        );
    }
}

/// The two families disagree in both directions, on purpose: each admits
/// exactly what the other forbids, and they agree on the surfaces neither
/// cell may ever have.
#[test]
fn the_selector_family_and_the_wrapper_family_are_genuinely_different_laws() {
    // Admitted by the selector family, refused by the wrapper family.
    for source in [
        "use vibe_extension_registry::{CompiledSelector, SelectorSubject};",
        "use vibe_extension_registry::SelectorSubject as Subject;",
        "use vibe_extension_registry::*;",
        "fn f(s: &vibe_extension_registry::SelectorSubject) {}",
        "fn f(c: &C, s: S) -> bool { c.matches(s) }",
    ] {
        assert!(
            offenders(source, &SELECTOR_RULES).is_empty(),
            "the admission cell exists to do this: `{source}`"
        );
        assert!(
            !offenders(source, &WRAPPER_RULES).is_empty(),
            "the wrapper cell's ban is unchanged: `{source}`"
        );
    }

    // Refused by the selector family, admitted by the wrapper family: the
    // admission cell is a pure decision and owns no behavior channel.
    for source in [
        "fn f(a: Arc<dyn TransformBehavior>) -> Arc<dyn TransformBehavior> { a }",
        "fn f(e: Box<VerificationError>) -> Box<VerificationError> { e }",
    ] {
        assert!(
            !offenders(source, &SELECTOR_RULES).is_empty(),
            "the admission cell holds no behavior channel: `{source}`"
        );
        assert!(
            offenders(source, &WRAPPER_RULES).is_empty(),
            "the wrapper cell keeps its two mandated surfaces: `{source}`"
        );
    }

    // Refused by BOTH: the surfaces no transform cell may ever acquire.
    for source in [
        "use vibe_extension_registry::collect_extensions;",
        "use crate::compiler::builtin::ArtifactCompileError;",
        "use std::path::{Path, PathBuf};",
        "use serde_json::Value;",
        "fn f(r: Result<u8, E>) { r.unwrap() }",
        "fn f(r: Result<u8, E>) { r.expect(\"x\") }",
        "fn f() { panic!(\"boom\"); }",
        "fn f() { todo!() }",
    ] {
        assert!(
            !offenders(source, &SELECTOR_RULES).is_empty(),
            "no transform cell may carry `{source}`"
        );
        assert!(
            !offenders(source, &WRAPPER_RULES).is_empty(),
            "no transform cell may carry `{source}`"
        );
    }
}

/// The plan cells remain behavior- and registry-free under the stronger
/// carrier rules — re-asserted here so this fence stays self-contained.
#[test]
fn the_plan_cells_stay_behavior_free_under_the_carrier_rules() {
    for (cell, source) in [
        ("plan.rs", include_str!("plan.rs")),
        ("plan_digest.rs", include_str!("plan_digest.rs")),
        ("plan_validate.rs", include_str!("plan_validate.rs")),
        ("config.rs", include_str!("config.rs")),
    ] {
        let found = offenders(source, &PLAN_CARRIER_RULES);
        assert!(
            found.is_empty(),
            "{cell} stays Arc/Box/dyn-free: {found:#?}"
        );
    }
}

/// Mutation fixtures: every banned spelling a future edit could introduce is
/// VISIBLE to the classifier — direct, GROUPED, RENAMED, GLOB and
/// fully-qualified imports alike — while the two mandated surfaces (name
/// rendering, `Arc<dyn …>`) pass the wrapper rules and refuse the plan-carrier
/// rules, proving the two families genuinely differ. Boxing a CONCRETE error
/// stays legal; `Box<dyn …>` behavior ownership never does; and the whole
/// kernel selector crate refuses under any import shape.
#[test]
fn the_wrapper_fence_detects_every_banned_spelling_and_admits_its_two_surfaces() {
    let cases: &[(&str, &str)] = &[
        (
            "use vibe_extension_registry::collect_extensions;",
            "identifier `collect_extensions`",
        ),
        // The kernel selector crate itself refuses — renamed, grouped, glob
        // and qualified alike (the unscoped-subject trap made mechanical).
        (
            "use vibe_extension_registry::SelectorSubject as Subject;",
            "identifier `vibe_extension_registry`",
        ),
        (
            "use vibe_extension_registry::{CompiledSelector, SelectorSubject};",
            "identifier `vibe_extension_registry`",
        ),
        (
            "use vibe_extension_registry::*;",
            "identifier `vibe_extension_registry`",
        ),
        (
            "fn f(s: &vibe_extension_registry::SelectorSubject) {}",
            "identifier `vibe_extension_registry`",
        ),
        (
            "fn f(s: &Subject) { s.matches(&t); }",
            "method `.matches()`",
        ),
        // The upward boundary is load-bearing: no builtin/driver spelling.
        (
            "use crate::compiler::builtin::ArtifactCompileError;",
            "identifier `builtin`",
        ),
        (
            "fn f(e: ArtifactCompileError) {}",
            "identifier `ArtifactCompileError`",
        ),
        // Manifest/collector/row/path/codec surfaces, grouped and renamed.
        (
            "use vibe_extension_registry::ExtensionRegistry;",
            "identifier `ExtensionRegistry`",
        ),
        ("use std::path::{Path, PathBuf};", "import of `std::path`"),
        ("use std::path as p;", "import of `std::path`"),
        ("use std::path::*;", "import of `std::path`"),
        (
            "fn f(x: std::path::PathBuf) {}",
            "fully-qualified `std::path`",
        ),
        ("use toml as codec;", "identifier `toml`"),
        ("use serde_json::Value;", "identifier `serde_json`"),
        (
            "fn f() { std::fs::read_to_string(\"x\").ok(); }",
            "identifier `fs`",
        ),
        // The production schedule cell never eliminates a fault by panic.
        (
            "fn f(r: Result<u8, E>) { r.unwrap() }",
            "method `.unwrap()`",
        ),
        (
            "fn f(r: Result<u8, E>) { r.expect(\"x\") }",
            "method `.expect()`",
        ),
        ("fn f() { panic!(\"boom\"); }", "macro `panic!`"),
        ("fn f() { todo!() }", "macro `todo!`"),
        // `Box<dyn …>` is not a behavior channel even inside the wrapper
        // cell, while boxing a CONCRETE error type stays legal there.
        (
            "fn f(a: Box<dyn TransformBehavior>) {}",
            "boxed trait object (`Box<dyn …>`)",
        ),
    ];
    for (source, expected) in cases {
        let found = offenders(source, &WRAPPER_RULES);
        assert!(
            found.iter().any(|finding| finding.contains(expected)),
            "fixture `{source}` must report {expected:?}, got {found:?}"
        );
    }

    // The classifier DISTINGUISHES the two Box spellings: a boxed concrete
    // error passes the wrapper rules, the boxed trait object refuses.
    let boxed_concrete = "fn f(e: Box<VerificationError>) -> Box<VerificationError> { e }";
    assert!(offenders(boxed_concrete, &WRAPPER_RULES).is_empty());
    assert!(!offenders("fn f(a: Box<dyn TransformBehavior>) {}", &WRAPPER_RULES).is_empty());

    // The clean fixture: BOTH mandated surfaces of the wrapper cell — the
    // one `Arc<dyn …>` channel and the rendered pass name — stay legal here.
    let clean = concat!(
        "use std::sync::Arc;\n",
        "fn name(stage: &str, key: &str) -> String { format!(\"transform:{stage}:{key}\") }\n",
        "fn f(a: Arc<dyn TransformBehavior>) -> Arc<dyn TransformBehavior> { a }\n",
    );
    assert!(
        offenders(clean, &WRAPPER_RULES).is_empty(),
        "name rendering and Arc<dyn …> are the wrapper cell's two mandated surfaces"
    );
    assert!(
        !offenders(clean, &PLAN_CARRIER_RULES).is_empty(),
        "the same fixture must refuse under the plan-carrier rules"
    );

    // Prose immunity: every needle in comments and string literals is
    // invisible to the AST fence.
    let prose = concat!(
        "// serde toml PathBuf fs collect_extensions ExtensionRegistry Box SelectorSubject matches\n",
        "// vibe_extension_registry builtin ArtifactCompileError unwrap expect panic todo\n",
        "const NEEDLES: &str = \"collect_extensions SelectorSubject matches std::path Box\n",
        "vibe_extension_registry builtin ArtifactCompileError unwrap expect panic todo\";\n",
        "fn f() { let _ = NEEDLES; }\n",
    );
    assert!(offenders(prose, &WRAPPER_RULES).is_empty());
}
