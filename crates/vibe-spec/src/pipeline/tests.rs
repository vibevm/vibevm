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
            "spec://vibevm/a#r",
            "# A {#r}\n#use spec://vibevm/b#r\n#embed spec://vibevm/c#r",
        ),
        ("spec://vibevm/b#r", "# B {#r}\nbee"),
        ("spec://vibevm/c#r", "cee"),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
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
    assert!(out.contains("<!-- vibe:begin spec://vibevm/a#r -->"));
    assert!(out.contains("<!-- vibe:end spec://vibevm/b#r -->"));
}

#[test]
fn a_lone_seed_compiles_to_itself() {
    let src = MockSource::new(&[("spec://vibevm/a#r", "# A {#r}\njust me")]);
    let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("just me"));
    assert!(out.contains("<!-- vibe:begin spec://vibevm/a#r -->"));
}

#[test]
fn a_cycle_fails_the_compile() {
    let src = MockSource::new(&[
        ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
        ("spec://vibevm/b#r", "#use spec://vibevm/a#r"),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
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
            "spec://vibevm/c#root",
            "# API {#root}\n#source spec://vibevm/impl#root\n- ##fact-a contract version\n",
        ),
        (
            "spec://vibevm/impl#root",
            "# Impl {#root}\n- ##fact-a source version\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/c#root").unwrap();
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
            "spec://vibevm/c#root",
            "# A {#a}\n#source spec://vibevm/impl#whole\n- ##dup contract's\n",
        ),
        (
            "spec://vibevm/impl#whole",
            "# A {#a}\nplain source a\n# B {#b}\n- ##dup source's\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/c#root").unwrap();
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
            "spec://vibevm/a#r",
            "# A {#r}\n#use spec://vibevm/b#r as dep\nSees @!dep here.\n",
        ),
        ("spec://vibevm/b#r", "# B {#r}\nb body\n"),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // The alias target's full address is spliced in for `@!dep`.
    assert!(out.contains("@spec://vibevm/b#r"), "{out}");
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
            "spec://vibevm/a#r",
            "# A {#r}\n#use spec://vibevm/b#r as dep\n```\n@!dep\n```\n",
        ),
        ("spec://vibevm/b#r", "# B {#r}\nb\n"),
    ]);
    let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
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
