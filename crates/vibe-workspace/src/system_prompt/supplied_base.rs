//! Reading a harness's own base prompt from versioned data with provenance
//! (PROP-076 `#SYS-SUPPLIED-BASE`).
//!
//! **Why this cell exists.** A harness whose only subagent-safe instruction
//! slot *replaces* its base prompt cannot be given a boot lane unless vibe
//! supplies that base itself — OpenCode's per-agent `system` and Qwen Code's
//! `QWEN_SYSTEM_MD` are both of that shape. The base is therefore data: it
//! ships in the harness's own vibe package, per harness version, beside a
//! `PROVENANCE.toml` that records where every byte came from.
//!
//! **This cell is the GENRE and `compose` is the interpretation of it.**
//! Here is what a `PROVENANCE.toml` may say and what every refusal of it is
//! called; the sibling `supplied_base/compose.rs` applies it. The design
//! rule spans both: no prompt text, no token spelling, no bullet string and
//! no model-matching substring may appear in either, because every one of
//! them is read from the document. A harness version whose composition needs
//! a rule the interpreter cannot express is **refused** —
//! [`Provenance::compose`] answers an error, the caller soft-launches, and a
//! human extends either the document or the interpreter deliberately. A
//! special case in Rust for one harness version would be a literal wearing a
//! different hat, and it would put us back where `#SYS-SUPPLIED-BASE`
//! started.
//!
//! **A wrong base is worse than no lane.** A base from a stale version, or
//! for the wrong model family, produces a session whose harness instructions
//! are not the ones that harness wrote, with no error anywhere. So every
//! answer here is either exact or an error: digests are verified before the
//! bytes are used, `verified = false` refuses, and there is deliberately no
//! nearest-version fallback — a base for 2.2.6 is not a base for 2.2.7.
//!
//! **Where a set lives on disk is not decided here.** That is a host
//! question — an installed application, a settings tree, an environment —
//! and it is answered in the command layer, which can reach the
//! installed-applications index and the package store.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SUPPLIED-BASE");

use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;

/// The schema version this interpreter understands. A document declaring
/// another one is refused rather than read optimistically: the whole point of
/// the genre is that nothing is guessed.
pub const PROVENANCE_SCHEMA: u32 = 1;

/// The file that describes one harness version's bases.
pub const PROVENANCE_FILE: &str = "PROVENANCE.toml";

/// The directory in a harness package that holds the per-version sets.
pub const PROMPT_BASES_DIR: &str = "prompt-bases";

/// The placeholder a document's `lane_token` puts the lane's path at.
pub const LANE_PLACEHOLDER: &str = "%LANE%";

/// One launch's carrier: the environment assignment that hands the harness
/// a configuration layer, the flags that make it arrive, and the name of the
/// agent it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierDocument {
    /// The environment variable the document travels in.
    pub env: String,
    /// The document itself. It holds the base and the lane's path — never
    /// the lane's bytes.
    pub value: String,
    /// The flags a launch must pass for the carrier to be read at all.
    pub flags: Vec<String>,
    /// The agent the document declares and selects.
    pub agent_name: String,
}

/// Why a base could not be supplied. Every variant is a sentence a soft
/// launch prints, because each names a different thing the operator can fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseError {
    /// No set for this harness version. Named rather than approximated:
    /// there is no nearest-version fallback by design.
    NoSet {
        harness: String,
        version: String,
        looked_in: PathBuf,
    },
    /// The document is there and unreadable, or not valid TOML.
    Unreadable { path: PathBuf, reason: String },
    /// A schema this build does not interpret.
    Schema { found: u32, understood: u32 },
    /// The document describes another harness or another version than the one
    /// asked for — a mis-filed directory, which must not be used silently.
    Mismatch {
        wanted: String,
        found: String,
        field: &'static str,
    },
    /// The proof has not been run for this set, so no launch may deliver it.
    NotVerified { harness: String, version: String },
    /// A recorded file is missing from the set.
    MissingFile { file: String },
    /// A recorded digest does not match the bytes on disk — the one failure
    /// that means the data itself is not what it claims.
    DigestMismatch {
        file: String,
        expected: String,
        found: String,
    },
    /// The launch's model is not knowable, so no selection row can be
    /// applied and the default would be a guess for a family that has its
    /// own base.
    UnknownModel,
    /// A rule the document states and this interpreter cannot express.
    Unsupported { what: String },
}

impl fmt::Display for BaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseError::NoSet {
                harness,
                version,
                looked_in,
            } => write!(
                f,
                "no prompt-base set for {harness} {version}: `{}` is not there. The set ships \
                 with the harness's own vibe package, one directory per harness version, and \
                 there is deliberately no fallback to another version's base",
                looked_in.display()
            ),
            BaseError::Unreadable { path, reason } => {
                write!(f, "`{}` could not be read ({reason})", path.display())
            }
            BaseError::Schema { found, understood } => write!(
                f,
                "the prompt-base provenance declares schema {found} and this build interprets \
                 {understood}"
            ),
            BaseError::Mismatch {
                wanted,
                found,
                field,
            } => write!(
                f,
                "the prompt-base provenance is for `{field}` {found}, not {wanted} — a mis-filed \
                 set is never used on the chance that it fits"
            ),
            BaseError::NotVerified { harness, version } => write!(
                f,
                "the prompt-base set for {harness} {version} is not verified: `verified` is \
                 false, so the base it carries has not been measured against the installed \
                 program and may not be delivered"
            ),
            BaseError::MissingFile { file } => {
                write!(
                    f,
                    "the prompt-base set records `{file}`, which is not in it"
                )
            }
            BaseError::DigestMismatch {
                file,
                expected,
                found,
            } => write!(
                f,
                "`{file}` in the prompt-base set does not match its recorded digest (expected \
                 {expected}, found {found}) — the data is not what its provenance claims"
            ),
            BaseError::UnknownModel => write!(
                f,
                "this launch's model could not be determined, and the base a harness uses \
                 depends on it, so no base may be chosen"
            ),
            BaseError::Unsupported { what } => write!(
                f,
                "the prompt-base provenance states a rule this build cannot express ({what}); \
                 it is refused rather than approximated"
            ),
        }
    }
}

/// One base file as the provenance records it.
#[derive(Debug, Clone, Deserialize)]
pub struct Base {
    /// The id selection rows refer to.
    pub id: String,
    /// Its name inside the set's directory.
    pub file: String,
    /// Where it came from upstream, for a reader rather than for the code.
    #[serde(default)]
    pub upstream_path: String,
    /// The length its bytes must have.
    pub bytes: usize,
    /// The digest its bytes must have.
    pub sha256: String,
    /// Whether this file carries the render token at all. Recorded because a
    /// base without it receives no rendered text, which is a visible
    /// behavioural difference rather than an optimisation.
    #[serde(default)]
    pub has_tool_guidance_token: bool,
    /// The base used when no selection row matches.
    #[serde(default)]
    pub default: bool,
}

/// A literal substitution a selection row applies to its base.
#[derive(Debug, Clone, Deserialize)]
pub struct Substitute {
    /// The literal token to replace.
    pub token: String,
    /// Which launch value replaces it. A name this interpreter knows, so an
    /// unknown one is a refusal rather than an empty string.
    pub value: String,
    /// `all` or `first`.
    #[serde(default = "all_occurrences")]
    pub occurrences: String,
}

fn all_occurrences() -> String {
    "all".to_owned()
}

/// How a base replaces or extends the default one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// This base replaces the default base.
    Override,
    /// The default base is kept and this one is appended behind a blank line.
    Append,
}

/// One row of the model→base table. Rows are tried in order and the first
/// whose every substring is present wins, which is how a recorded order
/// reproduces an upstream conditional without a special case in code.
#[derive(Debug, Clone, Deserialize)]
pub struct Select {
    /// Every one of these must appear in the lower-cased model id.
    pub substrings: Vec<String>,
    /// The [`Base::id`] this row selects.
    pub base: String,
    /// `override` or `append`.
    pub mode: String,
    /// Where the row was read from, for a human reading the record.
    #[serde(default)]
    pub source: String,
    /// Literal substitutions this row's base needs.
    #[serde(default)]
    pub substitute: Vec<Substitute>,
}

/// One gated line of the rendered guidance.
#[derive(Debug, Clone, Deserialize)]
pub struct Bullet {
    /// The tool whose presence in the launch's tool list adds this line.
    pub gate: String,
    /// The line itself.
    pub text: String,
}

/// How the harness expands its tool-guidance token.
#[derive(Debug, Clone, Deserialize)]
pub struct Render {
    /// The literal token.
    pub token: String,
    /// `first` or `all`.
    #[serde(default = "first_occurrence")]
    pub replace: String,
    /// What the kept bullets are joined with.
    #[serde(default = "newline")]
    pub join: String,
    /// Where the rule was read from.
    #[serde(default)]
    pub source: String,
    /// What the harness does to the composed base's own ends: `none`, or
    /// `end` for a program that emits its base with trailing whitespace
    /// removed.
    ///
    /// **Recorded rather than assumed, because it is one byte and it moves
    /// the seam.** OpenCode 2.2.7's `system.txt` ends with a newline and the
    /// installed binary's own first system part is 1,470 bytes rather than
    /// 1,471; a composition that kept that newline would put three of them
    /// between the base's last sentence and the lane's first marker, where a
    /// plain run has two — and condition (ii) of `#SYS-SUPPLIED-BASE` would
    /// then see the difference inside the insertion rather than beside it.
    #[serde(default = "no_trim")]
    pub trim: String,
    /// The bullets, in the order the harness emits them — which is the
    /// order written here, not the order of the tool list.
    #[serde(default)]
    pub bullet: Vec<Bullet>,
}

fn first_occurrence() -> String {
    "first".to_owned()
}

fn no_trim() -> String {
    "none".to_owned()
}

fn newline() -> String {
    "\n".to_owned()
}

/// How the lane is handed over, as the document states it.
#[derive(Debug, Clone, Deserialize)]
pub struct Carrier {
    /// The mechanism's name. An unknown one is refused by the caller.
    pub kind: String,
    /// The environment variable the configuration travels in.
    #[serde(default)]
    pub env: String,
    /// Flags the launch must pass for the carrier to apply.
    #[serde(default)]
    pub flags: Vec<String>,
    /// The carrier document's own key names, so vibe's Rust spells no
    /// configuration key of another program: the table of agents, the key
    /// inside one agent that takes the composed prompt, the key that
    /// declares its mode, and the key that selects the launch's primary
    /// agent.
    #[serde(default)]
    pub agents_key: String,
    /// The key inside the agent table that takes the composed prompt.
    #[serde(default)]
    pub system_key: String,
    /// The key that declares an agent's mode.
    #[serde(default)]
    pub mode_key: String,
    /// The key that selects the launch's primary agent by name.
    #[serde(default)]
    pub default_agent_key: String,
    /// The agent name vibe declares for its own launch.
    #[serde(default)]
    pub agent_name: String,
    /// The mode that agent is declared in.
    #[serde(default)]
    pub agent_mode: String,
    /// The gate tool names a launch through this carrier offers.
    ///
    /// **Data, because the render rule is gated on it and the launch cannot
    /// ask.** The carrier declares a `primary` agent with no tool
    /// restrictions of its own, so the launch receives the harness's default
    /// tool set; which of the recorded gates that set contains is a fact
    /// about the harness version, recorded here beside the bullets those
    /// gates admit and proved by the whole-request diff rather than assumed
    /// by vibe.
    #[serde(default)]
    pub gate_tools: Vec<String>,
    /// The token in the composed text that stands for the lane's path.
    #[serde(default)]
    pub lane_token: String,
    /// What the harness does to the inserted lane: `none` or `trim`.
    #[serde(default)]
    pub lane_transform: String,
    /// What goes between the base's last byte and the lane's first.
    #[serde(default)]
    pub separator_between_base_and_lane: String,
}

/// What has been proved about this set.
#[derive(Debug, Clone, Deserialize)]
pub struct Verification {
    /// True only when the whole-request proof has been run for this exact
    /// harness version. Nothing but that proof may set it.
    #[serde(default)]
    pub verified: bool,
    /// The proof's identifier, for the record.
    #[serde(default)]
    pub proof_id: String,
    /// When it was run.
    #[serde(default)]
    pub proof_date: String,
}

/// Where a set's bytes came from upstream.
///
/// Read by the sync tool rather than by a launch: a launch needs the base
/// and the digests, and provenance is for the human and the tool that
/// produce the next version.
#[derive(Debug, Clone, Deserialize)]
pub struct Upstream {
    /// The upstream repository.
    #[serde(default)]
    pub repository: String,
    /// The tag these bytes were read at.
    #[serde(default)]
    pub tag: String,
    /// The commit that tag pointed at, because a tag can move.
    #[serde(default)]
    pub commit: String,
    /// The licence that governs the vendored text.
    #[serde(default)]
    pub license: Option<License>,
}

/// The upstream licence carried beside the text it governs.
#[derive(Debug, Clone, Deserialize)]
pub struct License {
    /// Its SPDX identifier, as the record states it.
    #[serde(default)]
    pub spdx: String,
    /// The copyright line the licence asks be included.
    #[serde(default)]
    pub copyright: String,
    /// Its upstream path.
    #[serde(default)]
    pub upstream_path: String,
    /// Its name inside the set's directory.
    #[serde(default)]
    pub file: String,
    /// Its length.
    #[serde(default)]
    pub bytes: usize,
    /// Its digest.
    #[serde(default)]
    pub sha256: String,
}

/// One drift tripwire: a file upstream whose change could alter the
/// composition this set describes.
///
/// **A tripwire and not a proof.** A matching digest says the upstream
/// source has not been edited; it cannot say the published artifact carries
/// the tag's bytes, because the shipped program inlines these files through
/// a bundler. Only the whole-request measurement can say that.
#[derive(Debug, Clone, Deserialize)]
pub struct Fingerprint {
    /// The upstream path.
    pub path: String,
    /// Its digest, when one was recorded. A row with none is a file whose
    /// *existence* in the list matters — a reader's map of what decides
    /// what — rather than one the tool compares.
    #[serde(default)]
    pub sha256: Option<String>,
    /// What this file decides, in one clause, for whoever reads a diff of
    /// it.
    #[serde(default)]
    pub decides: String,
}

/// One harness version's prompt-base set.
#[derive(Debug, Clone, Deserialize)]
pub struct Provenance {
    /// The document schema.
    pub schema: u32,
    /// Which harness this describes.
    pub harness: String,
    /// Which version of it.
    pub harness_version: String,
    /// The base files.
    #[serde(default)]
    pub base: Vec<Base>,
    /// The model→base table, in order.
    #[serde(default)]
    pub select: Vec<Select>,
    /// The token-expansion rule.
    pub render: Option<Render>,
    /// The hand-over mechanism.
    pub carrier: Option<Carrier>,
    /// What has been proved.
    #[serde(default)]
    pub verification: Option<Verification>,
    /// Where the bytes came from. Read by the sync tool.
    #[serde(default)]
    pub upstream: Option<Upstream>,
    /// The drift tripwires. Read by the sync tool.
    #[serde(default)]
    pub fingerprint: Vec<Fingerprint>,
}

#[path = "supplied_base/compose.rs"]
mod compose;
pub use compose::{Chosen, Launch};

#[cfg(test)]
#[path = "supplied_base/tests.rs"]
mod tests;
