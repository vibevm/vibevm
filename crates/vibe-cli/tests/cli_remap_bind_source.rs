//! Explicit remap binding verifies its endpoint and mutates only the owning lock.
#![cfg(unix)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
mod common;
use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Output,
};
use vibe_core::manifest::{Lockfile, Manifest};

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !at.exists() {
            return;
        }
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == ".vibe" {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}
struct Fixture {
    _temp: tempfile::TempDir,
    project: PathBuf,
    source: PathBuf,
    binding: PathBuf,
    home: PathBuf,
    user: UserScratch,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let project = root.join("project");
        let source = root.join("original/org.example/policy/v1.0.0");
        write(
            &source.join("vibe.toml"),
            "[package]\ngroup='org.example'\nname='policy'\nversion='1.0.0'\nkind='remap'\n[remap]\ndescription='held source fixture'\n[[remap.rows]]\nprefix='https://example.invalid/'\nurl='https://mirror.invalid/'\nreason='fixture row; a remap package must carry one'\n",
        );
        write(&source.join("payload"), "original policy bytes");
        write(
            &project.join("vibe.toml"),
            "[project]\nname='project'\nversion='0.1.0'\n[requires.remaps]\n'org.example/policy'='1.0.0'\n",
        );
        Manifest::read(project.join("vibe.toml")).unwrap();
        let hash = vibe_registry::compute_content_hash(&source).unwrap();
        write(
            &project.join("vibe.lock"),
            &format!(
                "[meta]\nschema_version=7\ngenerated_by='fixture'\ngenerated_at='fixed'\n[[remap]]\npackage='org.example/policy'\nversion='1.0.0'\ncontent_hash='{hash}'\nregistry='old-alias'\n# keep this shared comment\n"
            ),
        );
        let binding = root.join("binding.toml");
        write(
            &binding,
            &format!(
                "source_url='file://{}'\nsource_kind='registry'\n",
                source.display()
            ),
        );
        let home = root.join("home");
        fs::create_dir(&home).unwrap();
        Self {
            _temp: temp,
            project,
            source,
            binding,
            home,
            user: UserScratch::new(),
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        self.user
            .vibe()
            .current_dir(&self.project)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env_remove("VIBE_REMAP")
            .args(args)
            .args(["--unattended", "--json", "--reject-remap"])
            .output()
            .unwrap()
    }
    fn bind(&self, selected: &Path) -> Output {
        self.run(&[
            "remap",
            "bind-source",
            "org.example/policy",
            "--source",
            self.binding.to_str().unwrap(),
            "--path",
            selected.to_str().unwrap(),
        ])
    }
    fn remap_home(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        files(&self.user.settings.join("remap"))
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn legacy_missing_source_refuses_before_project_or_home_effects_then_explicit_binding_is_idempotent()
 {
    let f = Fixture::new();
    let project = files(&f.project);
    let home = f.remap_home();
    let refused = f.run(&["install", "--path", f.project.to_str().unwrap()]);
    assert!(!refused.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(diagnostic.contains("missing-locked-source"), "{diagnostic}");
    assert_eq!(files(&f.project), project);
    assert_eq!(f.remap_home(), home);
    let report = success(f.bind(&f.project));
    assert_eq!(report["binding"], "newly_verified");
    assert_eq!(f.remap_home(), home);
    assert_eq!(
        fs::read(f.project.join("vibe.toml")).unwrap(),
        project[Path::new("vibe.toml")]
    );
    let lock = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    assert_eq!(lock.meta.schema_version, 9);
    assert_eq!(lock.meta.generated_at, "fixed");
    assert_eq!(
        lock.remaps[0].locked_package().unwrap().source_url.as_str(),
        format!("file://{}", f.source.display())
    );
    let bytes = fs::read(f.project.join("vibe.lock")).unwrap();
    assert_eq!(success(f.bind(&f.project))["changed"], false);
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), bytes);
    assert_eq!(f.remap_home(), home);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn explicit_wrong_source_cannot_be_rescued_by_cached_old_bytes() {
    let f = Fixture::new();
    success(f.bind(&f.project));
    let before = files(&f.project);
    let home = f.remap_home();
    write(
        &f.source.join("payload"),
        "different bytes at explicitly supplied source",
    );
    assert!(!f.bind(&f.project).status.success());
    assert_eq!(files(&f.project), before);
    assert_eq!(f.remap_home(), home);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn cold_bound_remap_ignores_edited_registry_and_keeps_shared_lock_exact() {
    let f = Fixture::new();
    success(f.bind(&f.project));
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    write(
        &f.user.settings.join("registry.toml"),
        &format!(
            "[[registry]]\nname='old-alias'\nurl='file://{}'\n",
            f.home.join("missing-current-registry").display()
        ),
    );
    success(f.run(&["install", "--path", f.project.to_str().unwrap()]));
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(
        fs::read(
            f.user
                .settings
                .join("remap/packages/org.example__policy/payload")
        )
        .unwrap(),
        b"original policy bytes"
    );
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn member_binding_uses_parent_lock_and_diagnoses_a_misplaced_child_lock() {
    let f = Fixture::new();
    let member = f.project.join("member");
    write(
        &member.join("vibe.toml"),
        "[project]\nname='member'\nversion='0.1.0'\n[requires.remaps]\n'org.example/policy'='1.0.0'\n",
    );
    write(
        &f.project.join("vibe.toml"),
        "[project]\nname='root'\nversion='0.1.0'\n[workspace]\nmembers=['member']\n",
    );
    let before = files(&f.project);
    let home = f.remap_home();
    assert!(
        !f.run(&["install", "--path", member.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(files(&f.project), before);
    assert_eq!(f.remap_home(), home);
    success(f.bind(&member));
    assert!(!member.join("vibe.lock").exists());
    assert_eq!(
        Lockfile::read(f.project.join("vibe.lock"))
            .unwrap()
            .meta
            .schema_version,
        9
    );
    assert_eq!(f.remap_home(), home);
    fs::copy(f.project.join("vibe.lock"), member.join("vibe.lock")).unwrap();
    let before = files(&f.project);
    assert!(!f.bind(&member).status.success());
    assert_eq!(files(&f.project), before);
}

#[test]
#[ignore = "explicit Git binding now runs on the controlled native transport, which admits only HTTPS/SSH (spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION); this local git+file fixture is refused before binding, and no CLI-level HTTPS Git fixture exists yet — re-enable on one (BACKLOG B-235)"]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn only_explicit_new_binding_records_newly_observed_git_commit() {
    let f = Fixture::new();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .current_dir(&f.source)
            .env("HOME", &f.home)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_ALLOW_PROTOCOL", "file")
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--initial-branch=main"]);
    git(&["add", "."]);
    git(&["commit", "-m", "fixture source"]);
    let commit = git(&["rev-parse", "HEAD"]);
    write(
        &f.binding,
        &format!(
            "source_url='git+file://{}'\nsource_ref='main'\nsource_kind='git'\n",
            f.source.display()
        ),
    );
    success(f.bind(&f.project));
    let path = f.project.join("vibe.lock");
    let mut lock = Lockfile::read(&path).unwrap();
    assert_eq!(
        lock.remaps[0]
            .source
            .as_ref()
            .unwrap()
            .resolved_commit
            .as_deref(),
        Some(commit.as_str())
    );
    lock.remaps[0].source.as_mut().unwrap().resolved_commit = None;
    lock.write(&path).unwrap();
    let absent = fs::read(&path).unwrap();
    success(f.run(&["install", "--path", f.project.to_str().unwrap()]));
    assert_eq!(
        fs::read(&path).unwrap(),
        absent,
        "ordinary replay cannot fill historical absence"
    );
    success(f.bind(&f.project));
    assert_eq!(
        Lockfile::read(&path).unwrap().remaps[0]
            .source
            .as_ref()
            .unwrap()
            .resolved_commit
            .as_deref(),
        Some(commit.as_str())
    );
}
