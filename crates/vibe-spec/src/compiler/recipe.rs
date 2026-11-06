//! Semantic recipe of reusable compiler artifacts, independent of a package's
//! graph fingerprint and of the native FFI ABI. Unstamped artifacts predate the
//! checked normal coverage and typed generated embed pipeline.

/// Current recipe required when reusing a previously compiled package unit.
/// The generated INDEX comment carrying it is cache metadata, not authored TOML.
pub const COMPILER_ARTIFACT_RECIPE: &str = "compiler-artifact/2";
