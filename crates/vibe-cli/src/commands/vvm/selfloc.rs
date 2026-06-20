//! Deriving the VVM location from the running binary's own path (PROP-019
//! §2.5): a managed `vibe` lives at
//! `…/<base>/opt/vibevm/versions/<kind>/<id>/<instance>/vibe[.exe]`, so it
//! knows its root and home from `current_exe()` — no env var required.

specmark::scope!("spec://vibevm/common/PROP-019#activation");

use std::path::{Path, PathBuf};

/// The VVM location derived from the running binary's own path (PROP-019
/// §2.5).
pub struct SelfLocation {
    /// The VVM root (`<base>/opt`).
    pub root: PathBuf,
    /// The running instance dir — the `$VIBEVM_HOME` truth.
    pub home: PathBuf,
}

/// Derive the VVM location from the running binary's path. `None` when not
/// run from a managed instance (a dev `cargo run`, a bare copy) — callers
/// fall back to the environment (PROP-019 §2.5).
pub fn derive_self(exe: Option<&Path>) -> Option<SelfLocation> {
    // `canonicalize()` on Windows returns a `\\?\` verbatim path; cmd.exe and
    // the shims cannot exec one once it lands in `current`, so strip it and
    // keep every derived path plain (PROP-019 §2.5).
    let exe = strip_verbatim(exe?.canonicalize().ok()?);
    let instance = exe.parent()?;
    let id_dir = instance.parent()?;
    let kind_dir = id_dir.parent()?;
    let versions = kind_dir.parent()?;
    let vibevm = versions.parent()?;
    let root = vibevm.parent()?;
    let shaped = versions.file_name().is_some_and(|n| n == "versions")
        && vibevm.file_name().is_some_and(|n| n == "vibevm")
        && root.file_name().is_some_and(|n| n == "opt");
    shaped.then(|| SelfLocation {
        root: root.to_path_buf(),
        home: instance.to_path_buf(),
    })
}

/// Strip the Windows `\\?\` verbatim prefix (drive-letter form only, e.g.
/// `\\?\C:\…` → `C:\…`) that `canonicalize()` adds; a no-op elsewhere.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    if let Some(rest) = p.to_str().and_then(|s| s.strip_prefix(r"\\?\"))
        && rest.as_bytes().get(1) == Some(&b':')
    {
        return PathBuf::from(rest);
    }
    p
}

/// Whether two paths name the same location, canonicalising when possible.
pub fn same_location(a: impl AsRef<Path>, b: impl AsRef<Path>) -> bool {
    let (a, b) = (a.as_ref(), b.as_ref());
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#activation", r = 1)]
    fn derive_self_parses_a_managed_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let inst = tmp
            .path()
            .join("opt")
            .join("vibevm")
            .join("versions")
            .join("branch")
            .join("main")
            .join("3");
        std::fs::create_dir_all(&inst).unwrap();
        let exe = inst.join("vibe");
        std::fs::write(&exe, b"x").unwrap();

        let loc = derive_self(Some(&exe)).unwrap();
        assert!(same_location(&loc.root, tmp.path().join("opt")));
        assert!(same_location(&loc.home, &inst));

        // A managed `vibe` must derive a PLAIN path — never a `\\?\` verbatim
        // one, which the shims cannot exec (the bug this guards).
        assert!(!loc.root.to_string_lossy().starts_with(r"\\?\"));
        assert!(!loc.home.to_string_lossy().starts_with(r"\\?\"));

        // A non-managed path derives nothing.
        let bare = tmp.path().join("random");
        std::fs::create_dir_all(&bare).unwrap();
        let bare_exe = bare.join("vibe");
        std::fs::write(&bare_exe, b"x").unwrap();
        assert!(derive_self(Some(&bare_exe)).is_none());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#activation", r = 1)]
    fn strip_verbatim_drops_the_windows_prefix() {
        assert_eq!(
            strip_verbatim(PathBuf::from(r"\\?\C:\Users\x\opt")),
            PathBuf::from(r"C:\Users\x\opt")
        );
        // Already-plain and POSIX paths are untouched.
        assert_eq!(
            strip_verbatim(PathBuf::from(r"C:\already\clean")),
            PathBuf::from(r"C:\already\clean")
        );
        assert_eq!(
            strip_verbatim(PathBuf::from("/posix/path")),
            PathBuf::from("/posix/path")
        );
    }
}
