//! The windows-zip adapter's structured config — §7.0.8's one member,
//! validated at `plan`.
//!
//! The table is OURS, so it is STRICT: an unknown member refuses naming
//! itself, exactly as the §6 packaging configs next door do. Four
//! members are deliberately absent and refuse BY NAME rather than as
//! "unknown", because a reader who reaches for them is reaching for the
//! very properties determinism is made of: the archive's placement (§3.2 —
//! a provider cannot mint an output path), its timestamps and its
//! compression parameters (§7.0.8 fixes both, and a knob would be a way to
//! make two runs differ).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use vibe_core::manifest::ExtensionConfig;

use crate::mechanism::MechanismError;
use crate::mechanism::contain::checked_relative;
use crate::mechanism::error::preview;

/// The engine-owned members a target may not set, each with the reason.
const ENGINE_OWNED: [(&str, &str); 5] = [
    (
        "output",
        "the archive's placement is engine-owned (§3.2: a provider cannot mint an output path); \
         it is always `<target-id>.zip` in the target's own package directory",
    ),
    (
        "output_dir",
        "the archive's placement is engine-owned (§3.2: a provider cannot mint an output path)",
    ),
    (
        "timestamp",
        "every entry carries one fixed timestamp constant (§7.0.8); a configurable one is a way \
         for two runs of the same content to produce two archives",
    ),
    (
        "compression",
        "the compression parameters are fixed (§7.0.8): every entry is STORED, because a \
         compressor's output is a property of its version and a byte-identical re-run is this \
         provider's acceptance",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one `package:windows-zip` target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WindowsZipConfig {
    /// An OPTIONAL archive-internal entry prefix, in portable
    /// forward-slashed segments. It renames nothing on disk: it is the
    /// directory every archived name is placed under, which is what §4's
    /// own example (`layout = "distribution/windows"`) asks for.
    pub(crate) layout: Option<String>,
}

impl WindowsZipConfig {
    /// Read and validate one target's `config` table.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let mut layout: Option<String> = None;
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
                    "layout" => layout = Some(layout_prefix(target, value)?),
                    _ => {
                        return Err(MechanismError::Config {
                            target: target.to_owned(),
                            member: preview(member),
                            reason: "unknown member; the windows-zip config is `layout`".to_owned(),
                        });
                    }
                }
            }
        }
        Ok(Self { layout })
    }

    /// One archived name under this layout.
    pub(crate) fn placed(&self, name: &str) -> String {
        match &self.layout {
            Some(prefix) => format!("{prefix}/{name}"),
            None => name.to_owned(),
        }
    }
}

/// The `layout` member: a non-empty, canonical, relative, forward-slashed
/// prefix.
fn layout_prefix(target: &str, value: &toml::Value) -> Result<String, MechanismError> {
    let refuse = |reason: String| MechanismError::Config {
        target: target.to_owned(),
        member: "layout".to_owned(),
        reason,
    };
    let toml::Value::String(spelling) = value else {
        return Err(refuse(format!(
            "expected a string, found {}",
            value.type_str()
        )));
    };
    checked_relative(spelling)
        .map_err(|fault| refuse(format!("`{}`: {}", preview(spelling), fault.reason())))
}
