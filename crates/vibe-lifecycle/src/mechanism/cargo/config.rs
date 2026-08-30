//! The Cargo adapter's structured config and output selector — §5 law 4
//! ("supports structured config for manifest path, package, target
//! kind/name, profile, target triple, features, `locked`, `offline` and
//! `frozen`"), validated at `plan`.
//!
//! Both tables are OURS, so both are STRICT: an unknown member refuses
//! naming itself. That is the opposite posture from the Cargo message
//! reader next door, which ignores unknown fields by design because that
//! stream is another tool's wire. The two postures are deliberate and the
//! difference is the whole reason they live in separate cells.
//!
//! One member is deliberately absent and refuses by name: the build output
//! directory. §3.2 gives the engine artifact paths, so `--target-dir` is
//! engine-owned and a target that tries to set it is told why rather than
//! told "unknown".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use vibe_core::manifest::ExtensionConfig;

use crate::mechanism::MechanismError;
use crate::mechanism::error::preview;

/// The engine-owned members a target may not set, each with the reason.
const ENGINE_OWNED: [(&str, &str); 4] = [
    (
        "target_dir",
        "the build output directory is engine-owned (§3.2: a provider cannot mint an output path)",
    ),
    (
        "target-dir",
        "the build output directory is engine-owned (§3.2: a provider cannot mint an output path)",
    ),
    (
        "message_format",
        "the message format is fixed at `json-render-diagnostics`; the artifact is taken only from \
         that stream",
    ),
    (
        "env",
        "a provider receives no environment from the manifest; VibeVM never places configuration \
         bytes into a provider environment",
    ),
];

/// The validated `config` table of one `[[artifacts.build]]` Cargo target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CargoBuildConfig {
    pub(crate) manifest_path: Option<String>,
    pub(crate) package: Option<String>,
    pub(crate) target_kind: Option<String>,
    pub(crate) target_name: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) target_triple: Option<String>,
    pub(crate) features: Vec<String>,
    pub(crate) no_default_features: bool,
    pub(crate) all_features: bool,
    pub(crate) locked: bool,
    pub(crate) offline: bool,
    pub(crate) frozen: bool,
}

impl CargoBuildConfig {
    /// Read and validate one target's `config` table.
    pub(crate) fn parse(
        target: &str,
        config: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let mut parsed = Self::default();
        let Some(config) = config else {
            return Ok(parsed);
        };
        for (member, value) in config.as_table() {
            if let Some((_, reason)) = ENGINE_OWNED.iter().find(|(name, _)| name == member) {
                return Err(MechanismError::Config {
                    target: target.to_owned(),
                    member: preview(member),
                    reason: (*reason).to_owned(),
                });
            }
            match member.as_str() {
                "manifest_path" => parsed.manifest_path = Some(string(target, member, value)?),
                "package" => parsed.package = Some(string(target, member, value)?),
                "target_kind" => parsed.target_kind = Some(string(target, member, value)?),
                "target_name" => parsed.target_name = Some(string(target, member, value)?),
                "profile" => parsed.profile = Some(string(target, member, value)?),
                "target_triple" => parsed.target_triple = Some(string(target, member, value)?),
                "features" => parsed.features = strings(target, member, value)?,
                "no_default_features" => parsed.no_default_features = flag(target, member, value)?,
                "all_features" => parsed.all_features = flag(target, member, value)?,
                "locked" => parsed.locked = flag(target, member, value)?,
                "offline" => parsed.offline = flag(target, member, value)?,
                "frozen" => parsed.frozen = flag(target, member, value)?,
                _ => {
                    return Err(MechanismError::Config {
                        target: target.to_owned(),
                        member: preview(member),
                        reason:
                            "unknown member; the Cargo build config is manifest_path, package, \
                                 target_kind, target_name, profile, target_triple, features, \
                                 no_default_features, all_features, locked, offline, frozen"
                                .to_owned(),
                    });
                }
            }
        }
        parsed.check_coherence(target)?;
        Ok(parsed)
    }

    /// Reject combinations Cargo itself would refuse only after spawning.
    fn check_coherence(&self, target: &str) -> Result<(), MechanismError> {
        if self.all_features && !self.features.is_empty() {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: "all_features".to_owned(),
                reason: "`all_features` and an explicit `features` list contradict each other"
                    .to_owned(),
            });
        }
        if let Some(kind) = &self.target_kind
            && kind != "bin"
        {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: "target_kind".to_owned(),
                reason: format!(
                    "`{}` is not a target kind this provider builds; it produces executables, so \
                     the only kind is `bin`",
                    preview(kind)
                ),
            });
        }
        Ok(())
    }

    /// The `cargo build` argv tail this config contributes, in a fixed
    /// order so two runs of one config produce byte-identical evidence.
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut argv = Vec::new();
        if let Some(package) = &self.package {
            argv.push("--package".to_owned());
            argv.push(package.clone());
        }
        if let Some(name) = &self.target_name {
            argv.push("--bin".to_owned());
            argv.push(name.clone());
        }
        if let Some(profile) = &self.profile {
            argv.push("--profile".to_owned());
            argv.push(profile.clone());
        }
        if let Some(triple) = &self.target_triple {
            argv.push("--target".to_owned());
            argv.push(triple.clone());
        }
        if self.all_features {
            argv.push("--all-features".to_owned());
        }
        if !self.features.is_empty() {
            argv.push("--features".to_owned());
            argv.push(self.features.join(","));
        }
        if self.no_default_features {
            argv.push("--no-default-features".to_owned());
        }
        argv.extend(self.posture_arguments());
        argv
    }

    /// The three source-posture flags, shared by `build` and `metadata`.
    pub(crate) fn posture_arguments(&self) -> Vec<String> {
        let mut argv = Vec::new();
        if self.locked {
            argv.push("--locked".to_owned());
        }
        if self.offline {
            argv.push("--offline".to_owned());
        }
        if self.frozen {
            argv.push("--frozen".to_owned());
        }
        argv
    }

    /// Whether this config, folded with the run's posture, permits the
    /// network. `frozen` implies `offline` in Cargo's own reading.
    pub(crate) const fn reaches_network(&self, run_offline: bool) -> bool {
        !(run_offline || self.offline || self.frozen)
    }
}

/// The validated `select` table of one declared output row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OutputSelect {
    pub(crate) package: Option<String>,
    pub(crate) bin: Option<String>,
}

impl OutputSelect {
    /// Read and validate one output's `select` table.
    pub(crate) fn parse(
        target: &str,
        output: &str,
        select: Option<&ExtensionConfig>,
    ) -> Result<Self, MechanismError> {
        let mut parsed = Self::default();
        let Some(select) = select else {
            return Ok(parsed);
        };
        for (member, value) in select.as_table() {
            let text = value.as_str().ok_or_else(|| MechanismError::Select {
                target: target.to_owned(),
                output: output.to_owned(),
                member: preview(member),
                reason: format!("expected a string, found {}", value.type_str()),
            })?;
            if text.trim().is_empty() {
                return Err(MechanismError::Select {
                    target: target.to_owned(),
                    output: output.to_owned(),
                    member: preview(member),
                    reason: "expected a non-blank string".to_owned(),
                });
            }
            match member.as_str() {
                "package" => parsed.package = Some(text.to_owned()),
                "bin" => parsed.bin = Some(text.to_owned()),
                _ => {
                    return Err(MechanismError::Select {
                        target: target.to_owned(),
                        output: output.to_owned(),
                        member: preview(member),
                        reason: "unknown member; a Cargo artifact is selected by `package` and/or \
                                 `bin`"
                            .to_owned(),
                    });
                }
            }
        }
        Ok(parsed)
    }

    /// The predicate in the one spelling every refusal quotes it by.
    pub(crate) fn describe(&self) -> String {
        match (&self.package, &self.bin) {
            (Some(package), Some(bin)) => format!("package `{package}` bin `{bin}`"),
            (Some(package), None) => format!("package `{package}`"),
            (None, Some(bin)) => format!("bin `{bin}`"),
            (None, None) => "any executable artifact of this build".to_owned(),
        }
    }
}

fn string(target: &str, member: &str, value: &toml::Value) -> Result<String, MechanismError> {
    let text = value.as_str().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: preview(member),
        reason: format!("expected a string, found {}", value.type_str()),
    })?;
    if text.trim().is_empty() {
        return Err(MechanismError::Config {
            target: target.to_owned(),
            member: preview(member),
            reason: "expected a non-blank string".to_owned(),
        });
    }
    Ok(text.to_owned())
}

fn strings(target: &str, member: &str, value: &toml::Value) -> Result<Vec<String>, MechanismError> {
    let rows = value.as_array().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: preview(member),
        reason: format!("expected an array of strings, found {}", value.type_str()),
    })?;
    let mut features = Vec::with_capacity(rows.len());
    for row in rows {
        let text = row.as_str().ok_or_else(|| MechanismError::Config {
            target: target.to_owned(),
            member: preview(member),
            reason: format!("expected an array of strings, found a {}", row.type_str()),
        })?;
        if text.trim().is_empty() || text.contains(',') {
            return Err(MechanismError::Config {
                target: target.to_owned(),
                member: preview(member),
                reason: format!(
                    "feature `{}` is blank or carries a comma; Cargo joins features with commas",
                    preview(text)
                ),
            });
        }
        features.push(text.to_owned());
    }
    Ok(features)
}

fn flag(target: &str, member: &str, value: &toml::Value) -> Result<bool, MechanismError> {
    value.as_bool().ok_or_else(|| MechanismError::Config {
        target: target.to_owned(),
        member: preview(member),
        reason: format!("expected a boolean, found {}", value.type_str()),
    })
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
