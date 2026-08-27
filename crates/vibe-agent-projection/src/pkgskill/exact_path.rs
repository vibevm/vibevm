//! Exact UTF-8 relative spelling for selected source files, and the lossless
//! escaped rendering its refusals are reported with.
//!
//! A skill's projected relative path is data the receipt commits to and the
//! target namespace stores. A lossy rendering would silently turn an
//! unrepresentable name into replacement characters and project *those*
//! bytes; this cell refuses instead, before the selected map exists and so
//! before any stage, durable intent, or target mutation.
//!
//! The refusal itself must not lossify either. `Path::display` and
//! `OsStr::to_string_lossy` both substitute `U+FFFD` for exactly the units
//! the operator needs to see, and `U+FFFD` is not reversible — two different
//! broken names render identically, and the real bytes are gone. So every
//! diagnostic in this cell carries an [`EscapedOsPath`]: a `Display` value
//! built once, losslessly, from the OS-native units.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::ffi::OsStr;
use std::path::{Component, Path};

use super::PackageSkillError;

/// One OS name or path rendered **losslessly** for diagnostics.
///
/// Every unit the host can store is represented exactly once: text that is
/// valid on this platform prints as itself, and anything that is not — an
/// invalid UTF-8 byte on Unix, an unpaired surrogate on Windows — prints as
/// an explicit escape (`\xHH` for a raw byte, `\uXXXX` for a raw UTF-16
/// unit). Control characters and the escape introducer are escaped too, so
/// the rendering is unambiguous and reviewable. Nothing is ever dropped and
/// `U+FFFD` is never emitted.
///
/// The value is built once at the failure site and stored as text, which is
/// what keeps the outer [`PackageSkillError`] from re-lossifying it:
/// `thiserror` renders a `Path`/`PathBuf` field through `Path::display`.
///
/// ```
/// use std::ffi::OsStr;
/// use vibe_agent_projection::pkgskill::EscapedOsPath;
///
/// // Legal text — including non-ASCII — renders as itself.
/// let name = EscapedOsPath::new(OsStr::new("references/Maße.md"));
/// assert_eq!(name.as_str(), "references/Maße.md");
///
/// // A control character is named rather than printed, and the escape
/// // introducer is escaped so the rendering stays reversible.
/// assert_eq!(EscapedOsPath::new(OsStr::new("a\u{1}b")).as_str(), "a\\u0001b");
/// assert_eq!(EscapedOsPath::new(OsStr::new("a\\b")).as_str(), "a\\\\b");
///
/// // Nothing is ever rendered as the replacement character.
/// assert!(!name.to_string().contains('\u{fffd}'));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EscapedOsPath(String);

impl EscapedOsPath {
    /// Escape one OS-native name or path.
    #[must_use]
    pub fn new(value: &OsStr) -> Self {
        Self(escape(value))
    }

    /// The escaped rendering.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EscapedOsPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(unix)]
fn escape(value: &OsStr) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::new();
    let mut rest = value.as_bytes();
    loop {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                push_text(&mut out, text);
                return out;
            }
            Err(error) => {
                let boundary = error.valid_up_to();
                if let Ok(text) = std::str::from_utf8(&rest[..boundary]) {
                    push_text(&mut out, text);
                }
                // `error_len() == None` means the tail is an incomplete
                // sequence: every remaining byte is raw. `max(1)` keeps the
                // walk total — a zero-length step could never terminate.
                let width = error.error_len().unwrap_or(rest.len() - boundary).max(1);
                let end = boundary.saturating_add(width).min(rest.len());
                for byte in &rest[boundary..end] {
                    out.push_str(&format!("\\x{byte:02X}"));
                }
                rest = &rest[end..];
            }
        }
    }
}

#[cfg(windows)]
fn escape(value: &OsStr) -> String {
    use std::os::windows::ffi::OsStrExt;
    let mut out = String::new();
    for unit in char::decode_utf16(value.encode_wide()) {
        match unit {
            Ok(character) => push_char(&mut out, character),
            Err(unpaired) => {
                out.push_str(&format!("\\u{:04X}", unpaired.unpaired_surrogate()));
            }
        }
    }
    out
}

/// Only the Unix walk decodes runs of text; the Windows walk is per-unit.
#[cfg(unix)]
fn push_text(out: &mut String, text: &str) {
    for character in text.chars() {
        push_char(out, character);
    }
}

fn push_char(out: &mut String, character: char) {
    match character {
        // The escape introducer must be escaped, or `\x41` and a literal
        // backslash followed by `x41` would render identically.
        '\\' => out.push_str("\\\\"),
        control if control.is_control() => {
            out.push_str(&format!("\\u{:04X}", control as u32));
        }
        other => out.push(other),
    }
}

/// One directory entry name rendered **exactly**, never lossily: a name that
/// is not valid UTF-8 (invalid bytes on Unix, an unpaired surrogate on
/// Windows) has no faithful relative spelling, so the source refuses here.
/// Replacement-character bytes are never projected — not into the target,
/// and not into the refusal.
pub(super) fn exact_utf8_component(name: &OsStr, path: &Path) -> Result<String, PackageSkillError> {
    name.to_str().map(str::to_string).ok_or_else(|| {
        unportable(
            path,
            format!(
                "entry name `{}` (escaped) is not valid UTF-8; rename it to a portable UTF-8 name \
             before projecting the skill",
                EscapedOsPath::new(name)
            ),
        )
    })
}

/// The forward-slashed relative spelling of `path` below `base`, built from
/// exact normal components only — no lossy conversion, no separator
/// rewriting, and no `.`/`..`/prefix component.
pub(super) fn exact_utf8_relative(base: &Path, path: &Path) -> Result<String, PackageSkillError> {
    let relative = path.strip_prefix(base).unwrap_or(path);
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return Err(unportable(
                path,
                "relative path contains a non-normal component".into(),
            ));
        };
        parts.push(exact_utf8_component(value, path)?);
    }
    if parts.is_empty() {
        return Err(unportable(path, "relative path is empty".into()));
    }
    Ok(parts.join("/"))
}

fn unportable(path: &Path, reason: String) -> PackageSkillError {
    PackageSkillError::UnportablePath {
        path: EscapedOsPath::new(path.as_os_str()),
        reason,
    }
}

#[cfg(test)]
#[path = "exact_path/tests.rs"]
mod tests;
