//! The typed error for layer persistence (PROP-040 §6 `#diff-from-default`, §3
//! `#role-marker`). See [`PersistError`].

use std::path::PathBuf;

/// Why persisting a settings layer failed (PROP-040 §6 `#diff-from-default`,
/// §3 `#role-marker`). Carries the file path on the I/O and edit variants so a
/// command-edge diagnostic can name the layer file involved; each `Display`
/// cites the governing REQ anchor.
///
/// ```
/// use vibe_settings::loader::Layer;
/// use vibe_settings::persist::PersistError;
///
/// let e = PersistError::Io {
///     path: "/repo/.vibe/settings.toml".into(),
///     source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
/// };
/// assert!(e.to_string().contains("settings.toml"));
/// assert!(e.to_string().contains("diff-from-default"));
/// # let _: Layer = Layer::L2;
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default")]
pub enum PersistError {
    /// A read/write/create_dir operation on a layer file failed.
    #[error(
        "could not read or write settings file `{path}`: {source} \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default; \
          fix: check the path and permissions, or remove the file to fall back to defaults)"
    )]
    Io {
        /// The file path involved (may be the parent dir on create_dir, or the
        /// staged temp file on a failed write).
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The layer table could not be serialised to TOML.
    #[error(
        "could not serialise the settings table: {source} \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
    )]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },

    /// The freshly-rendered body could not be parsed back as an editable
    /// document — an internal invariant violation (the `toml` serialiser
    /// produced something `toml_edit` cannot round-trip).
    #[error(
        "could not build an editable document for `{path}`: {source} \
         (violates spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
    )]
    Edit {
        /// The layer file the document was being staged for.
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
}
