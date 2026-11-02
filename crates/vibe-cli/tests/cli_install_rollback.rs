//! A failed install leaves the project as it was (BACKLOG B-206).
//!
//! The walk this pins: install a package with a static boot lane under
//! `spec_format = "xml"`, then install a second package whose static lane
//! cannot compile. The second install used to leave its slot in `vibedeps/`,
//! `vibe.toml` naming it, `vibe.lock` unwritten and the error naming
//! `INLINE.md`. Now it refuses naming `STATIC.xml`, and the project tree —
//! manifest, lock, dependency slots and boot lane — is byte-identical to the
//! tree before the command. The explicit install rewrites `vibe.toml` first,
//! so the manifest is part of what the failure must put back.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use common::UserScratch;

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// Write `org.example/<name>@1.0.0` in the monorepo registry shape with the
/// given manifest tail and one boot document.
fn write_package(registry: &Path, name: &str, manifest_tail: &str, boot: &str) {
    let dir = registry.join("org.example").join(name).join("v1.0.0");
    let boot_path = dir.join(common::boot_rel(&format!("{name}.md")));
    fs::create_dir_all(boot_path.parent().unwrap()).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"org.example\"\nname = \"{name}\"\nkind = \"flow\"\n\
             version = \"1.0.0\"\n{manifest_tail}\n[boot_snippet]\nsource = \"{}\"\n",
            common::boot_rel(&format!("{name}.md"))
        ),
    )
    .unwrap();
    fs::write(boot_path, boot).unwrap();
}

/// Every project file and directory except `.vibe/`, where the lifecycle
/// engine journals each run — a failed run included, which is a record the
/// operator needs, not a change the install made to the project.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if rel == ".vibe" {
                continue;
            }
            if path.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Only the paths and the first differing one, so a failure reads as a diff
/// rather than two byte dumps.
fn describe_difference(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> String {
    let changed: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    format!("paths that differ: {changed:?}")
}

#[test]
fn a_failed_static_compile_leaves_manifest_lock_slots_and_boot_untouched() {
    let user = UserScratch::new();
    let outer = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let registry = outer.path().join("registry");
    write_package(&registry, "kept", "", "# Kept\n\nkept rule.\n");
    write_package(
        &registry,
        "broken",
        "format = \"normal\"\n",
        "# Broken {#root}\n#use spec://org.example/ghost/boot/base#root\nBROKEN\n",
    );
    // The wrapper pulls `broken` statically, so `broken`'s lane is compiled
    // although the project's own edge to the wrapper is the default one.
    write_package(
        &registry,
        "wrapper",
        "\n[requires.packages]\n\"org.example/broken\" = { version = \"^1.0\", link = \"static\" }\n",
        "# Wrapper\n\nwrapper rule.\n",
    );
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"machine\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    user.init_project(project.path());
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        manifest.replacen("[project]\n", "[project]\nspec_format = \"xml\"\n", 1),
    )
    .unwrap();
    user.vibe()
        .args(["install", "org.example/kept", "--assume-yes", "--path"])
        .arg(project.path())
        .assert()
        .success();
    assert!(project.path().join("vibe.lock").is_file());

    let before = snapshot(project.path());
    let output = user
        .vibe()
        .args(["install", "org.example/wrapper", "--assume-yes", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "the install refuses\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let all = format!("{stdout}{stderr}");
    assert!(
        all.contains("STATIC.xml"),
        "the refusal names the lane: {all}"
    );
    assert!(!all.contains("INLINE"), "never the retired name: {all}");
    let after = snapshot(project.path());
    assert!(
        after == before,
        "the project is as it was before the command; {}",
        describe_difference(&before, &after)
    );
}
