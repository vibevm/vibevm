//! Unconnected incoming/removal operations refuse without disturbing accepted state.
use super::*;

fn add_root(fixture: &Fixture, name: &str, optional: Option<bool>) {
    let path = fixture.project.join("vibe.toml");
    let mut manifest = Manifest::read(&path).unwrap();
    manifest
        .requires
        .packages
        .push(vibe_core::PackageRef::parse(&format!("{GROUP}/{name}@^1")).unwrap());
    if let Some(optional) = optional {
        manifest
            .requires
            .optional
            .insert(coordinate(name), optional);
    }
    manifest.write(path).unwrap();
}
fn uninstall(fixture: &Fixture, name: &str) -> Output {
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
            "uninstall",
            &coordinate(name),
            "--path",
            fixture.project.to_str().unwrap(),
            "--assume-yes",
            "--json",
        ])
        .output()
        .unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CONSUMER-MERGE")]
fn bare_package_edge_cannot_disappear_behind_workspace_configuration() {
    let fixture = Fixture::new(true);
    fixture.install_ok();
    let parent = "oracle-parent";
    leaf(&fixture.registry, parent, "PARENT_MARKER");
    let path = fixture
        .registry
        .join(GROUP)
        .join(parent)
        .join("v1.0.0/vibe.toml");
    let mut manifest = Manifest::read(&path).unwrap();
    manifest
        .requires
        .packages
        .push(vibe_core::PackageRef::parse(&format!("{GROUP}/{DRIVER}@^1")).unwrap());
    manifest.write(path).unwrap();
    add_root(&fixture, parent, None);
    let lock = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let authored = fs::read(fixture.project.join("vibe.toml")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let output = fixture.install();
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "{diagnostic}");
    assert!(
        diagnostic.contains("candidate-conditioned")
            && diagnostic.contains(parent)
            && diagnostic.contains(DRIVER),
        "{diagnostic}"
    );
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(
        fs::read(fixture.project.join("vibe.toml")).unwrap(),
        authored
    );
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert!(!fixture.slot(parent).exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn affected_uninstall_refuses_but_unrelated_legacy_root_removal_preserves_configuration() {
    let fixture = Fixture::new(true);
    let unrelated = "oracle-unrelated";
    leaf(&fixture.registry, unrelated, "UNRELATED_MARKER");
    add_root(&fixture, unrelated, Some(false));
    fixture.install_ok();
    let lock = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let authored = fs::read(fixture.project.join("vibe.toml")).unwrap();
    let configuration = fixture.lock().configurations;
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let output = uninstall(&fixture, DRIVER);
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "{diagnostic}");
    assert!(
        diagnostic.contains("configuration-aware removal") && diagnostic.contains(DRIVER),
        "{diagnostic}"
    );
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(
        fs::read(fixture.project.join("vibe.toml")).unwrap(),
        authored
    );
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    let output = uninstall(&fixture, unrelated);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.slot(unrelated).exists());
    assert_eq!(fixture.lock().configurations, configuration);
    assert!(
        !Manifest::read(fixture.project.join("vibe.toml"))
            .unwrap()
            .requires
            .optional
            .contains_key(&coordinate(unrelated))
    );
    fixture.install_ok();
    assert_eq!(fixture.lock().configurations, configuration);
    assert_eq!(fixture.hooks(), hooks);
}
