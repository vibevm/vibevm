//! Publication ordering and direct public copy/writer bypass discriminators.
#![cfg(all(test, unix))]
use super::*;
use crate::{EventKind, HomeEvent};
use std::os::unix::fs::{PermissionsExt, symlink};

struct Fixture {
    root: tempfile::TempDir,
    home: SettingsRoot,
    old: Index,
    next: Index,
    global: PathBuf,
    session: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = SettingsRoot::new(&root.path().join("home"));
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/application/e2/index.json"
        ));
        let old = Index::parse(bytes).unwrap();
        old.save(&home, &event()).unwrap();
        let global = home.proxy_dir().join("edit");
        let session = home
            .app_proxies_dir()
            .join("org.example.editor@1.2.0/bin/edit");
        for path in [&global, &session] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"old reader").unwrap();
        }
        let mut next = old.clone();
        next.installations
            .get_mut("org.example/editor@1.2.0")
            .unwrap()
            .secret_origins
            .insert(
                "token".into(),
                vibe_wire::generated::shared::ConfigurationParameterOrigin::Explicit(Box::new(
                    vibe_wire::generated::shared::ConfigurationParameterOriginExplicit {},
                )),
            );
        Self {
            root,
            home,
            old,
            next,
            global,
            session,
        }
    }
    fn image(&self, protocols: &[&str]) -> PathBuf {
        let image = self.root.path().join("vibe-app-proxy");
        let choices = protocols
            .iter()
            .map(|p| format!("'{p}'"))
            .collect::<Vec<_>>()
            .join("|");
        fs::write(&image, format!("#!/bin/sh\n[ \"$1\" = --supports-index ] || exit 127\ncase \"$2\" in {choices}) printf '%s\\n' \"$2\";; *) exit 127;; esac\n")).unwrap();
        fs::set_permissions(&image, fs::Permissions::from_mode(0o755)).unwrap();
        image
    }
    fn unchanged(&self, bytes: &[u8]) {
        assert_eq!(fs::read(self.home.index_path()).unwrap(), bytes);
        assert_eq!(fs::read(&self.global).unwrap(), b"old reader");
        assert_eq!(fs::read(&self.session).unwrap(), b"old reader");
    }
}
fn event() -> HomeEvent {
    HomeEvent::new(EventKind::Updated)
}
const BOTH: [&str; 2] = ["vibe-user-applications/3", "vibe-user-applications/4"];

#[test]
fn raw_and_locked_writers_refuse_first_four_with_existing_readers() {
    let f = Fixture::new();
    let bytes = fs::read(f.home.index_path()).unwrap();
    assert!(matches!(
        f.next.save(&f.home, &event()),
        Err(StateError::ProxyUpgrade { .. })
    ));
    let lock = f.home.lock().unwrap();
    assert!(f.next.save_locked(&f.home, &event(), &lock).is_err());
    f.unchanged(&bytes);
    let other = SettingsRoot::new(&f.root.path().join("other"));
    let wrong = other.lock().unwrap();
    assert!(f.old.save_locked(&f.home, &event(), &wrong).is_err());
    let alias = f.root.path().join("alias");
    symlink(f.home.path(), &alias).unwrap();
    f.old
        .save_locked(&SettingsRoot::new(&alias), &event(), &lock)
        .unwrap();
}

#[test]
fn captured_both_reader_bytes_precede_publication_and_survive_later_failure() {
    for revision_failure in [false, true] {
        let f = Fixture::new();
        let unrelated = f.home.app_proxies_dir().join("unrelated");
        fs::write(&unrelated, b"leave me").unwrap();
        let old_temp = f
            .global
            .parent()
            .unwrap()
            .join(format!(".edit.{}.tmp", std::process::id()));
        fs::write(&old_temp, b"unowned temporary-looking file").unwrap();
        let image = f.image(&BOTH);
        let admitted = fs::read(&image).unwrap();
        let lock = f.home.lock().unwrap();
        let captured = for_publication(&f.home, &f.next, &image).unwrap().unwrap();
        // The source path changes after admission; it must not change the bytes copied.
        fs::write(&image, b"different unadmitted image").unwrap();
        if revision_failure {
            fs::remove_file(f.home.revision_path()).unwrap();
            fs::create_dir(f.home.revision_path()).unwrap();
        }
        let result = f
            .next
            .publish_locked(&f.home, &event(), &lock, Some(&captured));
        assert_eq!(result.is_err(), revision_failure);
        assert!(Index::load_with_protocol(&f.home).unwrap().1);
        assert_eq!(fs::read(&f.global).unwrap(), admitted);
        assert_eq!(fs::read(&f.session).unwrap(), admitted);
        assert_eq!(fs::read(unrelated).unwrap(), b"leave me");
        assert_eq!(
            fs::read(old_temp).unwrap(),
            b"unowned temporary-looking file"
        );
    }
}

#[test]
fn old_missing_and_four_only_images_refuse_without_touching_old_inventory() {
    let f = Fixture::new();
    let before = fs::read(f.home.index_path()).unwrap();
    for protocols in [&BOTH[..1], &BOTH[1..]] {
        assert!(for_publication(&f.home, &f.next, &f.image(protocols)).is_err());
        f.unchanged(&before);
    }
    assert!(for_publication(&f.home, &f.next, &f.root.path().join("missing")).is_err());
    f.unchanged(&before);
}

#[test]
fn unsafe_managed_destination_refuses_before_any_copy_or_foreign_write() {
    let f = Fixture::new();
    let before = fs::read(f.home.index_path()).unwrap();
    let foreign = f.root.path().join("foreign");
    fs::write(&foreign, b"foreign").unwrap();
    fs::remove_file(&f.session).unwrap();
    symlink(&foreign, &f.session).unwrap();
    let image = for_publication(&f.home, &f.next, &f.image(&BOTH))
        .unwrap()
        .unwrap();
    let lock = f.home.lock().unwrap();
    assert!(
        f.next
            .publish_locked(&f.home, &event(), &lock, Some(&image))
            .is_err()
    );
    assert_eq!(fs::read(&f.global).unwrap(), b"old reader");
    assert_eq!(fs::read(f.home.index_path()).unwrap(), before);
    assert_eq!(fs::read(foreign).unwrap(), b"foreign");
}

#[test]
fn stored_four_with_only_legacy_rows_still_blocks_old_public_sync_and_exposure() {
    let f = Fixture::new();
    let mut document = f.old.to_wire();
    document.protocol = crate::index::INDEX_PROTOCOL.into();
    fs::write(f.home.index_path(), serde_json::to_vec(&document).unwrap()).unwrap();
    let bytes = fs::read(f.home.index_path()).unwrap();
    assert!(!Index::load(&f.home).unwrap().requires_reader_four());
    assert!(Index::load_with_protocol(&f.home).unwrap().1);
    let old = f.image(&BOTH[..1]);
    let commands = BTreeSet::from(["edit".into()]);
    assert!(crate::sync_proxies(&f.home, &commands, &commands, &old).is_err());
    assert!(crate::expose_for_session(&f.home, "org.example/editor@=1.2.0", &old).is_err());
    f.unchanged(&bytes);
    let lock = f.home.lock().unwrap();
    assert!(crate::sync_proxies_locked(&f.home, &commands, &commands, &old, &lock).is_err());
    f.unchanged(&bytes);
}

#[test]
fn data_only_first_four_needs_no_image_and_raw_save_owns_the_lock() {
    let f = Fixture::new();
    fs::remove_file(&f.global).unwrap();
    fs::remove_file(&f.session).unwrap();
    let lock = f.home.lock().unwrap();
    assert!(matches!(
        f.next.save(&f.home, &event()),
        Err(StateError::Locked { .. })
    ));
    f.next.save_locked(&f.home, &event(), &lock).unwrap();
    assert!(Index::load_with_protocol(&f.home).unwrap().1);
}

#[test]
fn parallel_captures_and_a_brief_busy_snapshot_use_bounded_exact_image_admission() {
    let f = Fixture::new();
    let image = f.image(&BOTH);
    std::thread::scope(|scope| {
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let image = &image;
                scope.spawn(move || CapturedImage::read(image, true).unwrap())
            })
            .collect();
        for task in tasks {
            assert!(task.join().unwrap().supports_both);
        }
    });
    // A writable descriptor makes Linux exec return ETXTBSY. Release only that
    // descriptor; the query must retry the same bytes, not recapture a new image.
    let writable = fs::OpenOptions::new().write(true).open(&image).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        drop(writable);
    });
    query(&image, f.root.path(), &image, BOTH[0]).unwrap();
    release.join().unwrap();
}

#[test]
fn already_four_metadata_needs_no_new_image_or_session_refresh() {
    let mut f = Fixture::new();
    let image = f.image(&BOTH);
    let lock = f.home.lock().unwrap();
    let captured = for_publication(&f.home, &f.next, &image).unwrap().unwrap();
    f.next
        .publish_locked(&f.home, &event(), &lock, Some(&captured))
        .unwrap();
    let session = fs::read(&f.session).unwrap();
    let modified = fs::metadata(&f.session).unwrap().modified().unwrap();
    for record in f.next.installations.values_mut() {
        record.lifecycle.hidden = true;
    }
    let image = for_publication(
        &f.home,
        &f.next,
        &f.root.path().join("unused-missing-image"),
    )
    .unwrap();
    assert!(
        image.is_none(),
        "metadata with no actual global writes does not need an image"
    );
    f.next
        .publish_locked(&f.home, &event(), &lock, None)
        .unwrap();
    assert_eq!(fs::read(&f.session).unwrap(), session);
    assert_eq!(
        fs::metadata(&f.session).unwrap().modified().unwrap(),
        modified
    );
}
