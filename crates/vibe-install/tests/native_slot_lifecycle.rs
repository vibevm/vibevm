//! Production slot-native wiring through the shared ARTIFACT backend.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_install::{InstallSlotLifecycle, SlotLifecycleSeams};
use vibe_lifecycle::LifecycleLease;
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::install::{ResolvedDep, SlotLifecycle, SlotLifecycleContext};

fn fixture_library() -> PathBuf {
    assert_eq!(
        vibe_native_loader_fixture::fixture_marker(),
        "vibe-native-loader-fixture"
    );
    let executable = std::env::current_exe().expect("current test executable");
    let executable_dir = executable.parent().expect("test executable directory");
    let profile_dir = if executable_dir
        .file_name()
        .is_some_and(|name| name == "deps")
    {
        executable_dir.parent().expect("Cargo profile directory")
    } else {
        executable_dir
    };
    let exact_name = format!(
        "{}vibe_native_loader_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let mut candidates = Vec::new();
    collect_exact(profile_dir, &exact_name, &mut candidates);
    collect_exact(&profile_dir.join("deps"), &exact_name, &mut candidates);
    candidates.sort();
    candidates.dedup();
    assert_eq!(candidates.len(), 1, "one exact SDK fixture: {candidates:?}");
    candidates.pop().expect("one fixture DLL")
}

fn collect_exact(directory: &Path, exact_name: &str, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("read Cargo target entry");
        if entry.file_name() == exact_name && entry.file_type().expect("artifact type").is_file() {
            candidates.push(entry.path());
        }
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    host: Manifest,
    resolution: Vec<ResolvedDep>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(
            root.join("vibe.toml"),
            "[project]\nname='demo'\ngroup='org.demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        let host = Manifest::read(root.join("vibe.toml")).unwrap();
        let slot = root
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.native")
            .join("1.0.0");
        std::fs::create_dir_all(slot.join("prebuilt")).unwrap();
        let prebuilt_name = format!("slot-native{}", std::env::consts::DLL_SUFFIX);
        std::fs::copy(
            fixture_library(),
            slot.join("prebuilt").join(&prebuilt_name),
        )
        .unwrap();
        std::fs::create_dir_all(slot.join("native/src")).unwrap();
        std::fs::write(
            slot.join("native/Cargo.toml"),
            "[package]\nname='source-only'\nversion='0.1.0'\nedition='2024'\n\n[lib]\ncrate-type=['cdylib']\n",
        )
        .unwrap();
        std::fs::write(slot.join("native/src/lib.rs"), "pub fn never_built() {}\n").unwrap();
        let manifest_text = format!(
            "[package]\ngroup='org.demo'\nname='native'\nkind='tool'\nversion='1.0.0'\n\n\
             [[extension]]\nid='slot-pre-fixture'\npoint='slot:pre-install'\n\
             handler={{kind='native',prebuilt={{\"{}\"='prebuilt/{}'}}}}\n\n\
             [[extension]]\nid='slot-post-fixture'\npoint='slot:post-install'\n\
             handler={{kind='native',crate_dir='native'}}\n",
            platform_key(),
            prebuilt_name,
        );
        std::fs::write(slot.join("vibe.toml"), &manifest_text).unwrap();
        let manifest = Manifest::parse_str(&manifest_text).unwrap();
        let resolution = vec![ResolvedDep {
            kind: PackageKind::Tool,
            group: Group::parse("org.demo").unwrap(),
            name: "native".into(),
            version: "1.0.0".parse().unwrap(),
            content_dir: slot,
            source_hash: Some(ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap()),
            manifest,
            requires: Vec::new(),
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        }];
        Self {
            _dir: dir,
            root,
            host,
            resolution,
        }
    }
}

fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        pair => panic!("unsupported test platform {pair:?}"),
    }
}

fn metadata(root: &Path) -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: "2026-08-31T00:00:00Z".into(),
        selected: ".".into(),
    }
}

fn context<'a>(fixture: &'a Fixture) -> SlotLifecycleContext<'a> {
    let dep = &fixture.resolution[0];
    SlotLifecycleContext {
        group: &dep.group,
        name: &dep.name,
        version: &dep.version,
        kind: &dep.kind,
        slot: &dep.content_dir,
        manifest: &dep.manifest,
    }
}

#[test]
fn prebuilt_slot_native_runs_and_missing_source_record_refuses_without_cargo() {
    let fixture = Fixture::new();
    let lifecycle = InstallSlotLifecycle::from_resolution_observed(
        &fixture.root,
        &fixture.host,
        &fixture.resolution,
        metadata(&fixture.root),
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        Arc::new(LifecycleLease::acquire(&fixture.root).unwrap()),
    )
    .expect("slot lifecycle constructs");

    lifecycle
        .pre_install(context(&fixture))
        .expect("the current-platform prebuilt invokes");
    let image_root = fixture.root.join(".vibe/native-load/e1");
    let images = std::fs::read_dir(&image_root)
        .unwrap()
        .flat_map(|digest| std::fs::read_dir(digest.unwrap().path()).unwrap())
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension() == Some(std::ffi::OsStr::new(&std::env::consts::DLL_EXTENSION))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        images.len(),
        1,
        "prebuilt invocation publishes one immutable load image"
    );
    assert!(
        images[0]
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            }),
        "the prebuilt load image is keyed by its current SHA-256"
    );
    let error = lifecycle
        .post_install(context(&fixture))
        .expect_err("source without a prior ARTIFACT record refuses");
    assert!(error.contains("native source artifact"), "{error}");
    assert!(error.contains("fix: run `vibe build`"), "{error}");
    assert!(
        !fixture.resolution[0].content_dir.join("target").exists(),
        "slot dispatch never starts Cargo or creates its target directory",
    );

    let reports = lifecycle.take_reports().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].handler, "native");
    assert_eq!(reports[0].status, "ok");
    assert_eq!(
        reports[0].message.as_deref(),
        Some("fixture handled slot:pre-install")
    );
    assert_eq!(reports[1].status, "fail");
}
