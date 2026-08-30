//! Fixture builders shared by the agent red matrix.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use vibe_core::manifest::{ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionsControl};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_wire::generated::lifecycle::e1::context::{
    Context, Execution, Io, Project, Run, RunAgentMode, World,
};

use crate::agent::{
    AgentBackend, AgentCompletion, AgentRequest, AgentUsage, PromptRequest, ResolvedPrompt,
};
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionRegistry,
    ExtensionRegistryRow, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
    SelectorSubject, collect_extensions,
};

pub(crate) const PROVIDER_GROUP: &str = "org.demo";
pub(crate) const PROVIDER_NAME: &str = "tools";
pub(crate) const PROMPT: &str = "spec://org.demo/tools/common/PROMPT-001#root";
/// The materialised slot the fixture's declaring provider executes from.
pub(crate) const PROVIDER_ROOT: &str = "vibedeps/org.demo.tools/1.0.0";

/// A backend that records what it was asked and answers from a script. Every
/// red case asserts against these counters: the strongest evidence that a
/// guard fires *before* spend is that the paid method was never entered.
pub(crate) struct RecordingBackend {
    prompt_answer: Result<String, String>,
    /// Composition directives the fake backend reports as unsupported, so the
    /// lifecycle's own refusal has a counterexample without a real document.
    unsupported: Vec<String>,
    completion: Mutex<Vec<Result<AgentCompletion, String>>>,
    pub(crate) resolved: Mutex<Vec<PromptRequest>>,
    pub(crate) completed: Mutex<Vec<AgentRequest>>,
}

impl RecordingBackend {
    pub(crate) fn answering(text: &str) -> Self {
        Self::answering_prompt("Write the declared outputs.", text)
    }

    /// The same backend with an explicit prompt body, so a test can prove the
    /// resolved bytes — not just the address — reach the fingerprint.
    pub(crate) fn answering_prompt(instructions: &str, text: &str) -> Self {
        Self::new(
            Ok(instructions.to_string()),
            vec![Ok(AgentCompletion {
                text: text.to_string(),
                usage: Some(AgentUsage {
                    prompt_tokens: 11,
                    completion_tokens: 7,
                    total_tokens: 18,
                }),
            })],
        )
    }

    pub(crate) fn refusing_prompt(reason: &str) -> Self {
        Self::new(Err(reason.to_string()), Vec::new())
    }

    pub(crate) fn refusing_provider(reason: &str) -> Self {
        Self::new(
            Ok("Write the declared outputs.".into()),
            vec![Err(reason.to_string())],
        )
    }

    fn new(
        prompt_answer: Result<String, String>,
        completion: Vec<Result<AgentCompletion, String>>,
    ) -> Self {
        Self {
            prompt_answer,
            unsupported: Vec::new(),
            completion: Mutex::new(completion),
            resolved: Mutex::new(Vec::new()),
            completed: Mutex::new(Vec::new()),
        }
    }

    /// The same backend, but the resolved closure carries composition this
    /// handler does not perform.
    pub(crate) fn with_unsupported(mut self, unsupported: &[&str]) -> Self {
        self.unsupported = unsupported
            .iter()
            .map(|value| (*value).to_string())
            .collect();
        self
    }

    pub(crate) fn calls(&self) -> usize {
        self.completed.lock().unwrap().len()
    }
}

impl AgentBackend for RecordingBackend {
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.resolved.lock().unwrap().push(request.clone());
        Ok(ResolvedPrompt {
            text: self.prompt_answer.clone()?,
            unsupported: self.unsupported.clone(),
        })
    }

    fn complete(&self, request: &AgentRequest) -> Result<AgentCompletion, String> {
        self.completed.lock().unwrap().push(request.clone());
        let mut scripted = self.completion.lock().unwrap();
        if scripted.is_empty() {
            return Err("the fake backend was called more times than scripted".into());
        }
        scripted.remove(0)
    }
}

/// One dependency-provided agent contribution in a real collected registry.
/// `config_toml` is the authored `[extension.config]` body verbatim, so a
/// contract red case declares exactly what a manifest would declare.
pub(crate) fn row(config_toml: &str, prompt: &str) -> ExtensionRegistryRow {
    row_at(config_toml, prompt, PathBuf::from(PROVIDER_ROOT))
}

/// The same planned agent contribution with a declared input scope — the
/// seam the measurement-carriage reds need (A4b).
pub(crate) fn row_with_inputs(
    config_toml: &str,
    prompt: &str,
    inputs: Option<Vec<String>>,
) -> ExtensionRegistryRow {
    registry_with(config_toml, prompt, PathBuf::from(PROVIDER_ROOT), inputs)
        .plan("phase:create".parse().unwrap(), SelectorSubject::unscoped())
        .first()
        .copied()
        .expect("one planned agent contribution")
        .clone()
}

/// The same planned agent contribution, but executing from an EXPLICIT
/// provider root — the seam the hosted-backend reds need, because the real
/// resolver reads the provider's prompt documents from disk and the shared
/// fixture's slot root is a relative spelling that never exists.
pub(crate) fn row_at(
    config_toml: &str,
    prompt: &str,
    provider_root: PathBuf,
) -> ExtensionRegistryRow {
    registry_with(config_toml, prompt, provider_root, None)
        .plan("phase:create".parse().unwrap(), SelectorSubject::unscoped())
        .first()
        .copied()
        .expect("one planned agent contribution")
        .clone()
}

fn registry_with(
    config_toml: &str,
    prompt: &str,
    provider_root: PathBuf,
    inputs: Option<Vec<String>>,
) -> ExtensionRegistry {
    let config = (!config_toml.trim().is_empty()).then(|| {
        ExtensionConfig::from_table(
            toml::from_str::<toml::Table>(config_toml).expect("fixture config is valid TOML"),
        )
    });
    let declaration = ExtensionDecl {
        id: "produce".into(),
        point: "phase:create".parse().unwrap(),
        handler: ExtensionHandler::Agent {
            prompt: prompt.to_string(),
        },
        config,
        auto: None,
        inputs,
        applies_to: None,
        compiler_internals: None,
        pass: None,
        when: None,
    };
    collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    Group::parse(PROVIDER_GROUP).unwrap(),
                    PackageName::parse(PROVIDER_NAME).unwrap(),
                ),
                root: provider_root,
                version: "1.0.0".into(),
                kind: PackageKind::Tool,
                content_hash: ContentHash::parse("sha256:aa").unwrap(),
            },
            declarations: vec![declaration],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .expect("the fixture world collects")
}

/// The declared contract used by the happy path and most red cases.
pub(crate) const TWO_OUTPUTS: &str = r#"
outputs = [
  { path = "docs/guide.md", kind = "file", accept = "non-empty file" },
  { path = "docs/reference.md", kind = "file", accept = "non-empty file" },
]
"#;

/// The provider answer that satisfies [`TWO_OUTPUTS`].
pub(crate) const TWO_OUTPUTS_RESULT: &str = r##"{"outputs":[
  {"path":"docs/guide.md","content":"# Guide\n"},
  {"path":"docs/reference.md","content":"# Reference\n"}
]}"##;

/// An envelope carrying the row's effective config, rooted at `project_root`.
pub(crate) fn context(project_root: &Path, row: &ExtensionRegistryRow) -> Context {
    let config = row
        .effective_config()
        .map(|config| {
            config
                .as_table()
                .iter()
                .map(|(key, value)| (key.clone(), Some(serde_json::to_value(value).unwrap())))
                .collect()
        })
        .unwrap_or_default();
    let root = vibe_core::machine_json_path(project_root);
    Context {
        artifacts: Vec::new(),
        envelope: 1,
        execution: Execution {
            config,
            id: row.declaration().id.clone(),
            package: row.provider().to_string(),
        },
        io: Io {
            scratch: format!("{root}/.vibe/lifecycle/run/exec/"),
        },
        point: row.declaration().point.to_string(),
        project: Project {
            kind: "project".into(),
            manifest: format!("{root}/vibe.toml"),
            name: "demo".into(),
            root,
            spec_roots: Vec::new(),
            version: "0.1.0".into(),
        },
        run: Run {
            agent_mode: RunAgentMode::Cli,
            assume_yes: true,
            chain: vec!["create".into()],
            force: false,
            offline: false,
            phase: "create".into(),
            requested: "create".into(),
        },
        world: World {
            deps_root: "vibedeps".into(),
            lockfile: "vibe.lock".into(),
            packages: Vec::new(),
        },
        slot_target: None,
    }
}
