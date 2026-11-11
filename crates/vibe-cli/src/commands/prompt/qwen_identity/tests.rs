//! Finding the installed package, the Node to run its code with, and
//! writing vibe's helper — the three machine questions, over fixture trees
//! rather than over this machine's own installation.

use std::fs;

use super::*;

/// A package directory that identifies itself as Qwen Code at `version`.
fn package(dir: &Path, version: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("package.json"),
        format!("{{\n  \"name\": \"{QWEN_PACKAGE}\",\n  \"version\": \"{version}\"\n}}\n"),
    )
    .unwrap();
    dir.to_path_buf()
}

/// A symlinked program is the POSIX global install, so the test is where
/// that layout exists.
#[cfg(unix)]
#[test]
fn the_posix_global_layout_is_found_through_the_programs_own_target() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("node/v24.18.0");
    let installed = package(
        &prefix.join("lib/node_modules/@qwen-code/qwen-code"),
        "0.27.2",
    );
    // npm's `bin/qwen` is a symlink to the package's own entry file.
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::write(installed.join("cli-entry.js"), "#!/usr/bin/env node\n").unwrap();
    let program = prefix.join("bin/qwen");
    std::os::unix::fs::symlink(installed.join("cli-entry.js"), &program).unwrap();

    assert_eq!(
        package_dir(&program, "0.27.2")
            .unwrap()
            .canonicalize()
            .unwrap(),
        installed.canonicalize().unwrap()
    );
    // A version that is not the one the program reported is never sampled:
    // the sentence would come from an installation this launch is not
    // starting.
    let refusal = package_dir(&program, "0.28.0").unwrap_err();
    assert!(refusal.contains("0.28.0"), "{refusal}");
    assert!(refusal.contains("package.json"), "{refusal}");
}

#[test]
fn a_shim_beside_its_own_node_modules_is_found_too() {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("npm");
    let installed = package(&bin.join("node_modules/@qwen-code/qwen-code"), "0.27.2");
    fs::create_dir_all(&bin).unwrap();
    // A `.cmd`/`.ps1` shim is not a symlink into the package, so the
    // program's own directory is where the lookup has to look.
    let program = bin.join("qwen.cmd");
    fs::write(&program, "@node ...\n").unwrap();
    assert_eq!(package_dir(&program, "0.27.2").unwrap(), installed);
}

#[test]
fn the_prefix_layout_is_the_third_candidate() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("usr/local");
    let installed = package(
        &prefix.join("lib/node_modules/@qwen-code/qwen-code"),
        "0.27.2",
    );
    fs::create_dir_all(prefix.join("bin")).unwrap();
    // A wrapper script rather than a symlink: nothing about it points into
    // the package, so only the prefix layout finds it.
    let program = prefix.join("bin/qwen");
    fs::write(&program, "#!/bin/sh\nexec node ...\n").unwrap();
    assert_eq!(
        package_dir(&program, "0.27.2")
            .unwrap()
            .canonicalize()
            .unwrap(),
        installed.canonicalize().unwrap()
    );
}

#[test]
fn a_directory_that_is_not_this_package_is_refused_rather_than_sampled() {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("bin");
    let other = bin.join("node_modules/@qwen-code/qwen-code");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("package.json"),
        "{ \"name\": \"@someone/else\", \"version\": \"0.27.2\" }\n",
    )
    .unwrap();
    fs::create_dir_all(&bin).unwrap();
    let program = bin.join("qwen");
    fs::write(&program, "#!/bin/sh\n").unwrap();
    assert!(package_dir(&program, "0.27.2").is_err());
}

#[test]
fn the_node_beside_the_package_wins_over_the_one_on_path() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("node/v24.18.0");
    let installed = prefix.join("lib/node_modules/@qwen-code/qwen-code");
    fs::create_dir_all(&installed).unwrap();
    // The POSIX global layout: `<prefix>/lib/node_modules/<pkg>` names
    // `<prefix>/bin/node`, which is the interpreter Qwen Code itself runs
    // on — on a machine with several Node versions `PATH` may name another.
    fs::create_dir_all(prefix.join("bin")).unwrap();
    let node = prefix
        .join("bin")
        .join(if cfg!(windows) { "node.exe" } else { "node" });
    fs::write(&node, "").unwrap();
    assert_eq!(node_program(&installed).unwrap(), node);

    // The Windows global layout: `node_modules` directly in the prefix,
    // beside the interpreter.
    let flat = home.path().join("AppData/npm");
    let there = flat.join("node_modules/@qwen-code/qwen-code");
    fs::create_dir_all(&there).unwrap();
    let beside = flat.join(if cfg!(windows) { "node.exe" } else { "node" });
    fs::write(&beside, "").unwrap();
    assert_eq!(node_program(&there).unwrap(), beside);
}

#[test]
fn the_helper_is_written_once_and_named_by_its_own_bytes() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join("agent/qwen");
    let first = write_helper(&dir).unwrap();
    assert_eq!(fs::read_to_string(&first).unwrap(), HELPER);
    assert!(
        first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("carrier-"),
        "{}",
        first.display()
    );
    assert!(
        first
            .extension()
            .is_some_and(|extension| extension == "mjs")
    );
    // Idempotent: a second launch finds the same file and rewrites nothing.
    let before = fs::metadata(&first).unwrap().modified().unwrap();
    let second = write_helper(&dir).unwrap();
    assert_eq!(first, second);
    assert_eq!(fs::metadata(&second).unwrap().modified().unwrap(), before);
    // And nothing else is left behind.
    let names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
}
