//! Tests for the static compile's ordering / emission / qualification — the
//! phases around the fold. The `#source` fold itself (single-level, multi-source,
//! and the recursive closure) lives in [`super::fold_tests`], split out along the
//! responsibility seam so neither file breaches the 600-line budget.

use super::*;
use std::collections::HashMap;

/// An in-memory `SectionSource` for the pipeline tests. `pub(super)` so the fold
/// tests (a sibling module) reuse the same fixture instead of duplicating it.
pub(super) struct MockSource(HashMap<String, String>);

impl MockSource {
    pub(super) fn new(pairs: &[(&str, &str)]) -> Self {
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

// ---- B-056-L4B: a `#source` glob reaches the fold (one edge law, one place) -
//
// A `#source` may name a SET — a `*` in the package name — not a file. Both the
// fold guard (`source_fold_order`) and the fold itself (`fold_source_closure`)
// now reach a document's `#source` edges through ONE function —
// `use_graph::source_addresses` — which expands each directive (a glob → its
// sorted members, a point address → itself) in declaration order. These pin that
// contract: the glob folds its members, expands in place, degrades on an empty
// match, and recurses through an expanded edge — the proof the guard and the
// fold walked the same graph.

/// An in-memory `SectionSource` that ALSO expands a pattern address to a known,
/// sorted member set — the contract a real `FsSectionSource` delegates to
/// `FileResolver::expand_pattern`. A pattern absent from the map falls back to
/// the trait default (the address denotes itself), so a point `#source` behaves
/// exactly as in the plain `MockSource`: it resolves through `section_text`.
struct GlobMockSource {
    text: HashMap<String, String>,
    members: HashMap<String, Vec<String>>,
}

impl GlobMockSource {
    fn text(pairs: &[(&str, &str)]) -> Self {
        GlobMockSource {
            text: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            members: HashMap::new(),
        }
    }

    /// Map a pattern address (its without-pin key) to the sorted member raws the
    /// resolver's `expand_pattern` would return. Builder-style so each test names
    /// only the globs it exercises.
    fn with_glob(mut self, pattern: &str, members: &[&str]) -> Self {
        self.members.insert(
            pattern.to_string(),
            members.iter().map(|s| (*s).to_string()).collect(),
        );
        self
    }
}

impl SectionSource for GlobMockSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        self.text
            .get(&addr.without_pin())
            .cloned()
            .ok_or_else(|| "not in mock".to_string())
    }

    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        match self.members.get(&addr.without_pin()) {
            Some(raws) => Ok(raws
                .iter()
                .filter_map(|r| SpecAddress::parse(r).ok())
                .collect()),
            None => Ok(vec![addr.clone()]), // not a known pattern → point oracle
        }
    }
}

#[test]
fn a_glob_source_folds_all_its_members_in_sorted_order() {
    // One `#source` names a glob; it expands to two members (alpha, beta — the
    // sorted order the resolver returns). Both members' bodies reach the output,
    // in expansion order (alpha before beta), and the glob itself never reaches
    // `section_text` — it names a set, not a file.
    let src = GlobMockSource::text(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n#source spec://org.vibevm.plugins/plugin-*/impl#root\ncontract-body",
        ),
        (
            "spec://org.vibevm.plugins/plugin-alpha/impl#root",
            "# Alpha {#root}\nalpha-body",
        ),
        (
            "spec://org.vibevm.plugins/plugin-beta/impl#root",
            "# Beta {#root}\nbeta-body",
        ),
    ])
    .with_glob(
        "spec://org.vibevm.plugins/plugin-*/impl#root",
        &[
            "spec://org.vibevm.plugins/plugin-alpha/impl#root",
            "spec://org.vibevm.plugins/plugin-beta/impl#root",
        ],
    );
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    let alpha = out.find("alpha-body").expect("alpha-body");
    let beta = out.find("beta-body").expect("beta-body");
    assert!(
        alpha < beta,
        "glob members in sorted expansion order:\n{out}"
    );
    assert!(out.contains("contract-body"), "{out}");
    assert!(!out.contains("#source"), "{out}");
    assert!(
        !out.contains("plugin-*"),
        "the glob itself must not reach the compiled text:\n{out}"
    );
}

#[test]
fn a_glob_and_a_point_source_keep_their_declaration_order() {
    // `#source <glob>` then `#source <point>`: declaration order is preserved,
    // and the glob expands IN PLACE — its members sit where the directive sits,
    // not shuffled to the end after the point source. So the output order is
    // contract, [glob's members], point.
    let src = GlobMockSource::text(&[
        (
            "spec://org.vibevm.demo/lib/contract/api#root",
            "# API {#root}\n\
             #source spec://org.vibevm.plugins/plugin-*/impl#root\n\
             #source spec://org.vibevm.demo/lib/source/point#root\n\
             contract-body",
        ),
        (
            "spec://org.vibevm.plugins/plugin-alpha/impl#root",
            "# Alpha {#root}\nalpha-body",
        ),
        (
            "spec://org.vibevm.demo/lib/source/point#root",
            "# Point {#root}\npoint-body",
        ),
    ])
    .with_glob(
        "spec://org.vibevm.plugins/plugin-*/impl#root",
        &["spec://org.vibevm.plugins/plugin-alpha/impl#root"],
    );
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    let alpha = out.find("alpha-body").expect("alpha-body");
    let point = out.find("point-body").expect("point-body");
    assert!(
        alpha < point,
        "glob expands in place: its member before the later point source:\n{out}"
    );
}

#[test]
fn an_empty_glob_source_yields_no_sources() {
    // РТ-4: a glob matching nothing expands to the empty set, so the seed has no
    // `#source` edge — the fold takes the fast path and the document compiles as
    // if it declared no sources (no error, no phantom member, the glob stripped).
    let src = GlobMockSource::text(&[(
        "spec://org.vibevm.demo/lib/contract/api#root",
        "# API {#root}\n#source spec://org.vibevm.plugins/plugin-*/impl#root\ncontract-body",
    )])
    .with_glob("spec://org.vibevm.plugins/plugin-*/impl#root", &[]);
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    assert!(out.contains("contract-body"), "{out}");
    assert!(!out.contains("#source"), "{out}");
    assert!(
        !out.contains("plugin-*"),
        "the empty glob must not leak into the compiled text:\n{out}"
    );
}

#[test]
fn the_fold_reaches_a_source_through_a_glob_expanded_edge() {
    // The proof the guard and the fold ask ONE place: the seed's `#source` is a
    // glob that expands to a member `b`, and `b` ITSELF declares `#source c`. The
    // fold reaches `c` only if the guard expanded the glob to `b` AND then
    // followed `b`'s own `#source c` — the same expansion both walks use. Had the
    // guard expanded the glob while the fold did not, the fold would try to load
    // the glob address literally and fail; `c`-body in the output is the witness
    // they agree.
    let src = GlobMockSource::text(&[
        (
            "spec://org.vibevm.demo/lib/contract/a#root",
            "# A {#a}\na-body\n#source spec://org.vibevm.plugins/plugin-*/impl#root\n",
        ),
        (
            "spec://org.vibevm.plugins/plugin-alpha/impl#root",
            "# B {#b}\nb-body\n#source spec://org.vibevm.demo/lib/source/c#root\n",
        ),
        (
            "spec://org.vibevm.demo/lib/source/c#root",
            "# C {#c}\nc-body",
        ),
    ])
    .with_glob(
        "spec://org.vibevm.plugins/plugin-*/impl#root",
        &["spec://org.vibevm.plugins/plugin-alpha/impl#root"],
    );
    let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/a#root").unwrap();
    let out = compile_static(&seed, &src).unwrap();
    // c-body is present ONLY because the glob expanded to b, and b's own
    // `#source c` was then followed — the recursion runs through the expanded
    // edge, so the guard and the fold walked the same graph. (Distinct section
    // anchors `{#a}/{#b}/{#c}` isolate the glob-edge proof from the merge's
    // same-id section semantics, which a separate test covers.)
    assert!(
        out.contains("c-body"),
        "recursion through a glob edge:\n{out}"
    );
    assert!(out.contains("b-body"), "{out}");
    assert!(!out.contains("#source"), "{out}");
}

/// The dependency's canonical Markdown twin — byte-exact the projection of
/// [`XML_DEP`] (the trailing blank line included: blocks close with one).
/// `pub(super)` so the inheritance-parity twin family (a sibling module)
/// reuses the same fixture.
pub(super) const MD_DEP_TWIN: &str = concat!(
    "# Dep {#d}\n\n",
    "## The laws {#laws}\n\n",
    "`req r1`\n\n",
    "@fact:FACT-ONE the fact body @status:impl/done\n\n"
);
