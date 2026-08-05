use super::*;
use std::collections::HashMap;

struct MockSource(HashMap<String, String>);

impl MockSource {
    fn new(pairs: &[(&str, &str)]) -> Self {
        MockSource(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )
    }
}

impl SectionSource for MockSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&addr.without_pin())
            .cloned()
            .ok_or_else(|| "not in mock".to_string())
    }
}

#[test]
fn composes_use_ordering_and_embed_expansion() {
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#r",
            "# A {#r}\n#use spec://org.vibevm.core/vibevm/b#r\n#embed spec://org.vibevm.core/vibevm/c#r",
        ),
        ("spec://org.vibevm.core/vibevm/b#r", "# B {#r}\nbee"),
        ("spec://org.vibevm.core/vibevm/c#r", "cee"),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();

    // The dependency `b` is emitted before its user `a`.
    let bee = out.find("bee").unwrap();
    let a_heading = out.find("# A").unwrap();
    assert!(bee < a_heading, "dependency must precede its user:\n{out}");
    // The embed is spliced.
    assert!(out.contains("cee"));
    // No directive survives the compile.
    assert!(!out.contains("#use"), "{out}");
    assert!(!out.contains("#embed"), "{out}");
    // Node markers wrap each emission.
    assert!(out.contains("<!-- vibe:begin spec://org.vibevm.core/vibevm/a#r -->"));
    assert!(out.contains("<!-- vibe:end spec://org.vibevm.core/vibevm/b#r -->"));
}

#[test]
fn a_lone_seed_compiles_to_itself() {
    let src = MockSource::new(&[("spec://org.vibevm.core/vibevm/a#r", "# A {#r}\njust me")]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("just me"));
    assert!(out.contains("<!-- vibe:begin spec://org.vibevm.core/vibevm/a#r -->"));
}

#[test]
fn a_cycle_fails_the_compile() {
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#r",
            "#use spec://org.vibevm.core/vibevm/b#r",
        ),
        (
            "spec://org.vibevm.core/vibevm/b#r",
            "#use spec://org.vibevm.core/vibevm/a#r",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap();
    assert!(matches!(
        compile_static(&seed, &src),
        Err(CompileError::UseGraph(_))
    ));
}

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

#[test]
fn at_bang_alias_is_rewritten_to_the_full_address() {
    // B-011 §7.4: in the compiled lane every `@!<Alias>` becomes the full
    // `@spec://<target>` its `#use … as` binding denotes, and the `as` clause
    // leaves with the stripped `#use` line.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#r",
            "# A {#r}\n#use spec://org.vibevm.core/vibevm/b#r as dep\nSees @!dep here.\n",
        ),
        ("spec://org.vibevm.core/vibevm/b#r", "# B {#r}\nb body\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // The alias target's full address is spliced in for `@!dep`.
    assert!(out.contains("@spec://org.vibevm.core/vibevm/b#r"), "{out}");
    assert!(!out.contains("@!dep"), "{out}");
    // The declaration line (and its `as dep` clause) is gone with `#use`.
    assert!(!out.contains("#use "), "{out}");
    assert!(!out.contains("as dep"), "{out}");
    // The aliased dependency is still emitted before its user (topo order).
    assert!(out.contains("b body"), "{out}");
}

#[test]
fn at_bang_in_a_fence_is_not_rewritten() {
    // The fence mask governs the rewrite as it governs the scan: an `@!dep`
    // inside a fenced block is prose-as-data, not a use, so it stays put.
    let src = MockSource::new(&[
        (
            "spec://org.vibevm.core/vibevm/a#r",
            "# A {#r}\n#use spec://org.vibevm.core/vibevm/b#r as dep\n```\n@!dep\n```\n",
        ),
        ("spec://org.vibevm.core/vibevm/b#r", "# B {#r}\nb\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.vibevm.core/vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("@!dep"), "fenced @!dep must stay: {out}");
}

// ---- B-006 rider: per-node qualification (PROP-035 §8 phase 5) -----------
//
// `compile_static_qualified` qualifies each node under ITS OWN origin
// (derived from the topo key), then a second pass resolves the cross-node
// short links the per-node pass could not see. These six tests pin the
// contract (Q1–Q6 of the E4-W2-NODE-QUALIFY packet). The fixture addresses
// use distinct package coordinates (`org.a/a`, `org.b/b`, `org.c/c`) so the
// nodes carry DISTINCT origins — the case per-node qualify exists for.

#[test]
fn q1_two_origins_each_qualify_their_own_label() {
    // Two documents of different origins, linked by `#use`, each defining
    // `##THE-RULE`: the qualified compile emits TWO different qualified
    // names, each under its own origin, and the rename map carries both
    // origins. No ambiguity — coexisting definitions are qualified apart;
    // only an unresolved *cross-node reference* is ambiguous (q4).
    let src = MockSource::new(&[
        (
            "spec://org.a/a/doc#r",
            "# A {#root}\n##THE-RULE a's rule\n#use spec://org.b/b/doc#r\n",
        ),
        ("spec://org.b/b/doc#r", "# B {#root}\n##THE-RULE b's rule\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    let (out, renames) = compile_static_qualified(&seed, &src).unwrap();

    // Each node's THE-RULE is qualified under its own origin — never the
    // entry's.
    assert!(out.contains("##org-a--a--THE-RULE"), "{out}");
    assert!(out.contains("##org-b--b--THE-RULE"), "{out}");
    // The rename map carries both origins for THE-RULE.
    let rule_origins: Vec<&str> = renames
        .iter()
        .filter(|(_, r)| r.original == "THE-RULE")
        .map(|(o, _)| o.as_str())
        .collect();
    assert!(rule_origins.contains(&"org.a/a"), "{renames:?}");
    assert!(rule_origins.contains(&"org.b/b"), "{renames:?}");
}

#[test]
fn q2_within_node_self_reference_is_qualified_by_its_own_origin() {
    // A node referencing its own label is resolved within the node by the
    // per-node qualify — the same as the old whole-body behaviour — so the
    // second pass never touches it.
    let src = MockSource::new(&[(
        "spec://org.a/a/doc#r",
        "# A {#root}\nSee (#root) and (#OTHER).\n##OTHER a fact\n",
    )]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    let (out, _) = compile_static_qualified(&seed, &src).unwrap();
    assert!(out.contains("{#org-a--a--root}"), "{out}");
    assert!(out.contains("(#org-a--a--root)"), "{out}");
    assert!(out.contains("(#org-a--a--OTHER)"), "{out}");
}

#[test]
fn q3_cross_node_short_link_resolves_to_the_unique_definer() {
    // Node A (origin1) references `(#THE-RULE)`, which ONLY node B (origin2)
    // defines: the second pass rewrites it to B's qualified heir.
    let src = MockSource::new(&[
        (
            "spec://org.a/a/doc#r",
            "# A {#root}\nSee (#THE-RULE) live.\n#use spec://org.b/b/doc#r\n",
        ),
        ("spec://org.b/b/doc#r", "# B {#root}\n##THE-RULE b's\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    let (out, _) = compile_static_qualified(&seed, &src).unwrap();
    assert!(out.contains("(#org-b--b--THE-RULE)"), "{out}");
    assert!(
        !out.contains("(#THE-RULE)"),
        "the bare cross-node link must be gone: {out}"
    );
}

#[test]
fn q4_ambiguous_cross_node_short_link_fails_with_candidates() {
    // A short link to a label TWO nodes define is a build error citing both
    // candidate heirs (B-011: fail with candidates, never a silent pick).
    let src = MockSource::new(&[
        (
            "spec://org.a/a/doc#r",
            "# A {#root}\nSee (#SHARED).\n#use spec://org.b/b/doc#r\n#use spec://org.c/c/doc#r\n",
        ),
        ("spec://org.b/b/doc#r", "# B {#root}\n##SHARED b's\n"),
        ("spec://org.c/c/doc#r", "# C {#root}\n##SHARED c's\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    match compile_static_qualified(&seed, &src) {
        Err(CompileError::AmbiguousShortLink { label, candidates }) => {
            assert_eq!(label, "SHARED");
            let joined = candidates.join(" | ");
            assert!(joined.contains("org-b--b--SHARED"), "{joined}");
            assert!(joined.contains("org-c--c--SHARED"), "{joined}");
        }
        other => panic!("expected AmbiguousShortLink, got {other:?}"),
    }
}

#[test]
fn q5_fenced_blocks_are_untouched_by_both_passes() {
    // Fenced code is masked from the per-node qualify AND the second pass:
    // a `##FENCED` inside a fence is never treated as a definition, and a
    // `(#x)` inside a fence is never rewritten — even when `x` is defined
    // unfenced elsewhere. The same `(#x)` outside the fence IS rewritten.
    let src = MockSource::new(&[
        (
            "spec://org.a/a/doc#r",
            "# A {#root}\n#use spec://org.b/b/doc#r\nSee (#ONLY-IN-B) live.\n\
             ```\n##FENCED and (#ONLY-IN-B) and (#root)\n```\n",
        ),
        ("spec://org.b/b/doc#r", "# B {#root}\n##ONLY-IN-B b's\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    let (out, renames) = compile_static_qualified(&seed, &src).unwrap();

    // The fenced line is byte-identical — `##FENCED` is not a definition
    // (so it is absent from the rename map) and the fenced short links stay
    // bare.
    assert!(
        out.contains("##FENCED and (#ONLY-IN-B) and (#root)"),
        "{out}"
    );
    assert!(
        !renames.iter().any(|(_, r)| r.original == "FENCED"),
        "fenced ##FENCED must not become a definition: {renames:?}"
    );
    // The same cross-node link OUTSIDE the fence was resolved.
    assert!(out.contains("(#org-b--b--ONLY-IN-B) live"), "{out}");
}

#[test]
fn q6_plain_compile_static_emits_labels_unqualified() {
    // Regression guard for the reference semantics: `compile_static` (the
    // unqualified path) emits labels exactly as authored — no origin prefix,
    // no rename map — over a multi-document closure.
    let src = MockSource::new(&[
        (
            "spec://org.a/a/doc#r",
            "# A {#root}\n##FACT a\n#use spec://org.b/b/doc#r\n",
        ),
        ("spec://org.b/b/doc#r", "# B {#root}\n##FACT b\n"),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // Bare labels survive; no qualified form appears.
    assert!(out.contains("{#root}"), "{out}");
    assert!(out.contains("##FACT a"), "{out}");
    assert!(!out.contains("--root"), "{out}");
    assert!(!out.contains("--FACT"), "{out}");
}

// ---- B-056-L2: the pipeline folds ALL declared `#source` directives --------
//
// A contract declaring more than one `#source` used to fold only the FIRST —
// every later directive was silently dropped (defect B-055). These pin the
// fix: every declared source reaches the fold, in declaration order, with the
// degenerate single-source and no-source paths left byte-unchanged (covered by
// the existing `folds_source_into_a_contract_that_declares_it` and the no-
// `#source` seeds above).

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
    // РТ-3: when the SECOND declared source is unreachable, the Unresolved
    // error names THAT source's address — not the first source's, and not the
    // seed's.
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
