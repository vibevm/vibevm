use crate::source_query::derivatives::DerivativeCatalog;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

#[test]
fn equal_local_bytes_keep_distinct_base_and_result_after_sources_disappear() {
    let f = Fixture::new("vibe-spec");
    let current = crate::derivatives::capture_retained(&f.base, f.binding.base()).unwrap();
    let first = crate::derivatives::prepare_binding(&f.text, Some(&current))
        .unwrap()
        .value;
    let mut base = f.binding.base().clone();
    base.files[0]
        .bytes
        .extend_from_slice(b"\nchanged upstream\n");
    let binding = DerivationBinding::new(
        base.clone(),
        f.binding.local().clone(),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    fs::write(
        f.base.root_path().join(&base.files[0].path),
        &base.files[0].bytes,
    )
    .unwrap();
    let current = crate::derivatives::capture_retained(&f.base, &base).unwrap();
    let second =
        crate::derivatives::prepare_binding(&toml::to_string(&binding).unwrap(), Some(&current))
            .unwrap()
            .value;
    assert_eq!(
        first.record().local_identity,
        second.record().local_identity
    );
    assert_ne!(first.record().base_identity, second.record().base_identity);
    assert_ne!(
        first.record().result_identity,
        second.record().result_identity
    );
    let a = DerivativeCatalog::new(vec![first.record().clone()]).unwrap();
    let b = DerivativeCatalog::new(vec![second.record().clone()]).unwrap();
    let retained = a.inspect(&first.record().result_identity).unwrap();
    fs::remove_dir_all(f.base.root_path()).unwrap();
    fs::remove_dir_all(f.local.root_path()).unwrap();
    assert_eq!(
        a.inspect(&first.record().result_identity).unwrap(),
        retained
    );
    assert!(b.inspect(&first.record().result_identity).is_err());
    assert!(!f._home.path().join("DO_NOT_EXECUTE").exists());
}

#[test]
fn unmapped_native_emission_never_invents_inverse_source_or_parent() {
    // Native/foreign emitted bytes without a compiler carrier are deliberately
    // inspected as a captured summary, with unknown authored joins.
    let p = super::protocol::project(
        "native-output",
        super::protocol::CapturedKind::Static,
        b"<!-- vibe:static native/owner \xe2\x80\x94 generated.md -->\nopaque transformed text\n",
    )
    .unwrap();
    assert!(p.graph.carrier.is_none());
    assert_eq!(
        p.graph.contributions[0].occurrences[0].state,
        vibe_spec::ReferenceState::Unknown
    );
    assert!(!p.graph.gaps.is_empty());
    assert!(matches!(
        p.measurement.totals.inclusive_bytes,
        super::measurement::Count::Unavailable { .. }
    ));
}
