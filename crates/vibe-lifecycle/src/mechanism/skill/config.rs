//! The static-skill adapter's structured config — §6.1's inputs, validated
//! at `plan`.
//!
//! The table is OURS, so it is STRICT: an unknown member refuses naming
//! itself, exactly as the Cargo build config next door does. Two members
//! are deliberately absent and refuse BY NAME rather than as "unknown",
//! because a reader who reaches for them is reaching for something the
//! architecture gives the engine: the output path (§3.2 — a provider
//! cannot mint one) and the skill's name (§6.1 — identity comes from the
//! source directory and the frontmatter, and a third spelling would be a
//! third thing to disagree).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use vibe_core::manifest::ExtensionConfig;

use crate::mechanism::MechanismError;
use crate::mechanism::contain::checked_relative;
use crate::mechanism::error::preview;

/// The fixed entry document of a skill source directory.
pub(crate) const ENTRY_DOCUMENT: &str = "SKILL.md";

/// The engine-owned members a target may not set, each with the reason.
const ENGINE_OWNED: [(&str, &str); 4] = [
    (
        "output",
        "the distributable's placement is engine-owned (§3.2: a provider cannot mint an output \
         path); it is always `SKILL.md` in the target's own package directory",
    ),
    (
        "output_dir",
        "the distributable's placement is engine-owned (§3.2: a provider cannot mint an output \
         path)",
    ),
    (
        "name",
        "a static skill's name is the source directory's own name, aligned with the `name` \
         frontmatter member (§6.1); config is not a third place to spell it",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one `package:static-skill` target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticSkillConfig {
    /// The skill source directory, project-relative and canonical. Its
    /// final component is the identity the frontmatter must agree with.
    pub(crate) source: String,
}

impl StaticSkillConfig {
    /// Read and validate one target's `config` table.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let mut source: Option<String> = None;
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(name, _)| name == member) {
                    return Err(MechanismError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason: (*reason).to_owned(),
                    });
                }
                match member.as_str() {
                    "source" => source = Some(source_path(target, value)?),
                    _ => {
                        return Err(MechanismError::Config {
                            target: target.to_owned(),
                            member: preview(member),
                            reason: "unknown member; the static-skill config is `source`"
                                .to_owned(),
                        });
                    }
                }
            }
        }
        let source = source.ok_or_else(|| MechanismError::Config {
            target: target.to_owned(),
            member: "source".to_owned(),
            reason: "required; name the skill source directory holding `SKILL.md`".to_owned(),
        })?;
        Ok(Self { source })
    }

    /// The identity the frontmatter must agree with — the source
    /// directory's final component.
    pub(crate) fn directory_name(&self) -> &str {
        self.source.rsplit('/').next().unwrap_or(&self.source)
    }

    /// The project-relative path of the entry document.
    pub(crate) fn entry_relative(&self) -> String {
        format!("{}/{ENTRY_DOCUMENT}", self.source)
    }
}

/// One `source` member: a string naming a canonical, contained relative
/// directory.
fn source_path(target: &str, value: &toml::Value) -> Result<String, MechanismError> {
    let text = value.as_str().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: "source".to_owned(),
        reason: format!("expected a string, found {}", preview(value.type_str())),
    })?;
    checked_relative(text).map_err(|fault| MechanismError::Config {
        target: target.to_owned(),
        member: "source".to_owned(),
        reason: format!("`{}` is unusable: {}", preview(text), fault.reason()),
    })
}
