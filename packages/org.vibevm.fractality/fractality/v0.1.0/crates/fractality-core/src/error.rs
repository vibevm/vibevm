//! Error surface of the core crate.
//!
//! House law (plan D14): messages carry the violated contract (a Decision
//! number or spec anchor) and a fix surface — the reader should know what
//! to change without opening the source.

use specmark::spec;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Errors produced by core-model parsing and validation.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://fractality/PROP-001#model")]
pub enum CoreError {
    /// The packet declares a schema this build does not speak (D7 pins `schema = 1`).
    #[error(
        "packet schema {found} is not supported: this build speaks schema 1 (plan D7) \
         (violates spec://fractality/PROP-001#model; fix: update the packet or the fractality binaries)"
    )]
    PacketSchema { found: u32 },

    /// A required packet field is missing or empty.
    #[error(
        "packet field `{field}` must not be empty: {hint} (violates spec://fractality/PROP-001#model; fix: supply a non-empty value; plan D7)"
    )]
    PacketField {
        field: &'static str,
        hint: &'static str,
    },

    /// A worker spec failed validation before spawn.
    #[error(
        "worker spec is invalid: {message} (violates spec://fractality/PROP-001#model; fix: correct the worker spec; see fractality-core::worker)"
    )]
    WorkerSpec { message: String },

    /// A backend could not turn a packet into a worker spec.
    #[error(
        "backend `{backend}` cannot build a worker spec: {message} (violates spec://fractality/PROP-001#model; fix: correct the backend or the task packet)"
    )]
    Backend {
        backend: &'static str,
        message: String,
    },

    /// TOML input did not parse.
    #[error(
        "TOML does not parse: {0} (violates spec://fractality/PROP-001#model; fix: correct the TOML input)"
    )]
    TomlDe(#[from] toml::de::Error),

    /// TOML output did not serialize (should not happen for well-formed models).
    #[error(
        "TOML serialization failed: {0} (violates spec://fractality/PROP-001#model; fix: report a well-formed model)"
    )]
    TomlSer(#[from] toml::ser::Error),

    /// JSON (de)serialization failed.
    #[error(
        "JSON (de)serialization failed: {0} (violates spec://fractality/PROP-001#model; fix: provide JSON-compliant input)"
    )]
    Json(#[from] serde_json::Error),
}
