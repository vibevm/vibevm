//! Strict generated-result parsing and the contained capability-relative write.
//!
//! The provider's answer is machine data, so it is parsed through the
//! generated epoch-1 `AgentResult` type — never a hand-rolled JSON walk, never
//! a `serde_json::Value` catch-all. The **complete** result is validated
//! against the declared contract before a single byte reaches the project, so
//! an incomplete or reordered answer leaves the tree exactly as it was.
//!
//! Mutation goes through `vibe_safefs`: the project root is a pinned
//! capability, every ancestor is opened one component at a time without
//! following links, the stage is a `create_new` file this process provably
//! owns, and publication is a capability-relative rename. Nothing here
//! canonicalises a path and then re-opens it with ambient authority, so an
//! ancestor swapped between the check and the write cannot redirect it.
//!
//! What is deliberately not claimed: several filesystem paths are not one
//! transaction. Each file is replaced atomically; the set is applied in
//! declaration order, and a failure part-way through **names the rows already
//! applied** rather than leaving the operator to guess.

use std::path::Path;

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::reply::ReplyArtifact;
use vibe_wire::generated::lifecycle_state::StateArtifact;
use vibe_wire::generated::llm::openai_compatible::e1::agent_result::AgentResult;

use super::AgentError;
use super::contract::OutputContract;

/// Cap on the assistant document. A create result is file bodies, not a
/// stream; 8 MiB matches the envelope cap the process handlers already use.
const RESULT_CAP: usize = 8 * 1024 * 1024;

/// A fully validated result: every row matched its contract row and carries
/// acceptable content. Nothing has been written yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct ResultPlan {
    rows: Vec<PlannedOutput>,
    /// The prevalidated canonical artifact rows this plan will return. Owned,
    /// not accepted at apply time: a caller that could hand in "some rows of
    /// the same length" could return one contract's identities for another
    /// contract's files, and `debug_assert` would not catch it in release.
    planned: Vec<ReplyArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedOutput {
    path: String,
    content: String,
}

impl ResultPlan {
    /// Parse exactly one generated document and prove it equals the contract.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn parse(
        text: &str,
        contract: &OutputContract,
        planned: &[ReplyArtifact],
    ) -> Result<Self, AgentError> {
        if text.len() > RESULT_CAP {
            return Err(AgentError::Result {
                reason: format!("the result exceeds {RESULT_CAP} bytes"),
            });
        }
        let mut documents = serde_json::Deserializer::from_str(text).into_iter::<AgentResult>();
        let result = documents
            .next()
            .ok_or_else(|| AgentError::Result {
                reason: "the provider returned no JSON document".into(),
            })?
            .map_err(|error| AgentError::Result {
                reason: format!(
                    "the provider result is not one strict agent-result document: {error}"
                ),
            })?;
        // A trailing document is refused rather than ignored: two answers mean
        // the provider did not honour the contract, and silently taking the
        // first would make that indistinguishable from honouring it.
        if !text[documents.byte_offset()..].trim_start().is_empty() {
            return Err(AgentError::Result {
                reason: "the provider returned more than one JSON document".into(),
            });
        }
        let declared = contract.rows();
        if result.outputs.len() != declared.len() {
            return Err(AgentError::Result {
                reason: format!(
                    "the provider returned {} output row(s); the contract declares {}",
                    result.outputs.len(),
                    declared.len(),
                ),
            });
        }
        let mut rows = Vec::with_capacity(declared.len());
        for (index, (produced, expected)) in result.outputs.iter().zip(declared).enumerate() {
            if produced.path != expected.path() {
                return Err(AgentError::Result {
                    reason: format!(
                        "output row {index} is `{}`; the contract declares `{}` at that position \
                         (paths must match exactly once, in declaration order)",
                        echoable(&produced.path),
                        expected.path(),
                    ),
                });
            }
            if produced.content.is_empty() {
                return Err(AgentError::Result {
                    reason: format!(
                        "output row {index} (`{}`) is empty and cannot satisfy its declared \
                         `non-empty file` acceptance",
                        echoable(&produced.path),
                    ),
                });
            }
            rows.push(PlannedOutput {
                path: produced.path.clone(),
                content: produced.content.clone(),
            });
        }
        Ok(Self {
            rows,
            planned: planned.to_vec(),
        })
    }

    /// Publish every validated row and return this plan's **own** prevalidated
    /// artifact rows.
    ///
    /// The rows were judged by the generic artifact law before a token was
    /// spent and have travelled inside the plan ever since; the provider
    /// supplied content only. Nothing here re-derives an id, a kind or a path
    /// from provider text, and there is no parameter through which a caller
    /// could substitute a different contract's identities.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub(crate) fn apply(&self, project_root: &Path) -> Result<Vec<ReplyArtifact>, AgentError> {
        let project =
            vibe_safefs::Project::open(project_root).map_err(|error| AgentError::Output {
                path: machine_path(project_root),
                reason: format!("the selected project root is unusable: {error:#}"),
                applied: Vec::new(),
                possibly_applied: Vec::new(),
                created_directories: Vec::new(),
            })?;
        let mut applied: Vec<String> = Vec::with_capacity(self.rows.len());
        let mut created: Vec<String> = Vec::new();
        for row in &self.rows {
            let published = match project.write_atomic(&row.path, row.content.as_bytes()) {
                Ok(published) => published,
                Err(error) => {
                    created.extend(error.created_display());
                    return Err(partial(&applied, &created, &row.path, error));
                }
            };
            created.extend(
                published
                    .created_directories
                    .iter()
                    .map(|path| machine_path(path)),
            );
            match project.probe_regular_nonempty(&row.path) {
                vibe_safefs::Presence::RegularNonEmpty => {}
                // The rename already happened, so this row is *possibly* on
                // disk even though its acceptance failed.
                presence => {
                    return Err(AgentError::Output {
                        path: row.path.clone(),
                        reason: format!(
                            "{} after publication, so its declared `non-empty file` acceptance \
                             fails",
                            match presence {
                                vibe_safefs::Presence::Absent => "is absent",
                                _ => "is not a regular, exclusively-owned, non-empty file",
                            }
                        ),
                        applied: applied.clone(),
                        possibly_applied: vec![row.path.clone()],
                        created_directories: created.clone(),
                    });
                }
            }
            applied.push(row.path.clone());
        }
        Ok(self.planned.clone())
    }
}

/// The declared-output freshness probe: credential-free, provider-free, and
/// exactly the acceptance the contract promised.
///
/// The recorded rows must equal the rows the contract *would* produce —
/// ordered, and in every field. Comparing ids alone would let a tampered
/// `path` (including one pointing outside the project) or a tampered `kind`
/// survive into the hydrated envelope, where a later contribution would treat
/// it as a real artifact this run produced.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
pub fn probe_outputs(
    project_root: &Path,
    contract: &OutputContract,
    recorded: &[StateArtifact],
) -> bool {
    let expected = contract.planned_state_rows(&machine_path(project_root));
    if recorded != expected.as_slice() {
        return false;
    }
    let Ok(project) = vibe_safefs::Project::open(project_root) else {
        return false;
    };
    contract.rows().iter().all(|row| {
        project.probe_regular_nonempty(row.path()) == vibe_safefs::Presence::RegularNonEmpty
    })
}

/// Never hide partial state. Three separate facts, because they answer three
/// different operator questions: which rows are certainly on disk, which one
/// might be, and what else this invocation created on the way.
fn partial(
    applied: &[String],
    created: &[String],
    failed: &str,
    error: vibe_safefs::PublishError,
) -> AgentError {
    let possibly = match error.stage {
        vibe_safefs::PublishStage::BeforePublication => Vec::new(),
        vibe_safefs::PublishStage::PossiblyPublished => vec![failed.to_string()],
    };
    AgentError::Output {
        path: failed.to_string(),
        reason: format!("{error:#}"),
        applied: applied.to_vec(),
        possibly_applied: possibly,
        created_directories: created.to_vec(),
    }
}

/// The only provider-supplied bytes a diagnostic may quote: the offending
/// path, bounded. File contents never appear in an error, and a hostile
/// provider cannot turn a refusal into a log-flooding channel by returning a
/// megabyte-long path.
fn echoable(path: &str) -> String {
    const LIMIT: usize = 120;
    if path.chars().count() <= LIMIT {
        return path.to_string();
    }
    let head: String = path.chars().take(LIMIT).collect();
    format!("{head}… (truncated)")
}

fn machine_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
