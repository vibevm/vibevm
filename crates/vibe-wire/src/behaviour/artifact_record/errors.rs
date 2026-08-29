//! The typed refusals of the artifact record's scalar laws. The same
//! refusal discipline the trace cells carry: a record is read from
//! disk, so no variant here clones a wire string — every untrusted
//! scalar rides a bounded [`ScalarPreview`] (shared with the trace
//! index cell, one type, not a second preview), and every member name
//! is bounded by construction.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::behaviour::scalars::{ProviderKeyDefect, RelativePathDefect};

/// One broken scalar law, with the context needed to name the offender.
/// Typed end to end — no stringly `detail` — so a test can assert the
/// exact family a mutation lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactRecordError {
    /// `schema` is not this validator's epoch — a newer record must
    /// fail loudly, not parse into a wrong meaning.
    SchemaEpoch { found: u32 },
    /// `id` is not a portable token — the mechanism plane's one id grammar.
    IdNotPortableToken { id: ScalarPreview },
    /// `path_absolute` is not an absolute forward-slashed machine path.
    UnsafeAbsolutePath {
        path: ScalarPreview,
        reason: AbsolutePathUnsafety,
    },
    /// `path_relative.path` breaks the project-relative path grammar.
    UnsafeRelativePath {
        path: ScalarPreview,
        defect: RelativePathDefect,
    },
    /// `digest.value` is not exactly 64 lowercase hex.
    DigestValueNotHex { value: ScalarPreview },
    /// `producer.target` is not a portable token.
    ProducerTargetNotPortableToken { target: ScalarPreview },
    /// `producer.mechanism` is not a producing role-qualified key.
    BadMechanismKey {
        mechanism: ScalarPreview,
        reason: MechanismDefect,
    },
    /// `producer.provider.key` breaks the ExtensionKey shape.
    BadProviderKey {
        key: ScalarPreview,
        defect: ProviderKeyDefect,
    },
    /// `producer.provider.content_hash` is not `sha256:` + 64 lowercase
    /// hex — the one identity spelling every lockfile row carries.
    BadContentHash { content_hash: ScalarPreview },
    /// A freshness digest is present but not exactly 64 lowercase hex.
    /// `member` names it in wire spelling.
    BadFreshnessDigest {
        member: &'static str,
        value: ScalarPreview,
    },
    /// A free-text member is blank or carries CR, LF or NUL. `field`
    /// names the member in wire spelling.
    UnsafeScalar {
        field: &'static str,
        value: ScalarPreview,
    },
}

/// Why an absolute placement path failed its spelling law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsolutePathUnsafety {
    /// A Windows separator — the wire spelling is forward slashes.
    Backslash,
    /// CR, LF or NUL inside the path.
    ControlByte,
    /// Neither `/…` nor `X:/…`.
    NotAbsolute,
}

/// Why a mechanism key is not a producing role-qualified key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanismDefect {
    /// No `:` at all — the key names no role.
    MissingRolePrefix,
    /// The prefix is not one of the producing families (`build:` /
    /// `package:`).
    UnknownRole,
    /// The tail does not obey the portable-token grammar.
    BadTail,
}

impl std::error::Error for ArtifactRecordError {}

impl std::fmt::Display for ArtifactRecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ArtifactRecordError as E;
        match self {
            E::SchemaEpoch { found } => write!(
                f,
                "schema = {found} is not the artifact-record epoch {epoch}",
                epoch = super::RECORD_EPOCH
            ),
            E::IdNotPortableToken { id } => write!(
                f,
                "id {id} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::UnsafeAbsolutePath { path, reason } => match reason {
                AbsolutePathUnsafety::Backslash => write!(
                    f,
                    "path_absolute {path} contains a backslash; the wire spelling is forward \
                     slashes"
                ),
                AbsolutePathUnsafety::ControlByte => {
                    write!(f, "path_absolute {path} carries CR, LF or NUL")
                }
                AbsolutePathUnsafety::NotAbsolute => write!(
                    f,
                    "path_absolute {path} is not an absolute forward-slashed path (`/…` or \
                     `X:/…`)"
                ),
            },
            E::UnsafeRelativePath { path, defect } => {
                write!(f, "path_relative.path {path} {}", defect.phrase())
            }
            E::DigestValueNotHex { value } => write!(
                f,
                "digest.value {value} is not exactly 64 lowercase hex characters"
            ),
            E::ProducerTargetNotPortableToken { target } => write!(
                f,
                "producer.target {target} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::BadMechanismKey { mechanism, reason } => match reason {
                MechanismDefect::MissingRolePrefix => write!(
                    f,
                    "producer.mechanism {mechanism} names no role; a mechanism key is \
                     `<role>:<id>`"
                ),
                MechanismDefect::UnknownRole => write!(
                    f,
                    "producer.mechanism {mechanism} carries a role that produces no artifact \
                     record; the producing families are `build:` and `package:`"
                ),
                MechanismDefect::BadTail => write!(
                    f,
                    "producer.mechanism {mechanism} carries a tail that does not obey the \
                     portable-token grammar"
                ),
            },
            E::BadProviderKey { key, defect } => {
                write!(
                    f,
                    "producer.provider.key {key} {} (the spelling is `group/name#id`)",
                    defect.phrase()
                )
            }
            E::BadContentHash { content_hash } => write!(
                f,
                "producer.provider.content_hash {content_hash} is not `sha256:` followed by 64 \
                 lowercase hex"
            ),
            E::BadFreshnessDigest { member, value } => write!(
                f,
                "{member} {value} is not exactly 64 lowercase hex characters"
            ),
            E::UnsafeScalar { field, value } => write!(
                f,
                "{field} {value} is empty, whitespace-only or carries CR, LF or NUL"
            ),
        }
    }
}
