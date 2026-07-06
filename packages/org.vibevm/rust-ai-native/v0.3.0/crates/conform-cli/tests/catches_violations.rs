//! Proves the *shipped* conform engine catches a discipline violation and
//! passes a clean tree — the property the whole PROP-024 relocation exists to
//! preserve: an installed `conform` is a working checker, not a description of
//! one. Drives the real `run_check` over throwaway fixtures, so it guards the
//! end-to-end gate (config load → fact extraction → rules → baseline diff →
//! non-zero on a new finding), not just the rule units.

use std::path::Path;

use tempfile::{TempDir, tempdir};

/// A seam whose domain code unwraps — a `no-unwrap-in-domain` finding. The
/// doctest fence keeps `seam-has-doctest` satisfied, so the unwrap is the
/// only finding (a clean, single-rule signal).
const DIRTY: &str = r#"/// A seam with a domain unwrap.
///
/// ```
/// let _ = myapp::parse("3");
/// ```
pub fn parse(s: &str) -> i32 {
    s.parse::<i32>().unwrap()
}
"#;

/// The same seam returning through `Option` — trips no rule.
const CLEAN: &str = r#"/// A seam that returns through Option.
///
/// ```
/// let _ = myapp::parse("3");
/// ```
pub fn parse(s: &str) -> Option<i32> {
    s.parse::<i32>().ok()
}
"#;

/// A minimal project: a `conform.toml` gating one crate `myapp`, an empty
/// baseline, and `crates/myapp/src/lib.rs` carrying `lib_body`.
fn fixture(lib_body: &str) -> TempDir {
    let dir = tempdir().expect("tempdir");
    write(
        dir.path(),
        "conform.toml",
        "roots = [\"crates/*\"]\ngated_crates = [\"myapp\"]\n",
    );
    write(
        dir.path(),
        "conform-baseline.json",
        "{\"schema\":1,\"findings\":[]}\n",
    );
    write(dir.path(), "crates/myapp/src/lib.rs", lib_body);
    dir
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

#[test]
fn catches_a_no_unwrap_violation() {
    let dir = fixture(DIRTY);
    let err = conform_cli::run_check(dir.path(), "conform-baseline.json", None)
        .expect_err("a domain unwrap in a gated crate must fail the gate");
    assert!(
        err.to_string().contains("new finding"),
        "expected a new-finding failure, got: {err}"
    );
}

#[test]
fn passes_a_clean_gated_crate() {
    let dir = fixture(CLEAN);
    conform_cli::run_check(dir.path(), "conform-baseline.json", None)
        .expect("a clean gated crate must pass the gate");
}
