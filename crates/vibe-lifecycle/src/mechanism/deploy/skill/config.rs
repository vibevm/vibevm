//! The standalone-skill deploy target's structured config — §6.3.0.5's
//! one authored member, validated at `plan`.
//!
//! The table is OURS, so it is STRICT, exactly as the vibe-bin config
//! next door is: an unknown member refuses naming itself, and the members
//! a reader might reach for that this provider exists to keep OUT of the
//! table refuse BY NAME with the reason. The one member that exists is
//! `name`, because §6.3.0.5's config is exactly `{ name = "…" }` — the
//! destination directory under the client's skills root — and nothing
//! else is the caller's to choose.
//!
//! The name's grammar is NOT restated here: [`SkillDecl::valid_name`] is
//! the ONE portable Agent Skills identity law (lowercase kebab, 1..64, no
//! device name, no path component), shared with authored manifests and
//! package-skill receipts, and a second spelling of it would be a second
//! thing to disagree.
//!
//! [`SkillDecl::valid_name`]: vibe_core::manifest::SkillDecl::valid_name

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_core::manifest::{ExtensionConfig, SkillDecl};

use crate::mechanism::error::DeployProviderError;
use crate::mechanism::error::preview;

/// The one authored member.
const NAME: &str = "name";

/// The members a target may not set, each with the reason it may not.
const ENGINE_OWNED: [(&str, &str); 5] = [
    (
        "client",
        "the client is chosen by the selected provider row (`deploy:claude-skill`, \
         `deploy:codex-skill`, `deploy:opencode-skill`), never by config — provider id and \
         logical mechanism name are separate fields, and a `client` member would put routing \
         inside the table the routing law sits above (§3.1)",
    ),
    (
        "home",
        "the user home is injected authority (§6.3.0.6: the command surface resolves it once; \
         every lower cell is forbidden from reading an ambient directory resolver), so it is \
         never a config member",
    ),
    (
        "root",
        "the client's skills root is the closed client vocabulary's own data (§6.3's \
         commissioning matrix), resolved through the pure injected-home helper; config is not \
         a second place to spell a destination",
    ),
    (
        "path",
        "the entry is always `<skills root>/<name>/SKILL.md` (§6.3.0.5); a config-chosen path \
         would let one skill own an unbounded destination",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one standalone-skill deploy target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDeployConfig {
    /// The skill's portable identity — its directory name under the
    /// client's skills root and the frontmatter name it must agree with.
    pub(crate) name: String,
}

impl SkillDeployConfig {
    /// Read and validate one target's `config` table.
    ///
    /// `name` is required with no default: §6.3.0.5's "strict
    /// `config={name=\"portable-token\"}`" leaves nothing a default could
    /// be derived from, and inferring the name from the artifact id would
    /// be a third place to spell an identity that already has two (the
    /// frontmatter and this table) that must agree.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, DeployProviderError> {
        let mut name: Option<String> = None;
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(known, _)| known == member) {
                    return Err(DeployProviderError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason: (*reason).to_owned(),
                    });
                }
                match member.as_str() {
                    NAME => name = Some(skill_name(target, value)?),
                    _ => {
                        return Err(DeployProviderError::Config {
                            target: target.to_owned(),
                            member: preview(member),
                            reason: "unknown member; the config is exactly `name`, the skill's \
                                      portable identity under the client's skills root (§6.3.0.5)"
                                .to_owned(),
                        });
                    }
                }
            }
        }
        name.map(|name| Self { name })
            .ok_or_else(|| DeployProviderError::Config {
                target: target.to_owned(),
                member: NAME.to_owned(),
                reason: "required with no default; the deployment owns exactly \
                         `<skills root>/<name>/SKILL.md`, so the skill's identity is named \
                         explicitly and is never inferred from the artifact"
                    .to_owned(),
            })
    }
}

/// One `name` member: a string holding the shared portable Agent Skills
/// identity grammar.
fn skill_name(target: &str, value: &toml::Value) -> Result<String, DeployProviderError> {
    let refuse = |reason: String| DeployProviderError::Config {
        target: target.to_owned(),
        member: NAME.to_owned(),
        reason,
    };
    let toml::Value::String(spelling) = value else {
        return Err(refuse(format!(
            "expected a string, found {}",
            value.type_str()
        )));
    };
    // The ONE grammar, borrowed rather than restated: it refuses the
    // uppercase, the underscore, the multi-component path, the device
    // name and the over-long in one shared law.
    if !SkillDecl::valid_name(spelling) {
        return Err(refuse(format!(
            "`{}` is not a portable Agent Skills name: it must be lowercase ASCII letters and \
             digits with inner single hyphens, 1..64 bytes, exactly one path component, and not \
             a Windows device name — the same grammar a `[[skill]]` declaration and a \
             package-skill receipt hold it to",
            preview(spelling),
        )));
    }
    Ok(spelling.clone())
}
