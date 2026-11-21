//! Shared test scaffolding for the install cell's test modules, out-of-line
//! so each stays under the file-length budget. `pub(super)` so every sibling
//! module reaches it;
//! `#[cfg(test)]` so the file-grain conform frontend scopes the `unwrap`s
//! as test code (the out-of-line idiom `install/tests.rs` documents).

use super::*;
use tempfile::TempDir;
use vibe_core::{ContentHash, Group, PackageKind};

#[cfg(test)]
pub(super) fn write(dir: &Path, rel: impl AsRef<Path>, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[cfg(test)]
pub(super) fn boot_rel(tail: impl AsRef<Path>) -> String {
    crate::layout_paths::boot(tail)
}

#[cfg(test)]
pub(super) fn deps_rel(tail: impl AsRef<Path>) -> String {
    crate::layout_paths::vibedeps(tail)
}

#[cfg(test)]
pub(super) fn deps_slot_specs(slot_tail: impl AsRef<Path>, specs_tail: impl AsRef<Path>) -> String {
    crate::layout_paths::slot_specs(deps_rel(slot_tail), specs_tail)
}

#[cfg(test)]
pub(super) fn ver(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

#[cfg(test)]
pub(super) fn source_hash() -> ContentHash {
    ContentHash::parse("sha256:1111111111111111111111111111111111111111111111111111111111111111")
        .unwrap()
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
        source_hash: Some(source_hash()),
        embedded_sources: Vec::new(),
        manifest,
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
        configuration: None,
    };
    (dep, pkg)
}

/// Like [`dep_with_boot`] but with declared `requires` edges (the graph edges
/// for the boot closure). The manifest's own `[requires.packages]` — where the
/// per-edge `link` modes live (PROP-038 §2.2) — must be included in
/// `snippet_toml` for those modes to be read.
#[cfg(test)]
pub(super) fn dep_with_requires(
    name: &str,
    version: &str,
    snippet_toml: &str,
    boot_rel: &str,
    boot_body: &str,
    requires: &[&str],
) -> (ResolvedDep, TempDir) {
    let (mut dep, tmp) = dep_with_boot(name, version, snippet_toml, boot_rel, boot_body);
    dep.requires = requires
        .iter()
        .map(|n| (Group::parse("org.vibevm").unwrap(), (*n).to_string()))
        .collect();
    (dep, tmp)
}

/// A slot lifecycle that counts the pre-install events of one package, so a
/// test can assert that a refusal stopped the pass before any slot event.
#[cfg(test)]
pub(super) struct PreInstallCount {
    package: &'static str,
    seen: std::cell::Cell<usize>,
}

#[cfg(test)]
impl PreInstallCount {
    pub(super) fn of(package: &'static str) -> Self {
        Self {
            package,
            seen: std::cell::Cell::new(0),
        }
    }

    pub(super) fn get(&self) -> usize {
        self.seen.get()
    }
}

#[cfg(test)]
impl SlotLifecycle for PreInstallCount {
    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        if context.name == self.package {
            self.seen.set(self.seen.get() + 1);
        }
        Ok(())
    }

    fn post_install(&self, _context: SlotLifecycleContext<'_>) -> Result<(), String> {
        Ok(())
    }
}
