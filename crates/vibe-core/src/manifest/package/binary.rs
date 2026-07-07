//! `[[binary]]` — the runnable tools a code-bearing package declares
//! (PROP-025 §2). Declaration only at this layer: building and
//! dispatching live in `vibe-cli`'s `bin` family; `vibe-core` gives the
//! manifest shape and the structural validation `vibe check` runs.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-025#manifest");

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// `[[binary]]` — one shipped tool (PROP-025 §2).
///
/// `name` is the PATH-facing binary name (language-suffixed per the
/// collision convention — `conform-rust`, `conform-typescript`) and MUST
/// match a `[[bin]]` target inside `crate`. `crate` is the
/// package-relative directory of the Cargo package that builds it.
///
/// ```
/// use vibe_core::manifest::BinaryDecl;
///
/// let b: BinaryDecl = toml::from_str(r#"
///     name = "discipline-rust"
///     crate = "crates/discipline-cli"
///     description = "the umbrella discipline tool"
/// "#).unwrap();
/// assert_eq!(b.name, "discipline-rust");
/// assert_eq!(b.crate_dir.to_str(), Some("crates/discipline-cli"));
///
/// // `description` is optional.
/// let bare: BinaryDecl = toml::from_str(
///     "name = \"x\"\ncrate = \"crates/x\"",
/// ).unwrap();
/// assert!(bare.description.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryDecl {
    /// The PATH-facing binary name; unique within the package.
    pub name: String,
    /// Package-relative directory of the Cargo package building it
    /// (TOML key `crate` — a Rust keyword, hence the field rename).
    #[serde(rename = "crate")]
    pub crate_dir: PathBuf,
    /// Optional human description, surfaced by `vibe bin list`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
