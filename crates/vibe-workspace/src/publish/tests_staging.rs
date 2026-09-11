//! Staging oracles, split from the publish selection/topology tests.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#selective-publish");

use super::*;

#[cfg(test)]
fn run_git(cwd: &Path, args: &[&str]) -> std::process::Output {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed in {}: {}",
        args.join(" "),
        cwd.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn stage_node_writes_origin_section() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let source_spec = package_path("a").join(crate::layout_paths::specs_path("X.md"));
    write(tmp.path(), source_spec, "spec content");
    let staged = stage_package(tmp.path(), "a", &origin_info());
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    let origin = manifest.origin.as_ref().expect("origin written");
    assert_eq!(origin.upstream, "https://github.com/you/monorepo");
    assert_eq!(origin.path, package_rel("a"));
    assert_eq!(origin.commit.as_deref(), Some("abc123def456"));
    assert_eq!(origin.generated_by, "vibe 0.1.0");
    assert_eq!(origin.generated_at, "2026-05-21T00:00:00Z");
    assert!(
        staged
            .staging
            .path()
            .join(crate::layout_paths::specs_path("X.md"))
            .is_file()
    );
}

#[test]
fn stage_node_excludes_git_and_vibe_dirs() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("a/.git/HEAD"),
        "ref: refs/heads/main",
    );
    write(tmp.path(), package_path("a/.git/objects/x"), "obj");
    write(tmp.path(), package_path("a/.vibe/cache.bin"), "cache");
    write(tmp.path(), package_path("a/keep.md"), "keep me");
    let staged = stage_package(tmp.path(), "a", &origin_info());
    assert!(!staged.staging.path().join(".git").exists());
    assert!(!staged.staging.path().join(".vibe").exists());
    assert!(staged.staging.path().join("keep.md").is_file());
}

#[test]
fn stage_node_flattens_clean_submodule_without_dangling_git_metadata() {
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| !output.status.success())
        .unwrap_or(true)
    {
        eprintln!("skipping: git not on PATH");
        return;
    }

    let upstream = TempDir::new().unwrap();
    run_git(upstream.path(), &["init", "--initial-branch=main"]);
    run_git(
        upstream.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    run_git(upstream.path(), &["config", "user.name", "test"]);
    write(upstream.path(), "UPSTREAM.md", "pinned upstream bytes\n");
    run_git(upstream.path(), &["add", "-A"]);
    run_git(upstream.path(), &["commit", "-m", "upstream"]);
    let upstream_commit =
        String::from_utf8_lossy(&run_git(upstream.path(), &["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_string();

    let source = TempDir::new().unwrap();
    run_git(source.path(), &["init", "--initial-branch=main"]);
    run_git(
        source.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    run_git(source.path(), &["config", "user.name", "test"]);
    write(source.path(), "vibe.toml", &package("bridge", "feat"));
    run_git(source.path(), &["add", "-A"]);
    run_git(source.path(), &["commit", "-m", "bridge"]);
    let upstream_path = upstream.path().to_string_lossy();
    run_git(
        source.path(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            upstream_path.as_ref(),
            "vendor/upstream",
        ],
    );
    run_git(source.path(), &["commit", "-am", "pin upstream"]);

    let staged = stage_node(source.path(), ".", &origin_info()).unwrap();
    assert_eq!(
        staged.submodules,
        [SubmoduleProvenance {
            path: "vendor/upstream".to_string(),
            commit: upstream_commit,
        }]
    );
    assert!(
        staged
            .staging
            .path()
            .join("vendor/upstream/UPSTREAM.md")
            .is_file(),
        "populated submodule bytes must ship"
    );
    assert!(!staged.staging.path().join(".gitmodules").exists());
    assert!(!staged.staging.path().join("vendor/upstream/.git").exists());

    // A fresh index sees ordinary files, never a 160000 gitlink.
    run_git(staged.staging.path(), &["init", "--initial-branch=main"]);
    run_git(staged.staging.path(), &["add", "-A"]);
    let index =
        String::from_utf8_lossy(&run_git(staged.staging.path(), &["ls-files", "--stage"]).stdout)
            .into_owned();
    assert!(
        !index.lines().any(|line| line.starts_with("160000 ")),
        "{index}"
    );
}

#[test]
fn stage_node_prepends_readme_banner() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    write(
        tmp.path(),
        package_path("a/README.md"),
        "# Original readme\n",
    );
    let staged = stage_package(tmp.path(), "a", &origin_info());
    let readme = fs::read_to_string(staged.staging.path().join("README.md")).unwrap();
    assert!(readme.contains("Generated copy — do not contribute here"));
    assert!(readme.contains("https://github.com/you/monorepo"));
    assert!(readme.contains("# Original readme"));
    assert!(readme.starts_with("<!-- vibevm:generated-copy -->"));
}

#[test]
fn stage_node_creates_readme_when_absent() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let staged = stage_package(tmp.path(), "a", &origin_info());
    let readme_path = staged.staging.path().join("README.md");
    assert!(readme_path.is_file());
    let readme = fs::read_to_string(&readme_path).unwrap();
    assert!(readme.contains("Generated copy — do not contribute here"));
}

#[test]
fn stage_node_writes_pr_template() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let staged = stage_package(tmp.path(), "a", &origin_info());
    let pr_template = fs::read_to_string(
        staged
            .staging
            .path()
            .join(".github/PULL_REQUEST_TEMPLATE.md"),
    )
    .unwrap();
    assert!(pr_template.contains("does not accept pull requests"));
    assert!(pr_template.contains("https://github.com/you/monorepo"));
    assert!(pr_template.contains("org.vibevm/a"));
}

#[test]
fn stage_node_sets_generated_copy_description() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let staged = stage_package(tmp.path(), "a", &origin_info());
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    let desc = manifest
        .package
        .as_ref()
        .and_then(|p| p.description.clone())
        .expect("description set");
    assert!(desc.contains("Generated copy of `org.vibevm/a`"));
    assert!(desc.contains("https://github.com/you/monorepo"));
}

#[test]
fn stage_node_omits_commit_when_none() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        package_path("a/vibe.toml"),
        &package("a", "flow"),
    );
    let mut info = origin_info();
    info.commit = None;
    let staged = stage_package(tmp.path(), "a", &info);
    let manifest = Manifest::read(staged.staging.path().join("vibe.toml")).unwrap();
    assert!(manifest.origin.as_ref().unwrap().commit.is_none());
}

#[test]
fn stage_node_regenerates_boot_for_the_published_shape() {
    let tmp = TempDir::new().unwrap();
    let package_root = package_path("a");
    let package_boot = package_root.join(vibe_core::layout::current_boot_dir());
    write(
        tmp.path(),
        package_root.join("vibe.toml"),
        &package("a", "flow"),
    );
    write(tmp.path(), package_boot.join("00-core.md"), "# core");
    write(
        tmp.path(),
        package_root.join(vibe_core::layout::current_boot_index()),
        &format!(
            "schema = 1\n\n[[entry]]\npath = \"{}\"\nkind = \"static\"\n",
            crate::layout_paths::vibedeps("org.vibevm.dep/1.0.0/boot/dep.md")
        ),
    );
    write(
        tmp.path(),
        package_root.join(vibe_core::layout::current_boot_static_md()),
        "stale inline lane",
    );
    write(
        tmp.path(),
        package_root.join(vibe_core::layout::current_boot_static_xml()),
        "stale XML inline lane",
    );
    write(
        tmp.path(),
        package_root.join("CLAUDE.md"),
        "stale dev redirect",
    );

    let staged = stage_package(tmp.path(), "a", &origin_info());
    let index = fs::read_to_string(
        staged
            .staging
            .path()
            .join(vibe_core::layout::current_boot_index()),
    )
    .unwrap();
    let deps_prefix = format!("{}/", crate::layout_paths::vibedeps(""));
    assert!(!index.contains(&deps_prefix), "{index}");
    assert!(
        index.contains(&crate::layout_paths::boot("00-core.md")),
        "{index}"
    );
    assert!(
        !staged
            .staging
            .path()
            .join(vibe_core::layout::current_boot_static_md())
            .exists()
    );
    assert!(
        !staged
            .staging
            .path()
            .join(vibe_core::layout::current_boot_static_xml())
            .exists()
    );
    let claude = fs::read_to_string(staged.staging.path().join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("Generated by vibe")
            && claude.contains(&crate::layout_paths::boot(vibe_core::layout::INDEX_MD)),
        "{claude}"
    );
}
