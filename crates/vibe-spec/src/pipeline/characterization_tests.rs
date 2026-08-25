//! Byte-level characterization of the one-seed compiler that R3 will split
//! into explicit IR levels and named passes. These tests call only today's
//! public plain/qualified entry points: they are the before-refactor oracle,
//! not a sketch of the future types or their cardinality.

use specmark::verifies;

use super::tests::MockSource;
use super::*;
use crate::{DocTree, UseGraphError};

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
