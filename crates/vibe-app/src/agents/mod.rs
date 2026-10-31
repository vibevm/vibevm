//! Agent hooks (`PROP-064`): the registry `<settings>/agents.toml` with its
//! built-in presets, the choice of the agent for one run (`--agent NAME`,
//! then the external registration, then the default, otherwise none), and
//! the argument vector a definition or a registration expands to. The hook
//! itself runs as the `agent` step kind (`crate::steps`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#root");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#external-registration");

mod config;
mod error;
mod hooks;
mod presets;

use std::collections::BTreeMap;
use std::path::Path;

use specmark::spec;

pub use config::{AgentDefinition, AgentsConfig, agent_list, preset_table};
pub(crate) use error::io;
pub use error::{AgentError, ParkedStep};
pub use hooks::{AgentHooks, AgentNotice, AgentPrompt, ConsentPrompt};
pub use presets::{PLACEHOLDERS, Preset, preset, presets};

/// Where the chosen agent came from (`PROP-064#selection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
pub enum AgentSource {
    /// `--agent NAME` on the command line.
    Flag,
    /// `VIBE_AGENT_COMMAND` / `VIBE_AGENT_ARGS` of the invoking program.
    External,
    /// `default` in `agents.toml`.
    Default,
}

/// A definition merged with its preset: everything needed to spawn it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
pub struct ResolvedAgent {
    pub name: String,
    pub preset: Option<String>,
    pub command: String,
    pub interactive: Vec<String>,
    pub unattended: Vec<String>,
    /// The working-directory template; `{work}` when absent.
    pub cwd: Option<String>,
    pub env: BTreeMap<String, String>,
    /// Appended in unattended runs.
    pub extra_args: Vec<String>,
}

/// A command registered by the program that invokes `vibe`
/// (`PROP-064#external-registration`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#external-registration")]
pub struct ExternalAgent {
    pub command: String,
    /// Leading arguments placed before the hook arguments.
    pub args: Vec<String>,
}

impl ExternalAgent {
    /// Read the registration from the values of `VIBE_AGENT_COMMAND` and
    /// `VIBE_AGENT_ARGS` the caller looked up; an empty command is none.
    ///
    /// ```
    /// let agent = vibe_app::ExternalAgent::from_values(Some("tool"), Some(r#"["--x"]"#)).unwrap();
    /// assert_eq!(agent.unwrap().args, ["--x"]);
    /// assert!(vibe_app::ExternalAgent::from_values(None, Some("[]")).unwrap().is_none());
    /// assert!(vibe_app::ExternalAgent::from_values(Some("tool"), Some("--x")).is_err());
    /// ```
    pub fn from_values(
        command: Option<&str>,
        args: Option<&str>,
    ) -> Result<Option<Self>, AgentError> {
        let Some(command) = command.map(str::trim).filter(|command| !command.is_empty()) else {
            return Ok(None);
        };
        let args = match args.map(str::trim).filter(|args| !args.is_empty()) {
            None => Vec::new(),
            Some(text) => serde_json::from_str::<Vec<String>>(text).map_err(|error| {
                AgentError::Registration {
                    reason: format!("VIBE_AGENT_ARGS is not a JSON array of strings: {error}"),
                }
            })?,
        };
        Ok(Some(Self {
            command: command.to_owned(),
            args,
        }))
    }
}

/// How the chosen agent is started.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
pub enum Delivery {
    /// A definition (or a preset) of `agents.toml`.
    Definition(ResolvedAgent),
    /// The externally registered command.
    External(ExternalAgent),
}

/// The agent of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
pub struct SelectedAgent {
    /// The definition name, the preset name, or `external`.
    pub id: String,
    pub source: AgentSource,
    pub delivery: Delivery,
}

/// The two ways an agent runs (`PROP-064#modes`), after `inherit` is
/// resolved against the command's attended state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#modes")]
pub enum RunMode {
    Interactive,
    Unattended,
}

impl RunMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Unattended => "unattended",
        }
    }
}

/// The values of the four template placeholders.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
pub struct Placeholders<'a> {
    pub prompt: &'a str,
    pub prompt_file: &'a Path,
    pub context_file: &'a Path,
    pub work: &'a Path,
}

/// One program to start: the executable, its arguments and the directory.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-064#external-registration")]
pub struct Invocation {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
}

/// Choose the agent of one run: `flag` (`--agent NAME`), then the external
/// registration, then `default` of `config`, otherwise none. A name is a
/// definition of `config` or, when no definition has it, a built-in preset.
///
/// ```
/// use vibe_app::{AgentSource, AgentsConfig, ExternalAgent, select_agent};
///
/// let config: AgentsConfig = toml::from_str("default = \"codex\"").unwrap();
/// let external = ExternalAgent { command: "gui-agent".into(), args: vec![] };
/// let chosen = select_agent(&config, None, Some(external.clone())).unwrap().unwrap();
/// assert_eq!(chosen.source, AgentSource::External);
/// let chosen = select_agent(&config, Some("gemini"), Some(external)).unwrap().unwrap();
/// assert_eq!((chosen.id.as_str(), chosen.source), ("gemini", AgentSource::Flag));
/// assert_eq!(select_agent(&config, None, None).unwrap().unwrap().id, "codex");
/// assert!(select_agent(&AgentsConfig::default(), None, None).unwrap().is_none());
/// ```
pub fn select_agent(
    config: &AgentsConfig,
    flag: Option<&str>,
    external: Option<ExternalAgent>,
) -> Result<Option<SelectedAgent>, AgentError> {
    let named = |name: &str, source| -> Result<SelectedAgent, AgentError> {
        Ok(SelectedAgent {
            id: name.to_owned(),
            source,
            delivery: Delivery::Definition(config.resolve(name)?),
        })
    };
    if let Some(name) = flag {
        return named(name, AgentSource::Flag).map(Some);
    }
    if let Some(external) = external {
        return Ok(Some(SelectedAgent {
            id: "external".into(),
            source: AgentSource::External,
            delivery: Delivery::External(external),
        }));
    }
    config
        .default
        .as_deref()
        .map(|name| named(name, AgentSource::Default))
        .transpose()
}

impl SelectedAgent {
    /// The program to start for one step (`PROP-064#agent-definition`,
    /// `#external-registration`): templates are argument arrays expanded
    /// one placeholder at a time, never a shell string.
    pub fn invocation(&self, step: &str, mode: RunMode, values: &Placeholders<'_>) -> Invocation {
        let work = values.work.to_string_lossy().into_owned();
        match &self.delivery {
            Delivery::Definition(agent) => {
                let template = match mode {
                    RunMode::Interactive => &agent.interactive,
                    RunMode::Unattended => &agent.unattended,
                };
                let mut args: Vec<String> =
                    template.iter().map(|arg| expand(arg, values)).collect();
                if mode == RunMode::Unattended {
                    args.extend(agent.extra_args.iter().map(|arg| expand(arg, values)));
                }
                Invocation {
                    command: agent.command.clone(),
                    args,
                    cwd: agent.cwd.as_deref().map_or(work, |cwd| expand(cwd, values)),
                    env: agent.env.clone(),
                }
            }
            Delivery::External(external) => {
                let mut args = external.args.clone();
                for (flag, value) in [
                    ("--hook", step.to_owned()),
                    ("--mode", mode.as_str().to_owned()),
                    (
                        "--prompt-file",
                        values.prompt_file.to_string_lossy().into_owned(),
                    ),
                    (
                        "--context-file",
                        values.context_file.to_string_lossy().into_owned(),
                    ),
                    ("--work", work.clone()),
                ] {
                    args.push(flag.into());
                    args.push(value);
                }
                Invocation {
                    command: external.command.clone(),
                    args,
                    cwd: work,
                    env: BTreeMap::new(),
                }
            }
        }
    }
}

/// Replace every `{prompt}`, `{prompt_file}`, `{context_file}` and `{work}`
/// in one argument, left to right, in one pass; other braces stay literal.
fn expand(template: &str, values: &Placeholders<'_>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    'scan: while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let tail = &rest[open..];
        for (name, value) in [
            ("{prompt}", values.prompt.to_owned()),
            (
                "{prompt_file}",
                values.prompt_file.to_string_lossy().into_owned(),
            ),
            (
                "{context_file}",
                values.context_file.to_string_lossy().into_owned(),
            ),
            ("{work}", values.work.to_string_lossy().into_owned()),
        ] {
            if let Some(after) = tail.strip_prefix(name) {
                out.push_str(&value);
                rest = after;
                continue 'scan;
            }
        }
        out.push('{');
        rest = &tail[1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn values<'a>(prompt: &'a str, dir: &'a Path, file: &'a Path) -> Placeholders<'a> {
        Placeholders {
            prompt,
            prompt_file: file,
            context_file: file,
            work: dir,
        }
    }

    #[test]
    fn templates_expand_each_placeholder_once_and_keep_other_braces() {
        let dir = PathBuf::from("/w");
        let file = PathBuf::from("/w/p.md");
        let v = values("say {work}", &dir, &file);
        assert_eq!(
            expand("{prompt}", &v),
            "say {work}",
            "one pass: values are not re-expanded"
        );
        assert_eq!(expand("--in={work}/{x}", &v), "--in=/w/{x}");
        assert_eq!(expand("{prompt_file}{", &v), "/w/p.md{");
    }

    #[test]
    fn a_definition_and_a_registration_expand_to_their_argument_contracts() {
        let config: AgentsConfig = toml::from_str(
            "default = \"mine\"\n[agent.mine]\npreset = \"claude-code\"\ncommand = \"/opt/claude-wrapper\"\n\
             extra_args = [\"--verbose\"]\nenv = { GATEWAY = \"g\" }\n",
        )
        .unwrap();
        let chosen = select_agent(&config, None, None).unwrap().unwrap();
        let dir = PathBuf::from("/w");
        let file = PathBuf::from("/w/steps/fix/prompt.md");
        let v = values("Fix it", &dir, &file);
        let run = chosen.invocation("fix", RunMode::Unattended, &v);
        assert_eq!(run.command, "/opt/claude-wrapper");
        assert_eq!(run.args, ["-p", "Fix it", "--verbose"]);
        assert_eq!((run.cwd.as_str(), run.env["GATEWAY"].as_str()), ("/w", "g"));
        let interactive = chosen.invocation("fix", RunMode::Interactive, &v);
        assert_eq!(
            interactive.args,
            ["Fix it"],
            "extra_args are unattended only"
        );

        let external = ExternalAgent {
            command: "gui".into(),
            args: vec!["--lead".into()],
        };
        let chosen = select_agent(&config, None, Some(external))
            .unwrap()
            .unwrap();
        let run = chosen.invocation("fix", RunMode::Interactive, &v);
        assert_eq!(
            run.args,
            [
                "--lead",
                "--hook",
                "fix",
                "--mode",
                "interactive",
                "--prompt-file",
                "/w/steps/fix/prompt.md",
                "--context-file",
                "/w/steps/fix/prompt.md",
                "--work",
                "/w"
            ]
        );
        assert!(matches!(
            select_agent(&config, Some("ghost"), None),
            Err(AgentError::Unknown { .. })
        ));
    }
}
