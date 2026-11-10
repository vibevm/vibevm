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

use vibe_core::manifest::{HarnessDelivery, Manifest};

/// The closed set of harness kinds this version knows a launch tier for.
///
/// ```
/// use vibe_workspace::system_prompt::HarnessKind;
///
/// assert_eq!(HarnessKind::parse("claude"), Some(HarnessKind::Claude));
/// assert_eq!(HarnessKind::parse("gemini"), None);
/// assert_eq!(HarnessKind::Codex.as_str(), "codex");
/// assert_eq!(HarnessKind::ALL.len(), 2, "the set is closed at two");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessKind {
    /// Claude Code — `--append-system-prompt-file`, a hidden but present
    /// file-based slot.
    Claude,
    /// Codex — `developer_instructions` through a launch-time profile file.
    Codex,
}

impl HarnessKind {
    /// Every kind, in the order a refusal lists them.
    pub const ALL: [HarnessKind; 2] = [HarnessKind::Claude, HarnessKind::Codex];

    /// The kind's own name, which is also the manifest's harness key.
    pub fn as_str(self) -> &'static str {
        match self {
            HarnessKind::Claude => "claude",
            HarnessKind::Codex => "codex",
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
        }
    }

    /// This kind's declared delivery for a node.
    pub fn delivery(self, manifest: &Manifest) -> HarnessDelivery {
        manifest
            .system_prompt
            .as_ref()
            .map_or(HarnessDelivery::Launch, |section| {
                section.harness.delivery(self.as_str())
            })
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
        return LaunchFlags {
            flags: Vec::new(),
            notes: vec![format!(
                "`[system_prompt.harness.{}] delivery = \"read\"`, so the harness starts \
                 unchanged and the managed block's router has the agent read the boot files \
                 itself — no slot flag is passed",
                kind.as_str()
            )],
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
    }
}

/// The profile file for one node.
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
             [profiles.{name}]\n\
             developer_instructions = {}\n",
            toml_basic_string(lane_bytes)
        ),
        name,
    }
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
