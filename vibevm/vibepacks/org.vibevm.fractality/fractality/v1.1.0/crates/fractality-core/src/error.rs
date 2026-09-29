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

    /// The profiles file declares a schema this build does not speak (D6).
    #[error(
        "profiles schema {found} is not supported: this build speaks schema 1 (violates spec://fractality/PROP-001#architecture; fix: update profiles.toml or the fractality binaries)"
    )]
    ProfilesSchema { found: u32 },

    /// No profiles file at the expected location.
    #[error(
        "profiles file `{path}` does not exist (violates spec://fractality/PROP-001#architecture; fix: create it with a [profile.glm] entry — base_url, token_file, [profile.glm.models]; see spec/examples/profiles.sample.toml)"
    )]
    ProfilesMissing { path: camino::Utf8PathBuf },

    /// The profiles file exists but cannot be read.
    #[error(
        "profiles file `{path}` is unreadable: {message} (violates spec://fractality/PROP-001#architecture; fix: check permissions and encoding)"
    )]
    ProfilesIo {
        path: camino::Utf8PathBuf,
        message: String,
    },

    /// A packet routed to a profile that is not defined.
    #[error(
        "profile `{name}` is not defined (available: {available}) (violates spec://fractality/PROP-001#architecture; fix: add it to profiles.toml or route the packet to an existing profile)"
    )]
    ProfileUnknown { name: String, available: String },

    /// A required profile field is empty.
    #[error(
        "profile field `{profile}.{field}` must not be empty (violates spec://fractality/PROP-001#architecture; fix: {hint})"
    )]
    ProfileField {
        profile: String,
        field: &'static str,
        hint: &'static str,
    },

    /// A packet's routing slot is not part of the D7 vocabulary.
    #[error(
        "model slot `{slot}` is not a routing slot (violates spec://fractality/PROP-001#model; fix: use `big` or `small` in [routing].model)"
    )]
    ModelSlot { slot: String },
}
