//! Retained candidate routes preserve incumbent unbound refresh semantics.
#![cfg(all(test, unix))]
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn retained_route_does_not_replace_the_unbound_slots_actual_origin() {
    let canonical = Fixture::new();
    canonical.publish("1.0.0", "BASE", false);
    canonical.seed();
    canonical.publish("1.1.0", "CANONICAL", false);
    let alternate = Fixture::new();
    let actual = alternate.publish("1.1.0", "ACTUAL", true);
    let registry = canonical.registry();
    let (resolved, route) = registry
        .resolve_with_route(&PackageRef::parse("org.vibevm/giant@=1.1.0").unwrap())
        .unwrap();
    assert!(route.is_some());
    git(
        &canonical.slot,
        &[
            "remote",
            "set-url",
            "origin",
            alternate.upstream.to_str().unwrap(),
        ],
    );
    let before_head = git(&canonical.slot, &["rev-parse", "HEAD"]);
    let before_payload = tree(&canonical.slot);
    let prepared = registry
        .prepare_in_place_on_route(&resolved, &canonical.slot, route.as_ref())
        .unwrap();
    assert!(prepared.manifest().configuration.is_some());
    assert_eq!(prepared.resolved_commit(), actual);
    assert_eq!(git(&canonical.slot, &["rev-parse", "HEAD"]), before_head);
    assert_eq!(tree(&canonical.slot), before_payload);
    assert_eq!(
        git(&canonical.slot, &["remote", "get-url", "origin"]),
        alternate.upstream.to_str().unwrap()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model"
)]
fn retained_route_keeps_unbound_warm_cache_refresh_and_failover_order() {
    let canonical = Fixture::new();
    canonical.publish("1.0.0", "BASE", false);
    let mirror = Fixture::new();
    let mirrored = mirror.publish("1.1.0", "MIRROR", true);
    let registry = registry_with_mirrors(
        &canonical.cache,
        canonical.upstream.parent().unwrap().to_str().unwrap(),
        NamingConvention::Fqdn,
        vec![
            mirror
                .upstream
                .parent()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        ],
        canonical.backend.clone(),
    );
    let store = tempfile::tempdir().unwrap();
    let initial = registry
        .fetch(&canonical.selected(&registry, "1.0.0"), store.path())
        .unwrap();
    let cache = initial.cache_dir;
    let hidden = canonical.upstream.with_extension("temporarily-away");
    fs::rename(&canonical.upstream, &hidden).unwrap();
    let (resolved, route) = registry
        .resolve_with_route(&PackageRef::parse("org.vibevm/giant@=1.1.0").unwrap())
        .unwrap();
    assert!(route.is_some());
    fs::rename(&hidden, &canonical.upstream).unwrap();
    canonical.publish("1.1.0", "CANONICAL", false);
    let missing = canonical._root.path().join("missing-origin.git");
    git(
        &cache,
        &["remote", "set-url", "origin", missing.to_str().unwrap()],
    );
    let result = registry
        .fetch_on_route(&resolved, store.path(), None, None, route.as_ref())
        .unwrap();
    let PackageAcquisition::Cached(cached) = result else {
        panic!("cache operation must return cached content")
    };
    assert_eq!(cached.resolved_commit.as_deref(), Some(mirrored.as_str()));
    assert!(cached.manifest.configuration.is_some());
    assert_eq!(
        fs::read_to_string(cached.cache_dir.join("payload.txt")).unwrap(),
        "MIRROR"
    );
    assert_eq!(
        git(&cached.cache_dir, &["remote", "get-url", "origin"]),
        mirror.upstream.to_str().unwrap()
    );
}
