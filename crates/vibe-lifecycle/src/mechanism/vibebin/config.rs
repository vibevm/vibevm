//! The `deploy:vibe-bin` target's structured config — §7.1.0's one
//! authored member, validated at `plan`.
//!
//! The table is OURS, so it is STRICT, exactly as the three packaging
//! configs next door are: an unknown member refuses naming itself. Six
//! members are deliberately absent and refuse BY NAME rather than as
//! "unknown", because a reader who reaches for one is reaching for a
//! property this provider exists to guarantee — where the launcher lives
//! (§7.1.0 ruling 2: the layout is under the ONE settings dir and is the
//! engine's), and what is inside it (ruling 3: the body is version-free by
//! construction, so a version, a digest or an environment would each be a
//! way to put back what the ruling removed).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_core::manifest::ExtensionConfig;
use vibe_core::manifest::is_portable_token;

use super::launcher::POINTER_SUFFIX;
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::error::preview;

/// The one authored member.
const COMMAND: &str = "command";

/// The members a target may not set, each with the reason it may not.
const ENGINE_OWNED: [(&str, &str); 6] = [
    (
        "bin_dir",
        "the launcher's placement is engine-owned (§7.1.0 ruling 2: the layout lives under the \
         ONE vibevm settings directory, which the engine resolves and hands down); a provider \
         never resolves a home",
    ),
    (
        "store_dir",
        "the payload store's placement is engine-owned for the same reason as the launcher's \
         (§7.1.0 ruling 2), and a store outside the settings directory would be a second store \
         nobody garbage-collects",
    ),
    (
        "version",
        "the launcher is version-free by construction (§7.1.0 ruling 3): its body embeds only the \
         command name, the genre marker and the pointer indirection, and an update rewrites the \
         POINTER rather than the launcher",
    ),
    (
        "digest",
        "the launcher never embeds a payload digest (§7.1.0 ruling 3); the active payload is named \
         by the pointer file beside it, which is what makes one launcher body serve every \
         generation",
    ),
    (
        "path",
        "this provider never modifies PATH; it writes a launcher into the settings directory's own \
         `bin/`, and putting that directory on PATH is the operator's act, not a deployment's",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one `deploy:vibe-bin` target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VibeBinConfig {
    /// The command alias this deployment owns — the stem of both owned
    /// resources, `bin/<command>` and `bin/<command>.current`.
    pub(crate) command: String,
}

impl VibeBinConfig {
    /// Read and validate one target's `config` table.
    ///
    /// `command` is required: §7.1's "Only an explicit executable artifact
    /// and target may use this provider" leaves nothing for a default to
    /// be derived from — inferring the alias from the artifact id would be
    /// exactly the "merely producing an executable does not grant
    /// installation" the same paragraph forbids.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, DeployProviderError> {
        let mut command: Option<String> = None;
        if let Some(config) = config {
            for (member, value) in config.as_table() {
                if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(name, _)| name == member) {
                    return Err(DeployProviderError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason: (*reason).to_owned(),
                    });
                }
                match member.as_str() {
                    COMMAND => command = Some(command_alias(target, value)?),
                    _ => {
                        return Err(DeployProviderError::Config {
                            target: target.to_owned(),
                            member: preview(member),
                            reason: format!(
                                "unknown member; the vibe-bin config is `{COMMAND}` and nothing \
                                 else"
                            ),
                        });
                    }
                }
            }
        }
        command
            .map(|command| Self { command })
            .ok_or_else(|| DeployProviderError::Config {
                target: target.to_owned(),
                member: COMMAND.to_owned(),
                reason: "missing; a vibe-bin deployment owns the launcher `bin/<command>` and the \
                         pointer beside it, so the alias it installs under is named explicitly and \
                         is never inferred from the artifact"
                    .to_owned(),
            })
    }
}

/// The `command` member: a portable single-segment token that cannot
/// collide with the pointer beside it and cannot name a Windows device.
fn command_alias(target: &str, value: &toml::Value) -> Result<String, DeployProviderError> {
    let refuse = |reason: String| DeployProviderError::Config {
        target: target.to_owned(),
        member: COMMAND.to_owned(),
        reason,
    };
    let toml::Value::String(spelling) = value else {
        return Err(refuse(format!(
            "expected a string, found {}",
            value.type_str()
        )));
    };
    // The one identity law this project already has for a portable token,
    // borrowed rather than restated: lowercase ASCII letters, digits, `-`
    // and `.`, alphanumeric at both edges, no `..`. It is also what makes
    // the launcher template's substitutions safe without escaping — such a
    // token carries no quote, no `%`, no `$` and no path separator.
    if !is_portable_token(spelling) {
        return Err(refuse(format!(
            "`{}` is not a portable single-segment command token: it must be lowercase ASCII \
             letters, digits, `-` or `.`, start and end alphanumerically, and contain no `..`",
            preview(spelling),
        )));
    }
    if spelling.ends_with(POINTER_SUFFIX) {
        return Err(refuse(format!(
            "`{}` ends with the reserved suffix `{POINTER_SUFFIX}`, which names the \
             active-payload pointer of the command without it; two commands would then claim one \
             file",
            preview(spelling),
        )));
    }
    if is_reserved_device(spelling) {
        return Err(refuse(format!(
            "`{}` is a reserved Windows device name; a file called that — with or without a \
             suffix — is the device rather than a launcher, so the write would silently succeed \
             and install nothing",
            preview(spelling),
        )));
    }
    Ok(spelling.clone())
}

/// The DOS device names Windows resolves regardless of any suffix, so
/// `bin/nul.cmd` is the null device rather than a file.
///
/// Checked on every platform, not only Windows: a manifest is portable, and
/// a `command` that installs on Linux and silently vanishes on Windows is
/// exactly the accident a portable identity law exists to prevent.
fn is_reserved_device(value: &str) -> bool {
    const NAMES: [&str; 4] = ["con", "prn", "aux", "nul"];
    if NAMES.contains(&value) {
        return true;
    }
    let numbered = |stem: &str| {
        value
            .strip_prefix(stem)
            .is_some_and(|rest| rest.len() == 1 && matches!(rest.as_bytes()[0], b'1'..=b'9'))
    };
    numbered("com") || numbered("lpt")
}
