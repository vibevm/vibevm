//! The hosted-agent outbox — one safe Markdown task per parked execution.
//!
//! When the lifecycle runs under an agent host, an agent execution is not
//! paid for; it is PARKED. This cell owns everything durable about that
//! handoff: the deterministic (never authoritative) task filename, the task
//! document — frontmatter carrying the exact run/execution/phase and the
//! ordered output contract, then the two credential-free prose sections the
//! paid call would have carried — its pinned, no-follow, atomic publication
//! under `.vibe/agentic/outbox/<run-id>/`, and the narrow cleanup a satisfied
//! resume performs (exactly the task the state owns, then its proven-empty
//! run directory, nothing else).
//!
//! The filename is a pure function of the execution key, but generated state
//! carries the exact key and task path anyway: nothing ever parses a task
//! filename back into an execution. The document is for the hosting agent;
//! the machine authority is `.vibe/lifecycle.toml`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use std::path::Path;

use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_wire::generated::lifecycle::e1::context::Context;

use crate::agent::PreparedAgent;
use crate::agent::{system_prose, user_prose};
use crate::process::is_valid_run_id;

/// Cap on the complete task document — the envelope cap the process handlers
/// already use. Refusal happens before any write and before the delegated
/// checkpoint.
pub const TASK_CAP: usize = 8 * 1024 * 1024;

/// Every task filename starts here, so `CON.md`, `NUL.md` and friends can
/// never become a Windows device spelling: the basename is `task-…`, and no
/// device name has that stem.
pub const TASK_PREFIX: &str = "task-";

/// Every task filename ends here — including a truncated, digest-suffixed
/// one. The extension is part of the law, not of the encoded stem.
pub const TASK_SUFFIX: &str = ".md";

/// The outbox home, project-relative: `.vibe/agentic/outbox/<run-id>/`.
pub const OUTBOX_RELATIVE: &str = ".vibe/agentic/outbox";

/// A final component longer than this is truncated and digest-suffixed.
const COMPONENT_CAP: usize = 128;
/// `-` plus 16 lowercase hex characters, appended to a truncated stem.
const DIGEST_WIDTH: usize = 17;

/// One typed handoff to the hosting agent: the durable run identity, the
/// ordered task files awaiting it, and the exact command that resumes the
/// run. Human, quiet and JSON rendering all consume this same value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub struct Delegation {
    pub run_id: String,
    pub tasks: Vec<String>,
    pub resume: String,
}

/// Why a delegated task could not be published or cleaned up. No variant
/// carries a credential, a provider endpoint or a response body.
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME")]
pub enum DelegationError {
    #[error(
        "the run id `{run_id}` is not a valid 32-hex identity \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME; \
          fix: allocate the run id through the lifecycle before parking)"
    )]
    RunId { run_id: String },
    #[error(
        "the delegated task document for `{key}` exceeds {TASK_CAP} bytes \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: shorten the prompt document or its embed closure; nothing was written or \
          checkpointed)"
    )]
    TooLarge { key: String },
    /// A publication that failed. `stage` is the fact the caller cannot
    /// re-derive: `BeforePublication` proves the destination is untouched,
    /// `PossiblyPublished` means the deterministic task path MAY already hold
    /// the new bytes. Either way the run refuses before any checkpoint, so
    /// state never points at this task.
    #[error(
        "the delegated task for `{key}` cannot be published to `{path}`: {reason}; {} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: restore a writable, link-free project `.vibe/agentic` and rerun; nothing was \
          checkpointed, so no lifecycle state points at that path)",
        stage_evidence(*stage, path)
    )]
    Publish {
        key: String,
        /// The deterministic task path this attempt targeted — always named,
        /// so an operator can inspect exactly the file that may or may not
        /// exist.
        path: String,
        stage: vibe_safefs::PublishStage,
        reason: String,
    },
    #[error(
        "the recorded task path `{recorded}` is not the task run `{run_id}` owns for execution \
         `{key}` (expected `{expected}`) \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME; \
          fix: remove this erasable cache and rerun the lifecycle)"
    )]
    NotOwned {
        run_id: String,
        key: String,
        recorded: String,
        expected: String,
    },
}

/// The one sentence a crash diagnosis needs: whether the deterministic task
/// path is provably absent or may already exist. Written here rather than in
/// each `#[error]` literal so the two spellings cannot drift apart.
fn stage_evidence(stage: vibe_safefs::PublishStage, path: &str) -> String {
    match stage {
        vibe_safefs::PublishStage::BeforePublication => {
            format!("`{path}` is unchanged — the rename was never attempted")
        }
        vibe_safefs::PublishStage::PossiblyPublished => format!(
            "`{path}` MAY ALREADY EXIST — the rename was attempted, so treat that \
             path as an orphan to inspect"
        ),
    }
}

/// The deterministic task filename for one execution key — never authority.
///
/// The stem is the key percent-encoded over a conservative ASCII unreserved
/// set (uppercase hex), with a trailing `.`/space run re-encoded so the stem
/// cannot end in one. The mandatory `task-` prefix and `.md` suffix are
/// reserved out of the component budget BEFORE truncation, so even a
/// digest-suffixed name for a 300-character key is still a `.md` file. The
/// final component is judged by the one shared component law; nothing
/// downstream parses it back.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub fn task_filename(execution_key: &str) -> Result<String, DelegationError> {
    let budget = COMPONENT_CAP - TASK_PREFIX.len() - TASK_SUFFIX.len();
    let mut stem = encoded_stem(execution_key);
    if stem.len() > budget {
        stem.truncate(escape_boundary(&stem, budget - DIGEST_WIDTH));
        stem.push('-');
        stem.push_str(&digest_suffix(execution_key));
    }
    let component = format!("{TASK_PREFIX}{stem}{TASK_SUFFIX}");
    // The shared law is the final judge, not this encoder's own reasoning:
    // devices, ADS colons, separators, control characters and the staging
    // prefix all refuse here rather than at the filesystem.
    vibe_safefs::ensure_safe_component(&component).map_err(|reason| DelegationError::Publish {
        key: execution_key.to_string(),
        path: format!("{OUTBOX_RELATIVE}/<run>/{component}"),
        stage: vibe_safefs::PublishStage::BeforePublication,
        reason: format!("the derived component is unsafe: {reason}"),
    })?;
    Ok(component)
}

/// The one deterministic project-relative path a `(run id, execution key)`
/// pair owns. State records it verbatim, and every later reader — semantic
/// validation, cleanup — recomputes it rather than trusting the recorded text.
pub fn outbox_task_path(run_id: &str, execution_key: &str) -> Result<String, DelegationError> {
    if !is_valid_run_id(run_id) {
        return Err(DelegationError::RunId {
            run_id: run_id.to_string(),
        });
    }
    Ok(format!(
        "{OUTBOX_RELATIVE}/{run_id}/{}",
        task_filename(execution_key)?
    ))
}

/// The complete task document: frontmatter with the exact run/execution/
/// phase and the ordered output contract, then BOTH prose sections the paid
/// call would have carried — the system contract and the request — labelled
/// and verbatim.
///
/// Every frontmatter string goes through a real serializer. A legal output
/// path may contain a quote, a backslash or YAML-looking text; escaping it
/// through `serde_json` (whose output is a valid YAML double-quoted scalar)
/// means such a path can neither break a row nor add a field.
#[must_use]
fn task_document(
    run_id: &str,
    execution_key: &str,
    phase: &str,
    prepared: &PreparedAgent,
    context: &Context,
) -> String {
    let contract = prepared.contract();
    let mut document = String::new();
    document.push_str("---\n");
    document.push_str(&format!("run: {}\n", yaml_string(run_id)));
    document.push_str(&format!("execution: {}\n", yaml_string(execution_key)));
    document.push_str(&format!("phase: {}\n", yaml_string(phase)));
    document.push_str("outputs:\n");
    for row in contract.rows() {
        document.push_str(&format!(
            "  - path: {}\n    kind: {}\n    accept: {}\n",
            yaml_string(row.path()),
            yaml_string(crate::agent::OUTPUT_KIND_FILE),
            yaml_string(crate::agent::OUTPUT_ACCEPT_NON_EMPTY),
        ));
    }
    document.push_str("---\n\n");
    document.push_str(
        "# Hosted agent task\n\nThe two sections below are the exact credential-free prose the \
         paid provider call would have carried, verbatim.\n\n## System contract\n\n",
    );
    document.push_str(&system_prose());
    document.push_str("\n## Request\n\n");
    document.push_str(&user_prose(prepared.instructions(), context, contract));
    document
}

/// Publish the task document for one parked execution and return its
/// project-relative path. The file is written through the pinned project
/// capability (no-follow ancestors, unique owned stage, atomic rename), and
/// the size cap refuses BEFORE anything is written or checkpointed.
pub(crate) fn publish_task(
    project_root: &Path,
    run_id: &str,
    execution_key: &str,
    phase: &str,
    prepared: &PreparedAgent,
    context: &Context,
) -> Result<String, DelegationError> {
    let relative = outbox_task_path(run_id, execution_key)?;
    let filename = task_filename(execution_key)?;
    let document = task_document(run_id, execution_key, phase, prepared, context);
    if document.len() > TASK_CAP {
        return Err(DelegationError::TooLarge {
            key: execution_key.to_string(),
        });
    }
    let project =
        vibe_safefs::Project::open(project_root).map_err(|reason| DelegationError::Publish {
            key: execution_key.to_string(),
            path: relative.clone(),
            stage: vibe_safefs::PublishStage::BeforePublication,
            reason: format!("the selected project root is unusable: {reason:#}"),
        })?;
    let outbox = project
        .dir(&[".vibe", "agentic", "outbox", run_id], true)
        .map_err(|reason| DelegationError::Publish {
            key: execution_key.to_string(),
            path: relative.clone(),
            stage: vibe_safefs::PublishStage::BeforePublication,
            reason: format!("the outbox run directory is unusable: {reason:#}"),
        })?;
    project
        .write_atomic_in(&outbox, &filename, document.as_bytes())
        // `PublishError`'s own `Display` prints only its source: the two facts
        // a caller cannot re-derive — how far the rename got, and which
        // directories this run created — live in the struct, and `into_report`
        // is what preserves them. Flattening with `{error:#}` would throw away
        // exactly the evidence a crash diagnosis needs.
        .map_err(|error| DelegationError::Publish {
            key: execution_key.to_string(),
            path: relative.clone(),
            stage: error.stage,
            reason: format!("{:#}", error.into_report()),
        })?;
    Ok(relative)
}

/// Remove exactly the task this run owns for this execution, after its
/// success checkpoint is durable, then prune only its proven-empty run
/// directory. Ownership is PROVED by recomputing the deterministic path for
/// `(run_id, execution_key)` and refusing any recorded path that differs — a
/// merely plausible `.vibe/agentic/outbox/<hex>/task-*` spelling is not
/// ownership. Nothing else in the outbox is touched; old unrelated
/// outbox/scratch directories remain general-GC work. `Err` is a notice for
/// the transition's message, never a reason to erase outputs or downgrade the
/// completed execution.
pub(crate) fn cleanup_task(
    project_root: &Path,
    run_id: &str,
    execution_key: &str,
    recorded: &str,
) -> Result<(), String> {
    let expected = outbox_task_path(run_id, execution_key).map_err(|error| error.to_string())?;
    if recorded != expected {
        return Err(DelegationError::NotOwned {
            run_id: run_id.to_string(),
            key: execution_key.to_string(),
            recorded: recorded.to_string(),
            expected,
        }
        .to_string());
    }
    let filename = task_filename(execution_key).map_err(|error| error.to_string())?;
    let project = vibe_safefs::Project::open(project_root).map_err(|error| {
        format!("could not open the project to clean up `{expected}`: {error:#}")
    })?;
    let run_dir = project
        .dir_if_present(&[".vibe", "agentic", "outbox", run_id])
        .map_err(|error| format!("could not reach the run directory of `{expected}`: {error:#}"))?;
    let Some(run_dir) = run_dir else {
        // The task is already gone; the empty run directory may not be. Fall
        // through to the prune so a crash between the two steps still heals.
        return prune_run_directory(&project, run_id, &expected);
    };
    let removed = project
        .remove_file_in(&run_dir, &filename)
        .map_err(|error| format!("could not remove `{expected}`: {error:#}"));
    // Windows refuses to remove a directory an open handle still names, so
    // the run capability is dropped before the prune looks at it again.
    drop(run_dir);
    if !removed? {
        // Not ours to invent: an absent task file with a delegated past is
        // named honestly rather than treated as success.
        return Err(format!(
            "the recorded task `{expected}` was already absent; its run directory was left as \
             found",
        ));
    }
    prune_run_directory(&project, run_id, &expected)
}

fn prune_run_directory(
    project: &vibe_safefs::Project,
    run_id: &str,
    task_relative: &str,
) -> Result<(), String> {
    let outbox = project
        .dir_if_present(&[".vibe", "agentic", "outbox"])
        .map_err(|error| {
            format!("could not reach the outbox to prune `{task_relative}`: {error:#}")
        })?;
    let Some(outbox) = outbox else {
        return Ok(());
    };
    project
        .remove_dir_if_empty(&outbox, run_id)
        .map_err(|error| {
            format!("could not prune the run directory of `{task_relative}`: {error:#}")
        })?;
    Ok(())
}

/// Percent-encode over a conservative ASCII unreserved set — everything else
/// (separators, `#`, colon, wildcard, `%` itself, spaces, every non-ASCII
/// byte) becomes uppercase `%XX`, so the encoding is pure ASCII and injective
/// — then re-encode a trailing `.`/space run so the STEM cannot end in one.
fn encoded_stem(key: &str) -> String {
    let mut encoded = String::with_capacity(key.len());
    for &byte in key.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    // `.` and ` ` are in the safe set, but no component may END with one, and
    // the reserved `.md` suffix must not be what hides that.
    while let Some(&last) = encoded.as_bytes().last() {
        if !matches!(last, b'.' | b' ') {
            break;
        }
        encoded.truncate(encoded.len() - 1);
        encoded.push_str(&format!("%{last:02X}"));
    }
    encoded
}

/// Back a truncation point off a partially copied `%XX` escape, so a
/// truncated stem is still well-formed percent-encoding rather than a
/// dangling `%` or `%A`.
fn escape_boundary(stem: &str, mut keep: usize) -> usize {
    let bytes = stem.as_bytes();
    for back in 1..=2 {
        if keep >= back && bytes[keep - back] == b'%' {
            keep -= back;
            break;
        }
    }
    keep
}

/// Eight digest bytes as sixteen lowercase hex characters: enough that two
/// distinct overlong keys keep distinct filenames, short enough that the
/// component budget still holds a readable prefix.
fn digest_suffix(key: &str) -> String {
    Sha256::digest(key.as_bytes())
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A real serializer, not a format string: `serde_json`'s string output is a
/// valid YAML double-quoted scalar, so quotes, backslashes, newlines and
/// YAML-looking text in a legal path cannot break a row or add a field.
fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

#[cfg(test)]
#[path = "delegation/tests.rs"]
mod tests;
