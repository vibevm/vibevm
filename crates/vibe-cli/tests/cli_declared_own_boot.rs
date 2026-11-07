//! Real installation and regeneration preserve declared own-boot enrollment.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_test_support::UserScratch;

fn boot(name: &str) -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_dir().join(name))
}

fn write(root: &Path, relative: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn row(file: &str, media: &str, interpretation: &str, loading: &str) -> String {
    let role = if interpretation == "vibe-spec" {
        "spec"
    } else {
        "resource"
    };
    format!(
        "[[project.sources]]\nfile='{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='{loading}'\n"
    )
}

fn fixture(grouped: bool, target: &str) -> (tempfile::TempDir, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let package = registry.path().join("org.source/plain/v1.0.0");
    let dependency = boot("dependency.md");
    write(
        &package,
        "vibe.toml",
        format!(
            "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n[boot_snippet]\nsource='{dependency}'\nlink='static'\n"
        ),
    );
    write(
        &package,
        &dependency,
        "# Dependency\n\nDEPENDENCY-STATIC-CONTENT\n",
    );
    let group = if grouped { "group='org.host'\n" } else { "" };
    let rows = row(&boot("00-core.md"), "text/markdown", "vibe-spec", "scan")
        + &row(
            &boot("20-native.payload"),
            "application/xml",
            "vibe-spec",
            "scan",
        )
        + &row(&boot("00-core.xml"), "application/xml", "xml-data", "scan")
        + &row(
            &boot("30-explicit.md"),
            "text/markdown",
            "vibe-spec",
            "explicit",
        )
        + &row(
            "outside/broken.xml",
            "application/xml",
            "vibe-spec",
            "explicit",
        )
        + &row(
            "outside/missing.md",
            "text/markdown",
            "vibe-spec",
            "explicit",
        );
    write(
        root.path(),
        "vibe.toml",
        format!("[project]\n{group}name='host'\nversion='1.0.0'\nspec_format='{target}'\n{rows}"),
    );
    write(root.path(), &boot("00-core.md"), "# Core\n\nOWN-MARKDOWN\n");
    write(
        root.path(),
        &boot("20-native.payload"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Own XML</title><p>OWN-XML</p></spec>",
    );
    write(
        root.path(),
        &boot("00-core.xml"),
        "RAW-MUST-NOT-ENROLL\n#use spec://missing/package/rule\n#embed spec://missing/package/rule\n",
    );
    write(
        root.path(),
        &boot("30-explicit.md"),
        "# Explicit\n\nEXPLICIT-MUST-NOT-ENROLL\n",
    );
    write(root.path(), "outside/broken.xml", "<malformed");
    (root, registry)
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
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

fn install(user: &UserScratch, root: &Path, registry: &Path) -> std::process::Output {
    user.vibe()
        .args(["install", "org.source/plain@=1.0.0", "--path"])
        .arg(root)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .output()
        .unwrap()
}

fn assert_index(root: &Path) {
    let index = fs::read_to_string(root.join(boot("INDEX.md"))).unwrap();
    let parsed: toml::Value = toml::from_str(&index).unwrap();
    let entries = parsed["entry"].as_array().unwrap();
    let paths = entries
        .iter()
        .map(|entry| {
            assert_eq!(entry["kind"].as_str(), Some("static"), "{index}");
            assert!(entry.get("when").is_none(), "{index}");
            entry["path"].as_str().unwrap().to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![boot("00-core.md"), boot("20-native.payload")],
        "{index}"
    );
}

#[test]
fn grouped_and_ungrouped_install_and_reinstall_keep_own_inputs_as_index_references() {
    for (grouped, target) in [(true, "xml"), (false, "markdown")] {
        let user = UserScratch::new();
        let (root, registry) = fixture(grouped, target);
        let originals = inventory(root.path());
        let installed = install(&user, root.path(), registry.path());
        assert!(installed.status.success(), "{installed:?}");
        assert_index(root.path());
        let static_name = if target == "xml" {
            "STATIC.xml"
        } else {
            "STATIC.md"
        };
        let lane = fs::read_to_string(root.path().join(boot(static_name))).unwrap();
        assert!(lane.contains("DEPENDENCY-STATIC-CONTENT"), "{lane}");
        for marker in [
            "OWN-MARKDOWN",
            "OWN-XML",
            "RAW-MUST-NOT-ENROLL",
            "EXPLICIT-MUST-NOT-ENROLL",
        ] {
            assert!(
                !lane.contains(marker),
                "own input unexpectedly compiled: {lane}"
            );
        }
        for (relative, bytes) in &originals {
            if relative != Path::new("vibe.toml")
                && let Some(bytes) = bytes
            {
                assert_eq!(&fs::read(root.path().join(relative)).unwrap(), bytes);
            }
        }
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(root.path().join("vibe.toml")).unwrap()).unwrap();
        assert_eq!(manifest["project"].get("group").is_some(), grouped);
        fs::remove_file(root.path().join(boot("INDEX.md"))).unwrap();
        let regenerated = user
            .vibe()
            .arg("reinstall")
            .arg(root.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(regenerated.status.success(), "{regenerated:?}");
        assert_index(root.path());
    }
}

#[test]
fn invalid_or_missing_selected_own_boot_refuses_before_requested_product_publication() {
    for missing in [false, true] {
        let user = UserScratch::new();
        let (root, registry) = fixture(true, "xml");
        let selected = root.path().join(boot("20-native.payload"));
        if missing {
            fs::remove_file(&selected).unwrap();
        } else {
            fs::write(&selected, "<spec><unfinished").unwrap();
        }
        write(
            root.path(),
            ".vibe/retained-sentinel",
            "pre-existing product bytes",
        );
        let before = inventory(root.path());
        let result = install(&user, root.path(), registry.path());
        assert!(!result.status.success(), "{result:?}");
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            diagnostic.contains("20-native.payload") || diagnostic.contains("SOURCE-HOST-BOOT"),
            "{diagnostic}"
        );
        let mut after = inventory(root.path());
        // Only the established empty coordination bootstrap is allowed.
        for rel in [".vibe", ".vibe/lifecycle"] {
            let path = PathBuf::from(rel);
            if !before.contains_key(&path) && after.contains_key(&path) {
                assert_eq!(after.remove(&path), Some(None));
            }
        }
        let lock = PathBuf::from(".vibe/lifecycle.lock");
        if !before.contains_key(&lock) && after.contains_key(&lock) {
            assert!(
                !fs::symlink_metadata(root.path().join(&lock))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(after.remove(&lock), Some(Some(Vec::new())));
        }
        let runs = after
            .keys()
            .filter(|path| {
                path.parent() == Some(Path::new(".vibe/lifecycle")) && !before.contains_key(*path)
            })
            .cloned()
            .collect::<Vec<_>>();
        assert!(runs.len() <= 1, "unexpected coordination entries: {runs:?}");
        for run in runs {
            let name = run.file_name().unwrap().to_str().unwrap();
            assert_eq!(name.len(), 32);
            assert!(
                name.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            );
            assert_eq!(after.remove(&run), Some(None));
        }
        assert_eq!(after, before, "product mutation before source refusal");
    }
}
