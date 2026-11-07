//! Declared progress sources use their captured projection and original path.
mod common;

use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                result.insert(relative, None);
                pending.push(path);
            } else {
                result.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    result
}

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("corpus")).unwrap();
    fs::write(root.join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n[[project.sources]]\nfile='corpus/native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='corpus/twin.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='corpus/native.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n[[project.sources]]\nfile='corpus/bad.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    fs::write(root.join("progress.toml"), "include=['corpus/*']\nexclude=['corpus/bad.payload']\n[progress]\ncache_dir='payload-cache'\n").unwrap();
    fs::write(root.join("corpus/native.payload"), "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Native</title><p><CLAIM fact=\"true\" status=\"impl/done\">Captured claim.</CLAIM></p></spec>\n").unwrap();
    fs::write(
        root.join("corpus/twin.md"),
        "# Native\n\n@fact:CLAIM Captured claim. @impl/done\n",
    )
    .unwrap();
    fs::write(
        root.join("corpus/native.md"),
        "# Ordinary\n\nOrdinary poison. @fact:POISON @impl/done\n#use spec://fake/data/root\n",
    )
    .unwrap();
    fs::write(root.join("corpus/bad.payload"), "<broken").unwrap();
}

#[test]
fn declared_payload_check_report_and_weave() {
    let scratch = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let before = snapshot(tmp.path());
    for args in [
        vec!["facts", "check"],
        vec!["progress", "report"],
        vec!["progress", "weave"],
    ] {
        let output = scratch
            .vibe()
            .arg("--no-progress")
            .args(&args)
            .arg("--path")
            .arg(tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            snapshot(tmp.path()),
            before,
            "{args:?} changed project namespace or bytes"
        );
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(!text.contains("POISON"), "{text}");
        if args[1] == "weave" {
            assert!(text.contains("Captured claim."), "{text}");
            assert!(!text.contains("<spec"), "{text}");
        }
    }
    assert!(!tmp.path().join("payload-cache").exists());
}

#[test]
fn declared_derived_outputs_and_cache_erasure_keep_projection() {
    let scratch = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fixture(root);
    fs::create_dir_all(root.join("campaigns/c/run")).unwrap();
    let authored = snapshot(&root.join("corpus"));
    for command in ["scan", "mirror", "resume", "baseline"] {
        let output = scratch
            .vibe()
            .args(["--no-progress", "progress", command, "--path"])
            .arg(root)
            .output()
            .unwrap();
        assert!(output.status.success(), "{command}: {output:?}");
        assert_eq!(snapshot(&root.join("corpus")), authored);
    }
    for artifact in [
        "run/cache.json",
        "run/RESUME.md",
        "baseline.json",
        "run/mirror/corpus__native.payload.json",
    ] {
        assert!(
            root.join("campaigns/c").join(artifact).is_file(),
            "{artifact}"
        );
    }
    let cache =
        progress_core::cache::Cache::load(&root.join("campaigns/c/run/cache.json")).unwrap();
    assert_eq!(cache.files.len(), 2);
    assert!(cache.files.contains_key("corpus/native.payload"));
    assert!(!cache.files.contains_key("corpus/native.md"));
    let before = snapshot(root);
    let rescan = scratch
        .vibe()
        .args(["--no-progress", "progress", "rescan", "--path"])
        .arg(root)
        .arg("--baseline")
        .arg(root.join("campaigns/c/baseline.json"))
        .output()
        .unwrap();
    assert!(rescan.status.success(), "{rescan:?}");
    assert_eq!(snapshot(root), before, "rescan is read-only");
    let mut previous = None;
    for mode in ["warm", "cold", "no-cache"] {
        if mode == "cold" {
            fs::remove_dir_all(root.join("payload-cache")).unwrap();
        }
        let before = snapshot(root);
        let mut command = scratch.vibe();
        command
            .args(["--no-progress", "progress", "weave", "--path"])
            .arg(root);
        if mode == "no-cache" {
            command.arg("--no-cache");
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{mode}: {output:?}");
        assert_eq!(snapshot(root), before);
        if let Some(previous) = &previous {
            assert_eq!(&output.stdout, previous);
        }
        previous = Some(output.stdout);
    }
    let output = scratch
        .vibe()
        .args(["--no-progress", "facts", "check", "--write-state", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let output = scratch
        .vibe()
        .args(["--no-progress", "progress", "weave", "--path"])
        .arg(root)
        .arg("--out")
        .arg(root.join("shards"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let shard = fs::read_to_string(root.join("shards/weave-000.md")).unwrap();
    let projected = vibe_specdoc::project_spec_text(
        Path::new("native.xml"),
        &fs::read_to_string(root.join("corpus/native.payload")).unwrap(),
    )
    .unwrap()
    .0;
    assert!(
        shard.contains(&projected),
        "weave must contain exactly the parsed native projection: {shard}"
    );
    assert!(!shard.contains("POISON"));
}

#[test]
fn installed_formats_use_original_identity_and_independent_binding() {
    for format in ["mixed", "markdown", "xml"] {
        let scratch = UserScratch::new();
        let tmp = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let package = registry.path().join("org.source/plain/v1.0.0");
        fs::create_dir_all(package.join("corpus")).unwrap();
        fs::write(package.join("vibe.toml"), "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n[[package.sources]]\nfile='corpus/native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[boot_snippet]\nsource='corpus/native.payload'\nlink='dynamic'\n").unwrap();
        fs::write(package.join("corpus/native.payload"), "<spec xmlns=\"https://vibevm.org/spec/1\"><p><CLAIM fact=\"true\" status=\"impl/done\">Installed claim.</CLAIM></p></spec>\n").unwrap();
        fs::write(
            tmp.path().join("vibe.toml"),
            format!("[project]\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n"),
        )
        .unwrap();
        let install = scratch
            .vibe()
            .args(["install", "org.source/plain@=1.0.0", "--path"])
            .arg(tmp.path())
            .arg("--registry")
            .arg(registry.path())
            .args(["--offline", "--assume-yes"])
            .output()
            .unwrap();
        assert!(install.status.success(), "{install:?}");
        let lock_path = tmp.path().join("vibe.lock");
        let lock = vibe_core::manifest::Lockfile::read(&lock_path).unwrap();
        let p = &lock.packages[0];
        let slot = tmp.path().join(vibe_workspace::vibedeps::slot_rel_path(
            &p.group, &p.name, &p.version,
        ));
        let output = match format {
            "mixed" => "corpus/native.payload",
            "markdown" => "corpus/native.md",
            _ => "corpus/native.xml",
        };
        fs::write(
            slot.join("progress.toml"),
            format!("include=['{output}']\n[progress]\ncache_dir='payload-cache'\n"),
        )
        .unwrap();
        let before = snapshot(tmp.path());
        let original = snapshot(registry.path());
        let run = || {
            scratch
                .vibe()
                .args(["--no-progress", "progress", "weave", "--path"])
                .arg(&slot)
                .output()
                .unwrap()
        };
        let result = run();
        assert!(result.status.success(), "{format}: {result:?}");
        let text = String::from_utf8_lossy(&result.stdout);
        assert!(text.contains("corpus/native.payload"), "{text}");
        assert!(text.contains("Installed claim."), "{text}");
        assert!(!text.contains("<spec"), "{text}");
        assert_eq!(snapshot(tmp.path()), before);
        assert_eq!(snapshot(registry.path()), original);
        let checked = scratch
            .vibe()
            .args(["--no-progress", "facts", "check", "--exhaustive", "--path"])
            .arg(&slot)
            .output()
            .unwrap();
        assert!(checked.status.success(), "{checked:?}");
        if format != "mixed" {
            assert!(
                String::from_utf8_lossy(&checked.stdout)
                    .contains(&format!("materialized input `{output}`")),
                "{checked:?}"
            );
        }
        assert_eq!(snapshot(tmp.path()), before);
        for fault in ["missing", "lock", "hash", "record", "format"] {
            let source = fs::read(slot.join(output)).unwrap();
            let lock_bytes = fs::read(&lock_path).unwrap();
            let manifest_path = tmp.path().join("vibe.toml");
            let manifest = fs::read(&manifest_path).unwrap();
            let record_path = slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
            let record = fs::read(&record_path).unwrap();
            match fault {
                "missing" => fs::remove_file(slot.join(output)).unwrap(),
                "lock" => fs::remove_file(&lock_path).unwrap(),
                "hash" => {
                    let mut changed = lock.clone();
                    changed.packages[0].content_hash =
                        vibe_core::ContentHash::parse(&format!("sha256:{}", "0".repeat(64)))
                            .unwrap();
                    changed.write(&lock_path).unwrap();
                }
                "record" => fs::write(&record_path, "invalid").unwrap(),
                _ => fs::write(
                    &manifest_path,
                    format!(
                        "[project]\nname='host'\nversion='1.0.0'\nspec_format='{}'\n",
                        if format == "mixed" { "xml" } else { "mixed" }
                    ),
                )
                .unwrap(),
            }
            let before = snapshot(tmp.path());
            let result = run();
            assert!(!result.status.success(), "{format}/{fault}: {result:?}");
            assert!(result.stdout.is_empty(), "{format}/{fault}: partial output");
            assert_eq!(snapshot(tmp.path()), before);
            fs::write(slot.join(output), source).unwrap();
            fs::write(&lock_path, lock_bytes).unwrap();
            fs::write(&record_path, record).unwrap();
            fs::write(&manifest_path, manifest).unwrap();
        }
    }
}

#[test]
fn missing_or_ambiguous_selected_native_refuses_without_pruning_cache() {
    for fault in ["missing", "ambiguous", "malformed"] {
        let scratch = UserScratch::new();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fixture(root);
        fs::create_dir_all(root.join("campaigns/c/run")).unwrap();
        let scan = scratch
            .vibe()
            .args(["--no-progress", "progress", "scan", "--path"])
            .arg(root)
            .output()
            .unwrap();
        assert!(scan.status.success(), "{scan:?}");
        match fault {
            "missing" => fs::remove_file(root.join("corpus/native.payload")).unwrap(),
            "malformed" => fs::write(root.join("corpus/native.payload"), "<broken").unwrap(),
            _ => {
                let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
                fs::write(root.join("vibe.toml"), format!("{manifest}\n[[project.sources]]\nfile='corpus/native.payload'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n")).unwrap();
            }
        }
        for command in ["scan", "report", "mirror", "resume", "baseline", "weave"] {
            let before = snapshot(root);
            let output = scratch
                .vibe()
                .args(["--no-progress", "progress", command, "--path"])
                .arg(root)
                .output()
                .unwrap();
            assert!(!output.status.success(), "{fault}/{command}: {output:?}");
            assert!(
                output.stdout.is_empty(),
                "{fault}/{command} emitted partial output"
            );
            assert_eq!(snapshot(root), before, "{fault}/{command} wrote or pruned");
        }
    }
}

#[test]
fn ordinary_reclassification_cannot_reuse_native_cache_or_nested_authority() {
    let scratch = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fixture(root);
    fs::create_dir_all(root.join("campaigns/c/run")).unwrap();
    let scan = scratch
        .vibe()
        .args(["--no-progress", "progress", "scan", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(scan.status.success(), "{scan:?}");
    let manifest = fs::read_to_string(root.join("vibe.toml")).unwrap();
    let manifest = manifest.replace("file='corpus/twin.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'", "file='corpus/twin.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'");
    fs::write(root.join("vibe.toml"), manifest).unwrap();
    // An arbitrary nested manifest is inert under the declared outer owner.
    fs::write(root.join("corpus/vibe.toml"), "invalid manifest data").unwrap();
    let before = snapshot(root);
    let output = scratch
        .vibe()
        .args(["--json", "progress", "report", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains("corpus/twin.md"), "{text}");
    assert!(text.contains("corpus/native.payload"), "{text}");
    assert_eq!(snapshot(root), before);
}

#[test]
fn registered_member_policy_owns_selected_corpus() {
    let scratch = UserScratch::new();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("member")).unwrap();
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname='host'\nversion='1.0.0'\nsources=[]\n[workspace]\nmembers=['member']\n",
    )
    .unwrap();
    fs::write(root.join("progress.toml"), "include=['member/*.payload']\n").unwrap();
    let manifest = "[package]\nname='member'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n";
    fs::write(root.join("member/vibe.toml"), manifest).unwrap();
    fs::write(root.join("member/native.payload"), "<spec xmlns=\"https://vibevm.org/spec/1\"><p><A fact=\"true\" status=\"impl/done\">Member claim.</A></p></spec>").unwrap();
    let before = snapshot(root);
    let output = scratch
        .vibe()
        .args(["--no-progress", "progress", "weave", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Member claim."));
    assert_eq!(snapshot(root), before);
    fs::remove_file(root.join("member/native.payload")).unwrap();
    let before = snapshot(root);
    let output = scratch
        .vibe()
        .args(["--no-progress", "progress", "scan", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "registered selected File promise cannot vanish"
    );
    assert_eq!(snapshot(root), before);
}
