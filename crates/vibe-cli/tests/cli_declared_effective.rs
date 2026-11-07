//! The effective display selects native sources without changing its read universe.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-SHOW-READ",
    r = 1
);
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use vibe_test_support::UserScratch;

fn boot(name: &str) -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_dir().join(name))
}
fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn row(owner: &str, path: &str, media: &str, interpretation: &str, loading: &str) -> String {
    let role = if interpretation == "vibe-spec" {
        "spec"
    } else {
        "resource"
    };
    format!(
        "[[{owner}.sources]]\nfile='{path}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='{loading}'\n"
    )
}
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
fn show(user: &UserScratch, root: &Path) -> std::process::Output {
    let before = snapshot(root);
    let output = user
        .vibe()
        .args(["--json", "show", "effective", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        snapshot(root),
        before,
        "show changed project bytes or namespace"
    );
    output
}
fn document(output: std::process::Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}
fn install(user: &UserScratch, root: &Path, registry: &Path) {
    let output = user
        .vibe()
        .args(["install", "org.source/plain@=1.0.0", "--path"])
        .arg(root)
        .arg("--registry")
        .arg(registry)
        .args(["--offline", "--assume-yes"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}
fn registry() -> tempfile::TempDir {
    let registry = tempfile::tempdir().unwrap();
    let package = registry.path().join("org.source/plain/v1.0.0");
    write(
        &package,
        "vibe.toml",
        format!(
            "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n[boot_snippet]\nsource='{}'\nlink='static'\n",
            boot("package.md")
        ),
    );
    write(&package, &boot("package.md"), "# Dependency\n");
    registry
}
#[test]
fn installed_grouped_and_ungrouped_own_boot_uses_declared_scan_media() {
    for grouped in [true, false] {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let registry = registry();
        let native = boot("20-native.payload");
        let markdown = boot("30-native.md");
        let raw = boot("20-native.md");
        let explicit = boot("40-explicit.xml");
        let rows = row("project", &native, "application/xml", "vibe-spec", "scan")
            + &row("project", &markdown, "text/markdown", "vibe-spec", "scan")
            + &row("project", &raw, "text/markdown", "commonmark", "scan")
            + &row(
                "project",
                &explicit,
                "application/xml",
                "vibe-spec",
                "explicit",
            )
            + &row(
                "project",
                "unselected/missing.xml",
                "application/xml",
                "vibe-spec",
                "scan",
            );
        let group = if grouped { "group='org.host'\n" } else { "" };
        write(
            root.path(),
            "vibe.toml",
            format!("[project]\n{group}name='host'\nversion='1.0.0'\n{rows}"),
        );
        write(
            root.path(),
            &native,
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p>OWN-NATIVE</p></spec>",
        );
        write(root.path(), &markdown, "# OWN-MARKDOWN\n");
        write(root.path(), &raw, "#use spec://forged/package/rule\n");
        write(root.path(), &explicit, "<broken");
        install(&user, root.path(), registry.path());
        // Publication controls are excluded from the incumbent display universe.
        write(root.path(), &boot(".vibe-boot-stage"), [0xff, 0xfe]);
        let before = snapshot(registry.path());
        let doc = document(show(&user, root.path()));
        assert_eq!(snapshot(registry.path()), before);
        let sections = doc["sections"].as_array().unwrap();
        let selected = sections
            .iter()
            .find(|s| s["path"] == native)
            .expect("native arbitrary suffix selected");
        assert!(selected["body"].as_str().unwrap().contains("OWN-NATIVE"));
        assert!(!selected["body"].as_str().unwrap().contains("<spec"));
        assert_eq!(selected["origin"], "user");
        assert_eq!(
            selected["spec_uri"],
            "spec://project/boot/20-native.payload"
        );
        assert!(sections.iter().any(|s| s["path"] == markdown));
        assert!(
            !sections
                .iter()
                .any(|s| s["path"] == raw || s["path"] == explicit)
        );
        for name in ["STATIC.md", "INDEX.md"] {
            let path = boot(name);
            let text = fs::read_to_string(root.path().join(&path)).unwrap();
            assert_eq!(
                sections.iter().find(|s| s["path"] == path).unwrap()["body"],
                text
            );
        }
    }
}

fn refuses(user: &UserScratch, root: &Path, expected: &str) {
    let output = show(user, root);
    assert!(!output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "a refused view must not emit a partial report: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "expected {expected}: {output:?}"
    );
}

#[test]
fn named_installed_outputs_use_bound_original_rows_and_literal_data() {
    for format in ["mixed", "markdown", "xml"] {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let registry = registry();
        let package = registry.path().join("org.source/plain/v1.0.0");
        fs::remove_file(package.join(boot("package.md"))).unwrap();
        let native = boot("package.payload");
        let raw = "data/ordinary.xml";
        let binary = "data/binary.bin";
        let unselected = "outside/unselected.payload";
        let rows = row(
            "package",
            &native,
            "application/xml",
            "vibe-spec",
            "explicit",
        ) + &row("package", raw, "application/xml", "xml-data", "explicit")
            + &row(
                "package",
                binary,
                "application/octet-stream",
                "opaque",
                "explicit",
            )
            + &row(
                "package",
                unselected,
                "application/xml",
                "vibe-spec",
                "explicit",
            );
        write(
            &package,
            "vibe.toml",
            format!(
                "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n{rows}\n[boot_snippet]\nsource='{native}'\nlink='dynamic'\n"
            ),
        );
        write(
            &package,
            &native,
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p>INSTALLED-NATIVE</p></spec>",
        );
        let literal =
            "<invalid\n#use spec://forged/package/rule\n#embed spec://forged/other/rule\n";
        write(&package, raw, literal);
        write(&package, binary, [0xff, 0x00, 0xfe]);
        write(
            &package,
            unselected,
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p>initially valid</p></spec>",
        );
        write(
            root.path(),
            "vibe.toml",
            format!("[project]\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n"),
        );
        install(&user, root.path(), registry.path());
        let mut lock = vibe_core::manifest::Lockfile::read(root.path().join("vibe.lock")).unwrap();
        let p = &lock.packages[0];
        let slot = vibe_workspace::vibedeps::slot_rel_path(&p.group, &p.name, &p.version);
        let slot_path = root.path().join(&slot);
        let output = match format {
            "mixed" => native.clone(),
            "markdown" => boot("package.md"),
            _ => boot("package.xml"),
        };
        // files_written is the display request; no INDEX traversal is involved.
        lock.packages[0].files_written = vec![
            format!("{slot}/{output}").into(),
            format!("{slot}/{raw}").into(),
        ];
        lock.write(root.path().join("vibe.lock")).unwrap();
        let mut record = vibe_workspace::vibedeps::read_slot_record(&slot_path).unwrap();
        let unselected_row = record
            .files
            .iter_mut()
            .find(|r| r.source.as_deref().unwrap_or(&r.path) == unselected)
            .unwrap();
        write(&slot_path, &unselected_row.path, "<malformed native");
        unselected_row.sha256 = vibe_workspace::vibedeps::sha256_bytes(b"<malformed native");
        vibe_workspace::vibedeps::write_slot_record(&slot_path, &record).unwrap();
        let source_before = snapshot(registry.path());
        let doc = document(show(&user, root.path()));
        assert_eq!(snapshot(registry.path()), source_before);
        let sections = doc["sections"].as_array().unwrap();
        let selected = sections
            .iter()
            .find(|s| s["path"] == format!("{slot}/{output}"))
            .unwrap();
        assert!(
            selected["body"]
                .as_str()
                .unwrap()
                .contains("INSTALLED-NATIVE")
        );
        assert!(!selected["body"].as_str().unwrap().contains("<spec"));
        assert_eq!(selected["origin"], "package:org.source/plain@1.0.0");
        assert_eq!(
            sections
                .iter()
                .find(|s| s["path"] == format!("{slot}/{raw}"))
                .unwrap()["body"],
            literal
        );
        // Generated XML is a multi-root tape, not an authored native document.
        let tape = "<?xml version=\"1.0\"?><spec><title>ONE</title></spec>\n<?xml version=\"1.0\"?><spec><title>TWO</title></spec>\n";
        let static_md = root.path().join(boot("STATIC.md"));
        if static_md.exists() {
            fs::remove_file(static_md).unwrap();
        }
        write(root.path(), &boot("STATIC.xml"), tape);
        let doc = document(show(&user, root.path()));
        assert_eq!(
            doc["sections"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["path"] == boot("STATIC.xml"))
                .unwrap()["body"],
            tape
        );
        let mut binary_lock = lock.clone();
        binary_lock.packages[0]
            .files_written
            .push(format!("{slot}/{binary}").into());
        binary_lock.write(root.path().join("vibe.lock")).unwrap();
        refuses(&user, root.path(), "not UTF-8");
        lock.write(root.path().join("vibe.lock")).unwrap();
        let manifest_path = root.path().join("vibe.toml");
        let manifest = fs::read(&manifest_path).unwrap();
        let mut changed: toml::Value =
            toml::from_str(std::str::from_utf8(&manifest).unwrap()).unwrap();
        changed["project"]["spec_format"] =
            toml::Value::String(if format == "mixed" { "xml" } else { "mixed" }.into());
        fs::write(&manifest_path, toml::to_string(&changed).unwrap()).unwrap();
        refuses(&user, root.path(), "format");
        fs::write(manifest_path, manifest).unwrap();
        let mut wrong_hash = lock.clone();
        wrong_hash.packages[0].content_hash =
            vibe_core::ContentHash::parse(&format!("sha256:{}", "0".repeat(64))).unwrap();
        wrong_hash.write(root.path().join("vibe.lock")).unwrap();
        refuses(&user, root.path(), "source hash");
        lock.write(root.path().join("vibe.lock")).unwrap();
        let record_path = slot_path.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
        let installed_manifest_path = slot_path.join("vibe.toml");
        let installed_manifest = fs::read(&installed_manifest_path).unwrap();
        fs::remove_file(&installed_manifest_path).unwrap();
        refuses(&user, root.path(), "no manifest");
        fs::write(&installed_manifest_path, installed_manifest).unwrap();
        let mut wrong_output = lock.clone();
        wrong_output.packages[0]
            .files_written
            .push(format!("{slot}/unrecorded.payload").into());
        wrong_output.write(root.path().join("vibe.lock")).unwrap();
        refuses(&user, root.path(), "not recorded");
        lock.write(root.path().join("vibe.lock")).unwrap();
        let record_bytes = fs::read(&record_path).unwrap();
        fs::write(&record_path, "bad record").unwrap();
        refuses(&user, root.path(), "record");
        fs::write(&record_path, &record_bytes).unwrap();
        let bytes = fs::read(slot_path.join(&output)).unwrap();
        write(&slot_path, &output, "changed");
        refuses(&user, root.path(), "digest");
        write(&slot_path, &output, &bytes);
        fs::remove_file(slot_path.join(&output)).unwrap();
        refuses(&user, root.path(), "absent");
        write(&slot_path, &output, &bytes);
        fs::remove_file(&record_path).unwrap();
        refuses(&user, root.path(), "no record");
    }
}

#[test]
fn named_project_written_paths_and_wal_use_the_actual_host_policy() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let registry = registry();
    let native = "requested/native.payload";
    let raw = "requested/ordinary.xml";
    let wal = vibe_core::machine_json_path(&vibe_core::layout::current_wal_md());
    let rows = row(
        "project",
        native,
        "application/xml",
        "vibe-spec",
        "explicit",
    ) + &row("project", raw, "application/xml", "xml-data", "explicit")
        + &row("project", &wal, "text/markdown", "commonmark", "explicit");
    write(
        root.path(),
        "vibe.toml",
        format!("[project]\nname='host'\nversion='1.0.0'\n{rows}"),
    );
    write(
        root.path(),
        native,
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p>HOST-WRITTEN</p></spec>",
    );
    write(
        root.path(),
        raw,
        "<broken\n#use spec://ordinary/data/root\n",
    );
    write(root.path(), &wal, "#use spec://ordinary/wal/root\n");
    install(&user, root.path(), registry.path());
    let mut lock = vibe_core::manifest::Lockfile::read(root.path().join("vibe.lock")).unwrap();
    lock.packages[0].files_written = vec![native.into(), raw.into()];
    lock.write(root.path().join("vibe.lock")).unwrap();
    let doc = document(show(&user, root.path()));
    let sections = doc["sections"].as_array().unwrap();
    assert!(
        sections.iter().find(|s| s["path"] == native).unwrap()["body"]
            .as_str()
            .unwrap()
            .contains("HOST-WRITTEN")
    );
    assert_eq!(
        sections.iter().find(|s| s["path"] == raw).unwrap()["body"],
        "<broken\n#use spec://ordinary/data/root\n"
    );
    assert_eq!(
        sections.iter().find(|s| s["path"] == wal).unwrap()["origin"],
        "wal"
    );
    write(root.path(), native, "<malformed");
    refuses(&user, root.path(), "SOURCE-SHOW-READ");
}

#[test]
fn written_registered_member_uses_member_policy_and_exact_artifact_exclusion() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let registry = registry();
    write(
        root.path(),
        "vibe.toml",
        "[project]\nname='host'\nversion='1.0.0'\nsources=[]\n[workspace]\nmembers=['member']\n",
    );
    let rows = row(
        "package",
        "requested/STATIC.xml",
        "application/xml",
        "vibe-spec",
        "explicit",
    );
    write(
        root.path(),
        "member/vibe.toml",
        format!(
            "[package]\nkind='flow'\ngroup='org.member'\nname='member'\nversion='1.0.0'\n{rows}"
        ),
    );
    write(
        root.path(),
        "member/requested/STATIC.xml",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p>MEMBER-NATIVE</p></spec>",
    );
    install(&user, root.path(), registry.path());
    let mut lock = vibe_core::manifest::Lockfile::read(root.path().join("vibe.lock")).unwrap();
    lock.packages[0].files_written = vec!["member/requested/STATIC.xml".into()];
    lock.write(root.path().join("vibe.lock")).unwrap();
    let doc = document(show(&user, root.path()));
    let selected = doc["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == "member/requested/STATIC.xml")
        .unwrap();
    assert!(selected["body"].as_str().unwrap().contains("MEMBER-NATIVE"));
    assert!(!selected["body"].as_str().unwrap().contains("<spec"));
    // Origin label remains legacy display metadata, not the actual source owner.
    assert_eq!(selected["origin"], "package:org.source/plain@1.0.0");
    write(root.path(), "member/requested/STATIC.xml", "<malformed");
    refuses(&user, root.path(), "SOURCE-SHOW-READ");
}
