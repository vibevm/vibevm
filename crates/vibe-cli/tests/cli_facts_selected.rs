//! Actual installed declared package facts across materialized formats.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-FACT-COMMANDS",
    r = 1
);
mod common;
use common::UserScratch;
use std::fs;

struct Fixture {
    user: UserScratch,
    project: tempfile::TempDir,
    registry: tempfile::TempDir,
}
impl Fixture {
    fn install(format: &str) -> Self {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let source = registry.path().join("org.example/selected/v1.0.0");
        fs::create_dir_all(source.join("rules")).unwrap();
        fs::write(source.join("vibe.toml"), "[package]\ngroup='org.example'\nname='selected'\nkind='flow'\nversion='1.0.0'\nepoch=1\nformat='normal'\n[[package.sources]]\ndirectory='rules'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[[package.sources]]\nfile='rules/rule.bin'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n").unwrap();
        fs::write(source.join("rules/rule.native"), "<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"impl/done\">Real.</REAL></p><p><VOID fact=\"true\" status=\"spec/void\">Void.</VOID></p><p><UNMARKED fact=\"true\">Unmarked.</UNMARKED></p></spec>").unwrap();
        fs::write(source.join("rules/rule.bin"), [255, 0, 128]).unwrap();
        fs::write(project.path().join("vibe.toml"), format!("[project]\ngroup='org.consumer'\nname='demo'\nversion='1.0.0'\nspec_format='{format}'\n")).unwrap();
        let output = user
            .vibe()
            .args(["install", "org.example/selected@=1.0.0", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(output.status.success(), "{format} install: {output:?}");
        Self {
            user,
            project,
            registry,
        }
    }
    fn command(&self, args: &[&str]) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.project.path())
            .args(args)
            .output()
            .unwrap()
    }
    fn report(&self) -> std::process::Output {
        self.command(&["facts", "report", "--package", "org.example/selected"])
    }
    fn adopt(&self) -> std::process::Output {
        self.command(&["facts", "adopt", "--package", "org.example/selected"])
    }
    fn facts(&self) -> std::path::PathBuf {
        self.project
            .path()
            .join(common::facts_rel("org.example.selected.toml"))
    }
    fn slot(&self) -> std::path::PathBuf {
        self.project
            .path()
            .join(common::slot_dir("org.example.selected", "1.0.0"))
    }
}

fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn walk(
        root: &std::path::Path,
        at: &std::path::Path,
        out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn selected_formats_report_and_adopt() {
    use vibe_workspace::vibedeps;
    for format in ["mixed", "markdown", "xml"] {
        let f = Fixture::install(format);
        let before = vibedeps::read_slot_record(&f.slot()).unwrap();
        let tree = snapshot(f.project.path());
        let report = f.report();
        assert_eq!(tree, snapshot(f.project.path()));
        assert!(report.status.success(), "{format} report: {report:?}");
        assert!(
            String::from_utf8_lossy(&report.stdout).contains("0/3 (indeterminate 0)"),
            "{format}: {report:?}"
        );
        assert!(!f.facts().exists());
        let output = f.adopt();
        assert!(output.status.success(), "{format} adopt: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("added=2 kept=0"),
            "{output:?}"
        );
        let bytes = fs::read(f.facts()).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            text.contains("rules/rule.native#REAL") && text.contains("rules/rule.native#VOID"),
            "{text}"
        );
        assert!(!text.contains("UNMARKED") && !text.contains("source_hash"));
        assert!(text.contains("spec/void"));
        let after = vibedeps::read_slot_record(&f.slot()).unwrap();
        assert_eq!(before.source_hash, after.source_hash);
        assert_eq!(before.spec_format, after.spec_format);
        assert_eq!(
            fs::read(f.slot().join("rules/rule.bin")).unwrap(),
            [255, 0, 128]
        );
        if format != "mixed" {
            assert_eq!(
                after.converter_recipe.as_deref(),
                Some(vibedeps::CONVERTER_RECIPE)
            );
            assert_eq!(
                after.overlay_hash.as_ref().map(|h| h.as_str()),
                vibe_facts::overlay_file_hash(f.project.path(), "org.example/selected").as_deref()
            );
            assert_ne!(before.overlay_hash, after.overlay_hash);
            assert_eq!(
                after.derived_hash,
                Some(vibedeps::compute_recorded_payload_hash(&f.slot(), &after.files).unwrap())
            );
            assert!(vibedeps::verify_recorded_files(&f.slot(), &after).is_ok());
        } else {
            assert_eq!(before, after);
        }
        let repeat = f.adopt();
        assert!(repeat.status.success(), "{repeat:?}");
        assert!(String::from_utf8_lossy(&repeat.stdout).contains("added=0 kept=2"));
        assert_eq!(bytes, fs::read(f.facts()).unwrap());
    }
}

#[test]
fn prefix_preserves_existing_indeterminate_and_repeat() {
    let f = Fixture::install("markdown");
    let mut registry = vibe_facts::Registry::load(f.project.path()).unwrap();
    registry
        .upsert(
            f.project.path(),
            vibe_facts::FactEntry {
                address: "spec://org.example/selected/rules/rule.native#REAL".into(),
                origin: vibe_facts::FactOrigin::Package,
                package: Some("org.example/selected".into()),
                status: None,
                comment: Some("consumer choice".into()),
            },
        )
        .unwrap();
    let before = fs::read(f.facts()).unwrap();
    let output = f.command(&[
        "facts",
        "adopt",
        "--package",
        "org.example/selected",
        "--prefix",
        "spec://org.example/selected/rules/rule.native#REAL",
    ]);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("added=0 kept=1"));
    assert_eq!(before, fs::read(f.facts()).unwrap());
    let output = f.adopt();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("added=1 kept=1"));
    let registry = vibe_facts::Registry::load(f.project.path()).unwrap();
    let real = registry
        .get("spec://org.example/selected/rules/rule.native#REAL")
        .unwrap();
    assert_eq!(real.status, None);
    assert_eq!(real.comment.as_deref(), Some("consumer choice"));
}

#[test]
fn unavailable_or_invalid_selected_input_never_publishes_registry() {
    use vibe_workspace::vibedeps;
    for failure in [
        "store",
        "config",
        "record",
        "output",
        "hash",
        "selection",
        "native",
        "late-native",
        "destination",
    ] {
        let f = Fixture::install("markdown");
        match failure {
            // Unavailable means neither the machine store nor the locked
            // source can supply the pinned bytes: a held local-directory
            // source re-supplies a missing store entry (PROP-073
            // `#LOCKED-SOURCE-LAW`), so both are removed.
            "store" => {
                fs::remove_dir_all(f.user.settings.join("cache")).unwrap();
                fs::remove_dir_all(f.registry.path().join("org.example/selected")).unwrap();
            }
            "config" => {
                fs::write(f.project.path().join("vibe.toml"), "[broken").unwrap();
            }
            "record" => {
                fs::remove_file(f.slot().join(vibedeps::SLOT_RECORD_FILENAME)).unwrap();
            }
            "output" => {
                fs::remove_file(f.slot().join("rules/rule.md")).unwrap();
            }
            "hash" => {
                let mut lock =
                    vibe_core::manifest::Lockfile::read(f.project.path().join("vibe.lock"))
                        .unwrap();
                lock.packages[0].content_hash =
                    vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
                lock.write(f.project.path().join("vibe.lock")).unwrap();
            }
            "selection" => {
                let path = f.project.path().join("vibe.toml");
                fs::write(
                    &path,
                    fs::read_to_string(&path)
                        .unwrap()
                        .replace("markdown", "xml"),
                )
                .unwrap();
            }
            "native" | "late-native" => {
                let mut record = vibedeps::read_slot_record(&f.slot()).unwrap();
                let output = if failure == "native" {
                    "rules/rule.md"
                } else {
                    "rules/z.md"
                };
                fs::write(f.slot().join(output), [0xff, 0xfe]).unwrap();
                if failure == "late-native" {
                    let mut row = record
                        .files
                        .iter()
                        .find(|r| r.path == "rules/rule.md")
                        .unwrap()
                        .clone();
                    row.path = output.into();
                    row.source = Some("rules/z.native".into());
                    record.files.push(row);
                    record.files.sort_by(|a, b| a.path.cmp(&b.path));
                }
                record
                    .files
                    .iter_mut()
                    .find(|r| r.path == output)
                    .unwrap()
                    .sha256 = vibedeps::sha256_bytes(&fs::read(f.slot().join(output)).unwrap());
                record.derived_hash = Some(
                    vibedeps::compute_recorded_payload_hash(&f.slot(), &record.files).unwrap(),
                );
                vibedeps::write_slot_record(&f.slot(), &record).unwrap();
            }
            _ => {
                fs::create_dir_all(f.facts()).unwrap();
            }
        }
        let before = fs::read(f.facts()).ok();
        let result = f.adopt();
        assert!(!result.status.success(), "{failure}: {result:?}");
        assert_eq!(before, fs::read(f.facts()).ok(), "{failure}");
        if matches!(
            failure,
            "record" | "output" | "hash" | "selection" | "native" | "late-native"
        ) {
            let tree = snapshot(f.project.path());
            let report = f.report();
            assert_eq!(tree, snapshot(f.project.path()));
            assert!(report.status.success(), "{failure}: {report:?}");
            let text = String::from_utf8_lossy(&report.stdout);
            assert!(
                text.contains("0/?") && text.contains("authored=?"),
                "{failure}: {text}"
            );
            assert!(!text.contains("no materialised slot"), "{failure}: {text}");
            assert_eq!(before, fs::read(f.facts()).ok());
        }
    }
}

#[test]
fn transformed_adoption_executes_existing_overlay_into_actual_output() {
    for format in ["markdown", "xml"] {
        let f = Fixture::install(format);
        let output_path = f.slot().join(if format == "xml" {
            "rules/rule.xml"
        } else {
            "rules/rule.md"
        });
        let before = fs::read(&output_path).unwrap();
        let mut registry = vibe_facts::Registry::load(f.project.path()).unwrap();
        registry
            .upsert(
                f.project.path(),
                vibe_facts::FactEntry {
                    address: "spec://org.example/selected/rules/rule.native#REAL".into(),
                    origin: vibe_facts::FactOrigin::Package,
                    package: Some("org.example/selected".into()),
                    status: Some(vibe_facts::FactStatus::parse("spec/work").unwrap()),
                    comment: None,
                },
            )
            .unwrap();
        let result = f.adopt();
        assert!(result.status.success(), "{result:?}");
        let after = fs::read(&output_path).unwrap();
        assert_ne!(before, after);
        assert!(String::from_utf8_lossy(&after).contains("spec/work"));
        let record = vibe_workspace::vibedeps::read_slot_record(&f.slot()).unwrap();
        assert_eq!(
            record.derived_hash,
            Some(
                vibe_workspace::vibedeps::compute_recorded_payload_hash(&f.slot(), &record.files)
                    .unwrap()
            )
        );
    }
}

#[test]
fn member_adoption_preserves_workspace_overlay_owner() {
    use vibe_workspace::vibedeps;
    let f = Fixture::install("markdown");
    let root_manifest = f.project.path().join("vibe.toml");
    let text = fs::read_to_string(&root_manifest).unwrap();
    fs::write(root_manifest, text + "\n[workspace]\nmembers=['member']\n").unwrap();
    let member = f.project.path().join("member");
    fs::create_dir(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[project]\ngroup='org.consumer'\nname='member'\nversion='1.0.0'\n",
    )
    .unwrap();
    for (root, status) in [
        (f.project.path(), "spec/work"),
        (member.as_path(), "impl/work"),
    ] {
        let mut registry = vibe_facts::Registry::load(root).unwrap();
        registry
            .upsert(
                root,
                vibe_facts::FactEntry {
                    address: "spec://org.example/selected/rules/rule.native#REAL".into(),
                    origin: vibe_facts::FactOrigin::Package,
                    package: Some("org.example/selected".into()),
                    status: Some(vibe_facts::FactStatus::parse(status).unwrap()),
                    comment: None,
                },
            )
            .unwrap();
    }
    let root_bytes = fs::read(f.facts()).unwrap();
    let before = snapshot(f.project.path());
    let report = f
        .user
        .vibe()
        .current_dir(&member)
        .args(["facts", "report", "--package", "org.example/selected"])
        .output()
        .unwrap();
    assert!(report.status.success(), "{report:?}");
    assert!(String::from_utf8_lossy(&report.stdout).contains("1/3"));
    assert_eq!(before, snapshot(f.project.path()));
    let adopt = f
        .user
        .vibe()
        .current_dir(&member)
        .args(["facts", "adopt", "--package", "org.example/selected"])
        .output()
        .unwrap();
    assert!(adopt.status.success(), "{adopt:?}");
    assert!(String::from_utf8_lossy(&adopt.stdout).contains("added=1 kept=1"));
    assert_eq!(root_bytes, fs::read(f.facts()).unwrap());
    let member_registry = vibe_facts::Registry::load(&member).unwrap();
    assert_eq!(member_registry.len(), 2);
    assert_eq!(
        member_registry
            .get("spec://org.example/selected/rules/rule.native#REAL")
            .unwrap()
            .status,
        Some(vibe_facts::FactStatus::parse("impl/work").unwrap())
    );
    let record = vibedeps::read_slot_record(&f.slot()).unwrap();
    assert_eq!(
        record.overlay_hash.as_ref().map(|h| h.as_str()),
        vibe_facts::overlay_file_hash(f.project.path(), "org.example/selected").as_deref()
    );
    assert_ne!(
        record.overlay_hash.as_ref().map(|h| h.as_str()),
        vibe_facts::overlay_file_hash(&member, "org.example/selected").as_deref()
    );
    let output = fs::read_to_string(f.slot().join("rules/rule.md")).unwrap();
    assert!(output.contains("spec/work") && !output.contains("impl/work"));
    assert_eq!(
        record.derived_hash,
        Some(vibedeps::compute_recorded_payload_hash(&f.slot(), &record.files).unwrap())
    );
}

#[test]
fn report_absent_lock_or_slot_retains_unmaterialized_compatibility() {
    for lock in [true, false] {
        let f = Fixture::install("mixed");
        if lock {
            fs::remove_file(f.project.path().join("vibe.lock")).unwrap();
        } else {
            fs::remove_dir_all(f.slot()).unwrap();
        }
        let result = f.report();
        assert!(result.status.success(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stdout).contains("package has no materialised slot")
        );
        assert!(!f.facts().exists());
    }
}

#[test]
fn report_present_nonregular_lock_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    let lock = f.project.path().join("vibe.lock");
    fs::remove_file(&lock).unwrap();
    fs::create_dir(&lock).unwrap();
    let result = f.report();
    assert!(!result.status.success(), "{result:?}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

#[test]
fn report_present_nondirectory_slot_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    fs::remove_dir_all(f.slot()).unwrap();
    fs::write(f.slot(), "not a directory").unwrap();
    let result = f.report();
    assert!(result.status.success(), "{result:?}");
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(
        text.contains("selected source invalid or unavailable"),
        "{result:?}"
    );
    assert!(!text.contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

#[cfg(unix)]
#[test]
fn report_dangling_lock_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    let lock = f.project.path().join("vibe.lock");
    fs::remove_file(&lock).unwrap();
    std::os::unix::fs::symlink(f.project.path().join("absent-lock"), lock).unwrap();
    let result = f.report();
    assert!(!result.status.success(), "{result:?}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

#[cfg(unix)]
#[test]
fn report_dangling_slot_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    fs::remove_dir_all(f.slot()).unwrap();
    std::os::unix::fs::symlink(f.project.path().join("absent-slot"), f.slot()).unwrap();
    let result = f.report();
    assert!(result.status.success(), "{result:?}");
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(
        text.contains("selected source invalid or unavailable"),
        "{result:?}"
    );
    assert!(!text.contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

#[test]
fn report_member_present_nonregular_workspace_lock_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    let member = report_member(&f);
    let lock = f.project.path().join("vibe.lock");
    fs::remove_file(&lock).unwrap();
    fs::create_dir(&lock).unwrap();
    let result = f
        .user
        .vibe()
        .current_dir(member)
        .args(["facts", "report", "--package", "org.example/selected"])
        .output()
        .unwrap();
    assert!(!result.status.success(), "{result:?}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

#[cfg(unix)]
#[test]
fn report_member_dangling_workspace_lock_is_not_unmaterialized() {
    let f = Fixture::install("mixed");
    let member = report_member(&f);
    let lock = f.project.path().join("vibe.lock");
    fs::remove_file(&lock).unwrap();
    std::os::unix::fs::symlink(f.project.path().join("absent-lock"), lock).unwrap();
    let result = f
        .user
        .vibe()
        .current_dir(member)
        .args(["facts", "report", "--package", "org.example/selected"])
        .output()
        .unwrap();
    assert!(!result.status.success(), "{result:?}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("package has no materialised slot"));
    assert!(!f.facts().exists());
}

fn report_member(f: &Fixture) -> std::path::PathBuf {
    let manifest = f.project.path().join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest).unwrap() + "\n[workspace]\nmembers=['member']\n",
    )
    .unwrap();
    let member = f.project.path().join("member");
    fs::create_dir(&member).unwrap();
    fs::write(
        member.join("vibe.toml"),
        "[project]\nname='member'\nversion='1.0.0'\n",
    )
    .unwrap();
    member
}

#[test]
fn clean_absent_workspace_lock_retains_orphan_policy() {
    let f = Fixture::install("mixed");
    let adopted = f.adopt();
    assert!(adopted.status.success(), "{adopted:?}");
    fs::remove_file(f.project.path().join("vibe.lock")).unwrap();
    let before = fs::read(f.facts()).unwrap();
    let dry = f.command(&["facts", "clean", "--dry-run"]);
    assert!(dry.status.success(), "{dry:?}");
    assert!(String::from_utf8_lossy(&dry.stdout).contains("orphaned=1"));
    assert_eq!(before, fs::read(f.facts()).unwrap());
    let clean = f.command(&["facts", "clean"]);
    assert!(clean.status.success(), "{clean:?}");
    assert!(String::from_utf8_lossy(&clean.stdout).contains("removed=1 kept=0"));
    assert!(!f.facts().exists());
}
