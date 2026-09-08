mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::{UserScratch, boot_dir, slot_dir};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, SourceUrl};

fn fixture_library() -> PathBuf {
    assert_eq!(
        vibe_native_loader_json_backend_fixture::fixture_marker(),
        "vibe-native-loader-json-backend-fixture"
    );
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let exact = format!(
        "{}vibe_native_loader_json_backend_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    [profile, deps]
        .into_iter()
        .flat_map(|directory| fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .expect("JSON backend fixture cdylib")
}

fn locked() -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse("json-pass").unwrap(),
        group: Group::parse("org.demo").unwrap(),
        version: "1.0.0".parse().unwrap(),
        registry: None,
        source_url: SourceUrl::new("file:///fixture"),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").unwrap(),
        boot_snippet: Some("boot/10-json.md".to_owned()),
        files_written: Vec::new(),
        dependencies: Vec::new(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }
}

#[derive(Clone, Copy)]
enum Image {
    Ready,
    Missing,
    Stale,
    Source,
}

struct Fixture {
    root: tempfile::TempDir,
    user: UserScratch,
}

impl Fixture {
    fn new(image: Image, fail: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let user = UserScratch::new();
        let package = locked();
        let slot = root
            .path()
            .join(slot_dir("org.demo.json-pass", &package.version.to_string()));
        fs::create_dir_all(slot.join("prebuilt")).unwrap();
        fs::create_dir_all(slot.join("boot")).unwrap();
        fs::write(slot.join("boot/10-json.md"), "# Package {#root}\n\nbody\n").unwrap();
        let library = fixture_library();
        let basename = library.file_name().unwrap();
        let prebuilt = slot.join("prebuilt").join(basename);
        fs::copy(&library, &prebuilt).unwrap();
        let platform = vibe_lifecycle::native::NativePlatform::current().unwrap();
        let handler = if matches!(image, Image::Source) {
            "handler = { kind = \"native\", crate_dir = \"native\" }".to_owned()
        } else {
            format!(
                "handler = {{ kind = \"native\", prebuilt = {{ {} = \"prebuilt/{}\" }} }}",
                platform.key(),
                basename.to_string_lossy()
            )
        };
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.demo'\nname='json-pass'\nkind='tool'\nversion='1.0.0'\n\
                 [boot_snippet]\nsource='boot/10-json.md'\ncategory='flow'\n\
                 [[extension]]\nid='json'\npoint='compile:pass'\n{handler}\ncompiler_internals=true\n\
                 config={{fail=false}}\npass={{kind='backend',artifact='json'}}\n"
            ),
        )
        .unwrap();
        let config = if fail { "\nconfig={fail=true}" } else { "" };
        fs::write(
            root.path().join("vibe.toml"),
            format!(
                "[project]\nname='host'\nversion='0.1.0'\n\
                 [requires.packages]\n'org.demo/json-pass'={{version='=1.0.0',link='static'}}\n\
                 [[extensions.use]]\nref='org.demo/json-pass#json'{config}\n"
            ),
        )
        .unwrap();
        let boot = root.path().join(boot_dir());
        fs::create_dir_all(&boot).unwrap();
        fs::write(boot.join("00-core.md"), "# Host {#root}\n\nbody\n").unwrap();
        let mut lock = Lockfile::empty("test", "2026-09-08T00:00:00Z");
        lock.packages = vec![package];
        lock.write(root.path().join("vibe.lock")).unwrap();
        if matches!(image, Image::Ready | Image::Stale) {
            let bytes = fs::read(&prebuilt).unwrap();
            let digest = format!("{:x}", Sha256::digest(&bytes));
            let directory = root.path().join(".vibe/native-load/e1").join(digest);
            fs::create_dir_all(&directory).unwrap();
            let target = directory.join(basename);
            fs::write(&target, bytes).unwrap();
            if matches!(image, Image::Stale) {
                fs::write(&target, b"stale image").unwrap();
            }
        }
        Self { root, user }
    }

    fn command(&self, out: &Path) -> assert_cmd::Command {
        let mut command = vibe_test_support::cargo_bin("vibe");
        command
            .env(vibe_core::settings::SETTINGS_DIR_ENV, &self.user.settings)
            .env(vibe_test_support::REGISTRY_CACHE_ENV, &self.user.cache)
            .env(vibe_test_support::SEARCH_CACHE_ENV, &self.user.search_cache)
            .env_remove("VIBE_NO_DEFAULT_REGISTRY")
            .args(["extensions", "compile", "--path"])
            .arg(self.root.path())
            .args(["--backend", "json", "--out"])
            .arg(out);
        command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    bytes: Vec<u8>,
    modified: SystemTime,
}

fn snapshot(root: &Path) -> BTreeMap<String, Entry> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Entry>) {
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    Entry {
                        bytes: Vec::new(),
                        modified: fs::metadata(&path).unwrap().modified().unwrap(),
                    },
                );
                walk(root, &path, out);
            } else {
                let metadata = fs::metadata(&path).unwrap();
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    Entry {
                        bytes: fs::read(path).unwrap(),
                        modified: metadata.modified().unwrap(),
                    },
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn activated_backend_emits_raw_deterministic_stdout_and_atomically_replaces_a_file() {
    let fixture = Fixture::new(Image::Ready, false);
    let before = snapshot(fixture.root.path());
    let stdout = fixture.command(Path::new("-")).output().unwrap();
    assert!(
        stdout.status.success(),
        "{}",
        String::from_utf8_lossy(&stdout.stderr)
    );
    assert!(stdout.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&stdout.stdout).unwrap();
    assert_eq!(value["context"]["target"], "json");
    assert_eq!(snapshot(fixture.root.path()), before);

    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("lane.json");
    fs::write(&out, b"sentinel").unwrap();
    let file = fixture.command(&out).output().unwrap();
    assert!(
        file.status.success(),
        "{}",
        String::from_utf8_lossy(&file.stderr)
    );
    assert!(file.stderr.is_empty());
    assert_eq!(fs::read(&out).unwrap(), stdout.stdout);
    let summary = String::from_utf8(file.stdout).unwrap();
    assert_eq!(summary.lines().count(), 1);
    assert!(summary.contains("compiled backend `json`"));
    assert_eq!(snapshot(fixture.root.path()), before);
}

#[test]
fn provider_and_native_failures_preserve_output_boot_scratch_records_and_images() {
    for image in [Image::Missing, Image::Stale, Image::Source] {
        let fixture = Fixture::new(image, false);
        let before = snapshot(fixture.root.path());
        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("lane.json");
        fs::write(&out, b"sentinel").unwrap();
        let failed = fixture.command(&out).output().unwrap();
        assert!(!failed.status.success());
        assert_eq!(fs::read(&out).unwrap(), b"sentinel");
        assert_eq!(snapshot(fixture.root.path()), before);
    }

    let fixture = Fixture::new(Image::Ready, true);
    let before = snapshot(fixture.root.path());
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("lane.json");
    fs::write(&out, b"sentinel").unwrap();
    let failed = fixture.command(&out).output().unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("deterministic JSON backend refusal"));
    assert_eq!(fs::read(&out).unwrap(), b"sentinel");
    assert_eq!(snapshot(fixture.root.path()), before);
}
