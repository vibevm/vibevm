//! Typed errors for the settings loaders (PROP-040 §3 `#missing-is-default`).
//!
//! A missing file is **never** an error — [`crate::loader::load_layer`] maps a
//! missing file to an empty table; this type is the typed shape for a file that
//! is present but **unreadable** ([`SettingsError::Io`]) or **unparseable**
//! ([`SettingsError::Parse`]). PROP-040 §3 `#missing-is-default` mandates these
//! be **non-fatal diagnostics** (the layer is treated as absent), and each
//! variant's `Display` cites that REQ so a surface diagnostic can point the
//! reader at the contract clause.
//!
//! Spec: [PROP-040 §3](../../../../spec/modules/vibe-settings/PROP-040-settings.md#locations).

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#missing-is-default");

use std::path::PathBuf;

use crate::loader::Layer;

/// Why loading a present settings layer failed. A **missing** file is not here
/// — it is an empty table (PROP-040 §3 `#missing-is-default`); this error is
/// the non-fatal diagnostic for an unreadable or malformed file, carried so the
/// resolver can report it and then treat the layer as absent.
///
/// The layer tag ([`Layer`]) on each variant is filled by the path classifier
/// (§9 `#path-classifier`) inside [`crate::loader::load_layer`] — the caller
/// does not have to declare which layer failed.
///
/// ```
/// use vibe_settings::error::SettingsError;
/// use vibe_settings::loader::Layer;
///
/// let e = SettingsError::Io {
///     layer: Layer::L2,
///     path: "/repo/.vibe/settings.toml".into(),
///     source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
/// };
/// // The diagnostic carries the layer tag and the violated REQ anchor.
/// assert!(e.to_string().contains("L2"));
/// assert!(e.to_string().contains("missing-is-default"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#missing-is-default")]
pub enum SettingsError {
    /// The layer file exists but could not be read (permissions, I/O, ...).
    /// Non-fatal — the caller treats the layer as absent.
    #[error(
        "could not read {layer} file `{path}`: {source} \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#missing-is-default; \
          fix: check the file's permissions, or remove it to fall back to defaults)"
    )]
    Io {
        /// The layer the file belongs to (mechanically classified, §9).
        layer: Layer,
        /// The file that could not be read.
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The layer file is present but malformed TOML. Non-fatal — the caller
    /// reports the diagnostic and treats the layer as absent.
    #[error(
        "{layer} file `{path}` is malformed: {source} \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#missing-is-default; \
          fix: repair the TOML at the reported location)"
    )]
    Parse {
        /// The layer the file belongs to (mechanically classified, §9).
        layer: Layer,
        /// The file that failed to parse.
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}
