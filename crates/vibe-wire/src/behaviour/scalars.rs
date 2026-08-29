//! The wire-scalar grammars the hand-written validation cells share.
//!
//! Three of them were being written twice — once in
//! [`crate::behaviour::verification_evidence`], once in
//! [`crate::behaviour::requirements_report`] — and a fourth
//! (canonical unsigned decimal) already lived inside
//! [`crate::behaviour::compile_trace_report`] as a private helper. Two
//! copies of a grammar are two grammars: they drift on exactly the
//! edge case nobody ported. So the predicates live here, once, and the
//! cells keep their OWN typed errors on top of them — the shared thing
//! is the rule, not the refusal.
//!
//! Nothing here allocates, and nothing here parses into a machine
//! integer: a count on this wire may exceed `u64`, and the whole point
//! of carrying it as a canonical decimal string is that no reader
//! narrows it on the way past.

/// What is wrong with a project-relative forward-slashed path or glob,
/// or `None` when the grammar holds. One enum, both cells: the
/// evidence member's declared patterns and artifact paths and the
/// requirements report's edge files and selected node all answer to
/// the same spelling rule, because they are all «somewhere inside
/// this project, said portably».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePathDefect {
    /// Empty or whitespace-only.
    Blank,
    /// A Windows separator — the wire spelling is forward slashes.
    Backslash,
    /// CR, LF or NUL inside the value.
    ControlByte,
    /// A leading `/` — an absolute root, not a project-relative scope.
    Absolute,
    /// An `X:` drive prefix.
    DriveLetter,
    /// A `..` segment — a scope that leaves the project.
    ParentSegment,
    /// A `.` segment — two spellings of one path, and a reader with no
    /// rule for choosing between them.
    DotSegment,
    /// An empty segment (`a//b`, a trailing `/`) — same objection as
    /// [`RelativePathDefect::DotSegment`], different spelling.
    EmptySegment,
}

impl RelativePathDefect {
    /// The sentence a refusal reads as, after the offending value.
    #[must_use]
    pub fn phrase(self) -> &'static str {
        match self {
            RelativePathDefect::Blank => "is empty or whitespace-only",
            RelativePathDefect::Backslash => {
                "contains a backslash; the wire spelling is forward slashes"
            }
            RelativePathDefect::ControlByte => "carries CR, LF or NUL",
            RelativePathDefect::Absolute => "is an absolute root, not a project-relative path",
            RelativePathDefect::DriveLetter => {
                "carries a drive prefix, not a project-relative path"
            }
            RelativePathDefect::ParentSegment => "carries a `..` segment, so it leaves the project",
            RelativePathDefect::DotSegment => "carries a `.` segment",
            RelativePathDefect::EmptySegment => "carries an empty path segment",
        }
    }
}

/// The first thing wrong with a project-relative forward-slashed path
/// or glob. Order matters only for which refusal a reader sees first;
/// every arm is independently reachable and each has its own RED.
#[must_use]
pub fn relative_path_defect(value: &str) -> Option<RelativePathDefect> {
    if value.trim().is_empty() {
        return Some(RelativePathDefect::Blank);
    }
    if value.contains('\\') {
        return Some(RelativePathDefect::Backslash);
    }
    if has_control_bytes(value) {
        return Some(RelativePathDefect::ControlByte);
    }
    if value.starts_with('/') {
        return Some(RelativePathDefect::Absolute);
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Some(RelativePathDefect::DriveLetter);
    }
    for segment in value.split('/') {
        if segment == ".." {
            return Some(RelativePathDefect::ParentSegment);
        }
        if segment == "." {
            return Some(RelativePathDefect::DotSegment);
        }
        if segment.is_empty() {
            return Some(RelativePathDefect::EmptySegment);
        }
    }
    None
}

/// CR, LF or NUL anywhere in a value a reader will print or join on.
#[must_use]
pub fn has_control_bytes(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0'))
}

/// What is wrong with a provider key in the ExtensionKey spelling
/// `group/name#id`, or `None` when the grammar holds. One enum, both
/// R8A2 record cells: `artifact_record`'s producer and `deploy_receipt`'s
/// provider answer to the same spelling rule, because they name the same
/// thing — the exact provider a mechanism routed to. The check is a
/// SHAPE, not a registry lookup: full key validation (does the group
/// exist, is the provider installed) is the mechanism atom's, and a
/// record may outlive the registry row that minted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKeyDefect {
    /// Empty, whitespace-only, or carrying CR, LF or NUL.
    Blank,
    /// No `#` — the id component is missing.
    NoHash,
    /// More than one `#`.
    DuplicateHash,
    /// No `/` before the `#` — the group component is missing.
    NoSlash,
    /// More than one `/` before the `#`.
    DuplicateSlash,
    /// The `#` arrives before the `/`, so there is no group/name head.
    HashBeforeSlash,
    /// One of the three components is empty or whitespace-only.
    BlankComponent,
}

impl ProviderKeyDefect {
    /// The sentence a refusal reads as, after the offending value.
    #[must_use]
    pub fn phrase(self) -> &'static str {
        match self {
            ProviderKeyDefect::Blank => "is empty, whitespace-only or carries CR, LF or NUL",
            ProviderKeyDefect::NoHash => "carries no `#`, so it names no provider id",
            ProviderKeyDefect::DuplicateHash => "carries more than one `#`",
            ProviderKeyDefect::NoSlash => "carries no `/` before the `#`, so it names no group",
            ProviderKeyDefect::DuplicateSlash => "carries more than one `/` before the `#`",
            ProviderKeyDefect::HashBeforeSlash => {
                "carries its `#` before its `/`, so there is no group/name head"
            }
            ProviderKeyDefect::BlankComponent => "carries an empty group, name or id component",
        }
    }
}

/// The first thing wrong with a provider key in the ExtensionKey
/// spelling `group/name#id` (`org.vibevm/vibe#cargo` is the reserved
/// builtin; `__host__/<project>#<id>` is the host spelling). Order
/// matters only for which refusal a reader sees first; every arm is
/// independently reachable and each has its own RED.
#[must_use]
pub fn provider_key_defect(value: &str) -> Option<ProviderKeyDefect> {
    if value.trim().is_empty() || has_control_bytes(value) {
        return Some(ProviderKeyDefect::Blank);
    }
    if value.matches('#').count() != 1 {
        return Some(if value.contains('#') {
            ProviderKeyDefect::DuplicateHash
        } else {
            ProviderKeyDefect::NoHash
        });
    }
    let (head, id) = value.split_once('#').unwrap_or((value, ""));
    let slashes = head.matches('/').count();
    if slashes > 1 {
        return Some(ProviderKeyDefect::DuplicateSlash);
    }
    if slashes == 0 {
        // A `/` sitting AFTER the `#` is in the id component, not the
        // head — the key has its hash before its slash.
        return Some(if value.contains('/') {
            ProviderKeyDefect::HashBeforeSlash
        } else {
            ProviderKeyDefect::NoSlash
        });
    }
    let (group, name) = head.split_once('/').unwrap_or(("", ""));
    if group.trim().is_empty() || name.trim().is_empty() || id.trim().is_empty() {
        return Some(ProviderKeyDefect::BlankComponent);
    }
    None
}

/// `sha256:` followed by exactly 64 lowercase hex characters — the one
/// digest spelling every identity and witness on these wires carries.
#[must_use]
pub fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| is_lowercase_hex(hex, 64))
}

/// Exactly `len` lowercase hex characters. Re-exported from the trace
/// index cell, which owns the rule the run id has always answered to.
#[must_use]
pub fn is_lowercase_hex(value: &str, len: usize) -> bool {
    crate::behaviour::compiler_trace_index::is_lowercase_hex(value, len)
}

/// A CANONICAL unsigned decimal string: nonempty, ASCII digits only,
/// and no leading zero unless the whole value is `0`.
///
/// JTD has no `uint64`, so a count that may exceed a machine integer
/// rides a string both ways — the compile-trace member's `events` and
/// `snapshot_bytes` established the discipline, and the evidence
/// witness's `bytes` now shares it: a declared input set above 4 GiB
/// must stay representable, and a non-canonical spelling would smuggle
/// a narrowing or a locale-dependent render in through the one member
/// meant to be lossless.
#[must_use]
pub fn is_canonical_decimal(value: &str) -> bool {
    let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    digits && (value.len() == 1 || !value.starts_with('0'))
}

/// Whether `left <= right` for two canonical decimal strings: length
/// first, then lexicographic over equal lengths — which over ASCII
/// digits IS numeric order. No machine integer is involved, so a count
/// past `u64::MAX` compares correctly rather than wrapping.
#[must_use]
pub fn canonical_decimal_at_most(left: &str, right: &str) -> bool {
    match left.len().cmp(&right.len()) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => left <= right,
    }
}

/// A nonempty portable token: lowercase ASCII alphanumerics plus `-` and
/// `.`, starting and ending alphanumeric, with no `..` run.
///
/// This is the MECHANISM PLANE's one id law — target ids, artifact ids,
/// profile names and mechanism-key tails, exactly the vocabulary the R8A2
/// record members cite. Since the A1/A2 follow-up it DELEGATES to the one
/// exported manifest authority rather than carrying a byte-for-byte copy:
/// one grammar, one home, and the wire cites it by name. The compiler
/// plane's backend-id grammar is a DIFFERENT law and stays on the
/// trace-index members that cite it.
#[must_use]
pub fn is_portable_token(value: &str) -> bool {
    vibe_core::manifest::is_portable_token(value)
}

#[cfg(test)]
#[path = "scalars/tests.rs"]
mod tests;
