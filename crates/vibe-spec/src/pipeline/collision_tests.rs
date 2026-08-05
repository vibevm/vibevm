//! Tests for the B-056 source-section collision gate — two or more sources each
//! defining a top-level section the contract does NOT declare (two definitions
//! of one name). Split out of [`fold_tests`] along the responsibility seam so
//! neither file breaches the 600-line budget (`conform.toml` `max_file_lines`):
//! `fold_tests` keeps the recursion, the diamond, the cycle, and the inclusion
//! guard; these four pin the one-definition rule the post-merge gate cannot see.

use super::tests::MockSource;
use super::*;

// ---- B-056: two sources defining one source-only section (one definition) ---
//
// The post-merge `first_duplicate` gate deliberately does NOT flag a pure
// heading-vs-heading repeat — in the merged view it is indistinguishable from
// the accepted `:add` concatenation artifact. So two sources that each DECLARE
// a section the contract never did used to ship a twice-anchored section
// whenever no fact collided to flag it — the one-definition rule, violated
// silently. The pipeline closes that hole pre-fold (where each source's tree is
// still separate): a section anchor two or more sources DEFINE but the contract
// does not declare fails the build naming the anchor. These four pin the rule
// (B-056 acceptance 1–4).

#[test]
fn two_sources_defining_one_source_only_section_fails_even_without_a_fact() {
    // B-056 acceptance 1 — the case that passed SILENTLY before this change.
    // Two sources each declare the SAME source-only section `#extra` (the
    // contract never did) with NO fact inside either. `first_duplicate` skips
    // the pure heading repeat, so pre-B-056 this compiled: two definitions of
    // one name shipped as a twice-anchored `#extra`. The pre-fold check now
    // fails the build naming the anchor.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# Extra {#extra}\nbody from s1\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# Extra {#extra}\nbody from s2\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::DuplicateSourceSection { addr, anchor }) => {
            assert_eq!(anchor, "extra", "must name the colliding anchor");
            assert!(
                addr.contains("/contract/api"),
                "seed-level collision names the seed: {addr}"
            );
        }
        other => panic!("expected DuplicateSourceSection, got {other:?}"),
    }
}

#[test]
fn two_sources_matching_a_contract_section_is_a_legal_add_not_a_collision() {
    // B-056 acceptance 2: a section whose anchor the CONTRACT also declares is
    // an `:add` sum — one definition, many contributions — never a collision,
    // however many sources add to it. (Both bodies reach the output and the
    // build succeeds; the repeated heading is the accepted `:add` artifact.)
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# Shared {#shared}\ncontract-shared\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n\
             #source spec://org.vibevm.demo/lib/source/s2#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# Shared {#shared}\nfrom s1\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s2#root",
            "# Shared {#shared}\nfrom s2\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out =
        compile_static(&seed, &src).expect("a matching anchor is an :add sum, not a collision");
    assert!(out.contains("contract-shared"), "{out}");
    assert!(out.contains("from s1"), "{out}");
    assert!(out.contains("from s2"), "{out}");
}

#[test]
fn one_source_declaring_a_source_only_section_is_legal() {
    // B-056 acceptance 3: a source-only section declared by exactly ONE source
    // is a single definition — legitimate (a plugin contributing a section the
    // contract never named). Only TWO or more sources defining the same name
    // collide.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.demo/lib/source/s1#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/s1#root",
            "# Extra {#extra}\nonly from s1\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).expect("one source-only definition is legal");
    assert!(out.contains("only from s1"), "{out}");
    assert!(out.contains("# Extra {#extra}"), "{out}");
}

#[test]
fn a_source_section_collision_at_an_inner_level_names_the_inner_node() {
    // B-056 acceptance 4 / РТ-C: the pre-fold collision check runs at EVERY
    // level (each node's own contract + members), so two sources defining the
    // same source-only section while folding an INNER node (mid folds c1 and
    // c2, both `#extra`) fails naming `mid` — where the collision arose — not
    // `api`, the seed that merely declares `#source mid`. Mirrors the per-level
    // fact gate in `a_duplicate_at_an_inner_level_names_the_inner_node`.
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
            "# Extra {#extra}\nfrom c1\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c2#root",
            "# Extra {#extra}\nfrom c2\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    match compile_static(&seed, &src) {
        Err(CompileError::DuplicateSourceSection { addr, anchor }) => {
            assert_eq!(anchor, "extra");
            assert!(
                addr.contains("/source/mid"),
                "must name the inner node mid, got: {addr}"
            );
            assert!(
                !addr.contains("/contract/api"),
                "must NOT name the seed api, got: {addr}"
            );
        }
        other => panic!("expected DuplicateSourceSection at the inner node, got {other:?}"),
    }
}
