mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::{UserScratch, boot_dir, slot_dir};
use sha2::{Digest, Sha256};
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, SourceUrl};

fn fixture_library(stem: &str) -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let exact = format!(
        "{}{stem}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    [profile, deps]
        .into_iter()
        .flat_map(|directory| fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .unwrap_or_else(|| panic!("fixture cdylib `{exact}`"))
}

fn fixture_libraries() -> (PathBuf, PathBuf) {
    assert_eq!(
        vibe_native_loader_json_backend_fixture::fixture_marker(),
        "vibe-native-loader-json-backend-fixture"
    );
    assert_eq!(
        vibe_native_loader_compiler_fixture::fixture_marker(),
        "vibe-native-loader-compiler-fixture"
    );
    (
        fixture_library("vibe_native_loader_json_backend_fixture"),
        fixture_library("vibe_native_loader_compiler_fixture"),
    )
}

fn locked() -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse("json-pass").unwrap(),
        group: Group::parse("org.demo").unwrap(),
        version: "1.0.0".parse().unwrap(),
        bridge: false,
        authors: Vec::new(),
        registry: None,
        source_url: SourceUrl::new("file:///fixture"),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").unwrap(),
        embedded_sources: Vec::new(),
        boot_snippet: Some("vibevm/vibespecs/boot/10-json.md".to_owned()),
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
        Self::new_with(image, fail, false)
    }

    fn invalid_frontend() -> Self {
        Self::new_with(Image::Ready, true, true)
    }

    fn new_with(image: Image, fail: bool, invalid_frontend: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let user = UserScratch::new();
        let package = locked();
        let slot = root
            .path()
            .join(slot_dir("org.demo.json-pass", &package.version.to_string()));
        fs::create_dir_all(slot.join("prebuilt")).unwrap();
        fs::create_dir_all(slot.join("vibevm/vibespecs/boot")).unwrap();
        fs::create_dir_all(slot.join("vibevm/vibespecs/common")).unwrap();
        fs::write(
            slot.join("vibevm/vibespecs/boot/10-json.md"),
            "# Package {#root}\n#use spec://org.demo/json-pass/common/NOTE#root\n\nbody\n",
        )
        .unwrap();
        fs::write(
            slot.join("vibevm/vibespecs/common/NOTE.txt"),
            "first café\n\nsecond λ\n",
        )
        .unwrap();
        let (backend_library, frontend_library) = fixture_libraries();
        let backend_name = backend_library.file_name().unwrap();
        let frontend_name = frontend_library.file_name().unwrap();
        let backend_prebuilt = slot.join("prebuilt").join(backend_name);
        let frontend_prebuilt = slot.join("prebuilt").join(frontend_name);
        fs::copy(&backend_library, &backend_prebuilt).unwrap();
        fs::copy(&frontend_library, &frontend_prebuilt).unwrap();
        let platform = vibe_lifecycle::native::NativePlatform::current().unwrap();
        let backend_handler = if matches!(image, Image::Source) {
            "handler = { kind = \"native\", crate_dir = \"native\" }".to_owned()
        } else {
            format!(
                "handler = {{ kind = \"native\", prebuilt = {{ {} = \"prebuilt/{}\" }} }}",
                platform.key(),
                backend_name.to_string_lossy()
            )
        };
        let frontend_handler = format!(
            "handler = {{ kind = \"native\", prebuilt = {{ {} = \"prebuilt/{}\" }} }}",
            platform.key(),
            frontend_name.to_string_lossy()
        );
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.demo'\nname='json-pass'\nkind='tool'\nversion='1.0.0'\nformat='normal'\n\
                 [boot_snippet]\nsource='vibevm/vibespecs/boot/10-json.md'\ncategory='flow'\n\
                 [[extension]]\nid='compiler-ok'\npoint='compile:pass'\n{frontend_handler}\ncompiler_internals=true\n\
                 pass={{kind='frontend',formats=['txt']}}\n\
                 [[extension]]\nid='json'\npoint='compile:pass'\n{backend_handler}\ncompiler_internals=true\n\
                 config={{fail=false}}\npass={{kind='backend',artifact='json'}}\n"
            ),
        )
        .unwrap();
        let frontend_config = if invalid_frontend {
            "\nconfig={invalid_frontend=true}"
        } else {
            ""
        };
        let backend_config = if fail { "\nconfig={fail=true}" } else { "" };
        fs::write(
            root.path().join("vibe.toml"),
            format!(
                "[project]\nname='host'\nversion='0.1.0'\n\
                 [requires.packages]\n'org.demo/json-pass'={{version='=1.0.0',link='static'}}\n\
                 [[extensions.use]]\nref='org.demo/json-pass#compiler-ok'{frontend_config}\n\
                 [[extensions.use]]\nref='org.demo/json-pass#json'{backend_config}\n"
            ),
        )
        .unwrap();
        let boot = root.path().join(boot_dir());
        fs::create_dir_all(&boot).unwrap();
        fs::write(boot.join("00-core.md"), "# Host {#root}\n\nbody\n").unwrap();
        let mut lock = Lockfile::empty("test", "2026-09-08T00:00:00Z");
        lock.packages = vec![package];
        lock.write(root.path().join("vibe.lock")).unwrap();
        publish_image(root.path(), &frontend_prebuilt, false);
        if matches!(image, Image::Ready | Image::Stale) {
            publish_image(
                root.path(),
                &backend_prebuilt,
                matches!(image, Image::Stale),
            );
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

fn publish_image(root: &Path, prebuilt: &Path, stale: bool) {
    let bytes = fs::read(prebuilt).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let directory = root.join(".vibe/native-load/e1").join(digest);
    fs::create_dir_all(&directory).unwrap();
    let target = directory.join(prebuilt.file_name().unwrap());
    fs::write(&target, bytes).unwrap();
    if stale {
        fs::write(target, b"stale image").unwrap();
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

fn contains_txt_document(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => {
            text == "# NOTE {#org-demo--json-pass--common-note--root}\n\nfirst café\n\nsecond λ"
        }
        serde_json::Value::Array(values) => values.iter().any(contains_txt_document),
        serde_json::Value::Object(fields) => fields.values().any(contains_txt_document),
        _ => false,
    }
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
    assert!(stdout.stdout.ends_with(b"\n"));
    let value: serde_json::Value = serde_json::from_slice(&stdout.stdout).unwrap();
    assert_eq!(value["context"]["target"], "json");
    assert!(
        contains_txt_document(&value),
        "the mixed lane must contain the physical TXT document: {value:#}"
    );
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

#[test]
fn invalid_frontend_output_stops_before_backend_and_preserves_the_output() {
    let fixture = Fixture::invalid_frontend();
    let before = snapshot(fixture.root.path());
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("lane.json");
    fs::write(&out, b"sentinel").unwrap();

    let failed = fixture.command(&out).output().unwrap();

    assert!(!failed.status.success());
    let stderr = String::from_utf8_lossy(&failed.stderr);
    assert!(stderr.contains("span-bounds"), "{stderr}");
    assert!(!stderr.contains("deterministic JSON backend refusal"));
    assert_eq!(fs::read(&out).unwrap(), b"sentinel");
    assert_eq!(snapshot(fixture.root.path()), before);
}
