//! Integrated R5.3 source/prebuilt, cache-admission and lifecycle-law gates.

use super::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_install::{InstallSlotLifecycle, SlotLifecycleSeams};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::install::{ResolvedDep, SlotLifecycle, SlotLifecycleContext};

use crate::phase::LifecycleValues;

fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        pair => panic!("unsupported native gate platform {pair:?}"),
    }
}

fn fixture_name() -> String {
    format!(
        "{}vibe_native_loader_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test fixture setup uses immediate assertions to keep failures local"
)]
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
    let exact_name = fixture_name();
    let mut candidates = Vec::new();
    collect_exact(profile_dir, &exact_name, &mut candidates);
    collect_exact(&profile_dir.join("deps"), &exact_name, &mut candidates);
    candidates.sort();
    candidates.dedup();
    assert_eq!(candidates.len(), 1, "one exact SDK fixture: {candidates:?}");
    candidates.pop().expect("one fixture library")
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test fixture enumeration treats malformed target entries as assertion failures"
)]
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

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test fixture construction is intentionally fail-fast"
)]
fn copy_fixture(root: &Path) -> String {
    let name = fixture_name();
    std::fs::create_dir_all(root.join("prebuilt")).unwrap();
    std::fs::copy(fixture_library(), root.join("prebuilt").join(&name)).unwrap();
    format!("prebuilt/{name}")
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test-only source fixture construction is intentionally fail-fast"
)]
fn write_source_crate(root: &Path, extensions: &[(&str, &str)], message: &str) {
    let vibe_ext = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::create_dir_all(root.join("native/src")).unwrap();
    std::fs::write(
        root.join("native/Cargo.toml"),
        format!(
            "[package]\nname='native-gate-source'\nversion='0.1.0'\nedition='2024'\n\n[lib]\ncrate-type=['cdylib']\n\n[dependencies]\nvibe-ext={{path={vibe_ext:?}}}\n"
        ),
    )
    .unwrap();
    let rows = extensions
        .iter()
        .map(|(id, point)| {
            format!(
                "ManifestExtension {{ id: {id:?}.to_owned(), point: {point:?}.to_owned(), ir_schema: None }}"
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    std::fs::write(
        root.join("native/src/lib.rs"),
        format!(
            r#"use vibe_ext::{{Context, Manifest, ManifestExtension, Reply, ReplyStatus}};

fn handle(context: Context) -> Reply {{
    Reply {{
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        message: Some(format!("{message} {{}}", context.execution.id)),
    }}
}}

vibe_ext::vibe_extension!(
    manifest = Manifest {{ extensions: vec![{rows}] }},
    handler = handle,
);
"#
        ),
    )
    .unwrap();
}

fn prebuilt_rows(ids: &[&str], point: &str, relative: &str) -> String {
    ids.iter()
        .map(|id| {
            format!(
                "[[extension]]\nid={id:?}\npoint={point:?}\nhandler={{kind='native',prebuilt={{{:?}={relative:?}}}}}\n",
                platform_key()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test project construction is intentionally fail-fast"
)]
fn prebuilt_project(ids: &[&str]) -> tempfile::TempDir {
    let placeholder = format!("prebuilt/{}", fixture_name());
    let text = format!(
        "[project]\nname='native-gate'\nversion='0.1.0'\n\n{}",
        prebuilt_rows(ids, "phase:build", &placeholder)
    );
    let dir = manifested(&text);
    copy_fixture(dir.path());
    dir
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "qualified report keys are an asserted fixture invariant"
)]
fn report_ids(values: &LifecycleValues) -> Vec<(&str, &str)> {
    values
        .contributions
        .iter()
        .map(|row| {
            (
                row.key.rsplit('#').next().expect("qualified key"),
                row.status.as_str(),
            )
        })
        .collect()
}

fn image_files(root: &Path) -> Vec<PathBuf> {
    let image_root = root.join(".vibe/native-load/e1");
    let Ok(digests) = std::fs::read_dir(image_root) else {
        return Vec::new();
    };
    let mut files = digests
        .filter_map(Result::ok)
        .flat_map(|digest| {
            std::fs::read_dir(digest.path())
                .into_iter()
                .flat_map(|entries| entries.filter_map(Result::ok))
        })
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    files
}

struct SlotFixture {
    host: Manifest,
    resolution: Vec<ResolvedDep>,
}

impl SlotFixture {
    #[spec(
        deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
        reason = "test slot construction is intentionally fail-fast"
    )]
    fn new(root: &Path, rows: &[(&str, &str)]) -> Self {
        let slot = root
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.native-gate")
            .join("1.0.0");
        std::fs::create_dir_all(&slot).unwrap();
        let relative = copy_fixture(&slot);
        let declarations = rows
            .iter()
            .map(|(id, point)| prebuilt_rows(&[*id], point, &relative))
            .collect::<Vec<_>>()
            .join("\n");
        let manifest_text = format!(
            "[package]\ngroup='org.demo'\nname='native-gate'\nkind='tool'\nversion='1.0.0'\n\n{declarations}"
        );
        std::fs::write(slot.join("vibe.toml"), &manifest_text).unwrap();
        let manifest = Manifest::parse_str(&manifest_text).unwrap();
        Self {
            host: Manifest::read(root.join("vibe.toml")).unwrap(),
            resolution: vec![ResolvedDep {
                kind: PackageKind::Tool,
                group: Group::parse("org.demo").unwrap(),
                name: "native-gate".into(),
                version: "1.0.0".parse().unwrap(),
                content_dir: slot,
                source_hash: Some(
                    ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
                ),
                manifest,
                requires: Vec::new(),
                admitted_by: None,
                via_override: None,
                source_mutable: false,
                in_place_changed: None,
            }],
        }
    }

    fn context(&self) -> SlotLifecycleContext<'_> {
        let dep = &self.resolution[0];
        SlotLifecycleContext {
            group: &dep.group,
            name: &dep.name,
            version: &dep.version,
            kind: &dep.kind,
            slot: &dep.content_dir,
            manifest: &dep.manifest,
        }
    }
}

#[spec(
    deviates = "spec://org.vibevm.ai-native/rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test lifecycle composition is intentionally fail-fast"
)]
fn slot_lifecycle(root: &Path, fixture: &SlotFixture) -> InstallSlotLifecycle {
    let metadata = RunMetadata {
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
    };
    InstallSlotLifecycle::from_resolution_observed(
        root,
        &fixture.host,
        &fixture.resolution,
        metadata,
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        Arc::new(LifecycleLease::acquire(root).unwrap()),
    )
    .unwrap()
}

#[test]
fn source_and_prebuilt_compose_through_phase_and_slot_with_one_image_machine() {
    let relative = format!("prebuilt/{}", fixture_name());
    let text = format!(
        "[project]\nname='native-gate'\nversion='0.1.0'\n\n\
         [[extension]]\nid='source-ok'\npoint='phase:build'\nhandler={{kind='native',crate_dir='native'}}\n\n\
         [[extension]]\nid='prebuilt-ok'\npoint='phase:build'\nhandler={{kind='native',prebuilt={{{:?}={relative:?}}}}}\n\n\
         [[extension]]\nid='compile-native'\npoint='compile:source'\nhandler={{kind='native',crate_dir='native'}}\napplies_to={{paths=['never/**']}}\n",
        platform_key()
    );
    let dir = manifested(&text);
    write_source_crate(
        dir.path(),
        &[
            ("source-ok", "phase:build"),
            ("compile-native", "compile:source"),
        ],
        "source gate",
    );
    copy_fixture(dir.path());

    let PhaseOutcome::Completed(values) = run_phases_over(dir.path(), vec![Phase::Build]) else {
        panic!("source and prebuilt phase rows complete")
    };
    assert_eq!(
        report_ids(&values),
        vec![("source-ok", "ok"), ("prebuilt-ok", "ok")]
    );
    assert_eq!(
        values.contributions[0].message.as_deref(),
        Some("source gate source-ok")
    );
    assert_eq!(
        values.contributions[1].message.as_deref(),
        Some("prebuilt-ok handled phase:build")
    );
    assert!(
        values
            .contributions
            .iter()
            .all(|row| !row.key.ends_with("#compile-native"))
    );
    let before_slot = image_files(dir.path());
    assert_eq!(before_slot.len(), 2, "one source and one prebuilt image");

    let slot = SlotFixture::new(dir.path(), &[("slot-pre-fixture", "slot:pre-install")]);
    let lifecycle = slot_lifecycle(dir.path(), &slot);
    lifecycle.pre_install(slot.context()).unwrap();
    let reports = lifecycle.take_reports().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, "ok");
    assert_eq!(
        reports[0].message.as_deref(),
        Some("fixture handled slot:pre-install")
    );
    assert_eq!(
        image_files(dir.path()),
        before_slot,
        "slot composition reuses the same digest-addressed fixture image"
    );
}

#[test]
fn stale_source_config_refuses_before_the_loaded_handle_can_answer() {
    let manifest = |cache_mode: &str| {
        format!(
            "[project]\nname='native-cache-gate'\nversion='0.1.0'\n\n\
             [[extension]]\nid='source-built'\npoint='phase:build'\nhandler={{kind='native',crate_dir='native'}}\nconfig={{mode='stable'}}\n\n\
             [[extension]]\nid='source-cache'\npoint='phase:generate'\nhandler={{kind='native',crate_dir='native'}}\nconfig={{mode={cache_mode:?}}}\n"
        )
    };
    let dir = manifested(&manifest("A"));
    write_source_crate(
        dir.path(),
        &[
            ("source-built", "phase:build"),
            ("source-cache", "phase:generate"),
        ],
        "cache gate",
    );
    let PhaseOutcome::Completed(first) = run_phases_over(dir.path(), vec![Phase::Build]) else {
        panic!("the source builds and invokes once")
    };
    assert_eq!(report_ids(&first), vec![("source-built", "ok")]);

    let record_path = std::fs::read_dir(dir.path().join(".vibe/state/artifacts"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .expect("the source build wrote one artifact record");
    let corrupt_config = "b".repeat(64);
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
    *record.pointer_mut("/freshness/config").unwrap() = serde_json::json!(corrupt_config);
    std::fs::write(&record_path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    let PhaseOutcome::Failed {
        measurement,
        original,
        ..
    } = run_phases_over(dir.path(), vec![Phase::Generate])
    else {
        panic!("stale config must refuse before cached invocation")
    };
    let rendered = format!("{original:#}");
    assert!(rendered.contains("fix: run `vibe build`"), "{rendered}");
    assert!(
        rendered.contains("handler/config witness changed"),
        "{rendered}"
    );
    let Measurement::Lifecycle { rows, .. } = measurement else {
        panic!("phase failure remains lifecycle-shaped")
    };
    assert_eq!(rows.len(), 1);
    assert!(rows[0].key.ends_with("#source-cache"));
    assert_eq!(rows[0].status, "fail");
}

#[test]
fn native_skip_fail_and_panic_obey_phase_law_and_loader_survives() {
    let skip_dir = prebuilt_project(&["prebuilt-skip", "prebuilt-after"]);
    let PhaseOutcome::Completed(skipped) = run_phases_over(skip_dir.path(), vec![Phase::Build])
    else {
        panic!("skip is continuation-safe")
    };
    assert_eq!(
        report_ids(&skipped),
        vec![("prebuilt-skip", "skip"), ("prebuilt-after", "ok")]
    );

    let fail_dir = prebuilt_project(&["prebuilt-fail", "prebuilt-after"]);
    let PhaseOutcome::Failed {
        measurement,
        original,
        ..
    } = run_phases_over(fail_dir.path(), vec![Phase::Build])
    else {
        panic!("native fail stops the phase")
    };
    let rendered = format!("{original:#}");
    assert!(rendered.contains("prebuilt-fail"), "{rendered}");
    let Measurement::Lifecycle { rows, .. } = measurement else {
        panic!("native fail remains lifecycle-shaped")
    };
    assert_eq!(
        rows.iter()
            .map(|row| (row.key.rsplit('#').next().unwrap(), row.status.as_str()))
            .collect::<Vec<_>>(),
        vec![("prebuilt-fail", "fail")]
    );

    let panic_dir = prebuilt_project(&["prebuilt-panic", "prebuilt-after"]);
    let PhaseOutcome::Failed {
        measurement,
        original,
        ..
    } = run_phases_over(panic_dir.path(), vec![Phase::Build])
    else {
        panic!("SDK panic becomes a typed native failure")
    };
    let rendered = format!("{original:#}");
    assert!(rendered.contains("native execution failed"), "{rendered}");
    let Measurement::Lifecycle { rows, .. } = measurement else {
        panic!("panic failure remains lifecycle-shaped")
    };
    assert_eq!(rows.len(), 1);
    assert!(rows[0].key.ends_with("#prebuilt-panic"));
    assert_eq!(rows[0].status, "fail");
    let before = image_files(panic_dir.path());
    assert_eq!(before.len(), 1);

    let relative = format!("prebuilt/{}", fixture_name());
    std::fs::write(
        panic_dir.path().join("vibe.toml"),
        format!(
            "[project]\nname='native-gate'\nversion='0.1.0'\n\n{}",
            prebuilt_rows(&["prebuilt-ok"], "phase:build", &relative)
        ),
    )
    .unwrap();
    let PhaseOutcome::Completed(after) = run_phases_over(panic_dir.path(), vec![Phase::Build])
    else {
        panic!("the same process loader invokes a valid row after panic")
    };
    assert_eq!(report_ids(&after), vec![("prebuilt-ok", "ok")]);
    assert_eq!(image_files(panic_dir.path()), before);
}

#[test]
fn slot_pre_failure_stops_while_post_failure_is_flagged_and_continues() {
    let pre_dir = prebuilt_project(&[]);
    let pre = SlotFixture::new(
        pre_dir.path(),
        &[
            ("slot-pre-fail", "slot:pre-install"),
            ("slot-pre-fixture", "slot:pre-install"),
        ],
    );
    let pre_lifecycle = slot_lifecycle(pre_dir.path(), &pre);
    let error = pre_lifecycle
        .pre_install(pre.context())
        .expect_err("pre-install native fail stops the slot callback");
    assert!(error.contains("slot-pre-fail"), "{error}");
    let pre_reports = pre_lifecycle.take_reports().unwrap();
    assert_eq!(pre_reports.len(), 1);
    assert_eq!(pre_reports[0].status, "fail");
    assert!(!pre_reports[0].flagged);

    let post_dir = prebuilt_project(&[]);
    let post = SlotFixture::new(
        post_dir.path(),
        &[
            ("slot-post-fail", "slot:post-install"),
            ("slot-post-fixture", "slot:post-install"),
        ],
    );
    let post_lifecycle = slot_lifecycle(post_dir.path(), &post);
    post_lifecycle
        .post_install(post.context())
        .expect("post-install semantic failure is installed-but-flagged");
    let post_reports = post_lifecycle.take_reports().unwrap();
    assert_eq!(post_reports.len(), 2);
    assert_eq!(post_reports[0].status, "fail");
    assert!(post_reports[0].flagged);
    assert_eq!(post_reports[1].status, "ok");
}
