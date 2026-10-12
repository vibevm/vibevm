use super::*;
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
const A: &str = "[[items]]\nid='work-a'\ntouches=['src/a.rs']\nneeds=[{kind='implementation',target='spec://org.fixture/reusable-intent/FEAT-001#DELIVERY'}]\n";
const B: &str = "[[items]]\nid='work-b'\npredecessors=['work-a']\nneeds=[{kind='verification',target='tests/a.rs'}]\n";
const C: &str = "[[items]]\nid='work-c'\ntouches=['src/c.rs']\n";
fn change(text: &str) -> ChangeManifest {
    let c = ChangeContext::for_node(
        &Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n")
            .unwrap(),
        ".",
        Some(Path::new("docs/custom-specs")),
    )
    .unwrap();
    ChangeManifest::parse_str(text, &c, &c.directory(&ChangeId::parse(ID).unwrap())).unwrap()
}
fn graph(parts: &[&str]) -> WorkGraph {
    WorkGraph::parse_documents(&[&format!("schema=1\n{}", parts.concat())]).unwrap()
}
fn identity(change: &ChangeManifest, a: &[u8], b: &[u8]) -> IntentIdentity {
    change
        .intent_identity(&[
            NormativeInput {
                path: "intent.md",
                bytes: a,
            },
            NormativeInput {
                path: "acceptance.md",
                bytes: b,
            },
        ])
        .unwrap()
}
#[test]
fn connected_structural_graph() {
    let c = change(TEXT);
    let g = graph(&[B, A, C]);
    assert_eq!(
        g.items().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["work-a", "work-b", "work-c"]
    );
    let b = g.item("work-b").unwrap();
    assert_eq!(b.predecessors, ["work-a"]);
    assert_eq!(b.needs[0].kind, WorkNeedKind::Verification);
    assert_eq!(b.needs[0].target, "tests/a.rs");
    assert_eq!(g.identity(&c).recipe(), WORK_RECIPE);
    assert_ne!(
        g.identity(&c).digest(),
        identity(&c, b"Intent", b"Accept").digest()
    );
}
#[test]
fn canonical_graph_order_and_selected_contribution() {
    let c = change(TEXT);
    let multiple = "[[items]]\nid='work-b'\npredecessors=['work-c','work-a']\ntouches=['src/z.rs','src/b.rs']\nneeds=[{kind='verification',target='tests/z.rs'},{kind='implementation',target='src/z.rs'},{kind='documentation',target='docs/a.md'}]\n";
    let reversed = multiple.replace("['work-c','work-a']", "['work-a','work-c']")
        .replace("['src/z.rs','src/b.rs']", "['src/b.rs','src/z.rs']")
        .replace("[{kind='verification',target='tests/z.rs'},{kind='implementation',target='src/z.rs'},{kind='documentation',target='docs/a.md'}]", "[{kind='documentation',target='docs/a.md'},{kind='implementation',target='src/z.rs'},{kind='verification',target='tests/z.rs'}]");
    let multi = graph(&[A, multiple, C]);
    assert_eq!(multi.identity(&c), graph(&[C, &reversed, A]).identity(&c));
    assert_eq!(
        multi
            .item("work-b")
            .unwrap()
            .needs
            .iter()
            .map(|n| n.kind.as_str())
            .collect::<Vec<_>>(),
        ["documentation", "implementation", "verification"]
    );
    let first = graph(&[A, B, C]);
    let reordered = graph(&[C, B, A]);
    assert_eq!(first.identity(&c), reordered.identity(&c));
    assert_eq!(
        first.contribution_identity(&c, &["work-b"]).unwrap(),
        reordered.contribution_identity(&c, &["work-b"]).unwrap()
    );
    let edited = graph(&[A, B, &C.replace("src/c.rs", "src/other.rs")]);
    assert_ne!(first.identity(&c), edited.identity(&c));
    assert_eq!(
        first.contribution_identity(&c, &["work-b"]).unwrap(),
        edited.contribution_identity(&c, &["work-b"]).unwrap()
    );
    let ancestor = graph(&[&A.replace("src/a.rs", "src/changed.rs"), B, C]);
    assert_ne!(
        first.contribution_identity(&c, &["work-b"]).unwrap(),
        ancestor.contribution_identity(&c, &["work-b"]).unwrap()
    );
    assert_eq!(
        first
            .contribution_identity(&c, &["work-b", "work-c"])
            .unwrap(),
        first
            .contribution_identity(&c, &["work-c", "work-b"])
            .unwrap()
    );
}
#[test]
fn graph_refuses_cycle_duplicate_unknown_and_state() {
    let parse = |s: &str| WorkGraph::parse_documents(&[s]).unwrap_err();
    let duplicate = format!("schema=1\n{A}{A}");
    assert_eq!(parse(&duplicate).field, "work.items.id");
    assert_eq!(
        WorkGraph::parse_documents(&[&format!("schema=1\n{A}"), &format!("schema=1\n{A}")])
            .unwrap_err()
            .field,
        "work.items.id"
    );
    let cycle = format!(
        "schema=1\n{}{B}",
        A.replace("id='work-a'", "id='work-a'\npredecessors=['work-b']")
    );
    assert_eq!(parse(&cycle).kind, ChangeErrorKind::Cycle);
    assert_eq!(
        parse(&format!("schema=1\n{B}")).field,
        "work.items.predecessors"
    );
    for field in ["done=true", "owner='person'", "current=true"] {
        assert_eq!(
            parse(&format!("schema=1\n{C}{field}\n")).kind,
            ChangeErrorKind::Work
        );
        assert_eq!(
            parse(&format!("schema=1\n{field}\n{C}")).kind,
            ChangeErrorKind::Work
        );
    }
    for (s, field) in [
        (A.replace("src/a.rs", "../outside"), "work.items.touches"),
        (
            B.replace("['work-a']", "['work-a','work-a']"),
            "work.items.predecessors",
        ),
        (C.replace("work-c", "bad id"), "work.items.id"),
    ] {
        assert_eq!(parse(&format!("schema=1\n{A}{s}")).field, field);
    }
    assert_eq!(parse("schema=9\n").kind, ChangeErrorKind::UnknownEpoch);
    let g = graph(&[A]);
    for roots in [vec!["work-no"], vec!["work-a", "work-a"]] {
        assert_eq!(
            g.contribution_identity(&change(TEXT), &roots)
                .unwrap_err()
                .field,
            "work.selection"
        );
    }
}
#[test]
fn framed_intent_ignores_cosmetics_work_and_closure() {
    let c = change(TEXT);
    let original = identity(&c, b"Intent", b"Accept");
    let cosmetic = change(
        &TEXT
            .replace("first-title", "renamed")
            .replace("Initial user title", "Another title")
            .replace("Preserve me", "Other metadata"),
    );
    assert_eq!(original, identity(&cosmetic, b"Intent", b"Accept"));
    let roles = TEXT.replace("path = \"closure.md\"", "path = \"new-closure.xml\"");
    assert_eq!(original, identity(&change(&roles), b"Intent", b"Accept"));
    let with_work = format!(
        "{TEXT}\n[[documents]]\nrole='work'\npath='work.toml'\nnormative=false\ninput=true\n"
    );
    assert_eq!(
        original,
        identity(&change(&with_work), b"Intent", b"Accept")
    );
}
#[test]
fn normative_edit_and_binding_change_invalidate_intent() {
    let c = change(TEXT);
    let original = identity(&c, b"Intent", b"Accept");
    assert_ne!(original, identity(&c, b"Edited intent", b"Accept"));
    assert_ne!(original, identity(&c, b"Intent", b"Edited acceptance"));
    let capture = change(&TEXT.replace(&"b".repeat(64), &"c".repeat(64)));
    assert_ne!(original, identity(&capture, b"Intent", b"Accept"));
    let context = ChangeContext::for_node(
        &Manifest::parse_str("[project]\ngroup='org.fixture'\nname='beta'\nversion='1.0.0'\n")
            .unwrap(),
        ".",
        Some(Path::new("docs/custom-specs")),
    )
    .unwrap();
    let other = ChangeManifest::parse_str(TEXT, &context, &context.directory(c.id())).unwrap();
    assert_ne!(original, identity(&other, b"Intent", b"Accept"));
    assert_ne!(graph(&[A]).identity(&c), graph(&[A]).identity(&other));
    assert_eq!(
        c.intent_identity(&[]).unwrap_err().kind,
        ChangeErrorKind::Input
    );
    assert_eq!(
        c.intent_identity(&[
            NormativeInput {
                path: "intent.md",
                bytes: b"Intent"
            },
            NormativeInput {
                path: "acceptance.md",
                bytes: b"Accept"
            },
            NormativeInput {
                path: "closure.md",
                bytes: b"Closed"
            },
        ])
        .unwrap_err()
        .field,
        "intent.inputs"
    );
    assert_eq!(
        c.intent_identity(&[
            NormativeInput {
                path: "intent.md",
                bytes: b"x"
            },
            NormativeInput {
                path: "intent.md",
                bytes: b"x"
            }
        ])
        .unwrap_err()
        .kind,
        ChangeErrorKind::Input
    );
}
#[test]
fn framing_is_unambiguous_and_versioned() {
    let c = change(TEXT);
    assert_ne!(identity(&c, b"ab", b"c"), identity(&c, b"a", b"bc"));
    // Fixed independently framed recipe vector retained in B2 evidence.
    assert_eq!(
        identity(&c, b"ab", b"c").digest(),
        "sha256:5b9039445a19da392f03766344d84e49dc0cbc9836988972db3dfbefde155790"
    );
    let g = graph(&[A, B]);
    assert_ne!(
        g.identity(&c).digest(),
        g.contribution_identity(&c, &["work-b"]).unwrap().digest()
    );
    assert_eq!(identity(&c, b"ab", b"c").recipe(), INTENT_RECIPE);
    assert_eq!(
        g.contribution_identity(&c, &["work-b"]).unwrap().recipe(),
        WORK_CONTRIBUTION_RECIPE
    );
}
