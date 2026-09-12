//! End-to-end tests for the `vibe cache` command family (PROP-010
//! §2.8): `path` / `list` outside any project, `add` pre-warming the
//! machine store from a `file://` directory fixture without
//! materialising anything into a project, and `clean` — the
//! EXPLICIT-RECLAIM refusal without a target plus the `--package` /
//! `--older-than` / `--all` branches.
//!
//! Isolation: every test drives the real binary through `UserScratch`,
//! whose `$VIBE_SETTINGS` points the machine store (`<settings>/cache`)
//! at a temp home — the real `~/.vibe` is never touched.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;

/// The machine store root for a scratch home (`<settings>/cache`).
fn store_dir(user: &UserScratch) -> PathBuf {
    user.settings.join("cache")
}

/// A hermetic `file://` directory registry carrying two packages:
/// `org.example/parent@0.1.0`, which `[requires]` `org.example/child`
/// (so an add of the parent proves the dependency closure lands too),
/// and `org.example/child@0.1.0`. No git anywhere — a `file:` url
/// opens the filesystem backend (PROP-002 §2.2.2).
fn make_dir_registry(root: &Path) -> PathBuf {
    let registry = root.join("registry");
    let parent = registry.join("org.example").join("parent").join("v0.1.0");
    let child = registry.join("org.example").join("child").join("v0.1.0");
    fs::create_dir_all(parent.join(common::spec_rel("flows/parent"))).unwrap();
    fs::create_dir_all(child.join(common::spec_rel("flows/child"))).unwrap();
    fs::write(
        parent.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"parent\"\n\
         kind = \"flow\"\n\
         version = \"0.1.0\"\n\n\
         [requires.packages]\n\
         \"org.example/child\" = \"^0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        parent.join(common::spec_rel("flows/parent/P.md")),
        "# parent\n",
    )
    .unwrap();
    fs::write(
        child.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"child\"\n\
         kind = \"flow\"\n\
         version = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        child.join(common::spec_rel("flows/child/C.md")),
        "# child\n",
    )
    .unwrap();
    registry
}

/// The `file:///…` url form the local-directory registry backend opens
/// (`file:///C:/x` on Windows, `file:///home/x` on POSIX).
fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// Add the documentation half of the fixture registry: a `doc` package
/// documenting `org.example/parent`, its Russian adaptation, and a
/// manual whose subject no registry can serve — the three shapes the
/// warm-up closure has to tell apart.
fn add_documentation_to_registry(registry: &Path) {
    let write = |dir: PathBuf, manifest: String| {
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), manifest).unwrap();
        fs::write(dir.join("README.md"), "# manual\n").unwrap();
    };
    let card = "title = \"A Manual\"\nabstract = \"Four answers.\"\n";
    write(
        registry
            .join("org.example")
            .join("parent-docs")
            .join("v0.1.0"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"parent-docs\"\n\
             kind = \"doc\"\nversion = \"0.1.0\"\n{card}\n\
             [[documents]]\npackage = \"org.example/parent\"\nversion = \"^0.1\"\n"
        ),
    );
    write(
        registry
            .join("org.example")
            .join("parent-docs-ru")
            .join("v0.1.0"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"parent-docs-ru\"\n\
             kind = \"doc\"\nversion = \"0.1.0\"\n{card}\n\
             [i18n]\ncanonical = \"ru\"\n\n\
             [[documents]]\npackage = \"org.example/parent\"\nversion = \"^0.1\"\n\n\
             [translates]\npackage = \"org.example/parent-docs\"\nversion = \"^0.1\"\n"
        ),
    );
    write(
        registry
            .join("org.example")
            .join("host-docs")
            .join("v0.1.0"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"host-docs\"\n\
             kind = \"doc\"\nversion = \"0.1.0\"\n{card}\n\
             [[documents]]\npackage = \"org.example/not-a-package\"\nversion = \"^1.0\"\n"
        ),
    );
}

/// PROP-057 `##REL-WARMUP-CLOSURE`: warming documentation warms what it
/// documents, so a `spec://` citation on its pages resolves offline.
///
/// The adaptation is the load-bearing case: it pulls its SOURCE
/// documentation, and the source pulls its own subject — a level the
/// closure only reaches by running to a fixed point rather than once.
#[test]
fn cache_add_warms_a_doc_packages_subjects_and_its_source() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    add_documentation_to_registry(&registry);
    let user = UserScratch::new();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let bare = tempfile::tempdir().unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/parent-docs-ru")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let store = store_dir(&user);
    for owed in [
        // The adaptation itself.
        "org.example/parent-docs-ru/v0.1.0/vibe.toml",
        // Its `[translates]` source — one level out.
        "org.example/parent-docs/v0.1.0/vibe.toml",
        // The subject both of them document.
        "org.example/parent/v0.1.0/vibe.toml",
        // And the subject's own dependency closure, as before.
        "org.example/child/v0.1.0/vibe.toml",
    ] {
        assert!(
            store.join(owed).is_file(),
            "`{owed}` must be in the store after warming the adaptation; \
             stdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

/// A documented subject no registry can serve does not fail the
/// warm-up — the subject of a documentation may be a project
/// coordinate, and the host itself is one (`##REL-HOST-SUBJECT`). It is
/// reported instead, because one citation on those pages will not open
/// offline and silence would let the reader find that out on a plane.
#[test]
fn an_unwarmable_subject_is_reported_not_raised() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    add_documentation_to_registry(&registry);
    let user = UserScratch::new();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let bare = tempfile::tempdir().unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/host-docs")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an unwarmable subject must not fail the warm-up; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("org.example/not-a-package"),
        "the subject that stayed cold is named:\n{stdout}"
    );
    assert!(
        store_dir(&user)
            .join("org.example/host-docs/v0.1.0/vibe.toml")
            .is_file(),
        "and the documentation itself is warmed all the same"
    );
}

/// Seed store entries directly — `clean` tests act on the store as a
/// directory tree, so they plant fixtures rather than running `add`
/// first: `org.example/wal@{0.1.0,0.2.0}`, `org.example/other@1.0.0`,
/// `org.other/pkg@0.5.0`.
fn seed_store(user: &UserScratch) {
    for (group, name, version) in [
        ("org.example", "wal", "0.1.0"),
        ("org.example", "wal", "0.2.0"),
        ("org.example", "other", "1.0.0"),
        ("org.other", "pkg", "0.5.0"),
    ] {
        let entry = store_dir(user)
            .join(group)
            .join(name)
            .join(format!("v{version}"));
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("vibe.toml"), "seeded\n").unwrap();
    }
}

/// `path` and `list` answer from the settings home alone — no
/// `vibe.toml` anywhere in sight, which is the headline case the
/// top-level namespace ruling exists for.
#[test]
fn cache_path_and_list_work_outside_any_project() {
    let user = UserScratch::new();
    let bare = tempfile::tempdir().unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("path")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let expected = store_dir(&user).display().to_string();
    assert_eq!(
        stdout.trim(),
        expected,
        "`vibe cache path` must print the store root"
    );

    // An empty store is the honest answer, not an error.
    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("store is empty"),
        "expected the empty-store line; got:\n{stdout}"
    );

    // JSON: an empty array, not null and not an error.
    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("--json")
        .arg("cache")
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "cache:list");
    assert_eq!(payload["count"], 0);
    assert_eq!(payload["packages"].as_array().unwrap().len(), 0);
}

/// A projectless `add` resolves from the user-level registries
/// (PROJECTLESS-SOURCE), pulls the named package AND its `[requires]`
/// closure into the store, and materialises nothing into any project.
/// A second add is AlreadyPresent all the way down — the store's
/// bytes are untouched (write-once).
#[test]
fn cache_add_projectless_fills_store_with_closure_and_touches_no_project() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    let user = UserScratch::new();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let bare = tempfile::tempdir().unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/parent")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("2 fetched"),
        "expected parent + child to be fetched; got:\n{stdout}"
    );

    // The closure landed: parent AND child.
    let store = store_dir(&user);
    assert!(
        store.join("org.example/parent/v0.1.0/vibe.toml").is_file(),
        "the parent entry must be in the store"
    );
    assert!(
        store.join("org.example/child/v0.1.0/vibe.toml").is_file(),
        "the dependency closure (child) must be in the store"
    );

    // Nothing materialised into the working directory: no vibe.toml
    // appeared, no lockfile, no vibedeps/.
    assert!(!bare.path().join("vibe.toml").exists());
    assert!(!bare.path().join("vibe.lock").exists());
    assert!(!bare.path().join(common::deps_root()).exists());

    // Write-once: a second add reports AlreadyPresent and does not
    // rewrite the entries — a marker planted inside the entry survives.
    let parent_entry = store.join("org.example/parent/v0.1.0");
    let manifest_bytes = fs::read(parent_entry.join("vibe.toml")).unwrap();
    fs::write(parent_entry.join("MARKER"), "operator's proof\n").unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/parent")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("already present"),
        "the second add must report AlreadyPresent; got:\n{stdout}"
    );
    assert!(
        parent_entry.join("MARKER").is_file(),
        "a re-add must not rewrite the entry (write-once)"
    );
    assert_eq!(
        fs::read(parent_entry.join("vibe.toml")).unwrap(),
        manifest_bytes,
        "the entry's bytes are untouched by a re-add"
    );
}

/// Inside a project, `add` resolves from the project's own
/// `[[registry]]` — and the project stays byte-identical: no
/// `[requires]` recording, no lockfile, no materialisation.
#[test]
fn cache_add_in_project_uses_project_registries_and_changes_nothing() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        format!(
            "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
             [[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let manifest_before = fs::read_to_string(project.path().join("vibe.toml")).unwrap();

    let out = user
        .vibe()
        .current_dir(project.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/child")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        store_dir(&user)
            .join("org.example/child/v0.1.0/vibe.toml")
            .is_file(),
        "the store must hold the fetched child"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("vibe.toml")).unwrap(),
        manifest_before,
        "a pre-warm must not touch the project manifest"
    );
    assert!(!project.path().join("vibe.lock").exists());
    assert!(!project.path().join(common::deps_root()).exists());
}

/// A projectless `add` with no user-level registry configured refuses
/// with an actionable message rather than reaching for a network
/// default.
#[test]
fn cache_add_projectless_without_registries_refuses() {
    let user = UserScratch::new();
    let bare = tempfile::tempdir().unwrap();
    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/parent")
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected a refusal");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("registry"),
        "the refusal must name the missing registry config; got:\n{stderr}"
    );
}

/// The EXPLICIT-RECLAIM guard: a bare `vibe cache clean` refuses and
/// names its three targets. This is the test the red proof disarms.
#[test]
fn cache_clean_refuses_without_a_target() {
    let user = UserScratch::new();
    let out = user.vibe().arg("cache").arg("clean").output().unwrap();
    assert!(!out.status.success(), "a bare clean must refuse");
    let stderr = String::from_utf8_lossy(&out.stderr);
    for flag in ["--all", "--package", "--older-than"] {
        assert!(
            stderr.contains(flag),
            "the refusal must name {flag}; got:\n{stderr}"
        );
    }
}

/// `--package` with a version takes exactly that version; without one
/// it takes the whole name and prunes the emptied `<group>/` dir; an
/// absent target is an error, not a silent zero.
#[test]
fn cache_clean_package_versions_names_and_absent() {
    let user = UserScratch::new();
    seed_store(&user);
    let store = store_dir(&user);

    // One version.
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--package")
        .arg("org.example/wal@0.1.0")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Removed 1 entry"),
        "the version branch must report its count; got:\n{stdout}"
    );
    assert!(!store.join("org.example/wal/v0.1.0").exists());
    assert!(
        store.join("org.example/wal/v0.2.0").is_dir(),
        "the sibling survives"
    );

    // The whole name — its sibling `other` keeps the group dir alive…
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--package")
        .arg("org.example/wal")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!store.join("org.example/wal").exists());
    assert!(
        store.join("org.example/other/v1.0.0").is_dir(),
        "the sibling name survives"
    );
    assert!(
        store.join("org.other/pkg/v0.5.0").is_dir(),
        "other groups untouched"
    );

    // …and once the LAST name of the group goes, the emptied `<group>/`
    // dir goes with it — no husk survives to name the deleted packages.
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--package")
        .arg("org.example/other")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        !store.join("org.example").exists(),
        "the emptied group dir must not linger as a husk"
    );

    // An absent name is an error — the operator named a specific thing.
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--package")
        .arg("org.example/ghost")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no entries"),
        "an absent target must be named; got:\n{stderr}"
    );
}

/// `--older-than` partitions by age: a far-future threshold removes
/// nothing (and that is a success — a young store is not an error), a
/// zero-day threshold removes everything.
#[test]
fn cache_clean_older_than_partitions_by_age() {
    let user = UserScratch::new();
    seed_store(&user);
    let store = store_dir(&user);

    // 100 years: nothing qualifies.
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--older-than")
        .arg("36500")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Removed 0"),
        "a far cutoff must remove nothing; got:\n{stdout}"
    );
    assert!(
        store.join("org.example/wal/v0.1.0").is_dir(),
        "entries survive"
    );

    // 0 days: cutoff is now, every existing entry predates it.
    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--older-than")
        .arg("0")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Removed 4"),
        "a zero cutoff must remove every seeded entry; got:\n{stdout}"
    );
    assert!(
        fs::read_dir(&store).unwrap().count() == 0,
        "the store is empty"
    );
}

/// `--all` is confirm-gated the uninstall way: no TTY without an
/// opt-in is a hard error; `--assume-yes` proceeds and wipes the
/// store (the root directory itself survives, empty).
#[test]
fn cache_clean_all_is_confirm_gated_and_wipes() {
    let user = UserScratch::new();
    seed_store(&user);
    let store = store_dir(&user);

    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--all")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a non-TTY --all without an opt-in must refuse"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--assume-yes"),
        "the refusal must name the opt-in; got:\n{stderr}"
    );
    assert!(
        store.join("org.example/wal/v0.1.0").is_dir(),
        "a refused clean must not have deleted anything"
    );

    let out = user
        .vibe()
        .arg("cache")
        .arg("clean")
        .arg("--all")
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Removed 4"),
        "--all must report the total count; got:\n{stdout}"
    );
    assert!(store.is_dir(), "the store root itself survives");
    assert_eq!(
        fs::read_dir(&store).unwrap().count(),
        0,
        "the store is empty"
    );
}

/// `list` shows what `add` put there — table for the human, one
/// `group/name@version` per line for `--quiet`.
#[test]
fn cache_list_reports_store_contents() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    let user = UserScratch::new();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let bare = tempfile::tempdir().unwrap();
    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("add")
        .arg("org.example/parent")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("org.example"),
        "table lists the group; got:\n{stdout}"
    );
    assert!(
        stdout.contains("parent"),
        "table lists the name; got:\n{stdout}"
    );

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("--quiet")
        .arg("cache")
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["org.example/child@0.1.0", "org.example/parent@0.1.0"],
        "--quiet prints one group/name@version per line, sorted"
    );
}
