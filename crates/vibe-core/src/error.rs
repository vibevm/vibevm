//! Error types for `vibe-core`.
//!
//! Parsing, validation, and I/O errors surfaced from this crate. Concrete
//! operational errors (e.g. network, git) live in the crates that perform them.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#package-identity");
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The crate's error type — one `thiserror` enum for the parse, validate,
/// and I/O layer of `vibe-core`. Every variant's `Display` embeds the
/// `spec://` REQ it guards plus a fix hint, so a failing run is navigable
/// back to the requirement without source access.
///
/// ```
/// use vibe_core::Error;
///
/// let e = Error::BadPackageKind("xml".into());
/// let msg = e.to_string();
/// assert!(msg.contains("must be one of: flow, feat, stack, tool, mcp"));
/// assert!(msg.contains("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#four-installable-kinds"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#package-identity")]
pub enum Error {
    #[error(
        "invalid package reference `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: write the reference as `[kind:][group/]name[@version]`)"
    )]
    BadPackageRef { input: String, reason: String },

    #[error(
        "invalid package kind `{0}` — must be one of: flow, feat, stack, tool, mcp \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#four-installable-kinds; \
          fix: use one of the installable kinds)"
    )]
    BadPackageKind(String),

    #[error(
        "invalid package name `{0}` — must be kebab-case (lowercase letters, digits, \
         and internal hyphens only) \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: rename to kebab-case)"
    )]
    BadPackageName(String),

    #[error(
        "invalid package group `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: use a reverse-FQDN group like `org.vibevm`)"
    )]
    BadGroup { input: String, reason: String },

    #[error(
        "invalid capability reference `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#capability; \
          fix: write the capability as `interface:<name>[@version]`)"
    )]
    BadCapabilityRef { input: String, reason: String },

    #[error(
        "invalid content hash `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity; \
          fix: use a `sha256:<hex>` digest as produced by the indexer)"
    )]
    BadContentHash { input: String, reason: String },

    #[error(
        "invalid version spec `{input}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: use a Cargo-style requirement such as `^1.2`, `~1.2.3`, or `=1.2.3`)"
    )]
    BadVersionSpec {
        input: String,
        #[source]
        source: semver::Error,
    },

    #[error(
        "invalid dependency declaration for `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source; \
          fix: correct the [requires] entry in vibe.toml)"
    )]
    BadDependencyDecl { input: String, reason: String },

    #[error(
        "invalid `when` condition `{input}`: {reason} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: correct the `when` predicate on the dependency)"
    )]
    BadWhenCondition { input: String, reason: String },

    #[error(
        "invalid manifest: {reason} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: correct vibe.toml against the schema)"
    )]
    InvalidManifest { reason: String },

    #[error(
        "unsupported vibe.lock schema version {found} — expected {expected} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#lockfile-schema; \
          fix: regenerate with `vibe install`)"
    )]
    UnsupportedLockfile { found: u32, expected: u32 },

    #[error(
        "failed to read file at {path} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#directory-layout; \
          fix: check the path exists and is readable)"
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "failed to write file at {path} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#directory-layout; \
          fix: check the parent directory exists and is writable)"
    )]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "failed to parse `{path}`: {detail} \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: {fix})"
    )]
    ParseToml {
        path: PathBuf,
        /// The deserialiser's own diagnosis, rendered verbatim — the field
        /// name for a missing key (`missing field \`group\``), or the parser's
        /// `TOML parse error at line N, column M` plus caret for broken
        /// syntax. Surfacing this is the whole point: the `#[source]` it sits
        /// next to is never printed by `Display`, so before this field existed
        /// the operator saw neither the field name nor the position.
        detail: String,
        /// A remedy matched to the failure kind — not the generic "repair the
        /// syntax" that used to fire even when only a key was missing. See
        /// [`Error::parse_toml`] for how the kind is told apart.
        fix: String,
        // Boxed to keep the variant (and so every `Result<_, Error>`) under
        // clippy's `result_large_err` threshold: `toml::de::Error` is ~96 B on
        // its own, and `detail`/`fix` already push this variant past 128 B. The
        // chain is preserved — `Box<toml::de::Error>` still implements `Error`.
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error(
        "failed to serialize TOML \
         (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: act on the wrapped serializer error)"
    )]
    SerializeToml(#[from] toml::ser::Error),
}

impl Error {
    /// Wrap a `toml::de::Error` so its own diagnosis reaches the reader and
    /// the remedy matches the failure kind.
    ///
    /// `toml::de::Error` exposes no kind discriminator — no enum variant, no
    /// `is_*` predicate; its public surface is `message()` and `span()`. That
    /// is not enough to tell the two failures an operator actually hits — a
    /// missing required key versus genuinely broken TOML syntax — apart
    /// structurally. `span()` is `None` for a missing field, but also `None`
    /// for other deserialisation errors (integer overflow, for one), so it
    /// would mislabel those as "missing field"; and it is `Some` for both
    /// syntax errors and type/value rejections, so it cannot name the kind
    /// either. The one reliable signal is the message text: `missing field
    /// \`X\`` is serde's own `missing_field` contract, which toml does not
    /// override, so it is stable across toml versions. Everything else is a
    /// rejection the parser already describes — and, for real syntax errors,
    /// points at with a line/column — in its own `Display`, which we render
    /// verbatim into `detail`.
    pub(crate) fn parse_toml(path: PathBuf, source: toml::de::Error) -> Self {
        let detail = source.to_string();
        let detail = detail.trim_end();
        let missing_field = source.message().contains("missing field");
        let fix = if missing_field {
            "add the missing field to vibe.toml"
        } else {
            "repair the TOML at the location reported above"
        };
        Error::ParseToml {
            path,
            detail: detail.to_string(),
            fix: fix.to_string(),
            source: Box::new(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::manifest::Manifest;

    #[test]
    fn missing_required_field_names_it_and_avoids_syntax_advice() {
        // `[package]` without the required `group`: syntactically valid TOML,
        // missing a key. The diagnosis must name the field and must NOT tell
        // the operator to repair TOML syntax — the exact bug this fixes.
        let err = Manifest::parse_str(
            "[package]\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("missing field"), "must name the field: {msg}");
        assert!(msg.contains("group"), "must name `group`: {msg}");
        assert!(
            !msg.to_lowercase().contains("repair the toml syntax"),
            "a missing field is not a syntax error; got: {msg}"
        );
        assert!(
            msg.contains("add the missing field"),
            "remedy must be to add the field: {msg}"
        );
        assert!(
            msg.contains("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema"),
            "REQ citation must survive: {msg}"
        );
    }

    #[test]
    fn broken_syntax_carries_the_parser_position() {
        // `this is = not = toml` is malformed TOML. The parser's own framing
        // — `TOML parse error at line N, column M` — must now reach the
        // reader; the bare `#[source]` used to swallow it whole.
        let err = Manifest::parse_str("this is = not = toml\n").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("TOML parse error"),
            "a syntax error must surface the parser's framing: {msg}"
        );
        assert!(
            msg.contains("line") && msg.contains("column"),
            "a syntax error must carry a position: {msg}"
        );
        assert!(
            msg.contains("repair the TOML"),
            "remedy must speak to repairing the TOML: {msg}"
        );
        assert!(
            msg.contains("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema"),
            "REQ citation must survive: {msg}"
        );
    }

    #[test]
    fn valid_manifest_still_parses_cleanly() {
        // The classification only fires on failure; the happy path is
        // unchanged. (Broad unchanged-ness for valid manifests is also covered
        // by the manifest document suite.)
        let m = Manifest::parse_str("[project]\nname = \"demo\"\nversion = \"0.0.1\"\n").unwrap();
        assert_eq!(m.require_project().unwrap().name, "demo");
    }

    #[test]
    fn req_citation_present_in_every_parse_variant_via_constructor() {
        // Regression guard for the `error-message-cites-req` gate: whichever
        // bucket the constructor picks, the REQ citation is in the message.
        let missing = Error::parse_toml(
            "vibe.toml".into(),
            toml::from_str::<Manifest>(
                "[package]\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
            )
            .unwrap_err(),
        )
        .to_string();
        let broken = Error::parse_toml(
            "vibe.toml".into(),
            toml::from_str::<Manifest>("this is = not = toml\n").unwrap_err(),
        )
        .to_string();
        let req = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema";
        assert!(
            missing.contains(req),
            "missing-field msg lost REQ: {missing}"
        );
        assert!(broken.contains(req), "syntax-error msg lost REQ: {broken}");
    }
}
