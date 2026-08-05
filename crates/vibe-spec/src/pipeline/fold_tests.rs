//! Tests for the `#source` fold (PROP-035 §7.3, §8 phase 3) — the single-level
//! merge (B-055), the multi-source fold (B-056-L2), and the recursive fold under
//! `source_fold_order` with its inclusion guard (B-056-L3B). Split out of
//! `tests` along the responsibility seam so neither file breaches the 600-line
//! budget; ordering / emission / qualification stay in `tests`.

use super::tests::MockSource;
use super::*;

// ---- the contract↔source merge at the top of the pipeline ------------------
//
// `#source` links a contract to its source (§7.3); the fold merges them. These
// pin the single-source degenerate case and the post-merge uniqueness gate.

#[test]
fn a_clean_fact_override_compiles_to_the_source_version() {
    // Source's `##fact-a` overrides the contract's; the merged view holds one
    // `fact-a`, so the gate passes and the source text wins.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/c#root",
            "# API {#root}\n#source spec://org.vibevm.core/vibevm/impl#root\n- ##fact-a contract version\n",
        ),
        (
            "spec://org.vibevm.core/vibevm/impl#root",
            "# Impl {#root}\n- ##fact-a source version\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/c#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("source version"), "{out}");
    assert!(!out.contains("contract version"), "{out}");
    assert!(!out.contains("#source"), "{out}");
}

#[test]
fn a_cross_section_fact_collision_fails_the_gate() {
    // The contract's `##dup` (in #a) is not overridden — the matching source
    // section carries no `##dup` — and a source-only section #b re-declares
    // it, so the merged document holds `dup` twice across sections.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/c#root",
            "# A {#a}\n#source spec://org.vibevm.core/vibevm/impl#whole\n- ##dup contract's\n",
        ),
        (
            "spec://org.vibevm.core/vibevm/impl#whole",
            "# A {#a}\nplain source a\n# B {#b}\n- ##dup source's\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/c#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::DuplicateId { dup, .. }) => {
            assert_eq!(dup.id, "dup");
            assert_eq!(dup.first_section, "a");
            assert_eq!(dup.second_section, "b");
        }
        other => panic!("expected a DuplicateId gate error, got {other:?}"),
    }
}

#[test]
fn folds_source_into_a_contract_that_declares_it() {
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n#source spec://org.vibevm.demo/lib/source/impl#root\ncontract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/impl#root",
            "# Impl {#root}\nsource-body",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("contract-body"), "{out}");
    assert!(out.contains("source-body"), "{out}");
    // The #source directive is resolved by the fold, not left behind.
    assert!(!out.contains("#source"), "{out}");
}

// ---- B-056-L2: the pipeline folds ALL declared `#source` directives --------
//
// A contract declaring more than one `#source` used to fold only the FIRST —
// every later directive was silently dropped (defect B-055). These pin the
// fix: every declared source reaches the fold, in declaration order.

#[test]
fn two_sources_both_folded_in_declaration_order() {
    // B-055: a contract declaring TWO `#source` directives folds BOTH, in the
    // order they were declared — the second directive is no longer dropped.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# S1 {#root}\ns1-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# S2 {#root}\ns2-body",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // All three bodies survive — contract, s1, s2 — checked by INDEX order, not
    // just presence.
    let c = out.find("contract-body").expect("contract-body");
    let s1 = out.find("s1-body").expect("s1-body");
    let s2 = out.find("s2-body").expect("s2-body");
    assert!(c < s1, "contract before s1:\n{out}");
    assert!(s1 < s2, "s1 (declared first) before s2:\n{out}");
}

#[test]
fn sources_fold_in_declaration_order_not_alphabetical() {
    // The merge order is the order the author DECLARED the directives, not the
    // alphabetical order of the addresses: declared s2-before-s1 ⇒ s2 first in
    // the output, even though s1 < s2 alphabetically.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# S1 {#root}\ns1-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# S2 {#root}\ns2-body",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    let s2 = out.find("s2-body").unwrap();
    let s1 = out.find("s1-body").unwrap();
    assert!(s2 < s1, "declaration order, not alphabetical:\n{out}");
}

#[test]
fn replace_in_second_source_drops_contract_keeps_both_sources() {
    // `:replace` on the SECOND declared source drops the CONTRACT text only;
    // both sources survive, in declaration order — the sources still add
    // together (§7.3, the multi-source replace law).
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# S1 {#root}\ns1-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# S2 {#root} :replace\ns2-body",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(
        !out.contains("contract-body"),
        "contract text survived:\n{out}"
    );
    let s1 = out.find("s1-body").unwrap();
    let s2 = out.find("s2-body").unwrap();
    assert!(s1 < s2, "sources in declaration order:\n{out}");
}

#[test]
fn a_fact_duplicate_between_two_sources_fails_the_build() {
    // Two sources each declare the SAME source-only section `#extra`, each
    // carrying the fact `##dup`. The fold appends both sections (no dedup
    // between sources), and the post-merge uniqueness gate trips on the
    // surviving fact-vs-fact collision — a definition is not idempotent even
    // when the declaration is (PROP-035 §7.3 clause 3).
    //
    // NB: the gate does NOT flag a pure heading-vs-heading repeat (the `:add`
    // concatenation artifact), so a fact must be on at least one side for the
    // build to fail — see `a_repeated_section_heading_is_not_flagged` in gate.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# Extra {#extra}\n- ##dup from s1\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# Extra {#extra}\n- ##dup from s2\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::DuplicateId { dup, .. }) => assert_eq!(dup.id, "dup"),
        other => panic!("expected DuplicateId from the s1/s2 fact collision, got {other:?}"),
    }
}

#[test]
fn unreachable_second_source_names_that_source_not_the_first() {
    // РТ-4: when the SECOND declared source is unreachable, the guard
    // (`source_fold_order`) hits it first; the Unresolved error names THAT
    // source's address — not the first source's, and not the seed's.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# S1 {#root}\ns1-body",
        ),
        // s2 is deliberately absent from the mock — it does not resolve.
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::Unresolved { addr, .. }) => {
            assert!(
                addr.contains("/source/s2"),
                "error must name the unreachable s2, got: {addr}"
            );
            assert!(
                !addr.contains("/source/s1"),
                "error must not name the reachable s1, got: {addr}"
            );
        }
        other => panic!("expected Unresolved for s2, got {other:?}"),
    }
}

#[test]
fn no_source_directive_lines_remain_with_two_sources() {
    // strip_directive_lines cuts by directive KIND, so EVERY `#source` line —
    // not just the first — is gone from the compiled output.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# S1 {#root}\ns1-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# S2 {#root}\ns2-body",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(!out.contains("#source"), "a #source line survived:\n{out}");
}

// ---- B-056-L3B: the fold goes recursive under `source_fold_order` -----------
//
// The fold now descends: a source that itself declares `#source` folds BEFORE
// it merges into its parent, every node folds once, and a cycle is judged by
// the guard `source_fold_order` (§9). An inclusion guard holds each node's text
// to exactly one copy in the document. These pin the recursion law (РТ-1…РТ-5).

#[test]
fn two_level_recursion_folds_the_chain() {
    // `a #source b`, `b #source c`. The recursion law: b folds c INTO ITSELF
    // before b merges into a, so c's body reaches the output — the old single-
    // level fold inlined b raw and never reached c. Bodies land in fold order
    // a → b → c, checked by index.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\na-body\n#source spec://org.vibevm.demo/lib/source/b#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/b#root",
            "# B {#b}\nb-body\n#source spec://org.vibevm.demo/lib/source/c#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c#root",
            "# C {#c}\nc-body\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // c-body present at all is the proof the recursion reached c THROUGH b.
    let a = out.find("a-body").unwrap();
    let b = out.find("b-body").unwrap();
    let c = out.find("c-body").unwrap();
    assert!(a < b && b < c, "fold order a -> b -> c:\n{out}");
    assert!(!out.contains("#source"), "{out}");
}

#[test]
fn a_diamond_includes_the_shared_source_once() {
    // `a #source b` and `a #source c`; both `b` and `c #source d`. The inclusion
    // guard folds each node's text into the document EXACTLY ONCE: `b` (earlier
    // in the deterministic fold order) takes `d`, so `c` skips it on the second
    // path — the output is `a ⊕ (b ⊕ d) ⊕ c`, and `d` appears once, not once per
    // reaching path. Without the guard, `d` would inline twice; with a fact
    // inside `d` (see the next test) that doubling is a surviving duplicate
    // anchor and the build sinks — so the guard is load-bearing, not cosmetic.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\na-body\n\
             #source spec://org.vibevm.demo/lib/source/b#root\n\
             #source spec://org.vibevm.demo/lib/source/c#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/b#root",
            "# B {#b}\nb-body\n#source spec://org.vibevm.demo/lib/source/d#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c#root",
            "# C {#c}\nc-body\n#source spec://org.vibevm.demo/lib/source/d#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/d#root",
            "# D {#d}\nd-body\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert_eq!(
        out.matches("d-body").count(),
        1,
        "inclusion guard: d folded in exactly once:\n{out}"
    );
}

#[test]
fn a_diamond_with_a_shared_fact_compiles() {
    // The reason the inclusion guard exists: the shared source `d` carries a
    // FACT `##shared` (a fact is the unit of content here — the norm, not an
    // exotic case). Two inlines of `d` would put `##shared` in the merged
    // document twice and the post-merge gate would sink the build — an ordinary
    // plugin composition (two plugins on a common base) turned into an
    // un-buildable error. With the guard, `d` folds in once, `##shared` survives
    // exactly once, and the diamond compiles.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\n\
             #source spec://org.vibevm.demo/lib/source/b#root\n\
             #source spec://org.vibevm.demo/lib/source/c#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/b#root",
            "# B {#b}\n#source spec://org.vibevm.demo/lib/source/d#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c#root",
            "# C {#c}\n#source spec://org.vibevm.demo/lib/source/d#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/d#root",
            "# Base {#base}\n- ##shared common fact\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert_eq!(
        out.matches("##shared").count(),
        1,
        "shared fact survives exactly once:\n{out}"
    );
    assert!(out.contains("common fact"), "{out}");
}

#[test]
fn a_source_cycle_between_contracts_compiles() {
    // РТ-1: a `#source` cycle whose every node is a contract is a legal forward
    // declaration (§9). In fold order b's member a is its own ancestor — not yet
    // folded — so a contributes nothing to b (the forward-declaration drop, see
    // `fold_source_closure`); b still folds into a. No error; both bodies live.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\na-body\n#source spec://org.vibevm.demo/lib/contract/b#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/contract/b#root",
            "# B {#b}\nb-body\n#source spec://org.vibevm.demo/lib/contract/a#root\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("a-body"), "{out}");
    assert!(out.contains("b-body"), "{out}");
}

#[test]
fn a_source_cycle_through_an_impl_fails() {
    // The mirror of РТ-1: a `#source` cycle running through an implementation
    // (non-contract) node is a hard error. The guard judges it (§9) and the
    // error propagates as `CompileError::UseGraph` — the only guard error
    // possible here, since both nodes resolve (so not `Unresolved`).
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\n#source spec://org.vibevm.demo/lib/source/b#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/b#root",
            "# B {#b}\n#source spec://org.vibevm.demo/lib/contract/a#root\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    assert!(matches!(
        compile_static(&seed, &src),
        Err(CompileError::UseGraph(_))
    ));
}

#[test]
fn a_source_free_seed_is_byte_identical_to_the_fast_path() {
    // B-056-L3B acceptance 5: a seed with no `#source` takes the fast path —
    // text returned untouched, no parse, no re-emit — so the compiled lane is
    // byte-for-byte what it was before the recursion landed. Whole-string eq.
    let src = MockSource::new(&[(
        "spec://org.vibevm.demo/lib/contract/api#root",
        "# API {#root}\nplain contract, no sources",
    )]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    let expected = "\
<!-- vibe:begin spec://org.vibevm.demo/lib/contract/api#root -->
# API {#root}
plain contract, no sources
<!-- vibe:end spec://org.vibevm.demo/lib/contract/api#root -->
";
    assert_eq!(out, expected, "no-#source fast path must be byte-identical");
}

#[test]
fn a_duplicate_at_an_inner_level_names_the_inner_node() {
    // B-056-L3B acceptance 6: the post-merge gate runs at EVERY level, so a
    // duplicate fact arising while folding an INNER node (b folds c1 and c2,
    // both carrying ##dup) fails naming b — the node where the collision arose —
    // not a, the seed that merely declares `#source b`.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n#source spec://org.vibevm.demo/lib/source/mid#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/mid#root",
            "# Mid {#root}\n\
             #source spec://org.vibevm.demo/lib/source/c1#root\n\
             #source spec://org.vibevm.demo/lib/source/c2#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c1#root",
            "# Extra {#extra}\n- ##dup from c1\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c2#root",
            "# Extra {#extra}\n- ##dup from c2\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::DuplicateId { addr, dup }) => {
            assert_eq!(dup.id, "dup");
            assert!(
                addr.contains("/source/mid"),
                "must name the inner node mid, got: {addr}"
            );
            assert!(
                !addr.contains("/contract/api"),
                "must NOT name the seed api, got: {addr}"
            );
        }
        other => panic!("expected DuplicateId at the inner node, got {other:?}"),
    }
}

// (The four B-056 source-section collision tests moved to `collision_tests`
// when `fold_tests` neared the 600-line budget — same seam that split
// `fold_tests` out of `tests`: one responsibility per file.)
