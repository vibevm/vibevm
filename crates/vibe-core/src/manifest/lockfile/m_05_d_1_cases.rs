use super::*;
fn record() -> LockedConfiguration {
    let requested = r#"{"selection":"a","dormant":"later"}"#.to_owned();
    LockedConfiguration {
        epoch: 1,
        request_digest: LockedConfiguration::digest(&requested),
        requested,
        resolved: r#"{"active":["a"],"dormant":["later"],"closure":["a","derived"]}"#.into(),
    }
}
#[test]
fn lock_roundtrip_preserves_active_dormant_and_feature_closure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.lock");
    let mut lock = Lockfile::empty("test", "now");
    lock.meta.configuration = Some(record());
    lock.write(&path).unwrap();
    let read = Lockfile::read(&path).unwrap();
    assert_eq!(lock, read);
    assert_ne!(
        read.meta.configuration.as_ref().unwrap().requested,
        read.meta.configuration.as_ref().unwrap().resolved
    );
}
#[test]
fn legacy_lock_is_recognized_and_unknown_epoch_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.lock");
    let lock = Lockfile::empty("test", "now");
    lock.write(&path).unwrap();
    assert!(Lockfile::read(&path).unwrap().meta.configuration.is_none());
    let mut lock = lock;
    let mut r = record();
    r.epoch = 99;
    lock.meta.configuration = Some(r);
    // Bypass the writer to exercise hostile reader input.
    std::fs::write(&path, toml::to_string(&lock).unwrap()).unwrap();
    assert!(
        Lockfile::read(&path)
            .unwrap_err()
            .to_string()
            .contains("epoch")
    );
}
#[test]
fn derived_selection_cannot_supply_authored_authority() {
    let mut r = record();
    r.requested.clear();
    r.request_digest = LockedConfiguration::digest(&r.requested);
    assert!(r.validate().is_err());
    let mut r = record();
    r.request_digest.clear();
    assert!(r.validate().is_err());
}
