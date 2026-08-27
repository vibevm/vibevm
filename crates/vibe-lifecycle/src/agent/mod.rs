//! The `agent` handler — the only lifecycle execution that may spend tokens.
//!
//! Explicit agent declarations are advanced functionality, never the
//! algorithmic floor (PROP-054 `##LLM-IS-AN-ENHANCEMENT`). A project that
//! selects no agent contribution never enters this module, so it constructs no
//! provider configuration, reads no credential and reaches no network.
//!
//! The execution is in two halves, and the split is load-bearing:
//!
//! * [`prepare`] is **credential-free**. It proves the declared output
//!   contract, proves the prompt address names its own provider instance, and
//!   resolves the exact prompt bytes (instructions plus their spec closure)
//!   through the injected backend. Those bytes then enter the freshness
//!   fingerprint, which is what makes an edited prompt rerun instead of
//!   fresh-skipping, and the *same* bytes are handed to [`execute`] — nothing
//!   is resolved twice, so nothing can change between the decision and the use.
//! * [`execute`] is the paid half. It is reached only for a non-fresh
//!   execution, and only after every refusal above has already had its chance.
//!
//! Everything except the transport lives here — contract, addressing, envelope
//! prose, strict result parse, contained atomic write — so the terminal and a
//! later hosted or MCP surface converge on one library behaviour instead of
//! two copies of the output logic.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI");

mod contract;
mod prompt;
mod result;

use std::path::Path;

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::ExtensionHandler;
use vibe_wire::generated::lifecycle::e1::context::{Artifact, Context};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};

use crate::ExtensionRegistryRow;

pub use contract::{OUTPUT_ACCEPT_NON_EMPTY, OUTPUT_KIND_FILE, OutputContract, OutputRow};
pub use prompt::{PromptRequest, ResolvedPrompt};
pub(crate) use prompt::{system_prose, user_prose};
pub use result::{ResultPlan, probe_outputs};

/// Everything the backend is told about one agent execution. It carries prose
/// only: no credential, no endpoint and no token path ever enters a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub struct AgentRequest {
    /// The execution identity, for narration and budgeting only.
    pub key: String,
    /// The lifecycle phase this execution belongs to.
    pub phase: String,
    /// The invariant result discipline.
    pub system: String,
    /// Resolved instructions, the envelope projection and the exact contract.
    pub user: String,
}

/// Provider-independent usage counters reported by a completed call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-CREATE-BUDGET")]
pub struct AgentUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// One completed agent call.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT")]
pub struct AgentCompletion {
    /// The assistant text, expected to be exactly one agent-result document.
    pub text: String,
    /// Counters when the provider reported them; absent is not an error.
    pub usage: Option<AgentUsage>,
}

/// The injected owner of prompt resolution and of the paid call.
///
/// `resolve_prompt` must stay credential-free: it runs before the freshness
/// decision, on every selected agent row, including ones that turn out fresh.
///
/// ```
/// use vibe_lifecycle::agent::{
///     AgentBackend, AgentCompletion, AgentRequest, PromptRequest, ResolvedPrompt,
/// };
///
/// /// A backend that answers from a fixture instead of a provider.
/// struct Canned(&'static str);
///
/// impl AgentBackend for Canned {
///     fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
///         Ok(ResolvedPrompt {
///             text: format!("instructions for {}", request.address),
///             unsupported: Vec::new(),
///         })
///     }
///     fn complete(&self, _request: &AgentRequest) -> Result<AgentCompletion, String> {
///         Ok(AgentCompletion { text: self.0.into(), usage: None })
///     }
/// }
///
/// let backend: &dyn AgentBackend = &Canned(r#"{"outputs":[]}"#);
/// let request = PromptRequest {
///     address: "spec://org.demo/tools/x".into(),
///     provider_root: "vibedeps/org.demo.tools/1.0.0".into(),
///     provider_group: "org.demo".into(),
///     provider_name: "tools".into(),
///     selected_world: Default::default(),
/// };
/// assert!(backend.resolve_prompt(&request).unwrap().text.contains("org.demo/tools/x"));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub trait AgentBackend: Send + Sync {
    /// Resolve one already provider-scoped address inside the exact provider
    /// instance the request names, expanding `#embed` recursively through the
    /// request's selected world and reporting any composition directive this
    /// handler does not perform. Credential-free.
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String>;

    /// Perform the one paid call. Reached only after every guard passed.
    fn complete(&self, request: &AgentRequest) -> Result<AgentCompletion, String>;
}

/// The refusing default. A selected agent contribution is never silently
/// skipped: without a configured backend it fails with the remediation
/// PROP-054 `##AGENT-CLI` requires.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct NoAgentBackend;

impl AgentBackend for NoAgentBackend {
    fn resolve_prompt(&self, _request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        Err(MISSING_BACKEND.into())
    }

    fn complete(&self, _request: &AgentRequest) -> Result<AgentCompletion, String> {
        Err(MISSING_BACKEND.into())
    }
}

const MISSING_BACKEND: &str = "this lifecycle surface configures no agent backend; \
     configure user `[llm]` and invoke through the `vibe` CLI, or run under an agent host";

/// One agent execution's credential-free preparation: proven contract, proven
/// address, and the exact resolved prompt bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub struct PreparedAgent {
    contract: OutputContract,
    request: PromptRequest,
    instructions: String,
    /// The exact rows this execution will produce, already judged by the
    /// generic artifact law against everything the run has accumulated.
    planned: Vec<vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact>,
}

impl PreparedAgent {
    /// The proven output contract, for the freshness probe and the writer.
    #[must_use]
    pub fn contract(&self) -> &OutputContract {
        &self.contract
    }

    /// The exact bytes that must enter the execution fingerprint. Editing the
    /// prompt document — or any document its closure embeds — changes these,
    /// so the next run is `ok`, not `fresh`.
    #[must_use]
    pub fn fingerprint_material(&self) -> (&str, &[u8]) {
        (&self.request.address, self.instructions.as_bytes())
    }

    /// The prevalidated artifact rows. The provider may supply content for
    /// exactly these and nothing else.
    #[must_use]
    pub fn planned_rows(&self) -> &[vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact] {
        &self.planned
    }

    /// The exact resolved instructions — the credential-free prose a hosted
    /// task document publishes for the same execution.
    #[must_use]
    pub fn instructions(&self) -> &str {
        &self.instructions
    }
}

/// Why one agent execution refused. No variant retains a credential, an
/// endpoint or a provider response body.
#[derive(Debug, Clone, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum AgentError {
    #[error(
        "the declared output contract is invalid: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
          fix: declare `config.outputs` as a non-empty ordered array of \
          {{ path = \"project/relative\", kind = \"file\", accept = \"non-empty file\" }})"
    )]
    Contract { reason: String },
    #[error(
        "prompt address `{address}` is unusable: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT; \
          fix: write `handler.prompt` as `spec://<group>/<name>/<doc>#<anchor>`)"
    )]
    PromptAddress { address: String, reason: String },
    #[error(
        "prompt address `{address}` escapes provider `{provider}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT; \
          fix: ship the prompt document inside the contributing package)"
    )]
    PromptProvider {
        address: String,
        provider: String,
        reason: String,
    },
    #[error(
        "prompt `{address}` does not resolve inside its own provider instance: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT; \
          fix: ship an existing document/anchor in the selected provider version)"
    )]
    PromptUnresolved { address: String, reason: String },
    #[error(
        "prompt `{address}` uses composition this handler does not perform: {found} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-AGENT; \
          fix: an agent prompt is one addressed section plus recursive `#embed` \
          expansion — inline the material with `#embed`, or move `#use`/`#source` \
          composition out of the prompt closure)"
    )]
    PromptComposition { address: String, found: String },
    #[error(
        "the planned outputs of `{address}` are not a valid artifact set: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: correct `config.outputs`; nothing was spent and nothing was written)"
    )]
    PlannedArtifacts { address: String, reason: String },
    #[error(
        "declared output `{path}` cannot be published safely: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: remove the link/reparse point or the occupying entry; nothing was \
          spent and nothing was written)"
    )]
    Preflight { path: String, reason: String },
    #[error(
        "the agent provider is unavailable: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
          fix: apply that remediation — a selected agent contribution is never skipped)"
    )]
    Provider { reason: String },
    #[error(
        "the provider result is not the declared agent result: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: nothing was written; rerun once the provider honours the exact JSON contract)"
    )]
    Result { reason: String },
    #[error(
        "declared output `{path}` {reason}. {}{}{} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: inspect the state named above, then rerun)",
        applied_clause(applied),
        possibly_clause(possibly_applied),
        created_clause(created_directories)
    )]
    Output {
        path: String,
        reason: String,
        /// Rows verified on disk before this failure. Per-file replacement is
        /// atomic; the set is not a transaction, so these stay.
        applied: Vec<String>,
        /// The row whose rename had already been attempted — it may be the new
        /// bytes, and this invocation cannot prove otherwise.
        possibly_applied: Vec<String>,
        /// Directories this invocation created. Empty directories are still
        /// observable state, so "no file landed" is not "nothing changed".
        created_directories: Vec<String>,
    },
}

fn applied_clause(applied: &[String]) -> String {
    if applied.is_empty() {
        "No earlier declared output was applied.".to_string()
    } else {
        format!(
            "Per-file replacement is atomic, the set is not: {} ARE already applied and were \
             not rolled back.",
            applied.join(", ")
        )
    }
}

fn possibly_clause(possibly: &[String]) -> String {
    if possibly.is_empty() {
        " The failing row was refused before publication, so it is unchanged.".to_string()
    } else {
        format!(
            " The failing row {} was already renamed into place and MAY hold the new bytes.",
            possibly.join(", ")
        )
    }
}

fn created_clause(created: &[String]) -> String {
    if created.is_empty() {
        String::new()
    } else {
        format!(
            " This run also created the director{} {}, which remain{}.",
            if created.len() == 1 { "y" } else { "ies" },
            created.join(", "),
            if created.len() == 1 { "s" } else { "" },
        )
    }
}

/// The credential-free half. Everything refusable is refused here, before the
/// freshness decision and long before a token is read.
///
/// The order is the order of cost: the declaration first (free), then the
/// filesystem the declaration names (free), then the prompt world (free), and
/// only then — in [`execute`] — the provider.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub(crate) fn prepare(
    backend: &dyn AgentBackend,
    row: &ExtensionRegistryRow,
    context: &Context,
) -> Result<Option<PreparedAgent>, AgentError> {
    let ExtensionHandler::Agent {
        prompt: declared_prompt,
    } = &row.declaration().handler
    else {
        return Ok(None);
    };
    let contract = OutputContract::parse(context)?;
    let request = prompt::provider_scoped(declared_prompt, row, context)?;

    // The complete artifact plan, judged by the same generic law ordinary
    // replies are judged by — against everything this run already produced.
    let planned = contract.planned_rows(&context.project.root);
    crate::artifacts::validate_shape(&planned, &context.artifacts, &context.project.root).map_err(
        |reason| AgentError::PlannedArtifacts {
            address: request.address.clone(),
            reason,
        },
    )?;

    // The filesystem the declaration names, judged no-follow through a pinned
    // capability — against the declared set *and* against what earlier phases
    // already produced. A link, a junction, an occupied ancestor, a hard-linked
    // target or an OS alias to a prior artifact is refused here, for free; the
    // mutation path rechecks for races.
    preflight_outputs(
        Path::new(&context.project.root),
        &context.project.root,
        &contract,
        &context.artifacts,
    )?;

    let resolved =
        backend
            .resolve_prompt(&request)
            .map_err(|reason| AgentError::PromptUnresolved {
                address: request.address.clone(),
                reason,
            })?;
    prompt::refuse_unsupported_composition(&request.address, &resolved.unsupported)?;
    Ok(Some(PreparedAgent {
        contract,
        request,
        instructions: resolved.text,
        planned,
    }))
}

/// Walk every declared output through the capability-relative no-follow
/// checks before anything is spent. Missing ancestors are legal — they will be
/// created no-follow — so this refuses only what already exists and is wrong.
///
/// The comparison set is **one** set: the declared outputs plus every prior
/// artifact this project can legitimately open. The portable key already
/// refused the collisions it can model, for free and with no syscall; this is
/// the gate for the ones it cannot — an 8.3 short spelling, a bind mount, a
/// case-insensitive volume mounted inside a case-sensitive one. Leaving the
/// prior rows out would mean an agent could overwrite an earlier phase's
/// artifact under a name the key calls different, and nothing would notice
/// until the bytes were already gone.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
fn preflight_outputs(
    project_root: &Path,
    project_root_spelling: &str,
    contract: &OutputContract,
    prior: &[Artifact],
) -> Result<(), AgentError> {
    let project =
        vibe_safefs::Project::open(project_root).map_err(|error| AgentError::Preflight {
            path: project_root.to_string_lossy().replace('\\', "/"),
            reason: format!("the selected project root is unusable: {error:#}"),
        })?;
    let declared: Vec<String> = contract.paths();
    // Prior rows outside this project are skipped, not opened: the capability
    // is the boundary, and a recorded row is handler-supplied text. A row that
    // claims to be inside and is malformed refuses, because an artifact we
    // cannot locate is one we cannot prove we are not about to destroy.
    let mut eligible: Vec<String> = Vec::new();
    for row in prior {
        match crate::artifacts::eligible_relative(&row.id, &row.path, project_root_spelling) {
            Ok(Some(relative)) => eligible.push(relative.to_string()),
            Ok(None) => {}
            Err(reason) => {
                return Err(AgentError::Preflight {
                    path: row.path.clone(),
                    reason: format!(
                        "an artifact recorded by phase `{}` cannot be checked against the \
                         declared outputs: {reason}",
                        row.phase,
                    ),
                });
            }
        }
    }
    // Per row, then the set against itself, then the set against everything
    // this run already produced: rows that differ lexically can still be one
    // physical file, and only the filesystem knows.
    project
        .preflight_set_against(&declared, &eligible)
        .map_err(|error| AgentError::Preflight {
            path: declared.join(", "),
            reason: format!("{error:#}"),
        })?;
    Ok(())
}

/// The paid half. Reached only for a non-fresh execution, and only with the
/// exact bytes [`prepare`] already bound into the fingerprint.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub(crate) fn execute(
    backend: &dyn AgentBackend,
    key: &str,
    context: &Context,
    prepared: &PreparedAgent,
) -> Result<Reply, AgentError> {
    let completion = backend
        .complete(&AgentRequest {
            key: key.to_string(),
            phase: context.run.phase.clone(),
            system: prompt::system_prose(),
            user: prompt::user_prose(&prepared.instructions, context, &prepared.contract),
        })
        .map_err(|reason| AgentError::Provider { reason })?;
    // The complete result is proven before any of it is applied. The provider
    // supplied only content: the rows themselves were prevalidated.
    let plan = ResultPlan::parse(&completion.text, &prepared.contract, &prepared.planned)?;
    let artifacts = plan.apply(Path::new(&context.project.root))?;
    Ok(Reply {
        artifacts,
        envelope: 1,
        status: ReplyStatus::Ok,
        tasks: Vec::new(),
        message: Some(message(
            prepared.contract.rows().len(),
            completion.usage.as_ref(),
        )),
    })
}

/// The reply message: provider-independent counters when the provider gave
/// them, and an explicit statement of what atomicity was and was not achieved.
fn message(written: usize, usage: Option<&AgentUsage>) -> String {
    let usage = usage.map_or_else(
        || "usage not reported by the provider".to_string(),
        |usage| {
            format!(
                "usage prompt={} completion={} total={}",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens,
            )
        },
    );
    format!(
        "agent wrote {written} declared output(s); each file was replaced atomically, the set \
         was not one transaction; {usage}"
    )
}

#[cfg(test)]
pub(crate) mod tests;
