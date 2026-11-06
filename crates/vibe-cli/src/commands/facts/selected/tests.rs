use super::*;
use std::fs;

struct Fixture {
    root: tempfile::TempDir,
    _source: tempfile::TempDir,
    slot: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let group = vibe_core::Group::parse("org.example").unwrap();
        let version = "1.0.0".parse().unwrap();
        let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        fs::write(
            root.path().join("vibe.toml"),
            "[project]\nname='consumer'\nversion='1.0.0'\nspec_format='mixed'\n",
        )
        .unwrap();
        fs::write(root.path().join("vibe.lock"), format!(
            "[meta]\ngenerated_by='test'\ngenerated_at='2026-11-06T00:00:00Z'\nschema_version={}\n\n[[package]]\nkind='flow'\nname='rules'\ngroup='org.example'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='{}'\n",
            vibe_core::manifest::CURRENT_SCHEMA_VERSION, hash,
        )).unwrap();
        fs::write(source.path().join("vibe.toml"),
            "[package]\nname='rules'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n\n[[package.sources]]\nfile='rule.xml'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n").unwrap();
        fs::write(source.path().join("rule.xml"),
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><RULE fact=\"true\" status=\"impl/done\">Rule.</RULE></p></spec>").unwrap();
        // Use the actual original-byte materializer, not a fabricated record.
        vibedeps::materialise(root.path(), &group, "rules", &version, source.path(), &hash)
            .unwrap();
        let slot = vibedeps::slot_abs_path(root.path(), &group, "rules", &version);
        Self {
            root,
            _source: source,
            slot,
        }
    }
    fn prepare(&self) -> Adoption {
        Adoption::prepare(self.root.path(), "org.example/rules").unwrap()
    }
}

#[test]
fn retained_admission_rechecks_root_lock_record_manifest_and_exact_source() {
    for target in ["root", "lock", "record", "manifest", "document"] {
        let fixture = Fixture::new();
        let admitted = fixture.prepare();
        let path = match target {
            "root" => fixture.root.path().join("vibe.toml"),
            "lock" => fixture.root.path().join("vibe.lock"),
            "record" => fixture.slot.join(vibedeps::SLOT_RECORD_FILENAME),
            "manifest" => fixture.slot.join("vibe.toml"),
            _ => fixture.slot.join("rule.xml"),
        };
        let mut changed = fs::read(&path).unwrap();
        changed.extend_from_slice(b"\n");
        fs::write(path, changed).unwrap();
        assert!(admitted.recheck().is_err(), "missed {target} drift");
        assert!(
            !fixture
                .root
                .path()
                .join(vibe_core::layout::current_vibefacts_root())
                .exists()
        );
    }
}

#[cfg(unix)]
#[test]
fn retained_admission_refuses_root_and_slot_parent_replacements() {
    for replace_root in [true, false] {
        let fixture = Fixture::new();
        let admitted = fixture.prepare();
        let outside = tempfile::tempdir().unwrap();
        let target = if replace_root {
            fixture.root.path()
        } else {
            fixture.slot.parent().unwrap()
        };
        let moved = outside.path().join("original");
        fs::rename(target, &moved).unwrap();
        // Even a byte-identical alias to the old held directory is not a new
        // grant for a dependency descendant or a registry destination.
        std::os::unix::fs::symlink(&moved, target).unwrap();
        assert!(admitted.recheck().is_err());
        assert!(
            !moved
                .join(vibe_core::layout::current_vibefacts_root())
                .exists()
        );
    }
}

#[test]
fn lock_selected_kind_and_version_are_checked_before_native_parsing() {
    for change_kind in [true, false] {
        let fixture = Fixture::new();
        let path = fixture.slot.join("vibe.toml");
        let before = fs::read_to_string(&path).unwrap();
        let changed = if change_kind {
            before.replace("kind='flow'", "kind='feat'")
        } else {
            before.replace("version='1.0.0'", "version='2.0.0'")
        };
        fs::write(path, changed).unwrap();
        let error = Adoption::prepare(fixture.root.path(), "org.example/rules")
            .err()
            .unwrap();
        assert!(
            error.to_string().contains("kind/group/name/version"),
            "{error:#}"
        );
    }
}

#[test]
fn declared_in_place_slot_is_not_an_original_byte_adoption_source() {
    let fixture = Fixture::new();
    let lock_path = fixture.root.path().join("vibe.lock");
    let mut lock = Lockfile::read(&lock_path).unwrap();
    lock.packages[0].materialization = vibe_core::manifest::Materialization::InPlace;
    lock.write(&lock_path).unwrap();
    let parent = fixture.slot.parent().unwrap();
    let moved = fixture.root.path().join("temporary-slot");
    fs::rename(&fixture.slot, &moved).unwrap();
    fs::remove_dir(parent).unwrap();
    fs::rename(moved, parent).unwrap();
    let error = Adoption::prepare(fixture.root.path(), "org.example/rules")
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("in-place source provenance"),
        "{error:#}"
    );
}

#[cfg(unix)]
#[test]
fn selected_member_registry_destination_cannot_be_rebound() {
    let fixture = Fixture::new();
    let root_manifest = fixture.root.path().join("vibe.toml");
    let before = fs::read_to_string(&root_manifest).unwrap();
    fs::write(
        root_manifest,
        before + "\n[workspace]\nmembers=['member']\n",
    )
    .unwrap();
    let member = fixture.root.path().join("member");
    fs::create_dir(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[project]\nname='member'\nversion='1.0.0'\n",
    )
    .unwrap();
    let admitted = Adoption::prepare(&member, "org.example/rules").unwrap();
    let moved = fixture.root.path().join("old-member");
    fs::rename(&member, &moved).unwrap();
    fs::create_dir(&member).unwrap();
    fs::copy(moved.join("vibe.toml"), member.join("vibe.toml")).unwrap();
    let error = admitted.recheck().unwrap_err();
    assert!(
        error.to_string().contains("registry destination changed"),
        "{error:#}"
    );
    assert!(
        !member
            .join(vibe_core::layout::current_vibefacts_root())
            .exists()
    );
}
