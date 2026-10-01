use super::*;

#[test]
fn retained_byte_store_matches_vault_after_checkout_disappears() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let selected = candidate(&world);
    let (_vault_root, vault, manifest) = vault(&world);
    let (_store_root, store) = retained_store(&binary, &world.checkout, selected.indexed_oid.len());
    let initial = compare_retained_byte_store(
        &vault,
        &manifest,
        &store,
        &selected.path,
        &selected.indexed_oid,
        &binary,
        limits(),
    );
    assert!(
        matches!(
            initial,
            RawInspection::Exact(RawComparison { files: 2, .. })
        ),
        "{initial:?}"
    );
    fs::rename(
        &world.checkout,
        world.package.root_path().join("retired-checkout"),
    )
    .unwrap();
    assert!(matches!(
        compare_retained_byte_store(
            &vault,
            &manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Exact(RawComparison { files: 2, .. })
    ));
    let shallow = store.root_path().join("shallow");
    if !shallow.exists() {
        fs::write(&shallow, format!("{}\n", selected.indexed_oid)).unwrap();
    }
    assert!(matches!(
        compare_retained_byte_store(
            &vault,
            &manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Exact(_)
    ));
    fs::write(
        &shallow,
        format!("{}\n", "a".repeat(selected.indexed_oid.len())),
    )
    .unwrap();
    assert!(matches!(
        compare_retained_byte_store(
            &vault,
            &manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Unavailable(_)
    ));
}

#[test]
fn retained_byte_store_refuses_changed_vault_or_missing_blob() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; local submodule fixture skipped");
        return;
    };
    let world = world(&binary);
    let selected = candidate(&world);
    let (_vault_root, vault, manifest) = vault(&world);
    let (store_root, store) = retained_store(&binary, &world.checkout, selected.indexed_oid.len());
    fs::write(
        vault.root_path().join("payload/vendor/dep/root.txt"),
        b"tampered\n",
    )
    .unwrap();
    assert!(matches!(
        compare_retained_byte_store(
            &vault,
            &manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Unavailable(_)
    ));
    let (_fresh_root, fresh_vault, fresh_manifest) = self::vault(&world);
    let blob = git(&binary, &world.checkout, &["rev-parse", "HEAD:root.txt"]);
    let loose = store_root
        .path()
        .join(format!("objects/{}/{}", &blob[..2], &blob[2..]));
    if loose.exists() {
        fs::remove_file(loose).unwrap();
    } else {
        let pack = store_root.path().join("objects/pack");
        let file = fs::read_dir(&pack)
            .unwrap()
            .filter_map(|r| r.ok())
            .map(|r| r.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "pack"))
            .expect("retained pack absent");
        fs::remove_file(file).unwrap();
    }
    assert!(matches!(
        compare_retained_byte_store(
            &fresh_vault,
            &fresh_manifest,
            &store,
            &selected.path,
            &selected.indexed_oid,
            &binary,
            limits()
        ),
        RawInspection::Unavailable(_)
    ));
}
