//! Byte-level characterization of the one-seed compiler while R3 splits it
//! into explicit IR levels and named passes. Public-entry tests remain the
//! compatibility oracle rather than a sketch of future carrier shapes.

use specmark::verifies;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::tests::MockSource;
use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ArtifactId, ClosureContribution,
    ClosureDocument, ClosureIr, ClosureNodeId, ContributionAbsorption, ContributionMeta,
    DocumentAddress, QualificationState, StaticCompileMode,
};
use crate::{DocTree, UseGraphError, topo_order_from};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn legacy_continuation_emits_the_close_carrier_body() {
    let key = "spec://org.demo/pkg/boot/entry#root";
    let addr = SpecAddress::parse(key).unwrap();
    let closure = ClosureIr {
        artifact: ArtifactId::new("static-fragment").unwrap(),
        nodes: vec![ClosureDocument {
            address: DocumentAddress::Spec(addr.clone()),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse("# Closed {#closed}\nCLOSE-BODY"),
            aliases: Default::default(),
        }],
        edges: Vec::new(),
        contributions: vec![ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "boot/entry".to_string(),
            },
            seed: ClosureNodeId(0),
            emission_order: vec![ClosureNodeId(0)],
        }],
        renames: Vec::new(),
        qualification: QualificationState::Applied(StaticCompileMode::Plain),
        absorption: AbsorptionState::Applied(AbsorptionPlan {
            mode: StaticCompileMode::Plain,
            contributions: vec![ContributionAbsorption::Normal {
                meta: ContributionMeta {
                    origin: "org.demo/pkg".to_string(),
                    path: "boot/entry".to_string(),
                },
                seed: ClosureNodeId(0),
                seed_address: addr.clone(),
                occurrences: vec![AbsorptionOccurrence {
                    node: ClosureNodeId(0),
                    address: addr,
                    absorbed: false,
                }],
            }],
        }),
        pending_sources: None,
        pending_embeds: None,
    };
    let (out, renames) = compile_static_continuation(closure).unwrap();

    assert!(out.contains("CLOSE-BODY"), "{out}");
    assert!(!out.contains("RAW-BODY"), "{out}");
    assert!(renames.is_empty());
}

struct CountingSource {
    texts: HashMap<String, String>,
    loads: RefCell<HashMap<String, usize>>,
}

impl CountingSource {
    fn new(pairs: &[(&str, &str)]) -> Self {
        Self {
            texts: pairs
                .iter()
                .map(|(key, text)| ((*key).to_string(), (*text).to_string()))
                .collect(),
            loads: RefCell::new(HashMap::new()),
        }
    }
}

impl SectionSource for CountingSource {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        *self.loads.borrow_mut().entry(key.clone()).or_default() += 1;
        self.texts
            .get(&key)
            .cloned()
            .ok_or_else(|| "not in counting source".to_string())
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn production_close_reorders_a_diamond_and_loads_each_document_once() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let c = "spec://org.demo/pkg/boot/c#root";
    let d = "spec://org.demo/pkg/boot/d#root";
    let source = CountingSource::new(&[
        (a, &format!("# A {{#root}}\n#use {b}\n#use {c}\n")),
        (b, &format!("# B {{#root}}\n#use {d}\n")),
        (c, &format!("# C {{#root}}\n#use {d}\n")),
        (d, "# D {#root}\n"),
    ]);

    let out = compile_static(&SpecAddress::parse(a).unwrap(), &source).unwrap();
    let marker_positions: Vec<usize> = [d, b, c, a]
        .into_iter()
        .map(|key| out.find(&crate::markers::open(key)).unwrap())
        .collect();

    assert!(marker_positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        source.loads.into_inner(),
        HashMap::from([
            (a.to_string(), 1),
            (b.to_string(), 1),
            (c.to_string(), 1),
            (d.to_string(), 1),
        ])
    );
}

struct StatefulExpansionSource {
    seed: String,
    member: String,
    pattern: String,
    member_address: SpecAddress,
    expansion_calls: Cell<usize>,
}

impl SectionSource for StatefulExpansionSource {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() == self.member_address.without_pin() {
            Ok(self.member.clone())
        } else {
            Ok(self.seed.clone())
        }
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        assert_eq!(address.without_pin(), self.pattern);
        let call = self.expansion_calls.get() + 1;
        self.expansion_calls.set(call);
        if call == 1 {
            Ok(vec![self.member_address.clone()])
        } else {
            Ok(Vec::new())
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn named_merge_observes_each_source_expansion_once_and_emits_that_observation() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let pattern = "spec://org.demo/plugin-*/source/impl#root";
    let member = SpecAddress::parse("spec://org.demo/plugin-a/source/impl#root").unwrap();
    let source = StatefulExpansionSource {
        seed: format!("# API {{#root}}\n#source {pattern}\nCONTRACT\n"),
        member: "# Impl {#impl}\nMEMBER-FROM-FIRST-EXPANSION\n".to_string(),
        pattern: pattern.to_string(),
        member_address: member,
        expansion_calls: Cell::new(0),
    };

    let out = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert_eq!(source.expansion_calls.get(), 1);
    assert!(out.contains("MEMBER-FROM-FIRST-EXPANSION"), "{out}");
}

struct SharedSourceObservation {
    texts: HashMap<String, String>,
    pattern: String,
    target: SpecAddress,
    expansion_calls: Cell<usize>,
    target_loads: Cell<usize>,
}

impl SectionSource for SharedSourceObservation {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        if key == self.target.without_pin() {
            self.target_loads.set(self.target_loads.get() + 1);
        }
        self.texts
            .get(&key)
            .cloned()
            .ok_or_else(|| "not in shared source".to_string())
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        assert_eq!(address.without_pin(), self.pattern);
        self.expansion_calls.set(self.expansion_calls.get() + 1);
        Ok(vec![self.target.clone()])
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn two_use_roots_share_one_expansion_load_and_parse_but_fold_per_root() {
    let a = "spec://org.demo/pkg/boot/a#root";
    let b = "spec://org.demo/pkg/boot/b#root";
    let pattern = "spec://org.demo/plugin-*/source/impl#root";
    let target = SpecAddress::parse("spec://org.demo/plugin-a/source/impl#root").unwrap();
    let source = SharedSourceObservation {
        texts: HashMap::from([
            (
                a.to_string(),
                format!("# A {{#root}}\n#use {b}\n#source {pattern}\n"),
            ),
            (b.to_string(), format!("# B {{#root}}\n#source {pattern}\n")),
            (
                target.without_pin(),
                "# Shared source {#shared}\nSHARED-SOURCE-BODY\n".to_string(),
            ),
        ]),
        pattern: pattern.to_string(),
        target,
        expansion_calls: Cell::new(0),
        target_loads: Cell::new(0),
    };

    crate::compiler::merge::reset_merge_invocations();
    let out = compile_static(&SpecAddress::parse(a).unwrap(), &source).unwrap();

    assert_eq!(source.expansion_calls.get(), 1);
    assert_eq!(source.target_loads.get(), 1);
    assert_eq!(crate::compiler::merge::merge_invocations(), 1);
    assert_eq!(out.matches("SHARED-SOURCE-BODY").count(), 2, "{out}");
}

struct SourceEmbedOverlap {
    seed: String,
    target: SpecAddress,
    target_reads: Cell<usize>,
}

impl SectionSource for SourceEmbedOverlap {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        if address.without_pin() != self.target.without_pin() {
            return Ok(self.seed.clone());
        }
        let read = self.target_reads.get() + 1;
        self.target_reads.set(read);
        if read == 1 {
            Ok("# Source view {#source-view}\nSOURCE-OWNER-VIEW\n".to_string())
        } else {
            Ok("# Embed view {#embed-view}\nEMBED-OWNER-VIEW\n".to_string())
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn source_embed_overlap_shares_one_parse_but_replays_both_semantics() {
    let seed = "spec://org.demo/pkg/contract/api#root";
    let target = SpecAddress::parse("spec://org.demo/pkg/common/shared#root").unwrap();
    let source = SourceEmbedOverlap {
        seed: format!(
            "# API {{#root}}\n#source {}\n#embed {}\n",
            target.without_pin(),
            target.without_pin()
        ),
        target,
        target_reads: Cell::new(0),
    };

    let out = compile_static(&SpecAddress::parse(seed).unwrap(), &source).unwrap();

    assert_eq!(source.target_reads.get(), 1);
    assert_eq!(out.matches("SOURCE-OWNER-VIEW").count(), 2, "{out}");
    assert!(!out.contains("EMBED-OWNER-VIEW"), "{out}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn pinned_seed_preserves_the_exact_topo_key_in_marker_bytes() {
    let key = "spec://org.demo/pkg@0.2/boot/entry#root";
    let source = MockSource::new(&[(key, "# Entry {#root}\nbody\n")]);
    let seed = SpecAddress::parse(&format!("{key}~r7")).unwrap();
    let old_order = topo_order_from(&seed, &source).unwrap();

    assert_eq!(old_order, vec![key.to_string()]);
    assert_eq!(
        compile_static(&seed, &source).unwrap(),
        format!(
            "{}\n# Entry {{#root}}\nbody\n{}\n",
            crate::markers::open(&old_order[0]),
            crate::markers::close(&old_order[0]),
        )
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn one_seed_success_is_byte_exact_in_plain_and_qualified_modes() {
    let src = MockSource::new(&[
        (
            "spec://org.a/a/boot/entry#root",
            "# Entry {#root}\n#use spec://org.b/b/boot/dep#root\n\n#embed spec://org.c/c/boot/embed#piece\n\nUses (#DEP-RULE) and (#EMBED-RULE).\n\n##ENTRY-RULE entry body\n",
        ),
        (
            "spec://org.b/b/boot/dep#root",
            "# Dep {#root}\n\n##DEP-RULE dep body\n",
        ),
        (
            "spec://org.c/c/boot/embed#piece",
            "##EMBED-RULE embedded body\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/boot/entry#root").unwrap();

    assert_eq!(
        compile_static(&seed, &src).unwrap(),
        concat!(
            "<!-- vibe:begin spec://org.b/b/boot/dep#root -->\n",
            "# Dep {#root}\n\n",
            "##DEP-RULE dep body\n",
            "<!-- vibe:end spec://org.b/b/boot/dep#root -->\n",
            "<!-- vibe:begin spec://org.a/a/boot/entry#root -->\n",
            "# Entry {#root}\n\n",
            "<!-- embed: spec://org.c/c/boot/embed#piece -->\n",
            "##EMBED-RULE embedded body\n",
            "<!-- /embed: spec://org.c/c/boot/embed#piece -->\n\n",
            "Uses (#DEP-RULE) and (#EMBED-RULE).\n\n",
            "##ENTRY-RULE entry body\n",
            "<!-- vibe:end spec://org.a/a/boot/entry#root -->\n",
        )
    );

    assert_eq!(
        compile_static_qualified(&seed, &src).unwrap(),
        (
            concat!(
                "<!-- vibe:begin spec://org.b/b/boot/dep#root -->\n",
                "# Dep {#org-b--b--root}\n\n",
                "##org-b--b--DEP-RULE dep body\n",
                "<!-- vibe:end spec://org.b/b/boot/dep#root -->\n",
                "<!-- vibe:begin spec://org.a/a/boot/entry#root -->\n",
                "# Entry {#org-a--a--root}\n\n",
                "<!-- embed: spec://org.c/c/boot/embed#piece -->\n",
                "##org-a--a--EMBED-RULE embedded body\n",
                "<!-- /embed: spec://org.c/c/boot/embed#piece -->\n\n",
                "Uses (#org-b--b--DEP-RULE) and (#org-a--a--EMBED-RULE).\n\n",
                "##org-a--a--ENTRY-RULE entry body\n",
                "<!-- vibe:end spec://org.a/a/boot/entry#root -->\n",
            )
            .to_string(),
            vec![
                (
                    "org.b/b".to_string(),
                    RenameEntry {
                        original: "root".to_string(),
                        qualified: "org-b--b--root".to_string(),
                    },
                ),
                (
                    "org.b/b".to_string(),
                    RenameEntry {
                        original: "DEP-RULE".to_string(),
                        qualified: "org-b--b--DEP-RULE".to_string(),
                    },
                ),
                (
                    "org.a/a".to_string(),
                    RenameEntry {
                        original: "root".to_string(),
                        qualified: "org-a--a--root".to_string(),
                    },
                ),
                (
                    "org.a/a".to_string(),
                    RenameEntry {
                        original: "EMBED-RULE".to_string(),
                        qualified: "org-a--a--EMBED-RULE".to_string(),
                    },
                ),
                (
                    "org.a/a".to_string(),
                    RenameEntry {
                        original: "ENTRY-RULE".to_string(),
                        qualified: "org-a--a--ENTRY-RULE".to_string(),
                    },
                ),
            ],
        )
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn one_seed_modes_preserve_the_exact_graph_error_variant_and_message() {
    let key = "spec://org.demo/pkg/boot/entry#root";
    let missing = "spec://org.demo/dep/boot/missing#root";
    // The later B/C pair would make qualification fail on `SHARED`; the first
    // unresolved use must win in both modes, pinning graph-before-qualify error
    // precedence as well as the public variant and Display text.
    let src = MockSource::new(&[
        (
            key,
            &format!(
                "# Entry {{#root}}\nSee (#SHARED).\n#use {missing}\n#use spec://org.b/b/boot/b#root\n#use spec://org.c/c/boot/c#root\n"
            ),
        ),
        (
            "spec://org.b/b/boot/b#root",
            "# B {#root}\n##SHARED b's rule\n",
        ),
        (
            "spec://org.c/c/boot/c#root",
            "# C {#root}\n##SHARED c's rule\n",
        ),
    ]);
    let seed = SpecAddress::parse(key).unwrap();

    for error in [
        compile_static(&seed, &src).unwrap_err(),
        compile_static_qualified(&seed, &src).unwrap_err(),
    ] {
        assert_eq!(
            error.to_string(),
            "cannot resolve use spec://org.demo/dep/boot/missing#root: not in mock"
        );
        match error {
            CompileError::UseGraph(UseGraphError::Unresolved { addr, reason }) => {
                assert_eq!(addr, missing);
                assert_eq!(reason, "not in mock");
            }
            other => panic!("expected the exact UseGraph::Unresolved shape, got {other:?}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn qualified_error_keeps_the_exact_candidate_order_and_message() {
    let src = MockSource::new(&[
        (
            "spec://org.a/a/boot/entry#root",
            "# A {#root}\nSee (#SHARED).\n#use spec://org.z/z/boot/z#root\n#use spec://org.b/b/boot/b#root\n",
        ),
        (
            "spec://org.z/z/boot/z#root",
            "# Z {#root}\n##SHARED z's rule\n",
        ),
        (
            "spec://org.b/b/boot/b#root",
            "# B {#root}\n##SHARED b's rule\n",
        ),
    ]);
    let seed = SpecAddress::parse("spec://org.a/a/boot/entry#root").unwrap();
    let error = compile_static_qualified(&seed, &src).unwrap_err();

    assert_eq!(
        error.to_string(),
        "ambiguous short link `SHARED`: defined by org-b--b--SHARED (org.b/b), org-z--z--SHARED (org.z/z)"
    );
    match error {
        CompileError::AmbiguousShortLink { label, candidates } => {
            assert_eq!(label, "SHARED");
            assert_eq!(
                candidates,
                vec![
                    "org-b--b--SHARED (org.b/b)".to_string(),
                    "org-z--z--SHARED (org.z/z)".to_string(),
                ]
            );
        }
        other => panic!("expected AmbiguousShortLink, got {other:?}"),
    }
}

/// One physical Markdown document behind two logical section addresses. The
/// oracle deliberately observes only the selected fragments and their emitted
/// order, leaving parse/read caching free to change in the refactor.
struct SameDocumentSource {
    seed: String,
    document: String,
}

impl SectionSource for SameDocumentSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let key = addr.without_pin();
        if key == "spec://org.a/a/boot/entry#root" {
            return Ok(self.seed.clone());
        }
        if !key.starts_with("spec://org.b/b/common/shared#") {
            return Err(format!("no physical document for {key}"));
        }
        let tree = DocTree::parse(&self.document);
        tree.resolve_path(&addr.anchor)
            .map(|node| tree.text(node))
            .ok_or_else(|| format!("anchor not found for {key}"))
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#pipeline")]
fn two_anchors_of_one_physical_document_keep_independent_fragment_semantics() {
    // Both logical addresses select sibling fragments from ONE physical body.
    // Pin the two emitted fragments and their declaration order, but not how
    // many parser/source calls obtain them: R3 remains free to cache the doc.
    let source = SameDocumentSource {
        seed: concat!(
            "# Entry {#root}\n",
            "#use spec://org.b/b/common/shared#one\n",
            "#use spec://org.b/b/common/shared#two\n",
        )
        .to_string(),
        document: concat!(
            "# Shared {#root}\n\n",
            "## One {#one}\n\nONE_FRAGMENT\n\n",
            "## Two {#two}\n\nTWO_FRAGMENT\n",
        )
        .to_string(),
    };
    let seed = SpecAddress::parse("spec://org.a/a/boot/entry#root").unwrap();
    let result = compile_static(&seed, &source).unwrap();

    assert_eq!(
        result,
        concat!(
            "<!-- vibe:begin spec://org.b/b/common/shared#one -->\n",
            "## One {#one}\n\nONE_FRAGMENT\n",
            "<!-- vibe:end spec://org.b/b/common/shared#one -->\n",
            "<!-- vibe:begin spec://org.b/b/common/shared#two -->\n",
            "## Two {#two}\n\nTWO_FRAGMENT\n",
            "<!-- vibe:end spec://org.b/b/common/shared#two -->\n",
            "<!-- vibe:begin spec://org.a/a/boot/entry#root -->\n",
            "# Entry {#root}\n",
            "<!-- vibe:end spec://org.a/a/boot/entry#root -->\n",
        )
    );
}
