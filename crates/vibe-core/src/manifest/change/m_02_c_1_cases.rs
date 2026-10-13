use super::*;
const LEGACY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
const BOUND: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/bound-origin.toml"
));
const INVALID: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/semantic-gap/invalid-content-binding.toml"
));
fn parsed(text: &str) -> Result<ChangeManifest, ChangeError> {
    let context = ChangeContext::for_node(
        &Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n")
            .unwrap(),
        ".",
        Some(Path::new("docs/custom-specs")),
    )
    .unwrap();
    ChangeManifest::parse_str(
        text,
        &context,
        &context.directory(&ChangeId::parse("chg-0123456789abcdef0123456789abcdef").unwrap()),
    )
}
fn identity(manifest: &ChangeManifest) -> IntentIdentity {
    manifest
        .intent_identity(&[
            NormativeInput {
                path: "intent.md",
                bytes: b"ab",
            },
            NormativeInput {
                path: "acceptance.md",
                bytes: b"c",
            },
        ])
        .unwrap()
}
#[test]
fn bound_domain_preserves_legacy_vector_and_work_identity() {
    let old = parsed(LEGACY).unwrap();
    let bound = parsed(BOUND).unwrap();
    let original = identity(&old);
    assert_eq!(original.recipe(), INTENT_RECIPE);
    assert_eq!(
        original.digest(),
        "sha256:5b9039445a19da392f03766344d84e49dc0cbc9836988972db3dfbefde155790"
    );
    let new = identity(&bound);
    assert_eq!(new.recipe(), BOUND_INTENT_RECIPE);
    assert_ne!(original, new);
    assert_eq!(
        new.digest(),
        "sha256:51819f8406617674a5fee60afbe995a7e3cce43c81e719da8be6833ca393ab03"
    );
    let changed = parsed(&BOUND.replace(&"c".repeat(64), &"d".repeat(64))).unwrap();
    assert_ne!(new, identity(&changed));
    let graph =
        WorkGraph::parse_documents(&["schema=1\n[[items]]\nid='work-a'\ntouches=['src/a.rs']\n"])
            .unwrap();
    assert_eq!(graph.identity(&old), graph.identity(&bound));
    assert_eq!(
        graph.contribution_identity(&old, &["work-a"]).unwrap(),
        graph.contribution_identity(&bound, &["work-a"]).unwrap()
    );
    let other = format!(
        "[[origins]]\npackage='feat:org.fixture/other@=1.0.0'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.fixture/other/FEAT-OTHER#REQUIRED'\nsha256='sha256:{}'\n",
        "d".repeat(64),
        "e".repeat(64)
    );
    let partial = parsed(&BOUND.replacen("[[documents]]", &(other + "[[documents]]"), 1)).unwrap();
    assert!(partial.origins()[0].selected_content_hash.is_some());
    assert!(partial.origins()[1].selected_content_hash.is_none());
    assert_eq!(identity(&partial).recipe(), BOUND_INTENT_RECIPE);
    println!("BOUND_VECTOR {}", new.digest());
}
#[test]
fn native_corpus_and_invalid_binding_follow_shared_parser() {
    for (text, bound) in [(LEGACY, false), (BOUND, true)] {
        let value = parsed(text).unwrap();
        assert_eq!(value.origins()[0].selected_content_hash.is_some(), bound);
    }
    let error = parsed(INVALID).unwrap_err();
    assert_eq!(error.kind, ChangeErrorKind::Origin);
    assert_eq!(error.field, "origins.selected_content_hash");
    // Existing deny-unknown old shape is explicitly not a bidirectional reader:
    // an older native reader would refuse the newly understood optional field.
    let unknown = BOUND.replace("selected_content_hash", "unknown_binding");
    assert_eq!(parsed(&unknown).unwrap_err().kind, ChangeErrorKind::Syntax);
    let epoch = BOUND.replacen("schema = 1", "schema = 99", 1);
    assert_eq!(
        parsed(&epoch).unwrap_err().kind,
        ChangeErrorKind::UnknownEpoch
    );
}
