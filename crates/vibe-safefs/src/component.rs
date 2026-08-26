//! The one portable relative-path law every project mutation goes through.
//!
//! There is exactly one table here on purpose. A second, weaker copy beside a
//! new writer is how `guide.md:ads`, `COM1.json` and `trailing.` reach disk in
//! one subsystem while being refused in another; the Windows device spellings
//! themselves are delegated to the single shared manifest table in `vibe_core`
//! rather than restated.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use icu_normalizer::ComposingNormalizer;
use specmark::spec;
use unicode_casefold::{Locale, UnicodeCaseFold, Variant};
use vibe_core::manifest::{DeclarantPathFault, declarant_path, declarant_path_component};

/// The staging prefix this crate reserves inside a project. A declared output
/// may never spell one: a caller's own path must not be able to name another
/// caller's in-flight stage.
pub const STAGE_PREFIX: &str = ".vibe-stage-";

/// Why one component is not a portable, safely storable name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub enum UnsafeComponent {
    /// The shared declarant/component law in `vibe-core` refused it. The
    /// detailed reason remains that law's typed value; this crate owns no
    /// second punctuation, device or control-character table.
    Declarant(DeclarantPathFault),
    /// Reserved for this crate's own in-flight staging files.
    StagePrefix,
}

impl UnsafeComponent {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Declarant(fault) => fault.reason(),
            Self::StagePrefix => "is reserved for in-flight staging files",
        }
    }
}

/// Judge one path component. This is the whole law; callers add only their own
/// domain rules on top of it.
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn classify_component(component: &str) -> Option<UnsafeComponent> {
    if let Err(fault) = declarant_path_component(component) {
        return Some(UnsafeComponent::Declarant(fault));
    }
    // Case-insensitively: on a case-folding filesystem `.VIBE-STAGE-1-0` and
    // `.vibe-stage-1-0` are one name, so reserving only the lowercase spelling
    // would let a declared row alias an in-flight stage.
    if identity_key(component).starts_with(&identity_key(STAGE_PREFIX)) {
        return Some(UnsafeComponent::StagePrefix);
    }
    None
}

/// The physical-identity key of one component: **one** conservative
/// cross-platform key, deliberately coarser than any single filesystem.
///
/// Exact spelling is what a caller stores and compares for ownership; this key
/// exists only to detect that two *different* spellings would land on one
/// physical file somewhere we ship to. Three steps:
///
/// 1. **NFC** — a decomposed `e` + `U+0301` and a composed `é` are one
///    identity, which is exactly what macOS/HFS+ (NFD) and everyone else (NFC)
///    disagree about;
/// 2. **full, non-Turkic Unicode case fold** — `CaseFolding.txt`'s `C` + `F`
///    mappings, the operation APFS case-insensitive matching is defined on, so
///    `ß`/`ẞ`/`SS` are one key and `Θ`/`ϴ`/`θ`, `Σ`/`ς`/`σ` likewise;
/// 3. **NFC again** — folding can leave the result unnormalized, because a
///    full mapping may emit a base plus a combining mark.
///
/// Case folding, **not** uppercasing: uppercasing is not idempotent across the
/// title/uppercase-symbol families, so `ẞ` ≠ `ß` and `ϴ` ≠ `Θ` under it — two
/// spellings APFS treats as one file. The fold table is pinned to Unicode
/// **9.0.0** ([`unicode_casefold::UNICODE_VERSION`], asserted by a test), the
/// version APFS case-insensitive volumes use; a dependency bump must not
/// change filesystem identity silently.
///
/// **Safe over-refusal, documented deliberately.** Folding merges `Maße` with
/// `MASSE`, `ﬁle` with `FILE`, and NFC with NFD on *every* host, not only
/// where the filesystem does. Over-merging costs a refusal the operator
/// resolves by renaming; under-merging silently overwrites or orphans
/// someone's bytes. We take the refusal. No handwritten Unicode table is
/// involved: NFC is `icu_normalizer`'s compiled data, the fold is
/// `unicode-casefold`'s generated `CaseFolding.txt` table — the same algorithm
/// the package-skill receipt identity uses, so there is one law, not two.
#[must_use]
pub fn identity_key(component: &str) -> String {
    const NFC: icu_normalizer::ComposingNormalizerBorrowed<'static> =
        ComposingNormalizer::new_nfc();
    let composed = NFC.normalize(component);
    let folded = composed
        .as_ref()
        .case_fold_with(Variant::Full, Locale::NonTurkic)
        .collect::<String>();
    NFC.normalize(&folded).into_owned()
}

/// The physical-identity key of a whole forward-slashed path.
///
/// Built component-wise from [`identity_key`], so it inherits exactly the
/// NFC → full case fold → NFC law and adds only what a *path* has that a
/// component does not: an optional Windows prefix (a drive letter or a verbatim `\?\`
/// device path) and a leading root. Both are folded case-insensitively,
/// because `C:/x` and `c:/x` are one file, and empty segments produced by a
/// doubled separator are dropped so `docs//a.md` and `docs/a.md` do not read
/// as different files.
///
/// This is a **comparison** key only. Callers keep and store the exact
/// declared spelling; nothing here ever becomes a path that is opened.
///
/// ```
/// use vibe_safefs::path_identity_key as key;
/// assert_eq!(key("docs/A.md"), key("Docs/a.md"));
/// assert_eq!(key("C:/p/x.md"), key("c:/p/x.md"));
/// assert_ne!(key("docs/a.md"), key("docs/b.md"));
/// ```
#[must_use]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn path_identity_key(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let rooted = normalized.starts_with('/');
    let folded: Vec<String> = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(identity_key)
        .collect();
    let joined = folded.join("/");
    if rooted { format!("/{joined}") } else { joined }
}

/// Why one complete relative output set is not portable.
///
/// ```
/// use vibe_safefs::{SelectionFault, judge_selection};
///
/// assert!(matches!(
///     judge_selection(["SKILL.md", "skill.md"]),
///     Err(SelectionFault::Collision { .. })
/// ));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionFault {
    /// One row is not a portable literal relative path.
    Unsafe(String),
    /// Two distinct spellings share one physical identity.
    Collision { first: String, alias: String },
}

impl std::fmt::Display for SelectionFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsafe(path) => write!(formatter, "unsafe file path `{path}`"),
            Self::Collision { first, alias } => write!(
                formatter,
                "file paths `{first}` and `{alias}` are one file on a case-insensitive host"
            ),
        }
    }
}

impl std::error::Error for SelectionFault {}

/// Judge a complete set before staging: every row obeys the shared literal
/// path law and no two exact spellings collapse to one physical identity.
pub fn judge_selection<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> std::result::Result<(), SelectionFault> {
    let mut seen = BTreeMap::<String, &str>::new();
    for path in paths {
        if split_relative(path).is_err() {
            return Err(SelectionFault::Unsafe(path.to_string()));
        }
        if let Some(first) = seen.insert(path_identity_key(path), path) {
            return Err(SelectionFault::Collision {
                first: first.to_string(),
                alias: path.to_string(),
            });
        }
    }
    Ok(())
}

/// Lexical containment before a capability walk. Both paths must be absolute,
/// and the suffix below `root` may contain only ordinary components.
pub fn ensure_lexically_contained(root: &Path, path: &Path) -> Result<()> {
    if !root.is_absolute() || !path.is_absolute() {
        bail!(
            "containment requires absolute paths (root `{}`, path `{}`)",
            root.display(),
            path.display()
        );
    }
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "path `{}` escapes trusted root `{}`",
            path.display(),
            root.display()
        )
    })?;
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!(
            "path `{}` contains a non-normal component below `{}`",
            path.display(),
            root.display()
        );
    }
    Ok(())
}

/// Whether two paths overlap under the same component-wise physical identity.
#[must_use]
pub fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = path_components(left);
    let right = path_components(right);
    prefix_of(&left, &right) || prefix_of(&right, &left)
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| match component {
            Component::Prefix(prefix) => format!("prefix:{}", os_identity(prefix.as_os_str())),
            Component::RootDir => "root".to_string(),
            Component::CurDir => "cur".to_string(),
            Component::ParentDir => "parent".to_string(),
            Component::Normal(value) => format!("normal:{}", os_identity(value)),
        })
        .collect()
}

/// A comparison-only identity for one OS component. Valid Unicode takes the
/// NFC/fold path. An unrepresentable OS unit is encoded reversibly, with only
/// ASCII case folded; no replacement character can collapse two distinct
/// byte/unit sequences into one key.
fn os_identity(value: &OsStr) -> String {
    if let Some(text) = value.to_str() {
        return format!("utf8:{}", identity_key(text));
    }
    opaque_os_identity(value)
}

#[cfg(unix)]
fn opaque_os_identity(value: &OsStr) -> String {
    use std::os::unix::ffi::OsStrExt;

    let mut out = String::from("bytes:");
    for byte in value.as_bytes() {
        push_hex_byte(&mut out, byte.to_ascii_lowercase());
    }
    out
}

#[cfg(windows)]
fn opaque_os_identity(value: &OsStr) -> String {
    use std::os::windows::ffi::OsStrExt;

    let mut out = String::from("wide:");
    for unit in value.encode_wide() {
        let unit = if (b'A' as u16..=b'Z' as u16).contains(&unit) {
            unit + u16::from(b'a' - b'A')
        } else {
            unit
        };
        push_hex_u16(&mut out, unit);
    }
    out
}

#[cfg(not(any(unix, windows)))]
fn opaque_os_identity(value: &OsStr) -> String {
    // VibeVM's current filesystem targets are Unix and Windows. Keep an
    // explicit marker for any future target instead of silently presenting
    // this fallback as a portable law.
    format!("unsupported:{value:?}")
}

#[cfg(unix)]
fn push_hex_byte(out: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(HEX[(byte >> 4) as usize] as char);
    out.push(HEX[(byte & 0x0f) as usize] as char);
}

#[cfg(windows)]
fn push_hex_u16(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in [12, 8, 4, 0] {
        out.push(HEX[((unit >> shift) & 0x0f) as usize] as char);
    }
}

fn prefix_of(prefix: &[String], whole: &[String]) -> bool {
    prefix.len() <= whole.len() && prefix.iter().zip(whole).all(|(left, right)| left == right)
}

/// Read-only no-follow inspection for the standalone projection surface.
/// Automatic mutation uses retained [`Project`](crate::Project) capabilities;
/// this helper remains the shared preflight for an existing or missing path.
pub fn ensure_no_follow_walk(root: &Path, path: &Path, allow_missing: bool) -> Result<()> {
    ensure_lexically_contained(root, path)?;
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "path `{}` escapes trusted root `{}`",
            path.display(),
            root.display()
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || is_reparse(&metadata) {
                    bail!(
                        "path `{}` traverses symlink/junction/reparse component `{}`",
                        path.display(),
                        current.display()
                    );
                }
                if current != path && !metadata.is_dir() {
                    bail!(
                        "path `{}` traverses non-directory ancestor `{}`",
                        path.display(),
                        current.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_missing => break,
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("inspecting `{}`", current.display())));
            }
        }
    }
    Ok(())
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

/// Refuse anything but a portable component.
///
/// ```
/// assert!(vibe_safefs::ensure_safe_component("guide.md").is_ok());
/// assert!(vibe_safefs::ensure_safe_component("guide.md:ads").is_err());
/// assert!(vibe_safefs::ensure_safe_component("COM1.json").is_err());
/// assert!(vibe_safefs::ensure_safe_component("trailing.").is_err());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn ensure_safe_component(component: &str) -> Result<()> {
    match classify_component(component) {
        None => Ok(()),
        Some(reason) => bail!(
            "unsafe relative component `{component}`: it {}",
            reason.reason()
        ),
    }
}

/// Split a forward-slashed relative path into parent components and file name,
/// judging every component by the one law above.
///
/// ```
/// let (parents, name) = vibe_safefs::split_relative("docs/nested/guide.md").unwrap();
/// assert_eq!(parents, ["docs", "nested"]);
/// assert_eq!(name, "guide.md");
/// assert!(vibe_safefs::split_relative("docs/../escape.md").is_err());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub fn split_relative(relative: &str) -> Result<(Vec<String>, String)> {
    declarant_path(Path::new(relative)).map_err(|fault| {
        anyhow::anyhow!("unsafe relative path `{relative}`: {}", fault.reason())
    })?;
    let parts: Vec<&str> = relative.split('/').collect();
    let Some((last, parents)) = parts.split_last() else {
        bail!("empty relative path `{relative}`");
    };
    for component in parents {
        ensure_safe_component(component)
            .map_err(|error| error.context(format!("in relative path `{relative}`")))?;
    }
    ensure_safe_component(last)
        .map_err(|error| error.context(format!("in relative path `{relative}`")))?;
    Ok((
        parents.iter().map(|c| (*c).to_string()).collect(),
        (*last).to_string(),
    ))
}

#[cfg(test)]
#[path = "component/tests.rs"]
mod tests;
