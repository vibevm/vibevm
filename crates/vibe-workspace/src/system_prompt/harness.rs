//! Harness **kinds** and the launch flags each one takes.
//!
//! A kind — `claude`, `codex`, a closed set — is the flag dialect and the
//! delivery rules: which slot, which flags, which refusals. A **launcher**
//! is a name bound to a kind and to the argv that starts it, and only a user
//! may name one; that half lives in the command layer, because the program
//! that runs is a machine fact and this cell holds no opinion about it
//! (`#SYS-HARNESS-EXECUTABLE`).
//!
//! **Flags only, never a program.** Everything here composes argument
//! vectors a caller appends to whatever wrapper, alias or launcher a
//! developer already has. That is the property that makes `vibe prompt
//! flags` useful at all, and it is why the kind and the launcher are
//! separate things.
//!
//! **No lane bytes ever go on a command line.** A single argument is capped
//! at 128 KiB on Linux and the whole command line at about 32,767 characters
//! on Windows, against a lane of roughly 310 KB — so every launch mechanism
//! here is file-based and a harness with no file-based slot simply has no
//! launch tier (`#SYS-HARNESS-DELIVERY`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY");

use std::path::Path;

use vibe_core::manifest::{HarnessDelivery, HarnessTable, Manifest};

use super::qwen::QwenCarrier;
use super::supplied_base::CarrierDocument;

#[path = "harness/notes.rs"]
mod notes;
use notes::no_slot_note;
pub use notes::{
    OPENCODE_MEASURED_VERSION, QWEN_MEASURED_VERSION, opencode_carrier_description,
    qwen_carrier_description, qwen_closed_slots,
};

/// The closed set of harness kinds this version has a flag dialect for.
///
/// **Membership is not a launch tier**, and each of these four happens to
/// have one. A kind is here when vibe knows its dialect well enough to
/// answer "what does a launch of this pass" — and for the two harnesses
/// whose only subagent-safe slot *replaces* rather than adds, that answer
/// required vibe to supply the replaced text itself: OpenCode's whole base
/// prompt from versioned data, and Qwen Code's one identity sentence sampled
/// from the installed program at launch (`#SYS-OPENCODE`, `#SYS-QWEN`,
/// `#SYS-SUPPLIED-BASE`).
///
/// ```
/// use vibe_workspace::system_prompt::HarnessKind;
///
/// assert_eq!(HarnessKind::parse("claude"), Some(HarnessKind::Claude));
/// assert_eq!(HarnessKind::parse("opencode"), Some(HarnessKind::OpenCode));
/// assert_eq!(HarnessKind::parse("qwen"), Some(HarnessKind::Qwen));
/// assert_eq!(HarnessKind::parse("gemini"), None);
/// assert_eq!(HarnessKind::Codex.as_str(), "codex");
/// assert_eq!(HarnessKind::ALL.len(), 4, "the set is closed at four");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessKind {
    /// Claude Code — `--append-system-prompt-file`, a hidden but present
    /// file-based slot.
    Claude,
    /// Codex — `developer_instructions` through a launch-time profile file.
    Codex,
    /// OpenCode — one per-launch configuration layer declaring vibe's own
    /// primary agent, whose `system` is OpenCode's **own** base followed by
    /// the lane (`#SYS-OPENCODE`, `#SYS-SUPPLIED-BASE`).
    OpenCode,
    /// Qwen Code — `QWEN_SYSTEM_IDENTITY_MD`, a file that replaces the
    /// base prompt's **first paragraph** and nothing else, carrying Qwen
    /// Code's own identity sentence for this launch and then the lane
    /// (`#SYS-QWEN`).
    Qwen,
}

impl HarnessKind {
    /// Every kind, in the order a refusal lists them.
    pub const ALL: [HarnessKind; 4] = [
        HarnessKind::Claude,
        HarnessKind::Codex,
        HarnessKind::OpenCode,
        HarnessKind::Qwen,
    ];

    /// The kind's own name, which is also the manifest's harness key.
    pub fn as_str(self) -> &'static str {
        match self {
            HarnessKind::Claude => "claude",
            HarnessKind::Codex => "codex",
            HarnessKind::OpenCode => "opencode",
            HarnessKind::Qwen => "qwen",
        }
    }

    /// Parse a declared kind. `None` is an unknown kind, which a caller
    /// refuses **naming the field** rather than falling back to a default:
    /// a launcher bound to a kind nobody implements must not quietly start
    /// the program with no flags at all.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    /// The flags that must not already be in a user's own arguments,
    /// because passing two of them is how a session silently gets a system
    /// prompt nobody chose.
    pub fn conflicting_flags(self) -> &'static [&'static str] {
        match self {
            HarnessKind::Claude => &[
                "--append-system-prompt-file",
                "--append-system-prompt",
                "--system-prompt",
                "--system-prompt-file",
            ],
            HarnessKind::Codex => &["-p", "--profile"],
            // **Not a second prompt but a bypass of the first.** vibe's
            // carrier selects its own primary agent by name, and a user's
            // own `--agent` selects a different one — whose `system` is not
            // the one carrying the lane. The session then starts with no
            // boot lane and nothing says so, which is the same silence
            // every other entry here guards (`#SYS-OPENCODE`).
            HarnessKind::OpenCode => &["--agent"],
            // **Nothing, although this kind now delivers.** vibe passes
            // Qwen Code no flag at all — its carrier is one environment
            // path — so a flag of the user's own is never a *second* one
            // beside ours. The arguments that would defeat the carrier
            // (`--system-prompt`, `--safe-mode`, `--bare`) are handled the
            // other way round, as **soft-launch** preconditions rather than
            // refusals: each is a user asking Qwen Code for something the
            // carrier would contradict, and honouring the request while
            // saying what was not delivered beats refusing to start their
            // session (`#SYS-QWEN`, `#SYS-SOFT-LAUNCH`).
            HarnessKind::Qwen => &[],
        }
    }

    /// The vibe package that carries this harness's own prompt bases, when
    /// one exists.
    ///
    /// **A coordinate is packaging, not prompt text.** `#SYS-SUPPLIED-BASE`
    /// names it itself — "for OpenCode that is `org.vibevm.opencode/
    /// opencode`" — because the whole point of the rule is that the *data*
    /// ships with the harness's package; knowing which package that is, is
    /// how the lookup starts. Nothing about a prompt is decided here.
    pub fn base_package(self) -> Option<(&'static str, &'static str)> {
        match self {
            HarnessKind::OpenCode => Some(("org.vibevm.opencode", "opencode")),
            // Claude Code and Codex have additive slots and need no base.
            // Qwen Code's base is a function of the session rather than of
            // the version and so could never be recorded as data — which is
            // exactly why its carrier replaces one *sentence* and samples
            // that sentence from the installed program instead
            // (`#SYS-QWEN`).
            HarnessKind::Claude | HarnessKind::Codex | HarnessKind::Qwen => None,
        }
    }

    /// This kind's declared delivery for a node.
    ///
    /// A node with no `[system_prompt]` table at all takes the **table's own
    /// per-kind default**, not one default for every kind: `launch` for the
    /// two harnesses with a file-based slot and `read` for the rest. Reading
    /// `Launch` for every kind here was harmless while the set held only
    /// those two and is not once it does not.
    pub fn delivery(self, manifest: &Manifest) -> HarnessDelivery {
        manifest.system_prompt.as_ref().map_or_else(
            || HarnessTable::default().delivery(self.as_str()),
            |section| section.harness.delivery(self.as_str()),
        )
    }
}

/// What one launch needs beyond the program: the flags, the notes a human
/// should read, and the profile file a Codex launch writes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchFlags {
    /// The argument vector, in order.
    pub flags: Vec<String>,
    /// The environment assignments the launch makes for the harness it
    /// starts, in order. Empty for every kind whose carrier is a flag or a
    /// file.
    pub env: Vec<EnvAssignment>,
    /// Sentences a caller prints beside the flags — never part of them.
    pub notes: Vec<String>,
    /// The profile file a Codex launch must write before exec, or `None`.
    pub profile: Option<CodexProfile>,
}

/// One environment assignment a launch makes for the harness it starts.
///
/// **Scoped to the child and to nothing else.** It is set on the process
/// vibe execs, and is never exported, written to a profile or persisted, so
/// it cannot reach another session or another project — which is the
/// per-launch half of `#SYS-WORKER-EXCLUSION`, and the reason an
/// environment carrier is admissible here at all when an ambient
/// configuration file would not be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvAssignment {
    /// The variable's name.
    pub name: String,
    /// Its value. For OpenCode this is one configuration document carrying
    /// the harness's own base prompt and the lane's **path** — never the
    /// lane's bytes (`#SYS-HARNESS-DELIVERY`).
    pub value: String,
}

/// The Codex launch carrier: a profile file beside `$CODEX_HOME/config.toml`.
///
/// It is written by `vibe agent` at launch and by nothing else — never by an
/// install, because a file outside the project is not an install's to write,
/// and never by `vibe prompt flags`, which only prints. The alternative was
/// no launch tier for Codex at all: its ambient carriers
/// (`.codex/config.toml`) reach quiet workers, and the lane cannot go on a
/// command line (`#SYS-HARNESS-DELIVERY`, decisions D3 and D4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexProfile {
    /// The profile name, which is also the file's stem.
    pub name: String,
    /// The file's basename inside `$CODEX_HOME`.
    pub file_name: String,
    /// The file's whole contents.
    pub body: String,
}

/// What one launch knows that the flags are composed from.
///
/// A struct rather than a parameter list because the kinds need disjoint
/// parts of it: Claude Code wants only the path, Codex the bytes and the
/// limit, OpenCode a carrier document another cell had to read a package to
/// compose. Threading five positional arguments through one call that uses
/// two of them is how the wrong one gets passed.
#[derive(Debug, Clone, Copy)]
pub struct LaunchInputs<'a> {
    /// The node's router id, which names Codex's profile file.
    pub node_id: &'a str,
    /// The absolute path of the composed `AGENT-LANE.md`.
    pub lane: &'a Path,
    /// The lane's content. Needed only by Codex, whose carrier is a TOML
    /// value rather than a path.
    pub lane_bytes: &'a str,
    /// The `project_doc_max_bytes` this node declares for Codex.
    pub codex_limit: usize,
    /// The carrier document a supplied-base harness launches with, when one
    /// could be composed for this launch. `None` means the composition
    /// refused, and the caller's gate has already turned that into a soft
    /// launch (`#SYS-SUPPLIED-BASE`, `#SYS-SOFT-LAUNCH`).
    pub supplied: Option<&'a CarrierDocument>,
    /// Qwen Code's composed identity carrier, when one could be composed
    /// for this launch. A separate field from `supplied` and not a second
    /// spelling of it: that one is a configuration *document* with a base
    /// prompt inside it, and this one is a **file on disk** the harness
    /// re-reads on every prompt rebuild, so the two carry different facts
    /// and a launch that confused them would pass a document where a path
    /// belongs (`#SYS-QWEN`).
    pub qwen: Option<&'a QwenCarrier>,
}

/// Compose one kind's launch flags.
///
/// A kind in `read` delivery takes **no** flags and says so in one line:
/// the project asked for the router to do the work, and adding the slot
/// anyway would deliver the lane twice.
pub fn launch_flags(
    kind: HarnessKind,
    delivery: HarnessDelivery,
    inputs: &LaunchInputs<'_>,
) -> LaunchFlags {
    if delivery == HarnessDelivery::Read {
        let mut notes = vec![format!(
            "`[system_prompt.harness.{}] delivery = \"read\"`, so the harness starts \
             unchanged and the managed block's router has the agent read the boot files \
             itself — no slot flag is passed",
            kind.as_str()
        )];
        notes.extend(no_slot_note(kind, inputs.lane));
        return LaunchFlags {
            flags: Vec::new(),
            env: Vec::new(),
            notes,
            profile: None,
        };
    }
    match kind {
        HarnessKind::Claude => LaunchFlags {
            flags: vec![
                "--append-system-prompt-file".to_owned(),
                inputs.lane.display().to_string(),
                "--exclude-dynamic-system-prompt-sections".to_owned(),
            ],
            env: Vec::new(),
            notes: vec![
                "`--exclude-dynamic-system-prompt-sections` moves cwd, env info, memory paths \
                 and git status into the first user message, which keeps the slot's bytes \
                 cacheable across sessions"
                    .to_owned(),
            ],
            profile: None,
        },
        HarnessKind::Codex => {
            let profile = codex_profile(inputs.node_id, inputs.lane_bytes);
            let limit = inputs.codex_limit;
            LaunchFlags {
                flags: vec![
                    "-p".to_owned(),
                    profile.name.clone(),
                    "-c".to_owned(),
                    format!("project_doc_max_bytes={limit}"),
                ],
                env: Vec::new(),
                notes: vec![format!(
                    "`vibe agent codex` writes `$CODEX_HOME/{}` at launch and selects it with \
                     `-p {}`; the lane cannot go on a command line, and Codex's ambient \
                     project config would reach quiet workers",
                    profile.file_name, profile.name
                )],
                profile: Some(profile),
            }
        }
        // **The carrier is an environment assignment, and the flags only
        // make it reachable.** `--standalone` is not an option: OpenCode's
        // default mode serves sessions from one long-lived background
        // service that never sees this launch's environment, so without it
        // the document does not arrive at all (`#SYS-OPENCODE`).
        HarnessKind::OpenCode => match inputs.supplied {
            Some(carrier) => LaunchFlags {
                flags: carrier.flags.clone(),
                env: vec![EnvAssignment {
                    name: carrier.env.clone(),
                    value: carrier.value.clone(),
                }],
                notes: vec![format!(
                    "`{}` carries one configuration layer declaring the primary agent `{}`, \
                     whose prompt is OpenCode's OWN base for this version and model followed \
                     by `{}` — the lane's path, never its bytes. The agent is selected for \
                     this launch alone, a `subagent` child has a `system` of its own and \
                     receives none of it, and `--standalone` is required because OpenCode's \
                     default mode serves sessions from a background service that never sees \
                     this environment",
                    carrier.env,
                    carrier.agent_name,
                    inputs.lane.display()
                )],
                profile: None,
            },
            // A caller that reaches here declared `launch` and the
            // composition refused; the gate above it has already turned
            // that into a soft launch or a refusal naming which refusal it
            // was (`#SYS-SUPPLIED-BASE`, `#SYS-SOFT-LAUNCH`).
            None => LaunchFlags {
                flags: Vec::new(),
                env: Vec::new(),
                notes: no_slot_note(kind, inputs.lane).into_iter().collect(),
                profile: None,
            },
        },
        // **The carrier is an environment path and there are no flags at
        // all.** Qwen Code reads the variable on its own prompt path, so
        // nothing has to be passed to make it reachable — which is why this
        // kind's `conflicting_flags` is empty even under `launch`.
        HarnessKind::Qwen => match inputs.qwen {
            Some(carrier) => LaunchFlags {
                flags: Vec::new(),
                env: vec![EnvAssignment {
                    name: carrier.env.clone(),
                    value: carrier.path.display().to_string(),
                }],
                notes: vec![super::qwen::qwen_carrier_note(carrier, inputs.lane)],
                profile: None,
            },
            // A caller that reaches here declared `launch` and the
            // composition refused; the gate above it has already turned
            // that into a soft launch or a refusal naming which it was
            // (`#SYS-QWEN`, `#SYS-SOFT-LAUNCH`).
            None => LaunchFlags {
                flags: Vec::new(),
                env: Vec::new(),
                notes: no_slot_note(kind, inputs.lane).into_iter().collect(),
                profile: None,
            },
        },
    }
}

/// The profile file for one node.
///
/// **The key is at the file's top level, and that is the whole mechanism.**
/// `-p <name>` *layers* `$CODEX_HOME/<name>.config.toml` **on top of the
/// base user config** — codex-cli's own words for the flag — so the file is
/// read as a config file, not as a container of `[profiles.*]` tables. A
/// `[profiles.<name>]` table written inside it is an ordinary nested table
/// that codex-cli loads and never reads, which is exactly what P5 measured
/// and recorded as a falsified design claim. Top-level, the same value
/// arrives (`#SYS-CODEX-PROFILE-TOP-LEVEL`).
///
/// Layering rather than replacing is also why this carrier beats every
/// alternative: the user's own `config.toml` stays in force underneath, so
/// nothing of their configuration and nothing of Codex's base instructions
/// is lost, and one small file plus two short arguments is the entire
/// footprint — no overlay home, no symlinked databases, no copied
/// credentials, and nothing that behaves differently on Windows.
///
/// The value is emitted as a TOML **basic** string with escapes, never a
/// `'''` literal block: a literal block cannot contain `'''`, and the lane
/// is arbitrary authored text (`HARNESS-DELIVERY.md` §5).
fn codex_profile(node_id: &str, lane_bytes: &str) -> CodexProfile {
    let name = format!("vibe-{}", slug(node_id));
    CodexProfile {
        file_name: format!("{name}.config.toml"),
        body: format!(
            "# Generated by vibe at launch — do not edit; it is rewritten on every\n\
             # `vibe agent codex` and carries this node's composed boot lane.\n\
             #\n\
             # `-p {name}` layers this file on top of `$CODEX_HOME/config.toml`, so the\n\
             # key below is top-level. Under a `profiles.{name}` table header it would\n\
             # be a nested table codex-cli loads and never reads.\n\
             developer_instructions = {}\n",
            toml_basic_string(lane_bytes)
        ),
        name,
    }
}

/// The codex-cli version whose `developer_instructions` carrier this build
/// has verified, offline and at the real lane size.
///
/// It is **one** version and not a floor, because the claim is a
/// measurement: the carrier is a configuration key whose per-profile
/// behaviour already changed once under us, and its failure mode is silent.
/// A profile file codex does not read produces no warning, no error and no
/// non-zero exit — the session simply starts with no boot lane, which is the
/// one outcome a launch tier must never reach quietly. So an unverified
/// version is refused rather than attempted (`#SYS-CODEX-PROFILE-TOP-LEVEL`).
pub const CODEX_VERIFIED_VERSION: &str = "0.167.0";

/// What an installed harness's reported version says about its carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarrierVerdict {
    /// The reported version is one this build verified.
    Verified,
    /// The version was read and is not one this build verified.
    Unverified {
        /// The version as reported.
        installed: String,
    },
    /// The version could not be read, so nothing is claimed either way.
    Unknown {
        /// Why it could not be read.
        reason: String,
    },
}

/// Judge a `codex --version` line.
///
/// ```
/// use vibe_workspace::system_prompt::{CarrierVerdict, codex_carrier_verdict};
///
/// assert_eq!(codex_carrier_verdict("codex-cli 0.167.0"), CarrierVerdict::Verified);
/// assert!(matches!(
///     codex_carrier_verdict("codex-cli 0.168.0"),
///     CarrierVerdict::Unverified { .. }
/// ));
/// assert!(matches!(
///     codex_carrier_verdict(""),
///     CarrierVerdict::Unknown { .. }
/// ));
/// ```
pub fn codex_carrier_verdict(reported: &str) -> CarrierVerdict {
    let Some(version) = parse_version(reported) else {
        return CarrierVerdict::Unknown {
            reason: format!("`{}` carries no version number", reported.trim()),
        };
    };
    if version == CODEX_VERIFIED_VERSION {
        CarrierVerdict::Verified
    } else {
        CarrierVerdict::Unverified { installed: version }
    }
}

/// The version number out of a `--version` line, or `None` when the line
/// carries none.
///
/// Public because a kind whose carrier is gated on *data* rather than on a
/// constant still has to turn the program's own answer into the version
/// string that names a directory — and reading it out of a
/// [`CarrierVerdict`] would be reading a Codex judgement for an OpenCode
/// question.
///
/// ```
/// use vibe_workspace::system_prompt::reported_version;
///
/// assert_eq!(reported_version("opencode v2.2.7").as_deref(), Some("2.2.7"));
/// assert_eq!(reported_version(""), None);
/// ```
pub fn reported_version(reported: &str) -> Option<String> {
    parse_version(reported)
}

/// The version number out of a `--version` line: the last whitespace-
/// separated token that starts with a digit, with a leading `v` dropped.
fn parse_version(reported: &str) -> Option<String> {
    reported
        .split_whitespace()
        .map(|token| token.trim_start_matches('v'))
        .rfind(|token| token.starts_with(|character: char| character.is_ascii_digit()))
        .map(str::to_owned)
}

/// A node id as a profile name: every character outside `[A-Za-z0-9_-]`
/// becomes `-`, so a grouped coordinate like `org.vibevm/demo` is one legal
/// stem and two different coordinates cannot collide on a shortened name.
///
/// **The dot is replaced too, and that is not cosmetic.** A profile name
/// goes into a TOML table header, where `a.b` is a *nested* table — so
/// `[profiles.org.vibevm/demo]` would declare `profiles > org > vibevm > …`
/// and the profile `-p` asks for would not exist. Quoting the header would
/// work; not needing to is better, because the same string is also the
/// file's stem and an argument on a command line.
fn slug(node_id: &str) -> String {
    node_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

/// One TOML basic string, escaped as the specification requires.
fn toml_basic_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            other if (other as u32) < 0x20 || other == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", other as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[path = "harness/tests.rs"]
mod tests;
