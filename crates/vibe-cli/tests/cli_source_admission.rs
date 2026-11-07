//! Actual install refusal must precede requested metadata and lifecycle publication.
mod common;
use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out);
            } else {
                out.insert(rel, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALL-BOUNDARY"
)]
fn declared_install_refuses_before_requested_metadata_and_lifecycle_effects() {
    for (target, boot, sources) in [
        ("mixed", true, "sources=[]\n"),
        (
            "mixed",
            false,
            "[[package.sources]]\nfile='missing.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
        ),
    ] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let source = registry.path().join("org.source/declared/v1.0.0");
        fs::create_dir_all(&source).unwrap();
        let boot_file = common::boot_rel("authored.md");
        let extra = if boot {
            format!("\n[boot_snippet]\nsource='{boot_file}'\n")
        } else {
            String::new()
        };
        fs::write(source.join("vibe.toml"),format!("[package]\ngroup='org.source'\nname='declared'\nkind='flow'\nversion='1.0.0'\n{sources}{extra}")).unwrap();
        if boot {
            let path = source.join(&boot_file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "# Must not compile\n").unwrap();
        }
        fs::write(project.path().join("vibe.toml"),format!("[project]\ngroup='org.consumer'\nname='host'\nversion='1.0.0'\nspec_format='{target}'\n")).unwrap();
        // Unrelated pre-existing product bytes must survive, even inside .vibe.
        for rel in [".vibe/facts-sentinel", "vibevm/vibedeps/retained/payload"] {
            let path = project.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"pre-existing bytes").unwrap();
        }
        let before = inventory(project.path());
        let result = user
            .vibe()
            .args(["install", "org.source/declared@=1.0.0", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(!result.status.success(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("SOURCE-ADMISSION-BOUNDARY"),
            "{result:?}"
        );
        let mut after = inventory(project.path());
        // The outer coordination lease precedes the authoritative snapshot.
        // Its empty lock and fresh empty identity directory are infrastructure,
        // not lifecycle execution publication. Never ignore the .vibe subtree.
        for rel in [".vibe", ".vibe/lifecycle"] {
            let key = PathBuf::from(rel);
            if !before.contains_key(&key) {
                assert_eq!(after.remove(&key), Some(None));
            }
        }
        let lock = PathBuf::from(".vibe/lifecycle.lock");
        assert!(
            !fs::symlink_metadata(project.path().join(&lock))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(after.remove(&lock), Some(Some(Vec::new())));
        let runs: Vec<_> = after
            .keys()
            .filter(|path| {
                path.parent() == Some(Path::new(".vibe/lifecycle")) && !before.contains_key(*path)
            })
            .cloned()
            .collect();
        assert_eq!(runs.len(), 1);
        let run = &runs[0];
        let name = run.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 32);
        assert!(
            name.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert!(
            fs::symlink_metadata(project.path().join(run))
                .unwrap()
                .file_type()
                .is_dir()
        );
        assert_eq!(after.remove(run), Some(None));
        assert_eq!(
            after, before,
            "{target}/{boot}: product publication before admission"
        );
    }
}

#[test]
fn transformed_explicit_empty_policies_preserve_raw_sources_without_native_facts() {
    for policy in ["sources=[]\n", "source_exports=[]\n"] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let source = registry.path().join("org.source/declared/v1.0.0");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("vibe.toml"),format!("[package]\ngroup='org.source'\nname='declared'\nkind='flow'\nversion='1.0.0'\n{policy}")).unwrap();
        let relative = common::spec_rel("RAW.md");
        let body = b"# Raw\n\n@fact:FAKE ordinary bytes. @status:impl/done\n";
        fs::create_dir_all(source.join(&relative).parent().unwrap()).unwrap();
        fs::write(source.join(&relative), body).unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            "[project]\ngroup='org.consumer'\nname='host'\nversion='1.0.0'\nspec_format='xml'\n",
        )
        .unwrap();
        for rel in [".vibe/facts-sentinel", "vibevm/vibedeps/retained/payload"] {
            let path = project.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"pre-existing bytes").unwrap();
        }
        let result = user
            .vibe()
            .args(["install", "org.source/declared@=1.0.0", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(result.status.success(), "{policy}: {result:?}");
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            project.path(),
            &vibe_core::Group::parse("org.source").unwrap(),
            "declared",
            &"1.0.0".parse().unwrap(),
        );
        assert_eq!(fs::read(slot.join(&relative)).unwrap(), body);
        assert!(!slot.join(&relative).with_extension("xml").exists());
        for rel in [".vibe/facts-sentinel", "vibevm/vibedeps/retained/payload"] {
            assert_eq!(
                fs::read(project.path().join(rel)).unwrap(),
                b"pre-existing bytes"
            );
        }
        let queried = user
            .vibe()
            .args(["--json", "requirements", "--path"])
            .arg(project.path())
            .output()
            .unwrap();
        assert!(queried.status.success(), "{queried:?}");
        let report: serde_json::Value = serde_json::from_slice(&queried.stdout).unwrap();
        assert!(
            !report["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["source"]["package"] == "org.source/declared")
        );
        assert!(
            report["sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["source"]["package"] == "org.source/declared"
                    && row["state"] == "available"),
            "{report}"
        );
    }
}
