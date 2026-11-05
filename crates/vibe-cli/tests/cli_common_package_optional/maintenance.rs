//! Cold maintenance and original-state corruption through real CLI operations.
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
fn intent_only_change_is_not_fresh_and_survives_another_process() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let before_graph = graph(&fixture.lock());
    let before_hooks = fixture.hooks();
    let path = fixture.project.join("vibe.toml");
    let source = fs::read_to_string(&path).unwrap();
    let changed = source.replace("intent='exact'", "intent='additive'");
    assert_ne!(source, changed);
    fs::write(&path, &changed).unwrap();
    fixture.install_ok();
    assert_original_record(&fixture.lock(), changed.as_bytes(), true);
    assert_eq!(graph(&fixture.lock()), before_graph);
    assert_eq!(fixture.hooks(), before_hooks);
    fixture.install_ok();
    assert_original_record(&fixture.lock(), changed.as_bytes(), true);
    assert_eq!(fixture.hooks(), before_hooks);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn rich_missing_record_refuses_cold_reconstruction_before_effects() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let mut lock = fixture.lock();
    assert_eq!(lock.meta.schema_version, 8);
    lock.configurations.clear();
    let lock_path = fixture.project.join("vibe.lock");
    // Preserve the actual8 header while removing its sole historical record.
    let corrupt = toml::to_string(&lock).unwrap();
    fs::write(&lock_path, &corrupt).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let output = fixture.install();
    assert!(
        !output.status.success(),
        "missing retained configuration must not be silently recreated: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostic.contains("no lock record"), "{diagnostic}");
    assert_eq!(fs::read_to_string(&lock_path).unwrap(), corrupt);
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
}

fn reinstall(fixture: &Fixture) -> Output {
    fixture
        .user
        .vibe()
        .current_dir(&fixture.project)
        .env("HOME", fixture.root.join("home"))
        .env("USERPROFILE", fixture.root.join("home"))
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "reinstall",
            fixture.project.to_str().unwrap(),
            "--offline",
            "--assume-yes",
            "--json",
        ])
        .output()
        .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn missing_slot_repairs_from_real_store_without_losing_original_input() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let original = fixture.lock().configurations;
    let hooks = fixture.hooks();
    fs::remove_dir_all(fixture.slot(DRIVER)).unwrap();
    fixture.install_ok();
    fixture.assert_world(true);
    assert_eq!(fixture.lock().configurations, original);
    assert_eq!(fixture.hooks(), hooks);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn missing_slot_cannot_use_store_bytes_disagreeing_with_original_pin() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let lock = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let entry = vibe_registry::store::entry_dir(
        &fixture.user.settings.join("cache"),
        &vibe_core::Group::parse(GROUP).unwrap(),
        DRIVER,
        &semver::Version::parse(VERSION).unwrap(),
    );
    fs::write(entry.join("payload.txt"), "changed source bytes").unwrap();
    fs::remove_dir_all(fixture.slot(DRIVER)).unwrap();
    let output = fixture.install();
    assert!(!output.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostic.contains("exact lock pin"), "{diagnostic}");
    assert!(!fixture.slot(DRIVER).exists());
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
fn cold_reinstall_rebuilds_boot_without_reopening_unneeded_registry() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let lock = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    fs::rename(&fixture.registry, fixture.root.join("registry-unavailable")).unwrap();
    fixture.install_ok();
    assert_eq!(fixture.hooks(), hooks);
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
    for file in [common::static_md_rel(), common::static_xml_rel()] {
        let path = fixture.project.join(file);
        if path.is_file() {
            fs::remove_file(path).unwrap();
        }
    }
    let output = reinstall(&fixture);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn missing_record_with_missing_slot_refuses_using_exact_store_and_reinstall() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let mut lock = fixture.lock();
    lock.configurations.clear();
    let corrupt = toml::to_string(&lock).unwrap();
    fs::write(fixture.project.join("vibe.lock"), &corrupt).unwrap();
    fs::remove_dir_all(fixture.slot(DRIVER)).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let output = fixture.install();
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let diagnostic = format!(
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!("missing-slot refusal: {diagnostic}");
    assert!(diagnostic.contains("no lock record"), "{diagnostic}");
    let output = reinstall(&fixture);
    assert!(!output.status.success());
    assert!(!fixture.slot(DRIVER).exists());
    assert_eq!(
        fs::read_to_string(fixture.project.join("vibe.lock")).unwrap(),
        corrupt
    );
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#OWNER-LOCAL-ACTIVATION")]
fn on_to_off_drops_only_the_owner_edge_while_an_independent_root_keeps_extra() {
    let fixture = Fixture::new(true);
    let path = fixture.project.join("vibe.toml");
    let mut manifest = Manifest::read(&path).unwrap();
    manifest
        .requires
        .packages
        .push(vibe_core::PackageRef::parse(&format!("{GROUP}/{EXTRA}@^1")).unwrap());
    manifest
        .requires
        .links
        .insert(coordinate(EXTRA), vibe_core::manifest::LinkType::Static);
    manifest.write(&path).unwrap();
    fixture.install_ok();
    let before = fixture.lock();
    assert!(
        before
            .packages
            .iter()
            .find(|p| p.name.as_str() == DRIVER)
            .unwrap()
            .dependencies
            .iter()
            .any(|p| p.qualified_name() == coordinate(EXTRA))
    );
    let hooks = fixture.hooks();
    manifest
        .requires
        .configurations
        .get_mut(&coordinate(DRIVER))
        .unwrap()
        .selections
        .get_mut("extra")
        .unwrap()
        .members = Some(vec!["disabled".into()]);
    manifest.write(&path).unwrap();
    let authored = fs::read(&path).unwrap();
    for _ in 0..2 {
        fixture.install_ok();
        let lock = fixture.lock();
        assert_original_record(&lock, &authored, false);
        let driver = lock
            .packages
            .iter()
            .find(|p| p.name.as_str() == DRIVER)
            .unwrap();
        assert_eq!(
            driver
                .dependencies
                .iter()
                .map(|p| p.qualified_name())
                .collect::<Vec<_>>(),
            [coordinate(BASE)]
        );
        assert_eq!(lock.packages.len(), 3);
        assert!(
            lock.meta
                .root_dependencies
                .iter()
                .any(|p| p.qualified_name() == coordinate(EXTRA))
        );
        assert!(fixture.slot(EXTRA).is_dir());
        assert!(fixture.static_boot().contains(EXTRA_MARKER));
        assert_eq!(
            fixture.hooks(),
            hooks,
            "turning one owner off cannot repeat another root's effects"
        );
        assert_eq!(fs::read(&path).unwrap(), authored);
    }
}
