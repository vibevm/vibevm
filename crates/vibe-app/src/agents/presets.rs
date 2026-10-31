//! The built-in agent presets (`PROP-064#presets`): one row of data per
//! supported harness, recording the command and the interactive and
//! unattended (print-mode) argument shapes its vendor publishes. A preset
//! is never a code path; a definition in `agents.toml` may name one and
//! override any field.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use specmark::spec;

/// One harness: its command and the two argument templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#presets")]
pub struct Preset {
    /// The preset name a definition's `preset` field takes.
    pub name: &'static str,
    /// The harness it drives, for people.
    pub harness: &'static str,
    pub command: &'static str,
    /// Starts the harness's own terminal session with the prompt.
    pub interactive: &'static [&'static str],
    /// Runs the prompt to completion and exits (print mode).
    pub unattended: &'static [&'static str],
}

/// The placeholders an argument template may carry.
pub const PLACEHOLDERS: [&str; 4] = ["{prompt}", "{prompt_file}", "{context_file}", "{work}"];

const PRESETS: &[Preset] = &[
    Preset {
        name: "claude-code",
        harness: "Claude Code",
        command: "claude",
        interactive: &["{prompt}"],
        unattended: &["-p", "{prompt}"],
    },
    Preset {
        name: "codex",
        harness: "OpenAI Codex CLI",
        command: "codex",
        interactive: &["{prompt}"],
        unattended: &["exec", "{prompt}"],
    },
    Preset {
        name: "opencode",
        harness: "OpenCode",
        command: "opencode",
        interactive: &["--prompt", "{prompt}"],
        unattended: &["run", "{prompt}"],
    },
    Preset {
        name: "qwen-code",
        harness: "Qwen Code",
        command: "qwen",
        interactive: &["--prompt-interactive", "{prompt}"],
        unattended: &["--prompt", "{prompt}"],
    },
    Preset {
        name: "gemini",
        harness: "Gemini CLI",
        command: "gemini",
        interactive: &["--prompt-interactive", "{prompt}"],
        unattended: &["--prompt", "{prompt}"],
    },
    Preset {
        name: "cursor",
        harness: "Cursor CLI",
        command: "cursor-agent",
        interactive: &["{prompt}"],
        unattended: &["-p", "{prompt}"],
    },
];

/// Every built-in preset, in a stable order.
///
/// ```
/// let names: Vec<&str> = vibe_app::agent_presets().iter().map(|p| p.name).collect();
/// assert_eq!(names, ["claude-code", "codex", "opencode", "qwen-code", "gemini", "cursor"]);
/// ```
pub fn presets() -> &'static [Preset] {
    PRESETS
}

/// The preset of that name.
pub fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_carries_the_prompt_in_both_shapes() {
        for preset in presets() {
            for template in [preset.interactive, preset.unattended] {
                assert!(
                    template
                        .iter()
                        .any(|arg| arg.contains("{prompt}") || arg.contains("{prompt_file}")),
                    "{} hands the prompt over",
                    preset.name
                );
            }
            assert!(!preset.command.contains(char::is_whitespace));
        }
        assert_eq!(preset("codex").map(|p| p.command), Some("codex"));
        assert!(preset("nope").is_none());
    }
}
