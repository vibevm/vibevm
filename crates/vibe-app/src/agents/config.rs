//! `<settings>/agents.toml` (`PROP-064#agent-definition`): `default` and
//! `trusted_groups` at the top, one `[agent.NAME]` table per definition,
//! each optionally naming a preset whose fields it overrides; the edits
//! `vibe agent add | remove | use` make, and the two listing documents.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_wire::generated::agent_list as list_wire;
use vibe_wire::generated::agent_presets as presets_wire;

use crate::settings::{SettingsRoot, write_atomic};

use super::presets::{PLACEHOLDERS, preset, presets};
use super::{AgentError, ExternalAgent, ResolvedAgent, io};

/// One `[agent.NAME]` table as written; every field may come from `preset`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
pub struct AgentDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interactive: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unattended: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
}

/// The whole `agents.toml`.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
/// let mut config = vibe_app::AgentsConfig::load(&settings).unwrap();
/// assert!(config.agent.is_empty());
/// config.use_agent("codex").unwrap();
/// config.save(&settings).unwrap();
/// let again = vibe_app::AgentsConfig::load(&settings).unwrap();
/// assert_eq!(again.default.as_deref(), Some("codex"));
/// assert_eq!(again.resolve("codex").unwrap().unattended, ["exec", "{prompt}"]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
pub struct AgentsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Package groups whose prompts run without the consent question
    /// (`PROP-064#consent`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_groups: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent: BTreeMap<String, AgentDefinition>,
    /// `[launchers.NAME]` — the PROP-076 agent launchers: a name bound to a
    /// harness **kind** and to the argv prefix that starts it.
    ///
    /// **Only a user may name one, which is why this table exists here and
    /// nowhere else.** A repository must not choose which program runs on a
    /// contributor's machine: that is the one setting where a shared file
    /// would let one developer's convenience execute arbitrary code on
    /// everyone else's computer, so a project manifest that names a launcher
    /// is a refusal (`PROP-076#SYS-HARNESS-EXECUTABLE`).
    ///
    /// **The spelling.** `PROP-076` writes the table as
    /// `[agent.launchers.<name>]`, which is the path it has in a project
    /// manifest — the spelling the refusal above names. It cannot be written
    /// that way *here*: `[agent.<name>]` is already a PROP-064 agent
    /// definition, so `[agent.launchers]` would be an agent called
    /// "launchers". In this file, whose whole root is the agent
    /// configuration, the table is `[launchers.<name>]` — the same path with
    /// the file's own root elided.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub launchers: BTreeMap<String, LauncherDefinition>,
}

/// One `[launchers.NAME]` table as written.
///
/// `command` is an argv **prefix** rather than a path, so a personal wrapper
/// may carry its own standing arguments: the motivating case is a developer
/// who reaches a harness through a small script that sets the harness's own
/// config directory, drops API-key variables and `exec`s the real program.
///
/// `config_dir` exists for one honest reason: it names where that launcher's
/// harness keeps its session logs. A wrapper that sets `CLAUDE_CONFIG_DIR`
/// internally makes the directory unknowable from outside — vibe sees
/// neither the variable nor the harness's default — so a launcher may state
/// it (`PROP-076#SYS-HARNESS-EXECUTABLE`).
///
/// ```
/// let declared: vibe_app::LauncherDefinition = toml::from_str(
///     "kind = 'claude'\ncommand = ['my-claude', '--no-telemetry']\n",
/// )?;
/// assert_eq!(declared.kind, "claude");
/// assert_eq!(declared.command, ["my-claude", "--no-telemetry"]);
/// assert_eq!(declared.config_dir, None);
/// # Ok::<(), toml::de::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE")]
pub struct LauncherDefinition {
    /// The harness kind, from the closed set. Required: a launcher with no
    /// kind has no flag dialect, and guessing one is how a session silently
    /// starts with no boot lane at all.
    pub kind: String,
    /// The argv prefix to exec. Empty means "the kind's own program".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// Where this launcher's harness keeps its session logs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
}

impl AgentsConfig {
    /// Read `<settings>/agents.toml`; a missing file is an empty registry.
    pub fn load(settings: &SettingsRoot) -> Result<Self, AgentError> {
        let path = settings.agents_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(io("reading", &path, error)),
        };
        toml::from_str(&text).map_err(|error| AgentError::Config {
            path: path.display().to_string(),
            reason: error.to_string(),
        })
    }

    /// Replace `<settings>/agents.toml` atomically.
    pub fn save(&self, settings: &SettingsRoot) -> Result<(), AgentError> {
        let path = settings.agents_path();
        let text = toml::to_string(self).map_err(|error| AgentError::Config {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        write_atomic(&path, text.as_bytes()).map_err(|error| AgentError::Config {
            path: path.display().to_string(),
            reason: error.to_string(),
        })
    }

    /// A definition merged with its preset, or a bare preset name.
    pub fn resolve(&self, name: &str) -> Result<ResolvedAgent, AgentError> {
        let definition = match (self.agent.get(name), preset(name)) {
            (Some(definition), _) => definition.clone(),
            (None, Some(_)) => AgentDefinition {
                preset: Some(name.to_owned()),
                ..AgentDefinition::default()
            },
            (None, None) => {
                return Err(AgentError::Unknown {
                    name: name.into(),
                    known: self.known_names(),
                });
            }
        };
        let base = match definition.preset.as_deref() {
            Some(id) => Some(preset(id).ok_or_else(|| AgentError::Definition {
                name: name.into(),
                reason: format!(
                    "preset `{id}` is not built in; presets: {}",
                    presets().iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
                ),
            })?),
            None => None,
        };
        let owned = |template: &[&str]| template.iter().map(|arg| (*arg).to_owned()).collect();
        let missing = |field: &str| AgentError::Definition {
            name: name.into(),
            reason: format!("it has no `{field}` and names no preset that has one"),
        };
        let command = definition
            .command
            .or_else(|| base.map(|preset| preset.command.to_owned()))
            .filter(|command| !command.trim().is_empty())
            .ok_or_else(|| missing("command"))?;
        let interactive = definition
            .interactive
            .or_else(|| base.map(|preset| owned(preset.interactive)))
            .ok_or_else(|| missing("interactive"))?;
        let unattended = definition
            .unattended
            .or_else(|| base.map(|preset| owned(preset.unattended)))
            .ok_or_else(|| missing("unattended"))?;
        Ok(ResolvedAgent {
            name: name.into(),
            preset: definition.preset,
            command,
            interactive,
            unattended,
            cwd: definition.cwd,
            env: definition.env,
            extra_args: definition.extra_args,
        })
    }

    fn known_names(&self) -> String {
        let mut names: Vec<&str> = self.agent.keys().map(String::as_str).collect();
        names.extend(presets().iter().map(|preset| preset.name));
        names.dedup();
        names.join(", ")
    }

    /// Add a definition; an existing name is refused.
    pub fn add(&mut self, name: &str, definition: AgentDefinition) -> Result<(), AgentError> {
        if self.agent.contains_key(name) {
            return Err(AgentError::Exists { name: name.into() });
        }
        if name.trim().is_empty() || name.chars().any(|ch| ch.is_whitespace() || ch == '.') {
            return Err(AgentError::Definition {
                name: name.into(),
                reason: "a name is one word without whitespace or dots".into(),
            });
        }
        self.agent.insert(name.into(), definition);
        self.resolve(name).map(|_| ()).inspect_err(|_| {
            self.agent.remove(name);
        })
    }

    /// Remove a definition; the default naming it is cleared.
    pub fn remove(&mut self, name: &str) -> Result<(), AgentError> {
        if self.agent.remove(name).is_none() {
            return Err(AgentError::Unknown {
                name: name.into(),
                known: self.agent.keys().cloned().collect::<Vec<_>>().join(", "),
            });
        }
        if self.default.as_deref() == Some(name) {
            self.default = None;
        }
        Ok(())
    }

    /// Make `name` the default: a definition as it is, or a preset written
    /// as a `[agent.NAME] preset = "NAME"` table first (`vibe agent use`).
    pub fn use_agent(&mut self, name: &str) -> Result<(), AgentError> {
        if !self.agent.contains_key(name) {
            if preset(name).is_none() {
                return Err(AgentError::Unknown {
                    name: name.into(),
                    known: self.known_names(),
                });
            }
            self.agent.insert(
                name.into(),
                AgentDefinition {
                    preset: Some(name.into()),
                    ..AgentDefinition::default()
                },
            );
        }
        self.resolve(name)?;
        self.default = Some(name.into());
        Ok(())
    }
}

/// The `vibe-agent-list/1` document: every definition merged with its
/// preset (a definition that cannot be used carries its `problem`), the
/// default, the trusted groups and the external registration.
pub fn agent_list(
    settings: &SettingsRoot,
    config: &AgentsConfig,
    external: Option<&ExternalAgent>,
) -> list_wire::AgentList {
    list_wire::AgentList {
        protocol: "vibe-agent-list/1".into(),
        path: settings.agents_path().display().to_string(),
        // The launcher rows are the caller's: resolving them means merging
        // this file's `[launchers.*]` with the built-ins of a closed harness
        // kind set that is PROP-076's and not this crate's.
        launchers: Vec::new(),
        default: config.default.clone(),
        trusted_groups: config.trusted_groups.clone(),
        agents: config
            .agent
            .iter()
            .map(|(name, written)| {
                let default = config.default.as_deref() == Some(name.as_str());
                match config.resolve(name) {
                    Ok(agent) => list_wire::Agent {
                        name: name.clone(),
                        preset: agent.preset,
                        command: Some(agent.command),
                        interactive: agent.interactive,
                        unattended: agent.unattended,
                        cwd: agent.cwd,
                        env: agent.env,
                        extra_args: agent.extra_args,
                        default,
                        problem: None,
                    },
                    Err(error) => list_wire::Agent {
                        name: name.clone(),
                        preset: written.preset.clone(),
                        command: written.command.clone(),
                        interactive: written.interactive.clone().unwrap_or_default(),
                        unattended: written.unattended.clone().unwrap_or_default(),
                        cwd: written.cwd.clone(),
                        env: written.env.clone(),
                        extra_args: written.extra_args.clone(),
                        default,
                        problem: Some(error.to_string()),
                    },
                }
            })
            .collect(),
        external: external.map(|external| list_wire::External {
            command: external.command.clone(),
            args: external.args.clone(),
        }),
    }
}

/// The `vibe-agent-presets/1` document.
///
/// ```
/// let table = vibe_app::agent_preset_table();
/// assert_eq!(table.presets.len(), 6);
/// assert!(table.placeholders.contains(&"{prompt_file}".to_owned()));
/// ```
pub fn preset_table() -> presets_wire::AgentPresets {
    let owned = |template: &[&str]| template.iter().map(|arg| (*arg).to_owned()).collect();
    presets_wire::AgentPresets {
        protocol: "vibe-agent-presets/1".into(),
        placeholders: PLACEHOLDERS.iter().map(|name| (*name).to_owned()).collect(),
        presets: presets()
            .iter()
            .map(|preset| presets_wire::Preset {
                name: preset.name.into(),
                harness: preset.harness.into(),
                command: preset.command.into(),
                interactive: owned(preset.interactive),
                unattended: owned(preset.unattended),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_overrides_its_preset_and_edits_keep_the_default_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let mut config = AgentsConfig::default();
        config
            .add(
                "wrapped",
                AgentDefinition {
                    preset: Some("claude-code".into()),
                    command: Some("/opt/bin/claude-codex".into()),
                    ..AgentDefinition::default()
                },
            )
            .unwrap();
        assert!(matches!(
            config.add("wrapped", AgentDefinition::default()),
            Err(AgentError::Exists { .. })
        ));
        assert!(matches!(
            config.add("bare", AgentDefinition::default()),
            Err(AgentError::Definition { .. })
        ));
        assert!(
            !config.agent.contains_key("bare"),
            "a refused add leaves nothing"
        );
        config.use_agent("wrapped").unwrap();
        config.trusted_groups.push("org.vibevm".into());
        config.save(&settings).unwrap();
        let text = fs::read_to_string(settings.agents_path()).unwrap();
        assert!(text.starts_with("default = \"wrapped\""), "{text}");
        assert!(text.contains("[agent.wrapped]"), "{text}");
        let loaded = AgentsConfig::load(&settings).unwrap();
        let agent = loaded.resolve("wrapped").unwrap();
        assert_eq!(agent.command, "/opt/bin/claude-codex");
        assert_eq!(agent.unattended, ["-p", "{prompt}"]);
        let listed = agent_list(&settings, &loaded, None);
        assert!(listed.agents[0].default);
        let mut loaded = loaded;
        loaded.remove("wrapped").unwrap();
        assert_eq!(loaded.default, None);
        assert!(loaded.remove("wrapped").is_err());
        fs::write(settings.agents_path(), "[agent.x]\nshell = \"sh -c\"\n").unwrap();
        assert!(matches!(
            AgentsConfig::load(&settings),
            Err(AgentError::Config { .. })
        ));
    }
}
