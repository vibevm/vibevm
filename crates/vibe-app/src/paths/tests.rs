//! Slot link reconciliation: the links the index implies, the repoint a
//! default switch causes, and the refusal that protects a foreign name.

use super::*;
use crate::index::{
    AuthoredCommands, Installation, Location, Method, PackageCoordinate, SlotState,
};

fn record(name: &str, slot: &str, dirs: &[&str], root: &Path) -> Installation {
    Installation {
        source_evidence: None,
        secret_origins: Default::default(),
        configuration: None,
        pending: None,
        slot: slot.into(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: name.into(),
            version: "1.0.0".into(),
        },
        option: "linux".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        method: Method::Shell,
        location: Location::External {
            app_root: root.to_path_buf(),
        },
        portable: false,
        launchers: Vec::new(),
        public_commands: Vec::new(),
        path_dirs: dirs.iter().map(|dir| (*dir).to_owned()).collect(),
        version_file: Vec::new(),
        template: None,
        migrations: Vec::new(),
        migrated: Vec::new(),
        extras: Vec::new(),
        commands: AuthoredCommands::default(),
        features: Vec::new(),
        parameters: Default::default(),
        license: None,
        icon: None,
        menu: Default::default(),
        steps: Vec::new(),
        lifecycle: crate::index::Lifecycle::default(),
    }
}

/// Two providers of one slot, each with its own payload `bin`.
fn two_providers(home: &Path) -> (Index, PathBuf, PathBuf) {
    let alpha = home.join("payload/alpha");
    let beta = home.join("payload/beta");
    for (root, marker) in [(&alpha, "alpha"), (&beta, "beta")] {
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin").join("tool"), marker).unwrap();
    }
    let mut index = Index::default();
    for (name, root) in [("alpha", &alpha), ("beta", &beta)] {
        let entry = record(name, "tool", &["bin"], root);
        index.installations.insert(entry.package.key(), entry);
    }
    index.slots.insert(
        "tool".into(),
        SlotState::with_default(Some("org.example/alpha@1.0.0".into())),
    );
    (index, alpha, beta)
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
fn the_link_follows_the_default_and_goes_with_the_last_provider() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let lock = settings.lock().unwrap();
    let storage = Storage::load(&settings).unwrap();
    let (mut index, alpha, beta) = two_providers(dir.path());

    let synced = sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert_eq!(synced.needed.len(), 1);
    assert_eq!(synced.needed[0].slot, "tool");
    let link = settings.slot_link_path("tool", 0);
    assert_eq!(synced.needed[0].link_dirs, vec![link.clone()]);
    assert_eq!(std::fs::read(link.join("tool")).unwrap(), b"alpha");

    // A payload file written after installation — what `npm install -g`
    // does — is reachable through the link without any further work.
    std::fs::write(alpha.join("bin").join("later"), b"added").unwrap();
    assert_eq!(std::fs::read(link.join("later")).unwrap(), b"added");

    // Switching the default repoints the one link.
    index.set_default("tool", Some("org.example/beta@1.0.0".into()));
    sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert_eq!(std::fs::read(link.join("tool")).unwrap(), b"beta");
    assert!(beta.join("bin").join("tool").is_file());

    // Switching the slot off takes the link out; the directory stays, so
    // the PATH entry keeps working and a lookup falls through.
    index.slot_mut("tool").off = true;
    let synced = sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert!(!link.exists());
    assert!(settings.slot_links_dir("tool").is_dir());
    assert_eq!(synced.needed.len(), 1, "the slot still owns its link dir");

    // On again, and the link comes back pointed at the remembered default.
    index.slot_mut("tool").off = false;
    sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert_eq!(std::fs::read(link.join("tool")).unwrap(), b"beta");

    // The last provider going releases the slot: no link, no link
    // directory, and the slot is reported as released so its PATH entries
    // are taken out.
    index.installations.clear();
    index.slots.clear();
    let synced = sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert!(!link.exists());
    assert!(!settings.slot_links_dir("tool").exists());
    assert_eq!(synced.released, vec!["tool".to_owned()]);
    assert!(synced.needed.is_empty());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
fn a_foreign_directory_at_a_link_name_refuses_before_anything_changes() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let lock = settings.lock().unwrap();
    let storage = Storage::load(&settings).unwrap();
    let (index, _alpha, _beta) = two_providers(dir.path());

    let link = settings.slot_link_path("tool", 0);
    std::fs::create_dir_all(&link).unwrap();
    std::fs::write(link.join("the-operator-s-own"), b"keep me").unwrap();
    let error = sync_locked(&settings, &index, &storage, &lock).unwrap_err();
    assert_eq!(error.code(), "foreign_path_link", "{error}");
    assert!(error.to_string().contains("PROP-059#path-directories"));
    assert_eq!(
        std::fs::read(link.join("the-operator-s-own")).unwrap(),
        b"keep me"
    );
    assert!(!inventory_path(&settings).exists(), "nothing was recorded");
}

#[test]
fn a_payload_directory_that_is_not_there_leaves_the_link_out() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let lock = settings.lock().unwrap();
    let storage = Storage::load(&settings).unwrap();
    let root = dir.path().join("payload/absent");
    std::fs::create_dir_all(&root).unwrap();
    let mut index = Index::default();
    let entry = record("absent", "tool", &["bin"], &root);
    index.installations.insert(entry.package.key(), entry);
    index.slots.insert(
        "tool".into(),
        SlotState::with_default(Some("org.example/absent@1.0.0".into())),
    );
    let synced = sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert_eq!(synced.needed.len(), 1, "the slot still asks for its dir");
    assert!(!settings.slot_link_path("tool", 0).exists());
}

#[test]
fn a_hidden_provider_never_widens_or_claims_a_slot() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let lock = settings.lock().unwrap();
    let storage = Storage::load(&settings).unwrap();
    let root = dir.path().join("payload/hidden");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let mut index = Index::default();
    let mut entry = record("hidden", "tool", &["bin"], &root);
    entry.lifecycle.hidden = true;
    index.installations.insert(entry.package.key(), entry);
    index.slots.insert("tool".into(), SlotState::default());
    let synced = sync_locked(&settings, &index, &storage, &lock).unwrap();
    assert!(synced.needed.is_empty());
    assert!(!settings.slot_links_dir("tool").exists());
}

#[test]
fn a_declaration_that_escapes_the_payload_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let lock = settings.lock().unwrap();
    let storage = Storage::load(&settings).unwrap();
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let root = dir.path().join("payload/escape");
    std::fs::create_dir_all(&root).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("bin")).unwrap();
    #[cfg(not(unix))]
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let mut index = Index::default();
    let entry = record("escape", "tool", &["bin"], &root);
    index.installations.insert(entry.package.key(), entry);
    index.slots.insert(
        "tool".into(),
        SlotState::with_default(Some("org.example/escape@1.0.0".into())),
    );
    let result = sync_locked(&settings, &index, &storage, &lock);
    #[cfg(unix)]
    {
        let error = result.unwrap_err();
        assert_eq!(error.code(), "path_link_target_unsupported", "{error}");
        assert!(error.to_string().contains("outside the application root"));
    }
    #[cfg(not(unix))]
    result.unwrap();
}
