//! `[system_prompt.harness.<name>]` — how the composed lane reaches one
//! harness (PROP-076 `#SYS-HARNESS-DELIVERY`, `#SYS-CODEX-DECLARED-BUDGET`).
//!
//! A closed table: an unknown harness name refuses by name rather than being
//! ignored, because a setting silently ignored is the one failure a
//! configuration table must not have. Every member is omitted when absent,
//! so a manifest that declares none serialises byte-identically to one
//! written before the table existed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY");

use std::fmt;

use serde::{Deserialize, Serialize};

/// Codex's own default `project_doc_max_bytes`, and therefore the limit a
/// project is measured against when it declares none.
pub const CODEX_DOC_MAX_BYTES_DEFAULT: usize = 32 * 1024;

/// `[system_prompt.harness]` — per-harness delivery settings.
///
/// A closed table of four in this version. An unknown harness name refuses
/// by name rather than being ignored, because a setting silently ignored is
/// the one failure a configuration table must not have.
///
/// ```
/// use vibe_core::manifest::HarnessTable;
///
/// let empty: HarnessTable = toml::from_str("")?;
/// assert!(empty.is_empty());
/// assert!(toml::from_str::<HarnessTable>("[opencode]\n").is_ok());
/// assert!(toml::from_str::<HarnessTable>("[qwen]\n").is_ok());
/// assert!(toml::from_str::<HarnessTable>("[gemini]\n").is_err());
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessTable {
    /// `[system_prompt.harness.claude]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<ClaudeHarness>,
    /// `[system_prompt.harness.codex]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<CodexHarness>,
    /// `[system_prompt.harness.opencode]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<OpenCodeHarness>,
    /// `[system_prompt.harness.qwen]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qwen: Option<QwenHarness>,
}

impl HarnessTable {
    /// `true` when no harness declared anything.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The declared delivery of one harness by name, or its default.
    ///
    /// The default is `launch` for the two harnesses with a file-based
    /// instruction slot and `read` for every other, which is safe before any
    /// live run: a project in `launch` mode started *plain* has no lane
    /// markers in context and the router block has the agent read the files,
    /// so one byte-identical block serves both tiers and a project can never
    /// contradict itself (PROP-076 `#SYS-HARNESS-DELIVERY`, decision D1).
    ///
    /// ```
    /// use vibe_core::manifest::{HarnessDelivery, SystemPromptSection};
    ///
    /// let absent: SystemPromptSection = toml::from_str("")?;
    /// assert_eq!(absent.harness.delivery("claude"), HarnessDelivery::Launch);
    /// assert_eq!(absent.harness.delivery("codex"), HarnessDelivery::Launch);
    /// assert_eq!(absent.harness.delivery("opencode"), HarnessDelivery::Read);
    /// assert_eq!(absent.harness.delivery("qwen"), HarnessDelivery::Read);
    /// assert_eq!(absent.harness.delivery("gemini"), HarnessDelivery::Read);
    ///
    /// let read: SystemPromptSection =
    ///     toml::from_str("[harness.claude]\ndelivery = \"read\"\n")?;
    /// assert_eq!(read.harness.delivery("claude"), HarnessDelivery::Read);
    /// # Ok::<(), toml::de::Error>(())
    /// ```
    pub fn delivery(&self, harness: &str) -> HarnessDelivery {
        let declared = match harness {
            "claude" => self.claude.as_ref().and_then(|claude| claude.delivery),
            "codex" => self.codex.as_ref().and_then(|codex| codex.delivery),
            "opencode" => self
                .opencode
                .as_ref()
                .and_then(|opencode| opencode.delivery),
            "qwen" => self.qwen.as_ref().and_then(|qwen| qwen.delivery),
            _ => None,
        };
        declared.unwrap_or(match harness {
            "claude" | "codex" => HarnessDelivery::Launch,
            _ => HarnessDelivery::Read,
        })
    }
}

/// `[system_prompt.harness.claude]`.
///
/// Claude Code's own instruction slot is file-based
/// (`--append-system-prompt-file`), so `launch` is its default; `read`
/// changes nothing about the managed block, which is byte-identical for both
/// tiers.
///
/// ```
/// use vibe_core::manifest::{ClaudeHarness, HarnessDelivery};
///
/// let declared: ClaudeHarness = toml::from_str("delivery = \"read\"\n")?;
/// assert_eq!(declared.delivery, Some(HarnessDelivery::Read));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeHarness {
    /// How the lane reaches this harness. Absent means `launch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<HarnessDelivery>,
}

/// `[system_prompt.harness.opencode]`.
///
/// OpenCode has **no instruction slot a launch can scope to one session**.
/// Its only additive carrier is a configuration `instructions` array, and
/// the configuration is read per *process*: the same array is injected into
/// every session that process serves, `task` subagents included, which is
/// precisely what `PROP-076#SYS-WORKER-EXCLUSION` forbids. So `read` is this
/// harness's default and a declared `launch` delivers nothing — `vibe agent`
/// starts OpenCode softly with the reason, and `vibe prompt flags` refuses
/// (`#SYS-OPENCODE`). The key exists so a project can *declare* the tier it
/// wants the day OpenCode can scope instructions to one agent, and so the
/// declaration is refused by name rather than ignored.
///
/// ```
/// use vibe_core::manifest::{HarnessDelivery, OpenCodeHarness};
///
/// let declared: OpenCodeHarness = toml::from_str("delivery = \"launch\"\n")?;
/// assert_eq!(declared.delivery, Some(HarnessDelivery::Launch));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenCodeHarness {
    /// How the lane reaches this harness. Absent means `read`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<HarnessDelivery>,
}

/// `[system_prompt.harness.qwen]`.
///
/// Qwen Code's one additive per-launch slot is `--append-system-prompt`,
/// and it takes **text on the command line**. There is no file variant and
/// no additive settings or environment key beside it, so a real lane — ten
/// to a hundred times the 128 KiB a single argument may be — cannot go
/// through it: the launch fails before the program starts. So `read` is
/// this harness's default and a declared `launch` delivers nothing — `vibe
/// agent` starts Qwen Code softly with the reason, and `vibe prompt flags`
/// refuses (`PROP-076#SYS-QWEN`).
///
/// **Worker exclusion is not what closes this tier**, which is the one way
/// it differs from `opencode`: an ordinary Qwen Code subagent is rendered
/// from its own definition and never receives the append text, so the slot
/// would be worker-safe if it could carry the bytes. The key exists so a
/// project can *declare* the tier it wants the day Qwen Code takes those
/// bytes from a file, and so the declaration is refused by name rather than
/// ignored.
///
/// ```
/// use vibe_core::manifest::{HarnessDelivery, QwenHarness};
///
/// let declared: QwenHarness = toml::from_str("delivery = \"launch\"\n")?;
/// assert_eq!(declared.delivery, Some(HarnessDelivery::Launch));
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QwenHarness {
    /// How the lane reaches this harness. Absent means `read`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<HarnessDelivery>,
}

/// `delivery` — which of the three tiers of PROP-076 `#SYS-HARNESS-DELIVERY`
/// carries the lane to a harness.
///
/// `import` is **reserved and refused in version 1**, by name and with its
/// four blockers in the message, exactly as [`LaneForm`]'s unimplemented
/// value is: a setting that parses and is then ignored is the one failure a
/// configuration table must not have.
///
/// ```
/// use vibe_core::manifest::HarnessDelivery;
///
/// assert_eq!(HarnessDelivery::Launch.as_str(), "launch");
/// use vibe_core::manifest::ClaudeHarness;
/// let refusal = toml::from_str::<ClaudeHarness>("delivery = 'import'").unwrap_err();
/// assert!(refusal.to_string().contains("reserved"), "{refusal}");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum HarnessDelivery {
    /// vibe hands the composed lane to the harness's own system or developer
    /// slot through `vibe agent <launcher>`, before the first request.
    Launch,
    /// The router block of the managed block tells the agent to read the
    /// boot files with its own tools.
    Read,
}

impl HarnessDelivery {
    /// The declared spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            HarnessDelivery::Launch => "launch",
            HarnessDelivery::Read => "read",
        }
    }
}

impl fmt::Display for HarnessDelivery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for HarnessDelivery {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "launch" => Ok(HarnessDelivery::Launch),
            "read" => Ok(HarnessDelivery::Read),
            "import" => Err(
                "`delivery = \"import\"` is reserved and refused in version 1, and its four \
                 blockers are each a correctness problem rather than missing work: the three \
                 entrypoint files are byte-identical, so one `@` line reaches every \
                 import-processing harness and delivers the lane twice; the lane is not \
                 import-inert; an import is unconditional and so reaches quiet workers, \
                 against `PROP-076#SYS-WORKER-EXCLUSION`; and it follows a harness's \
                 downward discovery into nested projects. Use `launch` or `read`"
                    .to_owned(),
            ),
            other => Err(format!(
                "`delivery = \"{other}\"` is not a delivery tier; this version accepts \
                 `launch` and `read`, and reserves `import`"
            )),
        }
    }
}

impl From<HarnessDelivery> for String {
    fn from(value: HarnessDelivery) -> Self {
        value.as_str().to_owned()
    }
}

/// `[system_prompt.harness.codex]`.
///
/// Codex truncates a project instruction file at `project_doc_max_bytes` and
/// says nothing about it, and that limit is the *harness's* setting rather
/// than a property of the file — a user's `~/.codex/config.toml` or a trusted
/// project's `.codex/config.toml` may raise it. So the project declares the
/// value it is built for, and vibe measures against the declaration instead
/// of against a constant it cannot know is current.
///
/// ```
/// use vibe_core::manifest::CodexHarness;
///
/// let declared: CodexHarness = toml::from_str("doc_max_bytes = 65536\n")?;
/// assert_eq!(declared.doc_max_bytes.map(|value| value.get()), Some(65_536));
/// // Zero and negative say nothing a budget could mean.
/// assert!(toml::from_str::<CodexHarness>("doc_max_bytes = 0\n").is_err());
/// assert!(toml::from_str::<CodexHarness>("doc_max_bytes = -1\n").is_err());
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexHarness {
    /// How the lane reaches this harness. Absent means `launch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<HarnessDelivery>,
    /// The project-instruction byte limit this project is built for. Absent
    /// means [`CODEX_DOC_MAX_BYTES_DEFAULT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_max_bytes: Option<DocMaxBytes>,
}

/// A positive byte budget. Validated at **deserialisation**, so a value a
/// budget cannot mean refuses while naming its field rather than defaulting
/// to something the author did not write.
///
/// ```
/// use vibe_core::manifest::DocMaxBytes;
///
/// let declared: DocMaxBytes = serde_json::from_str("262144")?;
/// assert_eq!(declared.get(), 262_144);
/// assert!(serde_json::from_str::<DocMaxBytes>("0").is_err());
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct DocMaxBytes(usize);

impl DocMaxBytes {
    /// The declared byte count.
    pub fn get(self) -> usize {
        self.0
    }
}

impl TryFrom<i64> for DocMaxBytes {
    type Error = String;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0 {
            return Err(format!(
                "`[system_prompt.harness.codex] doc_max_bytes` must be a positive number of \
                 bytes, not {value}; omit the field to use Codex's own default of \
                 {CODEX_DOC_MAX_BYTES_DEFAULT}"
            ));
        }
        usize::try_from(value).map(DocMaxBytes).map_err(|_| {
            format!(
                "`[system_prompt.harness.codex] doc_max_bytes` is {value} bytes, past what this \
                 platform can address"
            )
        })
    }
}

impl From<DocMaxBytes> for i64 {
    fn from(value: DocMaxBytes) -> Self {
        i64::try_from(value.0).unwrap_or(i64::MAX)
    }
}
