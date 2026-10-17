use super::*;
use std::path::Path;
use vibe_core::manifest::{
    Manifest,
    change::{ChangeContext, ChangeErrorKind, ChangeId},
};
fn fixture() -> (String, ChangeContext, std::path::PathBuf) {
    let node =
        Manifest::parse_str("[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n")
            .unwrap();
    let context = ChangeContext::for_node(&node, ".", Some(Path::new("docs/custom"))).unwrap();
    let id = "chg-0123456789abcdef0123456789abcdef";
    let directory = context.directory(&ChangeId::parse(id).unwrap());
    let text = format!(
        "schema=1\n[change]\nid='{id}'\nselected_node='.'\nslug='cosmetic'\ntitle='Local'\n[[origins]]\npackage='feat:org.fixture/reusable-intent@=1.0.0'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.fixture/reusable-intent/FEAT-001#DELIVERY'\nsha256='sha256:{}'\n[[documents]]\nrole='intent'\npath='intent.md'\nnormative=true\ninput=true\n[[documents]]\nrole='acceptance'\npath='acceptance.md'\nnormative=true\ninput=true\n",
        "a".repeat(64),
        "b".repeat(64)
    );
    (text, context, directory)
}
#[test]
fn pure_retained_bytes_keep_cosmetics_separate_from_normative_intent() {
    let (text, context, dir) = fixture();
    let intent = dir.join("intent.md");
    let acceptance = dir.join("acceptance.md");
    let inputs = [
        (intent.as_path(), b"original local intent".as_slice()),
        (
            acceptance.as_path(),
            b"independent local acceptance".as_slice(),
        ),
    ];
    let original = analyze_change_bytes(&text, &context, &dir, &inputs).unwrap();
    let renamed = analyze_change_bytes(
        &text.replace("cosmetic", "renamed"),
        &context,
        &dir,
        &inputs,
    )
    .unwrap();
    assert_eq!(
        original.identities.as_ref().unwrap().intent,
        renamed.identities.as_ref().unwrap().intent
    );
    let changed = analyze_change_bytes(
        &text,
        &context,
        &dir,
        &[
            (intent.as_path(), b"changed local intent"),
            (acceptance.as_path(), inputs[1].1),
        ],
    )
    .unwrap();
    assert_ne!(
        original.identities.as_ref().unwrap().intent,
        changed.identities.as_ref().unwrap().intent
    );
    assert_eq!(
        original.identities.as_ref().unwrap().work,
        changed.identities.as_ref().unwrap().work
    );
}
#[test]
fn missing_and_duplicate_observations_refuse_exact_boundary() {
    let (text, context, dir) = fixture();
    let intent = dir.join("intent.md");
    let acceptance = dir.join("acceptance.md");
    assert!(
        matches!(analyze_change_bytes(&text,&context,&dir,&[(intent.as_path(),b"one")]),Err(SavedChangeError::MissingDocument(p)) if p=="acceptance.md")
    );
    assert!(
        matches!(analyze_change_bytes(&text,&context,&dir,&[(intent.as_path(),b"one"),(intent.as_path(),b"two"),(acceptance.as_path(),b"accept")]),Err(SavedChangeError::Declaration(e)) if e.kind==ChangeErrorKind::Input && e.field=="saved.inputs")
    );
}
