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

/// The closed set of harness kinds this version has a flag dialect for.
///
/// **Membership is not a launch tier.** A kind is here when vibe knows its
/// dialect well enough to answer "what does a launch of this pass", and
/// `opencode`'s answer is "nothing, and here is why" — which is an answer
/// worth having, because without the kind `vibe agent opencode` would not
/// exist and `[system_prompt.harness.opencode]` could not be declared
/// (`#SYS-OPENCODE`).
///
/// ```
/// use vibe_workspace::system_prompt::HarnessKind;
///
/// assert_eq!(HarnessKind::parse("claude"), Some(HarnessKind::Claude));
/// assert_eq!(HarnessKind::parse("opencode"), Some(HarnessKind::OpenCode));
/// assert_eq!(HarnessKind::parse("gemini"), None);
/// assert_eq!(HarnessKind::Codex.as_str(), "codex");
/// assert_eq!(HarnessKind::ALL.len(), 3, "the set is closed at three");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessKind {
    /// Claude Code — `--append-system-prompt-file`, a hidden but present
    /// file-based slot.
    Claude,
    /// Codex — `developer_instructions` through a launch-time profile file.
    Codex,
    /// OpenCode — a launcher and a dialect, but **no deliverable slot**: its
    /// only additive carrier is process-wide and reaches `task` subagents
    /// (`#SYS-OPENCODE`).
    OpenCode,
}

impl HarnessKind {
    /// Every kind, in the order a refusal lists them.
    pub const ALL: [HarnessKind; 3] = [
        HarnessKind::Claude,
        HarnessKind::Codex,
        HarnessKind::OpenCode,
    ];

    /// The kind's own name, which is also the manifest's harness key.
    pub fn as_str(self) -> &'static str {
        match self {
            HarnessKind::Claude => "claude",
            HarnessKind::Codex => "codex",
            HarnessKind::OpenCode => "opencode",
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
            // Nothing: vibe passes this kind no flag under any delivery, so
            // nothing of the user's could be a second one beside ours.
            HarnessKind::OpenCode => &[],
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
    /// Sentences a caller prints beside the flags — never part of them.
    pub notes: Vec<String>,
    /// The profile file a Codex launch must write before exec, or `None`.
    pub profile: Option<CodexProfile>,
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

/// Compose one kind's launch flags.
///
/// `lane` is the absolute path of the composed `AGENT-LANE.md`; `lane_bytes`
/// is its content, needed only by Codex, whose carrier is a TOML value
/// rather than a path. A kind in `read` delivery takes **no** flags and says
/// so in one line: the project asked for the router to do the work, and
/// adding the slot anyway would deliver the lane twice.
pub fn launch_flags(
    kind: HarnessKind,
    delivery: HarnessDelivery,
    node_id: &str,
    lane: &Path,
    lane_bytes: &str,
    codex_limit: usize,
) -> LaunchFlags {
    if delivery == HarnessDelivery::Read {
        let mut notes = vec![format!(
            "`[system_prompt.harness.{}] delivery = \"read\"`, so the harness starts \
             unchanged and the managed block's router has the agent read the boot files \
             itself — no slot flag is passed",
            kind.as_str()
        )];
        notes.extend(no_slot_note(kind, lane));
        return LaunchFlags {
            flags: Vec::new(),
            notes,
            profile: None,
        };
    }
    match kind {
        HarnessKind::Claude => LaunchFlags {
            flags: vec![
                "--append-system-prompt-file".to_owned(),
                lane.display().to_string(),
                "--exclude-dynamic-system-prompt-sections".to_owned(),
            ],
            notes: vec![
                "`--exclude-dynamic-system-prompt-sections` moves cwd, env info, memory paths \
                 and git status into the first user message, which keeps the slot's bytes \
                 cacheable across sessions"
                    .to_owned(),
            ],
            profile: None,
        },
        HarnessKind::Codex => {
            let profile = codex_profile(node_id, lane_bytes);
            LaunchFlags {
                flags: vec![
                    "-p".to_owned(),
                    profile.name.clone(),
                    "-c".to_owned(),
                    format!("project_doc_max_bytes={codex_limit}"),
                ],
                notes: vec![format!(
                    "`vibe agent codex` writes `$CODEX_HOME/{}` at launch and selects it with \
                     `-p {}`; the lane cannot go on a command line, and Codex's ambient \
                     project config would reach quiet workers",
                    profile.file_name, profile.name
                )],
                profile: Some(profile),
            }
        }
        // **No flags, and the note says what the carrier would have been.**
        // A caller that reaches here declared `launch` for a kind whose
        // carrier vibe will not deliver; the composition still answers, and
        // the carrier gate above it turns the answer into a soft launch or
        // a refusal (`#SYS-OPENCODE`, `#SYS-SOFT-LAUNCH`).
        HarnessKind::OpenCode => LaunchFlags {
            flags: Vec::new(),
            notes: no_slot_note(kind, lane).into_iter().collect(),
            profile: None,
        },
    }
}

/// The OpenCode version this build measured its carrier question against.
///
/// Version-scoped for the same reason Codex's is: the claim is a measurement
/// of how one program assembles a prompt, and nothing here is a floor.
/// Unlike Codex's it does **not** gate a delivery — there is none to gate —
/// it scopes the *findings* that keep the delivery closed.
pub const OPENCODE_MEASURED_VERSION: &str = "2.2.7";

/// Why OpenCode has no deliverable launch tier, as one sentence naming the
/// carrier, the two blockers and the version they were measured on.
///
/// **The carrier itself is excellent, and that is the frustrating part.**
/// `OPENCODE_CONFIG_CONTENT` is read as one configuration layer, its
/// `instructions` array is *set-unioned* with the project's own rather than
/// replacing it, and it writes nothing to disk — per-launch, additive,
/// side-effect-free. Both blockers are about *who* ends up reading it.
///
/// **Blocker one: the launch's environment does not reach the session.**
/// OpenCode's default mode hands the work to one long-lived background
/// service on a fixed port, and that process has the environment of whoever
/// started it. Observed on 2.2.7: with `OPENCODE_CONFIG_CONTENT` set in the
/// client's environment, `opencode debug config` against the running
/// service lists the global config and no such layer, while the same
/// command with `--standalone` lists it last. So the carrier either does
/// not arrive at all, or — when that launch is the one that starts the
/// service — arrives for **every session the service goes on to serve**,
/// across projects. Neither is a per-launch delivery.
///
/// **Blocker two: a subagent inherits it.** OpenCode assembles one system
/// array per session from process-wide state, and the `task` tool's child
/// session goes through the same assembly with no gate on its parent, so a
/// `task` subagent receives what the primary agent received. A quiet worker
/// would get the whole central prefix, which `#SYS-WORKER-EXCLUSION`
/// forbids outright and which `#SYS-HARNESS-DELIVERY`'s row C5 already
/// calls disqualifying: a harness whose subagents inherit the slot must not
/// have `launch` as its default at all. There is no per-agent
/// `instructions` key and no primary-only switch to narrow it with
/// (`#SYS-OPENCODE`).
pub fn opencode_launch_blocker() -> String {
    format!(
        "OpenCode has no instruction slot a launch can scope to one session. Its only additive \
         carrier is the configuration key `instructions`, which vibe would hand over per launch \
         as `OPENCODE_CONFIG_CONTENT`, and two things stop it: OpenCode's default mode serves \
         sessions from one long-lived background service that never sees the launch's own \
         environment, so the carrier either does not arrive or arrives for every session that \
         service serves; and the `instructions` array is injected into **every** session a \
         process serves, `task` subagents included, so a quiet worker would receive the whole \
         central prefix — which `PROP-076#SYS-WORKER-EXCLUSION` forbids. Measured against \
         OpenCode {OPENCODE_MEASURED_VERSION}"
    )
}

/// The note a kind with no deliverable slot prints: what the carrier is,
/// why it is not a flag, and why vibe does not use it.
///
/// It is printed under **both** deliveries, because the question it answers
/// — "where would this harness's lane have gone" — does not depend on which
/// tier the project declared.
fn no_slot_note(kind: HarnessKind, lane: &Path) -> Option<String> {
    match kind {
        HarnessKind::Claude | HarnessKind::Codex => None,
        HarnessKind::OpenCode => Some(format!(
            "{}. The carrier could never have been a flag in any case: OpenCode takes no \
             append-instructions argument, so the assignment would have been \
             `OPENCODE_CONFIG_CONTENT={{\"instructions\":[\"{}\"]}}` in the launch's own \
             environment — a path, never the lane's bytes. The agent reads the boot files \
             itself instead",
            opencode_launch_blocker(),
            lane.display()
        )),
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
