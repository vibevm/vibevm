//! Build script: derive the language pin for the vvm tool checks from the
//! workspace manifest instead of hard-coding it (tool-design-lessons S6 —
//! related knowledge follows the single-constant rule).
//!
//! `CARGO_PKG_RUST_VERSION` is the inherited `[workspace.package]
//! rust-version` (this crate sets `rust-version.workspace = true`), so the
//! workspace manifest is the single source; this script only normalises the
//! cargo form (`X.Y` allowed) to the full `X.Y.Z` semver the tool table
//! compares against.

fn main() {
    let raw = std::env::var("CARGO_PKG_RUST_VERSION")
        .expect("CARGO_PKG_RUST_VERSION is set by cargo for every build script");
    let msrv = match raw.matches('.').count() {
        0 => format!("{raw}.0.0"),
        1 => format!("{raw}.0"),
        _ => raw.clone(),
    };
    println!("cargo:rustc-env=VIBE_MSRV={msrv}");
    // Re-run when the inherited value changes (cargo tracks the env var
    // itself, but the manifest is the human-visible trigger).
    println!("cargo:rerun-if-changed=../../Cargo.toml");
}
