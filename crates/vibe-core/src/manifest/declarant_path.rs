//! The one portable declarant-relative path law for authored manifests.
//!
//! Every path a manifest author writes — `[[skill]] path`, `[[extension]]`
//! handler `base` / `crate_dir` / `prebuilt.*`, `[[mechanism]] config_schema`,
//! and `[[artifacts.*]] inputs` path rows — is judged here and nowhere else.
//! A declarant path is UTF-8, forward-slashed, relative to the declaring
//! manifest's root, and spellable *as written* on every host filesystem vibe
//! supports.
//!
//! One law, one table. A spelling refused for a skill cannot be smuggled back
//! in as a build input, and the Windows device table has a single home:
//! receipt containment in `vibe-mcp` already delegates to
//! [`is_windows_device_name`] through the public manifest surface, and
//! `vibe-safefs` joins it at R7 integration rather than keeping a copy.
//!
//! The law is deliberately *stricter* than "cannot escape the root". A path
//! that stays inside the root but names `nul`, carries an alternate-data-stream
//! colon, or ends in a space that Windows silently strips is refused too:
//! authored identity must mean the same thing on every host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-HANDLER-TABLES");

use std::path::{Component, Path};

/// The characters Windows refuses in a file name. `\` and `:` get their own
/// faults because they carry a specific meaning (separator, drive/ADS); the
/// rest are simply unspellable. `*` is here too: it is a wildcard, legal only
/// as *glob syntax* in [`DeclarantPathMode::Pattern`], never as a literal
/// component.
const WINDOWS_INVALID: [char; 6] = ['<', '>', '"', '|', '?', '*'];

/// Whether a spelling is judged as a literal path or as a glob pattern.
///
/// The table of hazards is the same in both modes — a pattern's *literal*
/// segments answer to every device / trailing / character law. The only
/// difference is that a pattern may additionally spell `*` and `**` as
/// wildcard syntax.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::{DeclarantPathMode, declarant_path, declarant_path_pattern};
///
/// // `declarant_path` is `Literal`; `declarant_path_pattern` is `Pattern`.
/// assert_ne!(DeclarantPathMode::Literal, DeclarantPathMode::Pattern);
/// assert!(declarant_path(Path::new("crates/*/Cargo.toml")).is_err());
/// assert!(declarant_path_pattern(Path::new("crates/*/Cargo.toml")).is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarantPathMode {
    /// Every character is literal. `[[skill]] path`, `[[extension]]` handler
    /// fields, `[[mechanism]] config_schema`.
    Literal,
    /// `*` and `**` are glob syntax. Artifact input path rows only.
    Pattern,
}

/// Why a spelling is not a portable declarant-relative path.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::{DeclarantPathFault, declarant_path, declarant_path_pattern};
///
/// assert_eq!(declarant_path(Path::new("a.json:evil")), Err(DeclarantPathFault::Colon));
/// assert_eq!(declarant_path(Path::new("dist/nul")), Err(DeclarantPathFault::WindowsDevice));
/// // A wildcard is syntax, not a literal component.
/// assert_eq!(declarant_path(Path::new("bad*.md")), Err(DeclarantPathFault::InvalidCharacter));
/// assert_eq!(declarant_path_pattern(Path::new("crates/*/Cargo.toml")), Ok("crates/*/Cargo.toml"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarantPathFault {
    /// The spelling is not UTF-8.
    NotUtf8,
    /// The value is empty.
    Empty,
    /// An ASCII/C1 control character appears in the spelling.
    Control,
    /// A backslash appears; authored paths are forward-slashed.
    Backslash,
    /// A `:` appears — a drive prefix or a Windows alternate data stream.
    Colon,
    /// A character Windows cannot store in a name: `< > " | ?`, or a literal
    /// `*` outside [`DeclarantPathMode::Pattern`].
    InvalidCharacter,
    /// The path is absolute rather than declarant-relative.
    Rooted,
    /// An empty segment (`a//b`, or a trailing slash).
    EmptySegment,
    /// A `.` or `..` segment.
    DotSegment,
    /// A segment is a Windows reserved device spelling.
    WindowsDevice,
    /// A segment ends in `.` or a space, which Windows silently strips.
    TrailingDotOrSpace,
    /// A wildcard run that is not well-formed glob syntax: `***` or more, or
    /// a `**` that is not a whole segment.
    MalformedGlob,
    /// A component the platform does not treat as an ordinary name.
    NotNormal,
}

impl DeclarantPathFault {
    /// The clause a diagnostic appends after the shared law sentence.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotUtf8 => "the spelling is not UTF-8",
            Self::Empty => "the value is empty",
            Self::Control => "it carries a control character",
            Self::Backslash => "it carries a backslash",
            Self::Colon => "it carries `:`, a drive prefix or a Windows alternate data stream",
            Self::InvalidCharacter => {
                "it carries a character Windows cannot store in a name (`<`, `>`, `\"`, `|`, `?`, or a literal `*`)"
            }
            Self::Rooted => "it is absolute rather than relative to the declaring manifest's root",
            Self::EmptySegment => "it carries an empty path segment",
            Self::DotSegment => "it carries a `.` or `..` segment",
            Self::WindowsDevice => {
                "a segment is a Windows reserved device name (`con`, `nul`, `com1` …, extension-bearing and superscript aliases included)"
            }
            Self::TrailingDotOrSpace => {
                "a segment ends in `.` or a space, which Windows silently strips"
            }
            Self::MalformedGlob => {
                "a wildcard run is not well-formed glob syntax — write `*` inside a segment, or `**` as a whole segment"
            }
            Self::NotNormal => "it carries a component that is not an ordinary name",
        }
    }
}

/// Judge one authored **literal** declarant-relative path, returning its
/// checked forward-slashed spelling. Wildcards are not syntax here.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::declarant_path;
///
/// assert!(declarant_path(Path::new("schemas/cargo-build-v1.jtd.json")).is_ok());
/// assert!(declarant_path(Path::new("../escape")).is_err());
/// assert!(declarant_path(Path::new("out/secret.txt. ")).is_err());
/// assert!(declarant_path(Path::new("bad?.md")).is_err());
/// ```
pub fn declarant_path(path: &Path) -> Result<&str, DeclarantPathFault> {
    judge(path, DeclarantPathMode::Literal)
}

/// Judge one authored **glob-bearing** declarant-relative path — the artifact
/// input row, the only caller that may spell wildcards. Every literal segment
/// still answers to the full law.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::declarant_path_pattern;
///
/// assert!(declarant_path_pattern(Path::new("crates/*/Cargo.toml")).is_ok());
/// assert!(declarant_path_pattern(Path::new("crates/**/src/*.rs")).is_ok());
/// assert!(declarant_path_pattern(Path::new("crates/helper/**")).is_ok());
/// // Still not a way past the literal law.
/// assert!(declarant_path_pattern(Path::new("crates/**/nul")).is_err());
/// assert!(declarant_path_pattern(Path::new("crates/a**b/x")).is_err());
/// assert!(declarant_path_pattern(Path::new("crates/?/x")).is_err());
/// ```
pub fn declarant_path_pattern(path: &Path) -> Result<&str, DeclarantPathFault> {
    judge(path, DeclarantPathMode::Pattern)
}

fn judge(path: &Path, mode: DeclarantPathMode) -> Result<&str, DeclarantPathFault> {
    let Some(text) = path.to_str() else {
        return Err(DeclarantPathFault::NotUtf8);
    };
    if text.is_empty() {
        return Err(DeclarantPathFault::Empty);
    }
    if text.chars().any(char::is_control) {
        return Err(DeclarantPathFault::Control);
    }
    if text.contains('\\') {
        return Err(DeclarantPathFault::Backslash);
    }
    if text.contains(':') {
        return Err(DeclarantPathFault::Colon);
    }
    if text.starts_with('/') || path.has_root() {
        return Err(DeclarantPathFault::Rooted);
    }
    for segment in text.split('/') {
        judge_segment(segment, mode)?;
    }
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(DeclarantPathFault::NotNormal);
    }
    Ok(text)
}

fn judge_segment(segment: &str, mode: DeclarantPathMode) -> Result<(), DeclarantPathFault> {
    if segment.is_empty() {
        return Err(DeclarantPathFault::EmptySegment);
    }
    if matches!(segment, "." | "..") {
        return Err(DeclarantPathFault::DotSegment);
    }
    // Windows-invalid characters other than `*` are refused in both modes.
    if segment
        .chars()
        .any(|c| WINDOWS_INVALID.contains(&c) && c != '*')
    {
        return Err(DeclarantPathFault::InvalidCharacter);
    }
    if !segment.contains('*') {
        // A literal segment — in either mode — answers to the whole law.
        if is_windows_unsafe_component(segment) {
            return if is_windows_device_name(segment) {
                Err(DeclarantPathFault::WindowsDevice)
            } else {
                Err(DeclarantPathFault::TrailingDotOrSpace)
            };
        }
        return Ok(());
    }
    if mode == DeclarantPathMode::Literal {
        return Err(DeclarantPathFault::InvalidCharacter);
    }
    // Glob syntax: `**` is a whole segment on its own; a single `*` is a
    // within-segment wildcard. `***`, and `**` glued to other text, are
    // neither and would mean different things to different matchers.
    if segment != "**" && segment.contains("**") {
        return Err(DeclarantPathFault::MalformedGlob);
    }
    // The wildcard cannot smuggle in a trailing `.`/space either.
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Err(DeclarantPathFault::TrailingDotOrSpace);
    }
    Ok(())
}

/// Windows reserved device spellings, judged on the basename before the
/// first `.` so extension-bearing aliases (`CON.txt`, `NUL.md`, `COM1.json`,
/// `LPT9.log`) are devices too. `CONIN$`, `CONOUT$`, `CLOCK$` and the
/// superscript `COM¹`/`COM²`/`COM³` (+ LPT equivalents) are included.
///
/// ```
/// use vibe_core::manifest::is_windows_device_name;
///
/// assert!(is_windows_device_name("CON.txt"));
/// assert!(is_windows_device_name("com²"));
/// assert!(!is_windows_device_name("context"));
/// ```
#[must_use]
pub fn is_windows_device_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or(component);
    let lowered = stem.to_lowercase();
    let normalized: String = lowered
        .chars()
        .map(|character| match character {
            '¹' => '1',
            '²' => '2',
            '³' => '3',
            other => other,
        })
        .collect();
    if !normalized.is_ascii() {
        return false;
    }
    matches!(
        normalized.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$" | "clock$"
    ) || (normalized.len() == 4
        && matches!(&normalized[..3], "com" | "lpt")
        && matches!(normalized.as_bytes()[3], b'1'..=b'9'))
}

/// A single forward-slash component Windows can never store **as written**,
/// judged literally: device spellings, components ending in `.` or a space,
/// any of `< > : " \ | ? *`, and control characters.
///
/// This is the whole literal truth on purpose. Receipt containment already
/// delegates here and `vibe-safefs` joins at R7; a caller that answers this
/// question cannot end up with a weaker answer than the manifest law gives.
/// A glob pattern is *not* a component — judge it with
/// [`declarant_path_pattern`], never by asking this about a `*` segment.
///
/// ```
/// use vibe_core::manifest::is_windows_unsafe_component;
///
/// assert!(is_windows_unsafe_component("trailing "));
/// assert!(is_windows_unsafe_component("LPT9.log"));
/// assert!(is_windows_unsafe_component("a?b"));
/// assert!(is_windows_unsafe_component("a*b"));
/// assert!(is_windows_unsafe_component("a:b"));
/// assert!(!is_windows_unsafe_component("helper.exe"));
/// ```
#[must_use]
pub fn is_windows_unsafe_component(segment: &str) -> bool {
    is_windows_device_name(segment)
        || segment.ends_with('.')
        || segment.ends_with(' ')
        || segment
            .chars()
            .any(|c| c.is_control() || c == '\\' || c == ':' || WINDOWS_INVALID.contains(&c))
}

/// The one diagnostic sentence every declarant-path fault renders with.
/// `table` names the declaring table, `field` the exact field, so a reader
/// can go straight to the line — and `declarant-root-relative` stays in the
/// text every existing RED pins.
pub(crate) fn declarant_path_error(
    table: &str,
    id: &str,
    field: &str,
    value: &Path,
    fault: DeclarantPathFault,
    anchor: &str,
) -> String {
    format!(
        "{table} `{id}` field `{field}` value `{}` must be a nonempty declarant-root-relative UTF-8 path with forward slashes: {} ({anchor})",
        value.display(),
        fault.reason(),
    )
}

#[cfg(test)]
#[path = "declarant_path/tests.rs"]
mod tests;
