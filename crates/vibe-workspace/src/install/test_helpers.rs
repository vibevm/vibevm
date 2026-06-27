//! Shared test scaffolding for the install cell's two test modules
//! (`tests` and `tests_hooks`), out-of-line so each stays under the
//! file-length budget. `pub(super)` so both sibling modules reach it;
//! `#[cfg(test)]` so the file-grain conform frontend scopes the `unwrap`s
//! as test code (the out-of-line idiom `install/tests.rs` documents).

use super::*;
use tempfile::TempDir;

#[cfg(test)]
pub(super) fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[cfg(test)]
pub(super) fn ver(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

/// A `ResolvedDep` with a content tree written into a fresh temp dir.
/// The `TempDir` is returned so the caller keeps it alive.
#[cfg(test)]
pub(super) fn dep_with_boot(
    name: &str,
    version: &str,
    snippet_toml: &str,
    boot_rel: &str,
    boot_body: &str,
) -> (ResolvedDep, TempDir) {
    let pkg = TempDir::new().unwrap();
    write(
        pkg.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n\n{snippet_toml}"
        ),
    );
    write(pkg.path(), boot_rel, boot_body);
    let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver(version),
        content_dir: pkg.path().to_path_buf(),
        manifest,
        requires: vec![],
        source_mutable: false,
    };
    (dep, pkg)
}
