//! Exact committed boot inputs, through the public read-only tree command.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-TREE-READ",
    r = 1
);

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_test_support::UserScratch;

const REAL: &str = "spec://org.source/plain/rules#root";
fn boot(name: &str) -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_dir().join(name))
}
fn write(root: &Path, relative: &str, text: impl AsRef<[u8]>) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
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
    let mut out = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                out.insert(relative, None);
                pending.push(path);
            } else {
                out.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    out
}
fn tree(user: &UserScratch, root: &Path) -> std::process::Output {
    let before = snapshot(root);
    let result = user
        .vibe()
        .args(["--json", "tree", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        snapshot(root),
        before,
        "tree changed project bytes or namespace"
    );
    result
}
fn document(output: &std::process::Output) -> Value {
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
            "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n[boot_snippet]\nsource='{}'\nlink='dynamic'\n",
            boot("package.md")
        ),
    );
    write(&package, &boot("package.md"), "# Dependency\n");
    registry
}

#[test]
fn installed_grouped_and_ungrouped_hosts_read_selected_native_media_only() {
    for grouped in [true, false] {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let registry = registry();
        let group = if grouped { "group='org.host'\n" } else { "" };
        let native = boot("20-native.payload");
        let rows = row("project", &native, "application/xml", "vibe-spec", "scan")
            + &row(
                "project",
                &boot("20-native.md"),
                "text/markdown",
                "commonmark",
                "scan",
            )
            + &row(
                "project",
                "outside/missing.xml",
                "application/xml",
                "vibe-spec",
                "explicit",
            );
        write(
            root.path(),
            "vibe.toml",
            format!("[project]\n{group}name='host'\nversion='1.0.0'\n{rows}"),
        );
        write(
            root.path(),
            &native,
            format!(
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Native</title><p>#use {REAL}</p></spec>"
            ),
        );
        write(
            root.path(),
            &boot("20-native.md"),
            "#use spec://forged/package/rule\n",
        );
        install(&user, root.path(), registry.path());
        let registry_before = snapshot(registry.path());
        let doc = document(&tree(&user, root.path()));
        assert_eq!(snapshot(registry.path()), registry_before);
        let references = doc["in_place_specs"].as_array().unwrap();
        assert_eq!(references.len(), 1, "{doc}");
        assert_eq!(references[0]["address"], REAL);
        assert_eq!(references[0]["file"], native);
        assert_eq!(references[0]["carrier"], "#use");
        assert_eq!(references[0]["resolved"], false);
        assert_eq!(doc["packages"][0]["load"]["type"], "dynamic");
    }
}

fn selected_entry(root: &Path, path: &str) {
    let index = root.join(boot("INDEX.md"));
    let mut text = fs::read_to_string(&index).unwrap_or_else(|_| "schema=1\n".into());
    text.push_str(&format!("\n[[entry]]\nkind='static'\npath='{path}'\n"));
    fs::create_dir_all(index.parent().unwrap()).unwrap();
    fs::write(index, text).unwrap();
}
fn refuses(user: &UserScratch, root: &Path, expected: &str) {
    let output = tree(user, root);
    assert!(!output.status.success(), "{output:?}");
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostic.contains(expected),
        "expected {expected}: {diagnostic}"
    );
}

#[test]
fn installed_declared_outputs_map_original_source_in_all_materialized_formats() {
    for format in ["mixed", "markdown", "xml"] {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let registry = registry();
        let package = registry.path().join("org.source/plain/v1.0.0");
        let original = boot("selected.payload");
        let ordinary = boot("selected.md");
        // Avoid physical output collisions after conversion while retaining an
        // ordinary XML-looking input that must never enter native parsing.
        let raw = "data/ordinary.xml";
        let rows = row(
            "package",
            &original,
            "application/xml",
            "vibe-spec",
            "explicit",
        ) + &row("package", raw, "application/xml", "xml-data", "explicit")
            + &row(
                "package",
                "unselected/invalid.payload",
                "application/xml",
                "vibe-spec",
                "explicit",
            );
        write(
            &package,
            "vibe.toml",
            format!(
                "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n{rows}\n[boot_snippet]\nsource='{original}'\nlink='dynamic'\n"
            ),
        );
        write(
            &package,
            &original,
            format!(
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Package</title><p>#use {REAL}</p></spec>"
            ),
        );
        write(&package, raw, "<invalid\n#use spec://forged/package/rule\n");
        write(
            &package,
            "unselected/invalid.payload",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p>Valid at install</p></spec>",
        );
        write(
            root.path(),
            "vibe.toml",
            format!("[project]\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n"),
        );
        install(&user, root.path(), registry.path());
        let lock = vibe_core::manifest::Lockfile::read(root.path().join("vibe.lock")).unwrap();
        let p = &lock.packages[0];
        let slot = vibe_workspace::vibedeps::slot_rel_path(&p.group, &p.name, &p.version);
        // A byte-valid installed binding with malformed *unselected* native
        // syntax must not cause the tree to parse outside the committed graph.
        let slot_path = root.path().join(&slot);
        let mut record = vibe_workspace::vibedeps::read_slot_record(&slot_path).unwrap();
        let unselected = record
            .files
            .iter_mut()
            .find(|row| row.source.as_deref().unwrap_or(&row.path) == "unselected/invalid.payload")
            .unwrap();
        let invalid = if format == "markdown" {
            "#use invalid\n"
        } else {
            "<invalid"
        };
        fs::write(slot_path.join(&unselected.path), invalid).unwrap();
        unselected.sha256 = vibe_workspace::vibedeps::sha256_bytes(invalid.as_bytes());
        vibe_workspace::vibedeps::write_slot_record(&slot_path, &record).unwrap();
        let output = match format {
            "mixed" => original.clone(),
            "markdown" => ordinary,
            _ => boot("selected.xml"),
        };
        let selected = format!("{slot}/{output}");
        selected_entry(root.path(), &format!("{slot}/{raw}"));
        let source_before = snapshot(registry.path());
        let doc = document(&tree(&user, root.path()));
        assert_eq!(snapshot(registry.path()), source_before);
        let refs = doc["in_place_specs"].as_array().unwrap();
        assert_eq!(refs.len(), 1, "{format}: {doc}");
        assert_eq!(refs[0]["file"], selected);
        assert_eq!(refs[0]["address"], REAL);
        assert_eq!(refs[0]["line"], 3, "canonical projection line");
        assert_eq!(doc["packages"][0]["load"]["type"], "dynamic");
        // Query format comes from config, even when STATIC.md cannot distinguish it.
        let manifest_path = root.path().join("vibe.toml");
        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let mut changed: toml::Value = toml::from_str(&manifest).unwrap();
        changed["project"]["spec_format"] = toml::Value::String(
            if format == "mixed" {
                "markdown"
            } else {
                "mixed"
            }
            .into(),
        );
        fs::write(&manifest_path, toml::to_string(&changed).unwrap()).unwrap();
        refuses(&user, root.path(), "format");
        fs::write(&manifest_path, manifest).unwrap();
        let lock_path = root.path().join("vibe.lock");
        let lock_bytes = fs::read(&lock_path).unwrap();
        let mut wrong = lock.clone();
        wrong.packages[0].content_hash =
            vibe_core::ContentHash::parse(&format!("sha256:{}", "0".repeat(64))).unwrap();
        wrong.write(&lock_path).unwrap();
        refuses(&user, root.path(), "source hash");
        fs::write(&lock_path, lock_bytes).unwrap();
        let record = root
            .path()
            .join(&slot)
            .join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
        let record_bytes = fs::read(&record).unwrap();
        let installed_manifest_path = slot_path.join("vibe.toml");
        let installed_manifest_bytes = fs::read(&installed_manifest_path).unwrap();
        let mut emptied: toml::Value =
            toml::from_str(std::str::from_utf8(&installed_manifest_bytes).unwrap()).unwrap();
        emptied["package"]["sources"] = toml::Value::Array(Vec::new());
        fs::write(&installed_manifest_path, toml::to_string(&emptied).unwrap()).unwrap();
        refuses(&user, root.path(), "manifest digest");
        fs::write(&installed_manifest_path, installed_manifest_bytes).unwrap();
        fs::write(&record, "not a record").unwrap();
        refuses(&user, root.path(), "record");
        fs::write(&record, &record_bytes).unwrap();
        let source_path = root.path().join(&selected);
        let bytes = fs::read(&source_path).unwrap();
        fs::write(&source_path, "changed").unwrap();
        refuses(&user, root.path(), "digest");
        fs::write(&source_path, &bytes).unwrap();
        let mut coherent = vibe_workspace::vibedeps::read_slot_record(&slot_path).unwrap();
        let selected_row = coherent
            .files
            .iter_mut()
            .find(|row| row.path == output)
            .unwrap();
        let invalid = if format == "markdown" {
            "#use invalid\n"
        } else {
            "<invalid"
        };
        fs::write(&source_path, invalid).unwrap();
        selected_row.sha256 = vibe_workspace::vibedeps::sha256_bytes(invalid.as_bytes());
        vibe_workspace::vibedeps::write_slot_record(&slot_path, &coherent).unwrap();
        refuses(&user, root.path(), "SOURCE-TREE-READ");
        fs::write(&record, &record_bytes).unwrap();
        fs::write(&source_path, &bytes).unwrap();
        fs::remove_file(&source_path).unwrap();
        refuses(&user, root.path(), "absent");
        fs::write(&source_path, bytes).unwrap();
        fs::remove_file(&record).unwrap();
        refuses(&user, root.path(), "no record");
    }
}

#[test]
fn selected_explicit_host_paths_refuse_bad_native_but_ignore_unselected_and_random_manifests() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let native = "nested/rule.payload";
    let rows = row(
        "project",
        native,
        "application/xml",
        "vibe-spec",
        "explicit",
    ) + &row(
        "project",
        "other/missing.xml",
        "application/xml",
        "vibe-spec",
        "scan",
    ) + &row(
        "project",
        "nested/raw.xml",
        "application/xml",
        "xml-data",
        "explicit",
    );
    write(
        root.path(),
        "vibe.toml",
        format!("[project]\nname='host'\nversion='1.0.0'\n{rows}"),
    );
    write(root.path(), "nested/vibe.toml", "malformed [manifest");
    write(
        root.path(),
        native,
        format!("<spec xmlns=\"https://vibevm.org/spec/1\"><p>#embed {REAL}</p></spec>"),
    );
    write(
        root.path(),
        "nested/raw.xml",
        format!("malformed <xml\n#embed {REAL}"),
    );
    selected_entry(root.path(), native);
    selected_entry(root.path(), "nested/raw.xml");
    let doc = document(&tree(&user, root.path()));
    assert_eq!(doc["in_place_specs"].as_array().unwrap().len(), 1);
    assert_eq!(doc["in_place_specs"][0]["carrier"], "#embed");
    write(root.path(), native, "<malformed");
    refuses(&user, root.path(), "SOURCE-TREE-READ");
    write(
        root.path(),
        native,
        "<spec xmlns=\"https://vibevm.org/spec/1\"><p>#use invalid</p></spec>",
    );
    refuses(&user, root.path(), "projection line");
    fs::remove_file(root.path().join(native)).unwrap();
    refuses(&user, root.path(), "missing");
}

#[test]
fn registered_member_has_its_own_policy_and_preserves_selected_file() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[project]\nname='host'\nversion='1.0.0'\nsources=[]\n[workspace]\nmembers=['member']\n",
    );
    let rows = row(
        "package",
        "native.payload",
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
        "member/native.payload",
        format!("<spec xmlns=\"https://vibevm.org/spec/1\"><p>#use {REAL}</p></spec>"),
    );
    selected_entry(root.path(), "member/native.payload");
    let doc = document(&tree(&user, root.path()));
    assert_eq!(doc["in_place_specs"].as_array().unwrap().len(), 1);
    assert_eq!(doc["in_place_specs"][0]["file"], "member/native.payload");
    assert_eq!(doc["in_place_specs"][0]["address"], REAL);
}
