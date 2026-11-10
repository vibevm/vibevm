//! Ordinary package init preserves the host target and existing installed bytes.

#![cfg(test)]

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#artifacts");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::common::{self, UserScratch};

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
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

fn changed(
    before: &BTreeMap<PathBuf, Vec<u8>>,
    after: &BTreeMap<PathBuf, Vec<u8>>,
) -> Vec<PathBuf> {
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}

fn preserves_host(spec_format: Option<&str>, static_name: &str) {
    let user = UserScratch::new();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Fixture", "--path"])
        .arg(root)
        .assert()
        .success();
    user.vibe()
        .args([
            "init",
            "package",
            "org.demo/first",
            "--kind",
            "flow",
            "--link",
            "static",
            "--author",
            "Fixture",
            "--path",
        ])
        .arg(root)
        .assert()
        .success();

    let manifest = root.join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    if let Some(format) = spec_format {
        let updated = text.replacen(
            "[project]\n",
            &format!("[project]\nspec_format = \"{format}\"\n"),
            1,
        );
        assert_ne!(updated, text, "fixture must declare its host dialect");
        text = updated;
    }
    text.push_str(
        "\n[requires.packages]\n\"org.demo/first\" = { version = \"^0.1\", link = \"static\" }\n",
    );
    fs::write(&manifest, text).unwrap();
    user.vibe()
        .args([
            "install",
            "--offline",
            "--no-default-registry",
            "--assume-yes",
            "--registry",
        ])
        .arg(root.join(common::packages_root()))
        .arg("--path")
        .arg(root)
        .assert()
        .success();

    let lane = common::boot_rel(static_name);
    assert!(
        root.join(&lane).is_file(),
        "fixture must have the chosen lane"
    );
    let deps = root.join(common::deps_root());
    let before_deps = files(&deps);
    assert!(
        before_deps.keys().any(|path| path.ends_with("vibe.toml")),
        "a real installed package must make dependency preservation non-vacuous"
    );
    fs::write(root.join("unrelated.bin"), b"\x00UNRELATED\xff").unwrap();
    // Authored text on both sides of the managed block, with a CRLF line to
    // prove the author's own line endings survive. The block is written at
    // the **top** of an instruction file since the owner decision of
    // 2026-11-10 (`PROP-076#SYS-BLOCK-AT-THE-TOP`), so planting it in the
    // middle like this means the next write migrates it — once — which is
    // asserted below rather than forbidden here.
    for launcher in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let path = root.join(launcher);
        let generated = fs::read_to_string(&path).unwrap();
        fs::write(path, format!("OWNER BEFORE\r\n{generated}\nOWNER AFTER\n")).unwrap();
    }
    let before_boot = files(&root.join(common::boot_rel("")));
    let before_packages = files(&root.join(common::packages_root()));
    // The instruction files are **not** in this set: vibe owns the managed
    // block inside each, and a block the fixture planted in the middle is
    // migrated to the top. What must hold of them is asserted separately.
    let protected: BTreeMap<_, _> = ["vibe.toml", "vibe.lock", ".gitignore", "unrelated.bin"]
        .into_iter()
        .map(|path| (PathBuf::from(path), fs::read(root.join(path)).unwrap()))
        .collect();

    // The new package's format/link must not choose the HOST's static dialect.
    user.vibe()
        .args([
            "init",
            "package",
            "org.demo/second",
            "--kind",
            "flow",
            "--format",
            "simple",
            "--link",
            "dynamic",
            "--author",
            "Fixture",
            "--path",
        ])
        .arg(root)
        .assert()
        .success();
    let added = PathBuf::from("org.demo/second/v0.1.0");
    let new_manifest = root
        .join(common::packages_root())
        .join(&added)
        .join("vibe.toml");
    let added_text = fs::read_to_string(new_manifest).unwrap();
    let added_manifest = vibe_core::manifest::Manifest::parse_str(&added_text).unwrap();
    assert_eq!(
        added_manifest.require_package().unwrap().format,
        vibe_core::manifest::PackageFormat::Simple
    );
    assert_eq!(
        added_manifest.boot_snippet.unwrap().link,
        Some(vibe_core::manifest::LinkType::Dynamic)
    );

    let after_deps = files(&deps);
    let after_boot = files(&root.join(common::boot_rel("")));
    let mut after_packages = files(&root.join(common::packages_root()));
    after_packages.retain(|path, _| !path.starts_with(&added));
    let after_protected = protected
        .keys()
        .map(|path| (path.clone(), fs::read(root.join(path)).unwrap()))
        .collect();
    let changes = [
        ("host boot", changed(&before_boot, &after_boot)),
        ("derived dependencies", changed(&before_deps, &after_deps)),
        (
            "existing package sources",
            changed(&before_packages, &after_packages),
        ),
        (
            "host and unrelated files",
            changed(&protected, &after_protected),
        ),
    ];
    println!(
        "host={spec_format:?}; installed files={}; changes={changes:?}",
        before_deps.len()
    );
    assert!(
        changes.iter().all(|(_, paths)| paths.is_empty()),
        "{changes:?}"
    );
    assert_instruction_files_migrated_once(root);

    let other_lane = if static_name == "STATIC.xml" {
        "STATIC.md"
    } else {
        "STATIC.xml"
    };
    assert!(root.join(&lane).is_file());
    assert!(!root.join(common::boot_rel(other_lane)).exists());
    let index: toml::Value =
        toml::from_str(&fs::read_to_string(root.join(common::index_rel())).unwrap()).unwrap();
    assert_eq!(index["static"].as_str(), Some(lane.as_str()));
    for launcher in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let text = fs::read_to_string(root.join(launcher)).unwrap();
        assert!(text.contains(&lane));
        assert!(!text.contains(&common::boot_rel(other_lane)));
    }
}

#[test]
#[specmark::verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-PAIR"
)]
fn init_package_keeps_existing_xml_host_and_derived_slots() {
    preserves_host(Some("xml"), "STATIC.xml");
}

#[test]
fn init_package_keeps_explicit_markdown_host_and_derived_slots() {
    preserves_host(Some("markdown"), "STATIC.md");
}

#[test]
fn init_package_keeps_default_host_and_derived_slots() {
    preserves_host(None, "STATIC.md");
}

/// The managed block leads each instruction file, the authored text keeps its
/// own order and its own line endings below it, and a second run changes
/// nothing (`PROP-076#SYS-BLOCK-AT-THE-TOP`).
fn assert_instruction_files_migrated_once(root: &Path) {
    for launcher in ["CLAUDE.md", "AGENTS.md", "GEMINI.md"] {
        let path = root.join(launcher);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("<vibevm>\n"), "{launcher}: {text}");
        assert_eq!(
            text.matches("<vibevm>\n").count(),
            1,
            "{launcher} must carry exactly one block"
        );
        // Both authored fragments survive, in their own order, and the
        // author's CRLF is the author's.
        let before = text.find("OWNER BEFORE\r\n").expect(launcher);
        let after = text.find("OWNER AFTER\n").expect(launcher);
        assert!(before < after, "{launcher}: authored order changed");
        assert!(
            text.find("</vibevm>").expect(launcher) < before,
            "{launcher}: the block must precede the authored text"
        );
    }
}
