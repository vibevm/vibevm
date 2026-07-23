//! PROP-030 §3.3 — discovery of the project-local `packages/` directory.
//!
//! The vibe-embedded registry (§2) derives from a *source-installed vibe*'s
//! `source_path` — it is a property of the running tool, carried ambient to
//! every project. The project-local registry (§3.3) is the parallel concept
//! for the *current project*: `<project_root>/packages/` is opened as a
//! `LocalRegistry` and composed into the local family alongside vibe-embedded,
//! ordered project-local first so a developer's own in-tree packages win a
//! clash.
//!
//! Unlike vibe-embedded discovery, this is **not** gated on the running vibe
//! being source-installed, and **not** CI-suppressed: it is per-project and
//! portable. A `cargo run`, a test binary, a distribution install, and a
//! source install all discover the same `<project_root>/packages/` — every
//! checkout of the project carries the same tree, so a lock entry resolved
//! from here is reproducible across machines (PROP-030 §5).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-030#project-local");

use std::path::{Path, PathBuf};

/// `<project_root>/packages/`, if that directory exists. `None` otherwise
/// (the project has no in-tree packages and the feature is inert for it).
///
/// `project_root` is the directory carrying the project's `vibe.toml` — the
/// caller resolves it via [`super::resolve_project_root`].
///
/// [`super::resolve_project_root`]: super::resolve_project_root
pub(crate) fn project_packages_root(project_root: &Path) -> Option<PathBuf> {
    let packages = project_root.join("packages");
    packages.is_dir().then_some(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_dir_present_resolves_to_it() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("packages")).unwrap();
        assert_eq!(
            project_packages_root(tmp.path()),
            Some(tmp.path().join("packages"))
        );
    }

    #[test]
    fn no_packages_dir_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(project_packages_root(tmp.path()), None);
    }

    #[test]
    fn a_packages_file_not_a_dir_is_none() {
        // `is_dir()` not `exists()` — a stray `packages` file does not
        // masquerade as a registry root.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("packages"), "not a dir").unwrap();
        assert_eq!(project_packages_root(tmp.path()), None);
    }
}
