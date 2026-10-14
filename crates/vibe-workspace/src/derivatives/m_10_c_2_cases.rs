use super::*;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

fn exercise(family: &str) {
    let f = Fixture::new(family);
    let old = capture_retained(&f.base, f.binding.base()).unwrap();
    let local = capture_retained(&f.local, f.binding.local()).unwrap();
    let mut retained = f.binding.base().clone();
    retained.files[0].bytes.extend_from_slice(b"\n");
    f.base
        .write_atomic(&retained.files[0].path, &retained.files[0].bytes)
        .unwrap();
    let new = capture_retained(&f.base, &retained).unwrap();
    assert!(prepare_binding(&f.text, Some(&new)).is_err());
    assert!(RebaseReview::compare(&f.text, None, &new, &local).is_err());
    let compare = || {
        RebaseReview::compare(&f.text, Some(&old), &new, &local)
            .unwrap()
            .value
    };
    let review = compare();
    assert_eq!(review.disposition, RebaseDisposition::RebaseRequired);
    assert_eq!(review.upstream_changes.len(), 1);
    assert!(!review.local_changes.is_empty());
    assert_eq!(review.old_base(), f.binding.base());
    let next = DerivationBinding::new(
        retained.clone(),
        f.binding.local().clone(),
        retained.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let text = toml::to_string(&next).unwrap();
    assert!(compare().select(&[&text, &text], &new, &local).is_err());
    assert!(compare().select(&[&text], &old, &local).is_err());
    let selected = review.select(&[&text], &new, &local).unwrap();
    assert_eq!(selected.effects, Effects::NoExternalEffects);
    assert_eq!(selected.value.previous().record().binding_toml, f.text);
    assert_ne!(
        selected.value.previous().record().provenance_identity,
        selected.value.selected().record().provenance_identity
    );
    assert_eq!(
        selected.value.selected().binding().local().files,
        f.expected
    );
    assert_eq!(selected.value.selected().record().binding_toml, text);
}
#[test]
fn commonmark_explicit_rebase() {
    exercise("commonmark")
}
#[test]
fn xml_data_explicit_rebase() {
    exercise("xml-data")
}
#[test]
fn vibe_spec_explicit_rebase() {
    exercise("vibe-spec")
}
#[test]
fn opaque_explicit_rebase() {
    exercise("opaque-file")
}
#[test]
fn nested_tree_explicit_rebase() {
    exercise("nested-tree")
}

#[test]
fn new_version_and_explicitly_saved_local_revision_are_reviewable() {
    let f = Fixture::new("opaque-file");
    let old = capture_retained(&f.base, f.binding.base()).unwrap();
    let mut base = f.binding.base().clone();
    base.owner = vibe_core::PackageRef::parse("flow:org.example/upstream@=2.0.0").unwrap();
    let new = capture_retained(&f.base, &base).unwrap();
    let mut authored = f.binding.local().clone();
    authored.files[0].bytes.extend_from_slice(b"revision");
    f.local
        .write_atomic(&authored.files[0].path, &authored.files[0].bytes)
        .unwrap();
    let local = capture_retained(&f.local, &authored).unwrap();
    let review = RebaseReview::compare(&f.text, Some(&old), &new, &local)
        .unwrap()
        .value;
    let selected = DerivationBinding::new(
        base.clone(),
        authored,
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let text = toml::to_string(&selected).unwrap();
    assert_eq!(
        review
            .select(&[&text], &new, &local)
            .unwrap()
            .value
            .selected()
            .binding(),
        &selected
    );
}
