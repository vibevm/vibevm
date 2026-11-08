//! `vibe update` against the sources a project does NOT declare itself.
//!
//! PROP-002 `##GLOBAL-REGISTRY-FILE` / `##MERGE-PROJECT-FIRST`: a project's
//! `vibe.toml` may carry no `[[registry]]` at all and still resolve, because
//! the machine-global `~/.vibe/registry.toml` is merged into the effective
//! set. `vibe init` writes no project registries (that is its own pinned
//! behaviour), so the machine file is the ordinary shape for a developer who
//! configures one registry for the whole machine — and the shape a reader of
//! the private-registry how-to ends up with.
//!
//! The red here is the update half of that contract. `vibe update` used to
//! refuse on `manifest.registries.is_empty()` — the PROJECT manifest alone —
//! fifteen lines before it built a resolver that honours the machine file, so
//! such a project could install and then never update. The second test keeps
//! the refusal honest for the case that really has no source anywhere.
//!
//! Isolation: every command is built by [`UserScratch`], whose
//! `$VIBE_SETTINGS` is where the machine `registry.toml` under test lives —
//! the developer's real `~/.vibe` is never read or written.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use vibe_core::manifest::Lockfile;

/// The `file:///…` url form the local-directory registry backend opens
/// (`file:///C:/x` on Windows, `file:///home/x` on POSIX).
fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// The one package the fixture registry serves: `org.example/my-skills@0.1.0`,
/// a `flow` whose flow tree carries a single file with `body` in it. Written
/// into `<root>/registry/` in the monorepo registry shape
/// (`<group>/<name>/v<version>/…`) and returned as the registry root.
fn make_registry(root: &Path, body: &str) -> PathBuf {
    let registry = root.join("registry");
    write_package(&registry, body);
    registry
}

/// (Re)write the fixture package's content in place, at the SAME version —
/// the edit a developer makes to a package they serve themselves, which
/// PROP-011 §2.6 refreshes through the write-once store.
fn write_package(registry: &Path, body: &str) {
    let dir = registry
        .join("org.example")
        .join("my-skills")
        .join("v0.1.0");
    let flow = dir.join(common::spec_rel("flows/my-skills"));
    fs::create_dir_all(&flow).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"my-skills\"\n\
         kind = \"flow\"\n\
         version = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(flow.join("RULES.md"), format!("# rules\n\n{body}\n")).unwrap();
}

/// The package's materialised copy of that one file, inside the project.
fn slot_rules(project: &Path) -> PathBuf {
    project.join(common::slot_rel(
        "org.example.my-skills",
        "0.1.0",
        common::spec_rel("flows/my-skills/RULES.md"),
    ))
}

/// The `content_hash` `vibe.lock` records for the fixture package.
fn locked_hash(project: &Path) -> String {
    let lock = Lockfile::read(project.join(Lockfile::FILENAME)).unwrap();
    lock.packages
        .iter()
        .find(|package| package.name.as_str() == "my-skills")
        .map(|package| package.content_hash.as_str().to_string())
        .unwrap_or_else(|| panic!("`org.example/my-skills` must be locked in {project:?}"))
}

/// The fixture package's `content_hash` inside raw `vibe.lock` bytes.
fn locked_hash_bytes(lock: &[u8]) -> String {
    let lock = Lockfile::parse_str(std::str::from_utf8(lock).unwrap()).unwrap();
    lock.packages
        .iter()
        .find(|package| package.name.as_str() == "my-skills")
        .map(|package| package.content_hash.as_str().to_string())
        .expect("fixture package is locked")
}

/// Seed `<settings>/registry.toml` with one `file://` registry — the
/// machine-wide configuration, with nothing in the project's own `vibe.toml`.
fn seed_machine_registry(user: &UserScratch, registry: &Path) {
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"machine\"\nurl = \"{}\"\n",
            file_url(registry)
        ),
    )
    .unwrap();
}

/// A project whose registries come ONLY from the machine file installs AND
/// updates. A plain update keeps the locked source; `--reselect-source`
/// re-fetches the edited package, materialises the new
/// bytes, and records the same `content_hash` a fresh install of the edited
/// package records — the two paths cannot disagree about what the registry
/// currently serves.
#[test]
fn a_project_whose_registries_come_from_the_machine_file_can_update() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let registry = make_registry(outer.path(), "first");
    seed_machine_registry(&user, &registry);

    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    // The premise: `vibe init` declares no project registries, so the machine
    // file is the only source. A fixture that accidentally wrote one would
    // make this test pass for the wrong reason.
    let manifest_text = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    let manifest = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
    assert!(
        manifest.registries.is_empty(),
        "fixture: the project must declare no `[[registry]]`; got {:?}",
        manifest.registries
    );

    user.vibe()
        .arg("install")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(
        fs::read_to_string(slot_rules(project.path()))
            .unwrap()
            .contains("first"),
        "fixture: the install must materialise the package's content"
    );

    // The package's author edits it, at the same version.
    write_package(&registry, "second");

    // A plain update keeps the locked source and its pinned bytes: the same
    // version is not a new selection (owner decision 2026-11-08, PROP-073
    // `#UPDATE-SOURCE-RESELECTION`).
    let held_lock = fs::read(project.path().join(Lockfile::FILENAME)).unwrap();
    let out = user
        .vibe()
        .arg("update")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a plain update over a held source must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        fs::read_to_string(slot_rules(project.path()))
            .unwrap()
            .contains("first"),
        "a plain update must not take unpinned same-version bytes"
    );
    assert_eq!(
        locked_hash_bytes(&held_lock),
        locked_hash(project.path()),
        "a plain update keeps the pinned content hash"
    );

    // An explicit reselection chooses the source again from the machine file.
    let out = user
        .vibe()
        .arg("update")
        .arg("org.example/my-skills")
        .arg("--reselect-source")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a machine-file registry must satisfy `vibe update`\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let materialised = fs::read_to_string(slot_rules(project.path())).unwrap();
    assert!(
        materialised.contains("second"),
        "the update must materialise the edited content; got:\n{materialised}"
    );

    // The oracle for the lock: a FRESH install of the same coordinate, from
    // the same machine file, into a project that never saw the old bytes.
    let fresh = tempfile::tempdir().unwrap();
    user.init_project(fresh.path());
    user.vibe()
        .arg("install")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(fresh.path())
        .arg("--assume-yes")
        .assert()
        .success();
    assert_eq!(
        locked_hash(project.path()),
        locked_hash(fresh.path()),
        "the updated lock must record what a fresh install records"
    );
}

/// The refusal stays for the project that really has no source: no
/// `[[registry]]` in `vibe.toml`, an EMPTY machine file, no project-local
/// packages root, and `VIBE_NO_DEFAULT_REGISTRY` (set by the test harness)
/// keeping the embedded family out. The message names both places an operator
/// can fix it in.
#[test]
fn a_project_with_no_registry_anywhere_is_still_refused() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let registry = make_registry(outer.path(), "first");
    // The machine file serves the install, so the lockfile has an entry to
    // update; it is then removed, which is the state under test.
    seed_machine_registry(&user, &registry);
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    user.vibe()
        .arg("install")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The machine file goes away — now no registry configuration can choose
    // a source. The locked source itself still exists, and a plain update
    // replays it (owner decision 2026-11-08); only an explicit reselection
    // needs a configured source and must refuse.
    fs::remove_file(user.settings.join("registry.toml")).unwrap();

    user.vibe()
        .arg("update")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let out = user
        .vibe()
        .arg("update")
        .arg("org.example/my-skills")
        .arg("--reselect-source")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "with no configured source a source reselection must refuse\nstdout:\n{}",
        String::from_utf8_lossy(&out.stdout),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The project manifest, spelled with the platform's own separator: the old
    // message formatted `{root}/vibe.toml` and so printed
    // `C:\p\project/vibe.toml` on Windows — one path, two separators.
    assert!(
        stderr.contains(&format!("{}vibe.toml", std::path::MAIN_SEPARATOR)),
        "the refusal must name the project manifest with one separator:\n{stderr}"
    );
    assert!(
        stderr.contains("~/.vibe/registry.toml"),
        "the refusal must name the machine file too:\n{stderr}"
    );
}

/// The locked row of the fixture package, read from the project's lock.
fn locked_row(project: &Path) -> vibe_core::manifest::LockedPackage {
    Lockfile::read(project.join(Lockfile::FILENAME))
        .unwrap()
        .packages
        .into_iter()
        .find(|package| package.name.as_str() == "my-skills")
        .expect("fixture package is locked")
}

/// Owner request 2026-11-08: one command refreshes where locked packages come
/// from after the machine `registry.toml` changes, without moving versions.
/// Moving the machine file from registry A to an identical registry B moves
/// the locked source to B; the version and pinned bytes stay; a later plain
/// install replays B. A target registry without the locked version refuses
/// and leaves the lock untouched.
#[test]
fn sources_only_moves_locked_sources_to_the_current_registry_and_keeps_versions() {
    let user = UserScratch::new();
    let outer_a = tempfile::tempdir().unwrap();
    let outer_b = tempfile::tempdir().unwrap();
    let registry_a = make_registry(outer_a.path(), "same");
    let registry_b = make_registry(outer_b.path(), "same");
    seed_machine_registry(&user, &registry_a);
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    user.vibe()
        .arg("install")
        .arg("org.example/my-skills")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let before = locked_row(project.path());
    assert!(
        before
            .source_url
            .to_string()
            .contains(&*outer_a.path().to_string_lossy()),
        "fixture: installed from registry A, got {}",
        before.source_url
    );

    seed_machine_registry(&user, &registry_b);
    let out = user
        .vibe()
        .arg("update")
        .arg("--sources-only")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "sources-only refresh must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let after = locked_row(project.path());
    assert!(
        after
            .source_url
            .to_string()
            .contains(&*outer_b.path().to_string_lossy()),
        "the locked source must move to registry B, got {}",
        after.source_url
    );
    assert_eq!(after.version, before.version, "versions never move");
    assert_eq!(
        after.content_hash, before.content_hash,
        "identical content keeps the same pinned hash"
    );

    // Registry B loses the locked version; registry A is configured again.
    // The refresh refuses rather than choosing another version, and the lock
    // stays byte-identical.
    fs::remove_dir_all(registry_b.join("org.example")).unwrap();
    let lock_bytes = fs::read(project.path().join(Lockfile::FILENAME)).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let registry_c = empty.path().join("registry");
    fs::create_dir_all(&registry_c).unwrap();
    seed_machine_registry(&user, &registry_c);
    let out = user
        .vibe()
        .arg("update")
        .arg("--sources-only")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a registry without the locked version must refuse\nstdout:\n{}",
        String::from_utf8_lossy(&out.stdout),
    );
    assert_eq!(
        fs::read(project.path().join(Lockfile::FILENAME)).unwrap(),
        lock_bytes,
        "a refused refresh leaves vibe.lock byte-identical"
    );
}
