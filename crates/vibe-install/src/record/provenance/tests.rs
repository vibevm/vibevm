#![cfg(test)]
use super::*;
use std::fs;
use vibe_core::RoutePublication;

fn fixture(
    version: &str,
) -> std::result::Result<(tempfile::TempDir, Lockfile, Fetched), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("registry");
    let package = root.join(format!("org.example/demo/v{version}"));
    fs::create_dir_all(&package)?;
    fs::write(
        package.join("vibe.toml"),
        format!("[package]\ngroup='org.example'\nname='demo'\nkind='feat'\nversion='{version}'\n"),
    )?;
    let registry = vibe_registry::LocalRegistry::new(root)?;
    let selected = registry.resolve(&format!("org.example/demo@={version}").parse()?)?;
    let mut cached = registry.fetch(&selected, &temp.path().join("store"))?;
    cached.source_uri = "https://canonical.invalid/demo.git".into();
    cached.source_ref = Some(format!("v{version}"));
    let lock = Lockfile::parse_str(&format!(
        "[meta]\nschema_version=7\ngenerated_by='fixture'\ngenerated_at='fixed'\n[[package]]\nkind='feat'\ngroup='org.example'\nname='demo'\nversion='{version}'\nsource_url='https://canonical.invalid/demo.git'\nsource_ref='v{version}'\nsource_kind='registry'\ncontent_hash='{}'\n",
        cached.content_hash
    ))?;
    let fetched = Fetched {
        cached,
        feature_expansion: Default::default(),
        meta: crate::NodeInstallMeta {
            dependencies: vec![],
            is_root: true,
        },
        embedded_sources: vec![],
        in_place: None,
        configuration: None,
    };
    Ok((temp, lock, fetched))
}
const RECORDED: &str = "# project-recorded, not current-machine evidence\n[[package]]\npackage='org.example/demo'\nversion='1.0.0'\nfetched_from='https://project-mirror.invalid/demo.git'\nrow='shared mirror'\n";

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
fn two_developer_routes_keep_shared_bytes_but_operation_evidence_differs() {
    let (temp, lock, mut fetched) = fixture("1.0.0").unwrap();
    let path = temp.path().join("vibe.lock");
    lock.write(&path).unwrap();
    let provenance = vibe_core::lock_provenance::provenance_path(&path);
    fs::write(&provenance, RECORDED).unwrap();
    let lock_bytes = fs::read(&path).unwrap();
    for route in [
        "https://developer-one.invalid/private",
        "https://developer-two.invalid/private",
    ] {
        fetched.cached.fetched_from = Some(route.into());
        fetched.cached.route_publication = RoutePublication::Local;
        assert_eq!(
            fetched.cached.source_evidence().fetched_from.as_deref(),
            Some(route)
        );
        let records =
            crate::ProjectRecordPublication::capture(temp.path(), &lock, &lock, &[&fetched.cached])
                .unwrap();
        records.publish(&lock).unwrap();
        assert_eq!(fs::read(&path).unwrap(), lock_bytes);
        assert_eq!(fs::read_to_string(&provenance).unwrap(), RECORDED);
    }
    assert!(
        vibe_registry::locked_source_evidence(&lock.packages[0])
            .fetched_from
            .is_none(),
        "listing a lock cannot invent current acquisition from a shared report"
    );
}
#[test]
fn project_origin_is_required_even_when_a_local_url_matches_shared_policy() {
    let (_temp, lock, mut fetched) = fixture("1.0.0").unwrap();
    fetched.cached.fetched_from = Some("https://project-mirror.invalid/demo.git".into());
    fetched.cached.route_publication = RoutePublication::Local;
    assert_eq!(
        merge_fetch_provenance(std::slice::from_ref(&fetched), &lock, &lock, None).unwrap(),
        None
    );
    fetched.cached.route_publication = RoutePublication::Project;
    let text = merge_fetch_provenance(&[fetched], &lock, &lock, None)
        .unwrap()
        .unwrap();
    assert!(text.contains("project-mirror.invalid"));
}
#[test]
fn scoped_update_and_uninstall_prune_old_record_without_publishing_local_route() {
    let (temp, before, _) = fixture("1.0.0").unwrap();
    let (_new, after, mut fetched) = fixture("1.1.0").unwrap();
    let path = temp.path().join("vibe.lock");
    let provenance = vibe_core::lock_provenance::provenance_path(&path);
    before.write(&path).unwrap();
    fs::write(&provenance, RECORDED).unwrap();
    fetched.cached.fetched_from = Some("https://user-only.invalid/new".into());
    fetched.cached.route_publication = RoutePublication::Local;
    crate::ProjectRecordPublication::capture(temp.path(), &before, &after, &[&fetched.cached])
        .unwrap()
        .publish(&after)
        .unwrap();
    assert!(!provenance.exists());
    let latest = RECORDED.replace("1.0.0", "1.1.0");
    fs::write(&provenance, latest).unwrap();
    let mut removed = after.clone();
    removed.packages.clear();
    let report =
        crate::ProjectRecordPublication::capture(temp.path(), &after, &removed, &[]).unwrap();
    removed.write(&path).unwrap();
    report.publish_provenance().unwrap();
    assert!(!provenance.exists());
}
#[test]
fn report_conflict_restores_owned_lock_and_preserves_foreign_report() {
    let (temp, before, _) = fixture("1.0.0").unwrap();
    let (_new, after, _) = fixture("1.1.0").unwrap();
    let path = temp.path().join("vibe.lock");
    before.write(&path).unwrap();
    let old = fs::read(&path).unwrap();
    let provenance = vibe_core::lock_provenance::provenance_path(&path);
    fs::write(&provenance, RECORDED).unwrap();
    let publication =
        crate::ProjectRecordPublication::capture(temp.path(), &before, &after, &[]).unwrap();
    fs::write(&provenance, "foreign newer report\n").unwrap();
    assert!(publication.publish(&after).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
    assert_eq!(
        fs::read_to_string(&provenance).unwrap(),
        "foreign newer report\n"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
fn publication_refuses_removed_wrong_version_and_wrong_hash_observations_before_writes() {
    let (temp, before, mut fetched) = fixture("1.0.0").unwrap();
    fetched.cached.route_publication = RoutePublication::Project;
    fetched.cached.fetched_from = Some("https://project.example/new".into());
    let lock = temp.path().join("vibe.lock");
    let sidecar = temp.path().join("vibe.lock.provenance");
    before.write(&lock).unwrap();
    fs::write(&sidecar, RECORDED).unwrap();
    let original = fs::read(&lock).unwrap();
    let mut removed = before.clone();
    removed.packages.clear();
    let mut wrong_version = before.clone();
    wrong_version.packages[0].version = "1.1.0".parse().unwrap();
    let mut wrong_hash = before.clone();
    wrong_hash.packages[0].content_hash = vibe_core::ContentHash::parse("sha256:1234").unwrap();
    for after in [removed, wrong_version, wrong_hash] {
        let result = crate::ProjectRecordPublication::capture(
            temp.path(),
            &before,
            &after,
            &[&fetched.cached],
        );
        assert!(
            result.is_err(),
            "visibility cannot grant membership or content identity"
        );
        assert_eq!(fs::read(&lock).unwrap(), original);
        assert_eq!(fs::read_to_string(&sidecar).unwrap(), RECORDED);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn non_store_replay_preserves_authoritative_absent_kind_and_rejects_invented_kind() {
    let (temp, mut before, mut fetched) = fixture("1.0.0").unwrap();
    before.packages[0].source_kind = None;
    let resolver = vibe_registry::MultiRegistryResolver::new(
        vec![],
        vec![],
        vec![],
        std::sync::Arc::new(vibe_registry::ShellGit::new()),
        temp.path().join("unused-cache"),
    )
    .with_locked_packages(before.packages.clone())
    .with_offline(true);
    let selected = resolver
        .resolve(&"org.example/demo@=1.0.0".parse().unwrap())
        .unwrap();
    fetched.cached.locked_selection = selected.locked;
    assert!(fetched.cached.locked_selection.is_some());
    fetched.cached.acquisition =
        Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository);
    fetched.cached.recorded_source_kind = Some(vibe_core::manifest::SourceKind::Registry);
    fetched.cached.route_publication = RoutePublication::Project;
    fetched.cached.fetched_from = Some("https://project-mirror.invalid/demo.git".into());
    assert_eq!(
        fetched.cached.source_kind(),
        None,
        "held absence precedes non-Store inference"
    );
    assert_eq!(
        merge_cached_provenance(&[&fetched.cached], &before, &before, Some(RECORDED))
            .unwrap()
            .as_deref(),
        Some(RECORDED)
    );
    let mut invented = before.clone();
    invented.packages[0].source_kind = Some(vibe_core::manifest::SourceKind::Registry);
    assert!(
        merge_cached_provenance(&[&fetched.cached], &before, &invented, Some(RECORDED)).is_err()
    );
}
