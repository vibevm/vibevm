//! `vibe update --all` (and a bare `vibe update`) moves every locked root to
//! the newest version its declared constraint admits, through its locked
//! source, exactly as a scoped `vibe update <pkg>` moves one.
//!
//! B-234: the whole-graph path used to replay every lock row like a plain
//! install, so a registry that gained a patch release was reported as
//! `unchanged` while `--help` promised "Update every package in the lockfile".
//!
//! Isolation: every command is built by [`UserScratch`]; the fixture registry
//! is a `file://` directory under a tempdir, named in the scratch machine
//! `registry.toml`.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::UserScratch;
use vibe_core::manifest::Lockfile;

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// Write `org.example/<name>@<version>` (a `flow`) into the monorepo registry
/// shape, requiring each `(coordinate, constraint)` in `requires`.
fn publish(registry: &Path, name: &str, version: &str, requires: &[(&str, &str)]) {
    let dir = registry
        .join("org.example")
        .join(name)
        .join(format!("v{version}"));
    let flow = dir.join(common::spec_rel(&format!("flows/{name}")));
    fs::create_dir_all(&flow).unwrap();
    let mut manifest = format!(
        "[package]\ngroup = \"org.example\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n"
    );
    if !requires.is_empty() {
        let rows: Vec<String> = requires
            .iter()
            .map(|(coordinate, constraint)| format!("\"{coordinate}\" = \"{constraint}\""))
            .collect();
        manifest.push_str(&format!(
            "\n[requires]\npackages = {{ {} }}\n",
            rows.join(", ")
        ));
    }
    fs::write(dir.join("vibe.toml"), manifest).unwrap();
    fs::write(
        flow.join("RULES.md"),
        format!("# {name} {version}\n\nbody of {name} at {version}\n"),
    )
    .unwrap();
}

struct Fixture {
    user: UserScratch,
    _outer: tempfile::TempDir,
    registry: PathBuf,
    project: tempfile::TempDir,
}

impl Fixture {
    /// Two roots (`p`, `q`); `p` requires `dep@^0.1.0`. Everything starts at
    /// 0.1.0 and is installed.
    fn installed() -> Self {
        let user = UserScratch::new();
        let outer = tempfile::tempdir().unwrap();
        let registry = outer.path().join("registry");
        publish(&registry, "p", "0.1.0", &[("org.example/dep", "^0.1.0")]);
        publish(&registry, "q", "0.1.0", &[]);
        publish(&registry, "dep", "0.1.0", &[]);
        fs::write(
            user.settings.join("registry.toml"),
            format!(
                "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
                file_url(&registry)
            ),
        )
        .unwrap();
        let project = tempfile::tempdir().unwrap();
        user.init_project(project.path());
        for package in ["org.example/p", "org.example/q"] {
            user.vibe()
                .args(["install", package, "--assume-yes", "--path"])
                .arg(project.path())
                .assert()
                .success();
        }
        let fixture = Self {
            user,
            _outer: outer,
            registry,
            project,
        };
        assert_eq!(
            fixture.versions(),
            [("dep", "0.1.0"), ("p", "0.1.0"), ("q", "0.1.0")].map(owned),
            "fixture: the install locks every package at 0.1.0",
        );
        fixture
    }

    fn update(&self, extra: &[&str]) -> serde_json::Value {
        let output = self
            .user
            .vibe()
            .args(["update", "--json", "--assume-yes"])
            .args(extra)
            .arg("--path")
            .arg(self.project.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let docs = common::hook_slot::documents(&output.stdout);
        docs.into_iter()
            .rev()
            .find(|doc| doc["command"] == "update")
            .unwrap_or_else(|| {
                panic!(
                    "an update root; stdout={}",
                    String::from_utf8_lossy(&output.stdout)
                )
            })
    }

    /// Bytes and modification time of every file a materialisation or boot
    /// regeneration could write: the dependency slots, the boot directory and
    /// the project's top-level files, keyed by project-relative path.
    fn written_tree(&self) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
        let root = self.project.path();
        let mut out = BTreeMap::new();
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                record(root, &path, &mut out);
            }
        }
        for dir in [
            vibe_core::layout::current_vibedeps_root(),
            vibe_core::layout::current_boot_dir(),
        ] {
            walk(root, &root.join(dir), &mut out);
        }
        out
    }

    fn lock_bytes(&self) -> Vec<u8> {
        fs::read(self.project.path().join(Lockfile::FILENAME)).unwrap()
    }

    /// `(name, version)` of every locked package, sorted by name.
    fn versions(&self) -> Vec<(String, String)> {
        let lock = Lockfile::read(self.project.path().join(Lockfile::FILENAME)).unwrap();
        let mut rows: Vec<(String, String)> = lock
            .packages
            .iter()
            .map(|row| (row.name.to_string(), row.version.to_string()))
            .collect();
        rows.sort();
        rows
    }

    fn slot(&self, name: &str, version: &str) -> PathBuf {
        self.project
            .path()
            .join(common::slot_dir(&format!("org.example.{name}"), version))
    }
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, (Vec<u8>, SystemTime)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(root, &path, out);
        } else {
            record(root, &path, out);
        }
    }
}

fn record(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (Vec<u8>, SystemTime)>) {
    let modified = fs::metadata(path).unwrap().modified().unwrap();
    out.insert(
        path.strip_prefix(root).unwrap().to_path_buf(),
        (fs::read(path).unwrap(), modified),
    );
}

fn owned((name, version): (&str, &str)) -> (String, String) {
    (name.to_string(), version.to_string())
}

/// `--all` bumps BOTH roots and the transitive dependency to the newest
/// version each constraint admits — and no further: `dep@0.2.0` is outside
/// `p`'s `^0.1.0`, `q@1.0.0` is outside the caret the install recorded.
#[test]
fn update_all_bumps_every_root_and_respects_the_closure_constraints() {
    let f = Fixture::installed();
    publish(&f.registry, "p", "0.1.1", &[("org.example/dep", "^0.1.0")]);
    publish(&f.registry, "q", "0.1.1", &[]);
    publish(&f.registry, "q", "1.0.0", &[]);
    publish(&f.registry, "dep", "0.1.1", &[]);
    publish(&f.registry, "dep", "0.2.0", &[]);

    let report = f.update(&["--all"]);
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["unchanged"], false, "{report:#}");
    assert_eq!(report["packages_resolved"], 3, "{report:#}");
    assert_eq!(
        f.versions(),
        [("dep", "0.1.1"), ("p", "0.1.1"), ("q", "0.1.1")].map(owned),
    );
    for name in ["p", "q", "dep"] {
        assert!(f.slot(name, "0.1.1").is_dir(), "{name}@0.1.1 materialised");
        assert!(!f.slot(name, "0.1.0").exists(), "{name}@0.1.0 pruned");
    }
    let bumps: Vec<&str> = report["version_bumps"]
        .as_array()
        .unwrap_or_else(|| panic!("version_bumps array: {report:#}"))
        .iter()
        .map(|bump| bump.as_str().unwrap())
        .collect();
    assert_eq!(bumps.len(), 3, "one bump per moved package, got {bumps:?}");
}

/// A bare `vibe update` is `--all`.
#[test]
fn bare_update_is_update_all() {
    let f = Fixture::installed();
    publish(&f.registry, "q", "0.1.1", &[]);
    let report = f.update(&[]);
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(
        f.versions(),
        [("dep", "0.1.0"), ("p", "0.1.0"), ("q", "0.1.1")].map(owned),
    );
}

/// Nothing newer anywhere: `--all` re-resolves every root, finds that nothing
/// would move, and writes nothing — no slot, boot artifact or lock file
/// changes bytes or modification time — and says so with `unchanged: true`.
#[test]
fn update_all_with_nothing_newer_is_an_unchanged_no_op() {
    let f = Fixture::installed();
    let before = f.lock_bytes();
    let tree_before = f.written_tree();
    assert!(
        tree_before
            .keys()
            .any(|path| path.starts_with(vibe_core::layout::current_boot_dir())),
        "fixture: the install generated boot artifacts: {:?}",
        tree_before.keys().collect::<Vec<_>>(),
    );
    let report = f.update(&["--all"]);
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["unchanged"], true, "{report:#}");
    assert_eq!(report["complete"], true, "{report:#}");
    assert_eq!(report["packages_resolved"], 3, "{report:#}");
    assert_eq!(report["version_bumps"], serde_json::json!([]), "{report:#}");
    assert_eq!(report["materialised"], serde_json::json!([]), "{report:#}");
    assert_eq!(report["pruned"], serde_json::json!([]), "{report:#}");
    // Boot is regenerated over the unchanged world, but a regeneration that
    // changes no byte publishes nothing — the tree comparison below proves it.
    assert_eq!(
        report["nodes_regenerated"],
        serde_json::json!(["."]),
        "{report:#}"
    );
    assert_eq!(f.lock_bytes(), before, "the lock must not move");
    let tree_after = f.written_tree();
    for (path, (bytes, modified)) in &tree_before {
        let Some((after_bytes, after_modified)) = tree_after.get(path) else {
            panic!("`{}` was removed by a no-op update", path.display());
        };
        assert_eq!(after_bytes, bytes, "`{}` bytes changed", path.display());
        assert_eq!(
            after_modified,
            modified,
            "`{}` was rewritten by a no-op update",
            path.display()
        );
    }
    assert_eq!(
        tree_after.keys().collect::<Vec<_>>(),
        tree_before.keys().collect::<Vec<_>>(),
        "a no-op update creates no slot or boot file",
    );
    for name in ["p", "q", "dep"] {
        assert!(f.slot(name, "0.1.0").is_dir(), "{name}@0.1.0 kept");
    }
}

/// A slot whose payload no longer matches its recorded source is NOT a no-op:
/// `--all` repairs it, and the report says it changed something.
#[test]
fn update_all_repairs_a_tampered_slot_instead_of_reporting_unchanged() {
    let f = Fixture::installed();
    let rules = f
        .slot("q", "0.1.0")
        .join(common::spec_rel("flows/q"))
        .join("RULES.md");
    let original = fs::read_to_string(&rules).unwrap();
    fs::write(&rules, "tampered\n").unwrap();
    let report = f.update(&["--all"]);
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(report["unchanged"], false, "{report:#}");
    assert_eq!(
        fs::read_to_string(&rules).unwrap(),
        original,
        "the slot is repaired"
    );
}

/// `--all --sources-only` still pins every locked version exactly, even when
/// newer versions exist.
#[test]
fn update_all_sources_only_keeps_every_version() {
    let f = Fixture::installed();
    publish(&f.registry, "p", "0.1.1", &[("org.example/dep", "^0.1.0")]);
    publish(&f.registry, "q", "0.1.1", &[]);
    publish(&f.registry, "dep", "0.1.1", &[]);
    let report = f.update(&["--all", "--sources-only"]);
    assert_eq!(report["ok"], true, "{report:#}");
    assert_eq!(
        f.versions(),
        [("dep", "0.1.0"), ("p", "0.1.0"), ("q", "0.1.0")].map(owned),
    );
}
