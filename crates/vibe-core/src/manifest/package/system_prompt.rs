//! `[system_prompt]` — the tuning of the cache-stable prefix that is not
//! text (PROP-076 `#SYS-MANIFEST-TABLE`).
//!
//! The template itself is literal text and carries no configuration
//! (`#SYS-TEMPLATE-LITERAL`), so this table is the one place a non-text
//! choice lives. The precedent is PROP-075 `#TEMPLATE-SPELLING`, which puts
//! `[template]` in the manifest for exactly that reason.
//!
//! Exactly three optional members exist in this version, and nothing else is
//! configurable: no inclusion policy, no segment order, no OS baking. Every
//! member is omitted when absent, so a manifest that sets none serialises
//! byte-identically to one written before the table existed.
//!
//! A bad value fails **deserialisation** rather than defaulting, the pattern
//! [`super::WhenCondition`] already establishes: a `lane_form` this version
//! cannot honour must refuse, never behave as `verbatim` while claiming
//! otherwise.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-MANIFEST-TABLE");

use std::fmt;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::TargetOs;

#[path = "system_prompt/validation.rs"]
mod validation;

/// `[system_prompt.harness.*]` — the per-harness delivery settings, out of
/// line along that seam: the table above is about the *prefix*, and these
/// are about how it reaches one harness.
#[path = "system_prompt/harness.rs"]
mod harness;
pub use harness::{
    CODEX_DOC_MAX_BYTES_DEFAULT, ClaudeHarness, CodexHarness, DocMaxBytes, HarnessDelivery,
    HarnessTable, OpenCodeHarness, QwenHarness,
};

/// `[system_prompt]` — opt-in tuning for the generated prefix.
///
/// ```
/// use vibe_core::manifest::{EntrypointChoice, LaneForm, SystemPromptSection};
///
/// let section: SystemPromptSection = toml::from_str("")?;
/// assert_eq!(section.entrypoint, EntrypointChoice::Default);
/// assert_eq!(section.lane_form, LaneForm::Verbatim);
/// assert_eq!(section.recipe, None);
///
/// let chosen: SystemPromptSection =
///     toml::from_str("entrypoint = \"CLAUDE.md\"\nrecipe = 1\n")?;
/// assert_eq!(chosen.entrypoint.file_name(), Some("CLAUDE.md"));
///
/// let none: SystemPromptSection = toml::from_str("entrypoint = false\n")?;
/// assert_eq!(none.entrypoint, EntrypointChoice::None);
/// assert_eq!(none.entrypoint.file_name(), None);
/// # Ok::<(), toml::de::Error>(())
/// ```
///
/// The two platform keys are independent, and absent means "change nothing":
///
/// ```
/// use vibe_core::manifest::{HostChoice, SystemPromptSection, TargetOs, TargetSelection};
///
/// let default: SystemPromptSection = toml::from_str("")?;
/// assert_eq!(default.host, HostChoice::Portable);
/// assert_eq!(default.target, TargetSelection::AllPlatforms);
/// // Absent `target` means every OS, so every `target:` row is included.
/// assert!(default.target.includes(TargetOs::Windows));
///
/// let pinned: SystemPromptSection =
///     toml::from_str("host = \"linux\"\ntarget = [\"linux\", \"macos\"]\n")?;
/// assert_eq!(pinned.host.fixed(), Some(TargetOs::Linux));
/// assert!(pinned.target.includes(TargetOs::Macos));
/// assert!(!pinned.target.includes(TargetOs::Windows));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-MANIFEST-TABLE")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemPromptSection {
    /// Which entrypoint's export becomes the prefix's second segment.
    /// Absent means the default spelling; `false` means no entrypoint
    /// segment at all (PROP-076 `#SYS-ENTRYPOINT-ONE`).
    #[serde(default, skip_serializing_if = "EntrypointChoice::is_default")]
    pub entrypoint: EntrypointChoice,

    /// `host` — **where the agent runs**, which is what an existing
    /// `when = "os:<name>"` gate has always meant: a devops, shell or
    /// package-manager rule. `"portable"` (the default) leaves those rows in
    /// the gated continuation band, evaluated per session against a supplied
    /// session target. A fixed OS resolves them at build time and bakes the
    /// matching rows into the prefix (PROP-076 `#SYS-HOST-AND-TARGET`).
    #[serde(default, skip_serializing_if = "HostChoice::is_default")]
    pub host: HostChoice,

    /// `target` — **which operating systems the product is for**, which is
    /// what the `when = "target:<os>"` gate means: a rule about the kernel,
    /// WinAPI or another platform surface the software itself addresses.
    /// It is a project property, so it is *always* resolved at build time.
    /// Absent means every OS, so no `target:` row is dropped.
    #[serde(default, skip_serializing_if = "TargetSelection::is_default")]
    pub target: TargetSelection,

    /// How the compiled static lane enters the prefix. `"verbatim"` is the
    /// default and this version's only accepted value; the member exists so
    /// the canonical-projection alternative of PROP-076 `#SYS-LANE-VERBATIM`
    /// needs no format change later.
    #[serde(default, skip_serializing_if = "LaneForm::is_default")]
    pub lane_form: LaneForm,

    /// Pins the generated transport-contract recipe. Absent means the
    /// current one. A recipe change moves every prefix digest in the world,
    /// so pinning it is a deliberate act (PROP-076 `#SYS-TRANSPORT-CONTRACT`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<u32>,

    /// Per-harness delivery settings. One harness has a setting in this
    /// version: Codex, whose project-instruction truncation limit is
    /// configurable on the harness side and therefore cannot be a constant
    /// here (PROP-076 `#SYS-HARNESS-DELIVERY`).
    #[serde(default, skip_serializing_if = "HarnessTable::is_empty")]
    pub harness: HarnessTable,
}

impl SystemPromptSection {
    /// `true` when the table carries nothing — every member at its default.
    /// A present-but-empty `[system_prompt]` is still an explicit table and
    /// is validated like any other; this only lets the serialiser skip it.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The project-instruction byte limit a Codex session is expected to
    /// apply, declared or defaulted.
    ///
    /// ```
    /// use vibe_core::manifest::{CODEX_DOC_MAX_BYTES_DEFAULT, SystemPromptSection};
    ///
    /// let absent: SystemPromptSection = toml::from_str("")?;
    /// assert_eq!(absent.codex_doc_max_bytes(), CODEX_DOC_MAX_BYTES_DEFAULT);
    ///
    /// let raised: SystemPromptSection =
    ///     toml::from_str("[harness.codex]\ndoc_max_bytes = 262144\n")?;
    /// assert_eq!(raised.codex_doc_max_bytes(), 262_144);
    /// # Ok::<(), toml::de::Error>(())
    /// ```
    pub fn codex_doc_max_bytes(&self) -> usize {
        self.harness
            .codex
            .as_ref()
            .and_then(|codex| codex.doc_max_bytes)
            .map_or(CODEX_DOC_MAX_BYTES_DEFAULT, DocMaxBytes::get)
    }
}

/// The agent instruction files a node may carry at its root, as a closed set
/// (PROP-012). vibe owns only the `<vibevm>` block inside each; the rest of
/// every file belongs to the developer. The spellings are identical to
/// vibe-workspace's `REDIRECT_FILES`, and a test there pins the agreement.
///
/// ```
/// use vibe_core::manifest::EntrypointFile;
///
/// assert_eq!(EntrypointFile::DEFAULT, EntrypointFile::Agents);
/// assert_eq!(EntrypointFile::Claude.as_str(), "CLAUDE.md");
/// assert_eq!(EntrypointFile::ALL.len(), 3, "the set is closed at three");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EntrypointFile {
    /// `CLAUDE.md`.
    #[serde(rename = "CLAUDE.md")]
    Claude,
    /// `AGENTS.md`.
    #[serde(rename = "AGENTS.md")]
    Agents,
    /// `GEMINI.md`.
    #[serde(rename = "GEMINI.md")]
    Gemini,
}

impl EntrypointFile {
    /// Every legal spelling, in the order a refusal lists them.
    pub const ALL: [EntrypointFile; 3] = [
        EntrypointFile::Claude,
        EntrypointFile::Agents,
        EntrypointFile::Gemini,
    ];

    /// The one spelling the prefix uses when `entrypoint` is absent.
    pub const DEFAULT: EntrypointFile = EntrypointFile::Agents;

    /// The file's exact basename.
    ///
    /// ```
    /// use vibe_core::manifest::EntrypointFile;
    /// assert_eq!(EntrypointFile::DEFAULT.as_str(), "AGENTS.md");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            EntrypointFile::Claude => "CLAUDE.md",
            EntrypointFile::Agents => "AGENTS.md",
            EntrypointFile::Gemini => "GEMINI.md",
        }
    }
}

impl fmt::Display for EntrypointFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `entrypoint` — the default spelling, one named file, or none at all.
///
/// The three instruction files are never concatenated: this repository's own
/// convention keeps them byte-identical, so joining them would triple the
/// text, and in a project where they differ by harness joining them would
/// deliver contradictions (PROP-076 `#SYS-ENTRYPOINT-ONE`).
///
/// ```
/// use vibe_core::manifest::{EntrypointChoice, EntrypointFile};
///
/// // Absent resolves to the default spelling rather than to "nothing".
/// assert_eq!(EntrypointChoice::Default.file(), Some(EntrypointFile::Agents));
/// assert_eq!(
///     EntrypointChoice::File(EntrypointFile::Gemini).file_name(),
///     Some("GEMINI.md"),
/// );
/// // `entrypoint = false` is the only way to get no entrypoint segment.
/// assert_eq!(EntrypointChoice::None.file(), None);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "EntrypointWire", into = "EntrypointWire")]
pub enum EntrypointChoice {
    /// Absent — the default spelling, [`EntrypointFile::DEFAULT`].
    #[default]
    Default,
    /// An explicitly named file from the closed set.
    File(EntrypointFile),
    /// `entrypoint = false` — no entrypoint segment in the prefix.
    None,
}

impl EntrypointChoice {
    /// `true` when nothing was declared.
    pub fn is_default(&self) -> bool {
        matches!(self, EntrypointChoice::Default)
    }

    /// The selected file, or `None` when the node declared no entrypoint
    /// segment. The default resolves to [`EntrypointFile::DEFAULT`], so a
    /// caller never has to re-derive the default spelling.
    pub fn file(&self) -> Option<EntrypointFile> {
        match self {
            EntrypointChoice::Default => Some(EntrypointFile::DEFAULT),
            EntrypointChoice::File(file) => Some(*file),
            EntrypointChoice::None => None,
        }
    }

    /// [`Self::file`]'s basename.
    pub fn file_name(&self) -> Option<&'static str> {
        self.file().map(EntrypointFile::as_str)
    }
}

/// `host` — the operating system the **agent** runs on.
///
/// This is the axis an `os:` gate has always been about, and the two words
/// are kept apart deliberately: `host` is where the session is, `target` is
/// what the product is for. A rule that says "use `apt`" is a `host` rule; a
/// rule about WinAPI is a `target` rule, and conflating them is what the two
/// keys exist to prevent (PROP-076 `#SYS-HOST-AND-TARGET`).
///
/// ```
/// use vibe_core::manifest::{HostChoice, TargetOs};
///
/// // Portable is the default: no host decision is baked into the prefix.
/// assert_eq!(HostChoice::default(), HostChoice::Portable);
/// assert_eq!(HostChoice::Portable.fixed(), None);
/// assert_eq!(HostChoice::Portable.as_str(), "portable");
/// // A fixed host resolves `os:` rows at build time.
/// assert_eq!(HostChoice::Fixed(TargetOs::Linux).fixed(), Some(TargetOs::Linux));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum HostChoice {
    /// `"portable"` — the prefix carries no host decision, and `os:` rows
    /// stay in the gated continuation band for a per-session verdict.
    #[default]
    Portable,
    /// A named OS — `os:` rows are resolved at build time and the matching
    /// ones are baked into the prefix. A session on a different OS is
    /// **warned about**, not refused.
    Fixed(TargetOs),
}

impl HostChoice {
    /// `true` when nothing was declared.
    pub fn is_default(&self) -> bool {
        matches!(self, HostChoice::Portable)
    }

    /// The pinned OS, or `None` for a portable prefix.
    pub fn fixed(&self) -> Option<TargetOs> {
        match self {
            HostChoice::Portable => None,
            HostChoice::Fixed(os) => Some(*os),
        }
    }

    /// The declared spelling: `"portable"` or the OS name.
    pub fn as_str(&self) -> &'static str {
        match self {
            HostChoice::Portable => PORTABLE,
            HostChoice::Fixed(os) => os.as_str(),
        }
    }
}

impl fmt::Display for HostChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The `"portable"` spelling of [`HostChoice`], which is not an OS name.
pub(crate) const PORTABLE: &str = "portable";

/// `target` — the operating systems the **product** is for.
///
/// Always resolved at build time, because it is a property of the project
/// rather than of a session: a `target:` row is included exactly when its OS
/// is selected here. Absent selects every OS, which is why an existing
/// project sees no change (PROP-076 `#SYS-HOST-AND-TARGET`).
///
/// ```
/// use vibe_core::manifest::{TargetOs, TargetSelection};
///
/// // Absent selects every OS, so no `target:` row is ever dropped.
/// assert_eq!(TargetSelection::default(), TargetSelection::AllPlatforms);
/// assert!(TargetSelection::AllPlatforms.includes(TargetOs::Windows));
///
/// let only = TargetSelection::Only(vec![TargetOs::Linux]);
/// assert!(only.includes(TargetOs::Linux));
/// assert!(!only.includes(TargetOs::Windows));
/// assert_eq!(only.platforms(), vec![TargetOs::Linux]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub enum TargetSelection {
    /// The key was absent — every OS is a target, so no `target:` row is
    /// dropped. This is the default precisely so adding the gate spelling
    /// changes no existing project's boot.
    #[default]
    AllPlatforms,
    /// An explicit, non-empty selection in the author's own order.
    Only(Vec<TargetOs>),
}

impl TargetSelection {
    /// `true` when nothing was declared.
    pub fn is_default(&self) -> bool {
        matches!(self, TargetSelection::AllPlatforms)
    }

    /// Whether `os` is one of the product's targets. An absent selection
    /// includes every OS, so this is the one question a `target:` gate asks.
    pub fn includes(&self, os: TargetOs) -> bool {
        match self {
            TargetSelection::AllPlatforms => true,
            TargetSelection::Only(selected) => selected.contains(&os),
        }
    }

    /// The selected systems, in the author's order; every OS when absent.
    pub fn platforms(&self) -> Vec<TargetOs> {
        match self {
            TargetSelection::AllPlatforms => ALL_TARGET_OS.to_vec(),
            TargetSelection::Only(selected) => selected.clone(),
        }
    }
}

/// Every operating system the closed `TargetOs` set holds, in the order a
/// refusal lists them.
pub(crate) const ALL_TARGET_OS: [TargetOs; 3] =
    [TargetOs::Windows, TargetOs::Macos, TargetOs::Linux];

/// `lane_form` — how the compiled static lane enters the prefix.
///
/// One variant today. `"projected"` is reserved for the canonical-Markdown
/// projection PROP-076 `#SYS-LANE-VERBATIM` rejects for this version, and is
/// refused by name rather than silently treated as `verbatim`.
///
/// ```
/// use vibe_core::manifest::LaneForm;
///
/// assert_eq!(LaneForm::default(), LaneForm::Verbatim);
/// assert_eq!(LaneForm::Verbatim.as_str(), "verbatim");
/// assert!(LaneForm::Verbatim.is_default());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum LaneForm {
    /// The lane's admitted bytes, opaque and unparsed.
    #[default]
    Verbatim,
}

impl LaneForm {
    /// `true` when nothing was declared.
    pub fn is_default(&self) -> bool {
        matches!(self, LaneForm::Verbatim)
    }

    /// The declared spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            LaneForm::Verbatim => "verbatim",
        }
    }
}

impl fmt::Display for LaneForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `entrypoint`'s two admitted TOML shapes: a string or the boolean `false`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum EntrypointWire {
    /// `entrypoint = false`. `true` is refused — it says nothing.
    Disabled(bool),
    /// `entrypoint = "AGENTS.md"`.
    Named(String),
}

#[cfg(test)]
#[path = "system_prompt/tests.rs"]
mod tests;
