//! Structured installation (`PROP-065`): user-selectable features, an
//! ordered list of steps of a closed kind set, conditions of a closed
//! vocabulary, needs and the declared result of an installer option.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#steps");

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};
use specmark::spec;

use super::{ShellInterpreter, SourceInputDecl};

/// One `[[installer_options.features]]` row a user selects.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#features")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureDecl {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub default: bool,
}

/// When a step runs relative to placement or registration.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#result")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepPhase {
    #[default]
    Prepare,
    Finish,
}

impl StepPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Finish => "finish",
        }
    }

    fn is_prepare(&self) -> bool {
        *self == Self::Prepare
    }
}

/// One condition table: exactly one key of the closed vocabulary.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    Feature(String),
    CommandPresent(String),
    FilePresent(String),
    EnvPresent(String),
    AgentConfigured(bool),
    Offline(bool),
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Feature(id) => write!(f, "feature = \"{id}\""),
            Self::CommandPresent(name) => write!(f, "command_present = \"{name}\""),
            Self::FilePresent(path) => write!(f, "file_present = \"{path}\""),
            Self::EnvPresent(name) => write!(f, "env_present = \"{name}\""),
            Self::AgentConfigured(value) => write!(f, "agent_configured = {value}"),
            Self::Offline(value) => write!(f, "offline = {value}"),
        }
    }
}

/// What a step requires before anything starts.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#needs")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Need {
    Network,
    Command(String),
    Agent,
    Terminal,
}

impl TryFrom<String> for Need {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        match value.as_str() {
            "network" => Ok(Self::Network),
            "agent" => Ok(Self::Agent),
            "terminal" => Ok(Self::Terminal),
            other => match other.strip_prefix("command:") {
                Some(name) if !name.is_empty() && !name.chars().any(char::is_whitespace) => {
                    Ok(Self::Command(name.to_owned()))
                }
                _ => Err(format!(
                    "need `{other}` is not one of network, command:NAME, agent, terminal"
                )),
            },
        }
    }
}

impl From<Need> for String {
    fn from(need: Need) -> Self {
        need.to_string()
    }
}

impl fmt::Display for Need {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network => f.write_str("network"),
            Self::Command(name) => write!(f, "command:{name}"),
            Self::Agent => f.write_str("agent"),
            Self::Terminal => f.write_str("terminal"),
        }
    }
}

/// The container of an `archive` step.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#kinds")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormatDecl {
    Zip,
    TarGz,
}

/// The format of a `native` step.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#kinds")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeFormatDecl {
    Exe,
    Msi,
}

/// How an agent step runs (`PROP-064#modes`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#modes")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Interactive,
    Unattended,
    #[default]
    Inherit,
}

/// The closed kind set of a step and the fields of each kind, tagged on
/// `kind` inside the step table.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#kinds")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StepKind {
    Sources {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<SourceInputDecl>,
    },
    Archive {
        format: ArchiveFormatDecl,
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_root: Option<PathBuf>,
    },
    Recipe {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpreter: Option<ShellInterpreter>,
        script: PathBuf,
    },
    Shell {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpreter: Option<ShellInterpreter>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_install: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        update: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uninstall: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_uninstall: Option<String>,
    },
    Native {
        format: NativeFormatDecl,
        source: String,
        sha256: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        arguments: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_arguments: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uninstall: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        force_uninstall: Option<String>,
    },
    Copy {
        from: PathBuf,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        to: String,
    },
    Agent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_file: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<AgentMode>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        required: bool,
    },
}

impl StepKind {
    /// The stable `kind` spelling.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sources { .. } => "sources",
            Self::Archive { .. } => "archive",
            Self::Recipe { .. } => "recipe",
            Self::Shell { .. } => "shell",
            Self::Native { .. } => "native",
            Self::Copy { .. } => "copy",
            Self::Agent { .. } => "agent",
        }
    }

    /// Whether a prepare step of this kind can write `$VIBE_OUT`.
    pub fn writes_out(&self) -> bool {
        matches!(
            self,
            Self::Archive { .. } | Self::Recipe { .. } | Self::Copy { .. }
        )
    }
}

/// Where a step came from: authored in `[[steps]]` (or the method form),
/// or a `pre_install` / `post_install` hook mapped by the sugar. Hook steps
/// keep the hook failure rules of `PROP-059#force-and-hooks`; this marker
/// is never authored and never serialised.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#sugar")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StepOrigin {
    #[default]
    Authored,
    PreInstall,
    PostInstall,
}

/// One `[[installer_options.steps]]` row.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#steps")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepDecl {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "one_or_many",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub when: Vec<Condition>,
    #[serde(
        default,
        deserialize_with = "one_or_many",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub unless: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Need>,
    #[serde(default, skip_serializing_if = "StepPhase::is_prepare")]
    pub phase: StepPhase,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    #[serde(flatten)]
    pub kind: StepKind,
    #[serde(skip)]
    pub origin: StepOrigin,
}

impl StepDecl {
    /// A prepare step of `kind` with no conditions, needs or flags.
    pub fn new(id: &str, title: &str, kind: StepKind) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            when: Vec::new(),
            unless: Vec::new(),
            needs: Vec::new(),
            phase: StepPhase::Prepare,
            optional: false,
            kind,
            origin: StepOrigin::Authored,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(Condition),
    Many(Vec<Condition>),
}

fn one_or_many<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Condition>, D::Error> {
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(condition) => vec![condition],
        OneOrMany::Many(conditions) => conditions,
    })
}

/// What the steps of an option produce.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#result")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultDecl {
    /// `result = "owned"`: the non-empty `$VIBE_OUT` is placed as the payload.
    Owned(OwnedWord),
    /// `result = { external = "<app_root template>" }`.
    External(ExternalResult),
}

impl ResultDecl {
    pub fn owned() -> Self {
        Self::Owned(OwnedWord::Owned)
    }

    pub fn external(app_root: &str) -> Self {
        Self::External(ExternalResult {
            external: app_root.into(),
        })
    }

    pub fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }

    /// The external `app_root` template, if any.
    pub fn external_root(&self) -> Option<&str> {
        match self {
            Self::Owned(_) => None,
            Self::External(external) => Some(&external.external),
        }
    }
}

/// The literal word `owned`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#result")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnedWord {
    #[serde(rename = "owned")]
    Owned,
}

/// The table form `{ external = "…" }`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#result")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalResult {
    pub external: String,
}
