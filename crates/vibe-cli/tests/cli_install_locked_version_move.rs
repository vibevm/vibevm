//! `vibe install <pkg>@<req>` moves a held version at its locked source.
//!
//! Owner decision 2026-11-08 (PROP-073 `#LOCKED-SOURCE-LAW`): an explicit
//! constraint the held version does not satisfy moves the version at the SAME
//! locked source, like `cargo add pkg@2`; install never changes the source
//! (`vibe update --reselect-source` does). A request the locked source cannot
//! serve refuses with the source named and that fix, leaving the lock
//! byte-identical. Packages the install merely carries keep strict replay.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
use vibe_core::manifest::Lockfile;

fn package(root: &Path, name: &str, version: &str) {
    let dir = root.join(format!("org.policy/{name}/v{version}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[package]\ngroup='org.policy'\nname='{name}'\nversion='{version}'\nkind='tool'\n"),
    )
    .unwrap();
}

struct Fixture {
    user: UserScratch,
    project: tempfile::TempDir,
    first: tempfile::TempDir,
    second: tempfile::TempDir,
}

impl Fixture {
    /// `demo` and `keep` held at 1.0.0 from registry `first`; afterwards
    /// `first` also serves demo 2.0.0 and keep 1.1.0, and a registry
    /// `second`, declared ahead of `first`, serves demo 2.0.0 and 3.0.0.
    fn new() -> Self {
        let user = UserScratch::new();
        fs::write(user.settings.join("registry.toml"), "").unwrap();
        let f = Self {
            user,
            project: tempfile::tempdir().unwrap(),
            first: tempfile::tempdir().unwrap(),
            second: tempfile::tempdir().unwrap(),
        };
        package(f.first.path(), "demo", "1.0.0");
        package(f.first.path(), "keep", "1.0.0");
        fs::write(
            f.project.path().join("vibe.toml"),
            format!(
                "[project]\nname='moves'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'='^1'\n'org.policy/keep'='*'\n[[registry]]\nname='first'\nurl='file://{}'\n",
                f.first.path().display()
            ),
        )
        .unwrap();
        success(&f.install(&[]));
        assert_eq!(f.row("demo").version.to_string(), "1.0.0");
        package(f.first.path(), "demo", "2.0.0");
        package(f.first.path(), "keep", "1.1.0");
        package(f.second.path(), "demo", "2.0.0");
        package(f.second.path(), "demo", "3.0.0");
        let manifest = fs::read_to_string(f.project.path().join("vibe.toml")).unwrap();
        let (head, first) = manifest.split_once("[[registry]]").unwrap();
        fs::write(
            f.project.path().join("vibe.toml"),
            format!(
                "{head}[[registry]]\nname='second'\nurl='file://{}'\n[[registry]]{first}",
                f.second.path().display()
            ),
        )
        .unwrap();
        f
    }
    fn install(&self, packages: &[&str]) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.project.path())
            .env("HOME", &self.user.settings)
            .args(["install", "--json", "--assume-yes", "--no-default-registry"])
            .args(packages)
            .arg("--path")
            .arg(self.project.path())
            .output()
            .unwrap()
    }
    fn lock_bytes(&self) -> Vec<u8> {
        fs::read(self.project.path().join("vibe.lock")).unwrap()
    }
    fn row(&self, name: &str) -> vibe_core::manifest::LockedPackage {
        Lockfile::read(self.project.path().join("vibe.lock"))
            .unwrap()
            .packages
            .into_iter()
            .find(|row| row.name.as_str() == name)
            .unwrap()
    }
}

fn success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn explicit_constraint_moves_the_version_at_the_locked_source_and_carried_rows_replay() {
    let f = Fixture::new();
    let keep = f.row("keep");
    // The plan shows the same move first and writes nothing.
    let held = f.lock_bytes();
    let plan = f.install(&["org.policy/demo@=2.0.0", "--plan-only"]);
    success(&plan);
    let plan: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    let row = plan["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["package"] == "org.policy/demo")
        .unwrap();
    assert_eq!(row["version"], "2.0.0");
    assert_eq!(row["source_evidence"]["registry"], "first");
    assert_eq!(f.lock_bytes(), held, "a plan writes no lock");
    success(&f.install(&["org.policy/demo@=2.0.0"]));
    let demo = f.row("demo");
    assert_eq!(demo.version.to_string(), "2.0.0");
    // Same locked source: the `first` registry, never the earlier-declared
    // `second` that also serves 2.0.0. A directory registry records the
    // version directory, so the URL moves only within that registry root.
    assert_eq!(demo.registry.as_deref(), Some("first"));
    let url = demo.source_url.as_str();
    assert!(
        url.contains(f.first.path().to_str().unwrap()) && url.ends_with("demo/v2.0.0"),
        "{url}"
    );
    assert!(!url.contains(f.second.path().to_str().unwrap()), "{url}");
    // The carried package keeps strict replay: neither moved to 1.1.0 nor rewritten.
    assert_eq!(f.row("keep"), keep);
    // A bare install afterwards is a strict replay of the moved lock.
    let moved = f.lock_bytes();
    success(&f.install(&[]));
    assert_eq!(f.lock_bytes(), moved);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn a_version_absent_at_the_locked_source_refuses_naming_it_and_the_fix() {
    let f = Fixture::new();
    let lock = f.lock_bytes();
    let manifest = fs::read(f.project.path().join("vibe.toml")).unwrap();
    let output = f.install(&["org.policy/demo@=3.0.0"]);
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("not available at its locked source")
            && text.contains("registry `first`")
            && text.contains("vibe update org.policy/demo --reselect-source")
            && text.contains("PROP-073#LOCKED-SOURCE-LAW"),
        "{text}"
    );
    assert_eq!(
        f.lock_bytes(),
        lock,
        "a refusal leaves vibe.lock byte-identical"
    );
    assert_eq!(
        fs::read(f.project.path().join("vibe.toml")).unwrap(),
        manifest,
        "a refusal leaves vibe.toml byte-identical"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn a_satisfied_constraint_and_a_bare_install_keep_strict_replay() {
    let f = Fixture::new();
    // Row order follows root order; the replayed rows themselves must not move.
    let rows = || {
        let mut rows = Lockfile::read(f.project.path().join("vibe.lock"))
            .unwrap()
            .packages;
        rows.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        rows
    };
    let held = rows();
    // `^1` is satisfied by the held 1.0.0: no move to 1.1.0 or anything else.
    // Only the recorded root constraint changes; every package row replays.
    success(&f.install(&["org.policy/keep@^1"]));
    assert_eq!(rows(), held);
    let lock = f.lock_bytes();
    success(&f.install(&[]));
    assert_eq!(f.lock_bytes(), lock);
}
