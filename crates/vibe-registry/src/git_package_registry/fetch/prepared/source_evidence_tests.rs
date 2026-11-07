//! Captured source observations and the incumbent held-mirror behavior.
#![cfg(all(test, unix))]
use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS"
)]
fn prepared_report_is_current_owned_redacted_data_not_another_source_read() {
    let f = Fixture::new();
    let commit = f.publish("1.0.0", "HELD", false);
    let mut r = f.registry();
    r.name = "current-alias".into();
    let mut prepared = r
        .prepare_in_place(&f.selected(&r, "1.0.0"), &f.slot)
        .unwrap();
    r.name = "later-alias".into();
    // Exercise the display boundary with synthetic credential-bearing owner values.
    let held = Arc::get_mut(&mut prepared.0).unwrap();
    held.canonical = "https://u:SYN_CANARY@canonical.invalid/pkg?token=SYN_QUERY".into();
    held.origin = "https://u:SYN_CANARY@actual.invalid/pkg#SYN_FRAGMENT".into();
    held.authenticated_origin = Some("SYN_AUTHENTICATED_TRANSPORT".into());
    fs::rename(&f.upstream, f.upstream.with_extension("gone")).unwrap();
    let evidence = prepared.source_evidence();
    assert_eq!(evidence.registry.as_deref(), Some("current-alias"));
    assert_eq!(
        evidence.source_url.as_deref(),
        Some("https://canonical.invalid/pkg")
    );
    assert_eq!(
        evidence.fetched_from.as_deref(),
        Some("https://actual.invalid/pkg")
    );
    assert_eq!(evidence.resolved_commit.as_deref(), Some(commit.as_str()));
    assert!(!serde_json::to_string(&evidence).unwrap().contains("SYN_"));
    assert!(!f.slot.exists());
}

#[test]
fn mirror_switch_is_prepared_before_admission_and_applies_only_its_held_commit() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    let mirror = Fixture::new();
    let commit = mirror.publish("1.1.0", "MIRROR", false);
    let r = registry_with_mirrors(
        &f.cache,
        f.upstream.parent().unwrap().to_str().unwrap(),
        NamingConvention::Fqdn,
        vec![
            mirror
                .upstream
                .parent()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ],
        f.backend.clone(),
    );
    let mut selected = f.selected(&r, "1.0.0");
    selected.version = semver::Version::new(1, 1, 0);
    let old = git(&f.slot, &["rev-parse", "HEAD"]);
    let prepared = r.prepare_in_place(&selected, &f.slot).unwrap();
    assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), old);
    assert_eq!(prepared.resolved_commit(), commit);
    let evidence = prepared.source_evidence();
    assert_eq!(evidence.registry.as_deref(), Some(r.name.as_str()));
    assert_eq!(
        evidence.source_url.as_deref(),
        Some(f.upstream.to_str().unwrap())
    );
    assert_eq!(
        evidence.fetched_from.as_deref(),
        Some(mirror.upstream.to_str().unwrap())
    );
    assert_eq!(evidence.resolved_commit.as_deref(), Some(commit.as_str()));
    assert_eq!(evidence.store_reused, Some(false));
    fs::rename(&mirror.upstream, mirror.upstream.with_extension("gone")).unwrap();
    assert_eq!(
        prepared.source_evidence(),
        evidence,
        "observation must use retained facts without another source read"
    );
    prepared.materialise().unwrap();
    drop(prepared);
    assert_eq!(
        fs::read_to_string(f.slot.join("payload.txt")).unwrap(),
        "MIRROR"
    );
    assert!(!f.slot.join(".git/objects/info/alternates").exists());
    git(&f.slot, &["fsck", "--full"]);
}
