//! `check` — read-only freshness, answered from the sidecar's own witnesses.
//!
//! **The capability contract, stated as what this cell can reach.** It reads
//! retained inputs, compares witnesses and composes in memory. It writes no
//! product, cache, profile or config file; it opens no network connection;
//! and — the part that is easy to get wrong — it never hands control to an
//! effectful native or backend compile provider to recompute freshness
//! (`#SYS-READ-ONLY-CHECK`). The stronger form is what is implemented here:
//! no analysis entrypoint is called **at all**, so no provider can be
//! reached even on a mismatch, and a test over this module's own sources
//! pins that rather than arguing it in prose.
//!
//! **Why that is possible, and why it is fast.** Spike S5 measured
//! provider-free `analyze_node_lane` at 328–342 ms warm on this repository,
//! far too slow to sit in a caller's request path. It is not needed: the
//! lane segment's witness is compared against the **committed** `STATIC.*`
//! on disk, and the continuation plan's identity against the digest of the
//! committed `INDEX.md` — both of which this generation already published.
//! Whether that committed lane is itself behind its sources is `vibe
//! install`'s question, not the prefix's, and conflating the two is what
//! would have put a compile in the request path.
//!
//! **The one assertion the band model lives or dies by.** Changing the bytes
//! of an applicable band-U file under an unchanged plan leaves the verdict
//! `equal`: no continuation row's content is witnessed anywhere, `vibe
//! agent` recomposes `AGENT-LANE.md` at every launch, and a runner reads
//! those files per request. Changing the plan — a row added, removed or
//! reordered, a condition changed — does not (`#SYS-NO-FROZEN-INDEX`,
//! `#SYS-CONTINUATION-CACHEABLE`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use std::path::{Path, PathBuf};

use vibe_core::layout;
use vibe_core::manifest::{HostChoice, Manifest, TargetOs};
use vibe_wire::generated::system_prompt_meta as wire;

use crate::boot_artifacts;
use crate::{WorkspaceError, path_to_slash};

use super::compose::{PrefixSettings, compose_prefix};
use super::markers::sha256_hex;
use super::meta::SIDECAR_PROTOCOL;

/// Which of the six answers one node's published prefix earns.
///
/// Each carries its reasons as rendered sentences rather than as codes: the
/// consumer is a developer reading one line or a program printing it, and a
/// reason that cannot be printed is a reason nobody acts on. `Unavailable`
/// and `FreshnessUnverifiable` each name the exact `vibe prompt build` that
/// would resolve them, because an answer a caller cannot act on is the one
/// failure a freshness report must not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckVerdict {
    /// The published artifacts are the composition of the current inputs.
    Equal,
    /// An input changed. Every reason is named.
    Stale(Vec<String>),
    /// The node opted in and at least one generated artifact is absent.
    Missing(Vec<String>),
    /// An ownership refusal: a generated path is not ours to replace, or is
    /// tracked by git against `#SYS-ARTIFACTS-NOT-TRACKED`.
    Conflict(String),
    /// World integrity, not freshness: something the node declares cannot be
    /// read at all.
    Unavailable(String),
    /// Freshness cannot be established without work this command may not do.
    FreshnessUnverifiable(String),
}

impl CheckVerdict {
    /// The verdict's own one-word name, as a report spells it.
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckVerdict::Equal => "equal",
            CheckVerdict::Stale(_) => "stale",
            CheckVerdict::Missing(_) => "missing",
            CheckVerdict::Conflict(_) => "conflict",
            CheckVerdict::Unavailable(_) => "unavailable",
            CheckVerdict::FreshnessUnverifiable(_) => "freshness_unverifiable",
        }
    }

    /// The reasons, in report order.
    pub fn reasons(&self) -> Vec<String> {
        match self {
            CheckVerdict::Equal => Vec::new(),
            CheckVerdict::Stale(reasons) | CheckVerdict::Missing(reasons) => reasons.clone(),
            CheckVerdict::Conflict(reason)
            | CheckVerdict::Unavailable(reason)
            | CheckVerdict::FreshnessUnverifiable(reason) => vec![reason.clone()],
        }
    }

    /// `true` for the one verdict that needs no action.
    pub fn is_equal(&self) -> bool {
        matches!(self, CheckVerdict::Equal)
    }
}

/// One generated artifact as it stands on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFacts {
    /// The artifact's absolute path. Always a real path, even when the file
    /// is absent: an empty path is the one answer a shell alias turns
    /// silently into a broken launch (`#SYS-OPERATIONS-PATH-AND-FLAGS`).
    pub path: PathBuf,
    /// Whether the file exists.
    pub present: bool,
    /// Its byte length, when present.
    pub len: Option<usize>,
    /// SHA-256 of its bytes, when present.
    pub sha256: Option<String>,
}

impl ArtifactFacts {
    fn read(path: PathBuf) -> Result<Self, WorkspaceError> {
        let bytes = read_owned(&path)?;
        Ok(Self {
            present: bytes.is_some(),
            len: bytes.as_ref().map(Vec::len),
            sha256: bytes.as_ref().map(|bytes| sha256_hex(bytes)),
            path,
        })
    }
}

/// Which generated paths a version-control system tracks.
///
/// A seam rather than a direct call, for the same reason
/// [`PrefixReader`](super::PrefixReader) is one: the ownership question is
/// answered *about* a working tree, and a caller that cannot ask (no
/// repository, no `git`) must be able to say so instead of guessing. A
/// caller that asks nothing passes [`NotTracked`].
///
/// ```
/// use std::path::Path;
/// use vibe_workspace::system_prompt::{NotTracked, TrackedPaths};
///
/// struct Index(Vec<&'static str>);
///
/// impl TrackedPaths for Index {
///     fn is_tracked(&self, path: &Path) -> Option<bool> {
///         Some(self.0.iter().any(|tracked| path.ends_with(tracked)))
///     }
/// }
///
/// let index = Index(vec!["SYSTEM.md"]);
/// assert_eq!(index.is_tracked(Path::new("boot/SYSTEM.md")), Some(true));
/// assert_eq!(index.is_tracked(Path::new("boot/AGENT-LANE.md")), Some(false));
/// // "Cannot tell" is a third answer, and never reported as clean.
/// assert_eq!(NotTracked.is_tracked(Path::new("boot/SYSTEM.md")), None);
/// ```
pub trait TrackedPaths {
    /// Whether `path` is tracked. `None` is "cannot tell", which is
    /// reported as unknown rather than as clean.
    fn is_tracked(&self, path: &Path) -> Option<bool>;
}

/// The answer when nobody asked: nothing is known to be tracked.
pub struct NotTracked;

impl TrackedPaths for NotTracked {
    fn is_tracked(&self, _path: &Path) -> Option<bool> {
        None
    }
}

/// What one check is asked about.
pub struct CheckRequest<'a> {
    /// The node's canonical workspace-relative path (`.` for the root).
    pub node_rel: &'a str,
    /// The node's absolute directory.
    pub node_dir: &'a Path,
    /// The workspace root, against which a continuation row's path resolves.
    pub workspace_root: &'a Path,
    /// The node's own manifest — the authority for `[system_prompt]`.
    pub manifest: &'a Manifest,
    /// A stated model context window in bytes, against which the lane's size
    /// is reported. Absent reports the size and no verdict about it.
    pub model_window: Option<usize>,
    /// The operating system **this session** runs on, as the caller observed
    /// it. `None` is "not known", which warns about nothing.
    ///
    /// A seam rather than a direct `TargetOs::current()`, for the reason
    /// [`TrackedPaths`] is one: the answer is *about* the caller's machine,
    /// and a test must be able to state it instead of inheriting whatever
    /// host happens to run the suite.
    ///
    /// **It decides no gate, ever.** It feeds exactly one sentence — the
    /// host mismatch of [`host_mismatch_warning`](super::host_mismatch_warning)
    /// — and an `os:` row is still selected only by an explicitly supplied
    /// session target, because the generator's own operating system must
    /// never become a session verdict (`#SYS-SESSION-TARGET`).
    pub session_os: Option<TargetOs>,
    /// Which generated paths version control tracks.
    pub tracked: &'a dyn TrackedPaths,
}

/// One node's whole answer: the verdict, the artifacts and the warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCheck {
    /// The node this is about.
    pub node_rel: String,
    /// Whether an authored `SYSTEM-TEMPLATE.md` is present — the whole
    /// opt-in, and nothing else (`#SYS-OPT-IN`).
    pub opted_in: bool,
    /// The verdict.
    pub verdict: CheckVerdict,
    /// `SYSTEM.md`.
    pub system: ArtifactFacts,
    /// `SYSTEM.meta.json`.
    pub meta: ArtifactFacts,
    /// `AGENT-LANE.md`.
    pub lane: ArtifactFacts,
    /// Observations that are not freshness: sizes, budgets, `@` censuses.
    pub warnings: Vec<String>,
}

impl PromptCheck {
    /// The `vibe prompt build` that resolves a non-`equal` verdict for this
    /// node. Spelled once, so every surface that prints a remedy prints the
    /// same command.
    pub fn remedy(&self) -> String {
        if self.node_rel == "." {
            "vibe prompt build".to_owned()
        } else {
            format!("vibe prompt build --node {}", self.node_rel)
        }
    }
}

/// Check one node's published prefix against its current inputs.
pub fn check_node_prompt(request: &CheckRequest<'_>) -> Result<PromptCheck, WorkspaceError> {
    let boot = request.node_dir.join(layout::current_boot_dir());
    let system = ArtifactFacts::read(boot.join(layout::SYSTEM_MD))?;
    let meta = ArtifactFacts::read(boot.join(layout::SYSTEM_META_JSON))?;
    let lane = ArtifactFacts::read(boot.join(layout::AGENT_LANE_MD))?;
    let template = read_owned(
        &request
            .node_dir
            .join(layout::current_boot_system_template()),
    )?;
    let mut answer = PromptCheck {
        node_rel: request.node_rel.to_owned(),
        opted_in: template.is_some(),
        verdict: CheckVerdict::Equal,
        system,
        meta,
        lane,
        warnings: Vec::new(),
    };
    answer.verdict = decide(request, &answer, template)?;
    Ok(answer)
}

/// The verdict proper, kept out of [`check_node_prompt`] so the artifact
/// reads happen exactly once and in one place.
fn decide(
    request: &CheckRequest<'_>,
    answer: &PromptCheck,
    template: Option<Vec<u8>>,
) -> Result<CheckVerdict, WorkspaceError> {
    if let Some(conflict) = tracked_conflict(request, answer) {
        return Ok(CheckVerdict::Conflict(conflict));
    }
    let Some(template) = template else {
        // Opting out is deleting the template, and a leftover artifact is
        // then stale rather than missing: the inputs say there should be
        // none (`#SYS-REMOVAL-AND-CONFLICT`).
        let leftover: Vec<&ArtifactFacts> = [&answer.system, &answer.meta, &answer.lane]
            .into_iter()
            .filter(|facts| facts.present)
            .collect();
        return Ok(if leftover.is_empty() {
            CheckVerdict::Equal
        } else {
            CheckVerdict::Stale(vec![format!(
                "the node has no `{}`, so it has opted out, but {} generated artifact(s) are \
                 still on disk",
                path_to_slash(&layout::current_boot_system_template()),
                leftover.len()
            )])
        });
    };
    let missing: Vec<String> = [
        (&answer.system, layout::SYSTEM_MD),
        (&answer.meta, layout::SYSTEM_META_JSON),
        (&answer.lane, layout::AGENT_LANE_MD),
    ]
    .into_iter()
    .filter(|(facts, _)| !facts.present)
    .map(|(_, name)| format!("`{name}` has not been generated"))
    .collect();
    if !missing.is_empty() {
        return Ok(CheckVerdict::Missing(missing));
    }
    compare(request, answer, &template)
}

/// Compare the published set with the current inputs.
fn compare(
    request: &CheckRequest<'_>,
    answer: &PromptCheck,
    template: &[u8],
) -> Result<CheckVerdict, WorkspaceError> {
    let meta = match read_sidecar(&answer.meta.path)? {
        Ok(meta) => meta,
        Err(reason) => return Ok(CheckVerdict::FreshnessUnverifiable(reason)),
    };
    let settings = PrefixSettings::of(request.manifest)?;
    let mut reasons = declared_changes(&meta, &settings);

    // The lane segment: the **committed** `STATIC.*`, not a recompile.
    let static_path = match boot_artifacts::resolve_static_path(request.node_dir) {
        Ok(path) => path,
        Err(error) => return Ok(CheckVerdict::Unavailable(error.to_string())),
    };
    let lane_bytes = match &static_path {
        Some(path) => read_owned(path)?,
        None => None,
    };
    if meta.lane != wire::LaneSpelling::None && lane_bytes.is_none() {
        return Ok(CheckVerdict::FreshnessUnverifiable(format!(
            "the sidecar witnesses a compiled lane but no `STATIC.*` is on disk, and only a \
             compile can reproduce one — run `{}`",
            answer.remedy()
        )));
    }

    // The plan's identity is the committed INDEX's digest, so no row's
    // content is ever consulted and a band-U edit cannot move the verdict.
    let index = read_owned(&request.node_dir.join(layout::current_boot_index()))?;
    match (
        &meta.index_sha256,
        index.as_ref().map(|bytes| sha256_hex(bytes)),
    ) {
        (Some(witness), Some(current)) if *witness != current => reasons.push(
            "the continuation plan changed — `INDEX.md` differs from the digest the sidecar \
             witnesses"
                .to_owned(),
        ),
        (Some(_), None) => reasons.push(
            "the continuation plan's `INDEX.md` is absent, so its identity cannot be \
                   confirmed"
                .to_owned(),
        ),
        _ => {}
    }

    let captured = match recapture(request, &meta, template, lane_bytes.as_deref(), &settings) {
        Ok(captured) => captured,
        Err(error) => return Ok(CheckVerdict::Unavailable(error.to_string())),
    };
    reasons.extend(input_changes(&meta, &captured));

    if reasons.is_empty() {
        // Only now is composition worth running: every witness agreed, so a
        // difference here is the published bytes not being the composition
        // of their own inputs at all.
        let prefix = compose_prefix(&captured, &settings)?;
        if Some(prefix.sha256.as_str()) != answer.system.sha256.as_deref() {
            reasons
                .push("`SYSTEM.md` is not the composition of its own witnessed inputs".to_owned());
        }
    }
    Ok(if reasons.is_empty() {
        CheckVerdict::Equal
    } else {
        CheckVerdict::Stale(reasons)
    })
}

/// The declared tuning the sidecar recorded against the manifest's current
/// values. A `recipe` or `host` change moves every prefix byte, so it is a
/// staleness of its own and is named as one rather than being discovered as
/// a digest mismatch with no explanation.
fn declared_changes(meta: &wire::SystemPromptMeta, settings: &PrefixSettings) -> Vec<String> {
    let mut reasons = Vec::new();
    if meta.node_id != settings.node_id {
        reasons.push(format!(
            "the node identity is now `{}`; the sidecar witnesses `{}`",
            settings.node_id, meta.node_id
        ));
    }
    if meta.recipe != settings.recipe {
        reasons.push(format!(
            "`[system_prompt] recipe` is now {}; the sidecar witnesses {}",
            settings.recipe, meta.recipe
        ));
    }
    let host = match settings.host {
        HostChoice::Portable => wire::Host::Portable,
        HostChoice::Fixed(vibe_core::manifest::TargetOs::Linux) => wire::Host::Linux,
        HostChoice::Fixed(vibe_core::manifest::TargetOs::Macos) => wire::Host::Macos,
        HostChoice::Fixed(vibe_core::manifest::TargetOs::Windows) => wire::Host::Windows,
    };
    if meta.host != host {
        reasons.push(format!(
            "`[system_prompt] host` is now `{}`; the sidecar witnesses another",
            settings.host
        ));
    }
    reasons
}

/// The ownership half: a generated artifact git tracks is a conflict, by the
/// decision that none of the three is ever committed
/// (`#SYS-ARTIFACTS-NOT-TRACKED`).
fn tracked_conflict(request: &CheckRequest<'_>, answer: &PromptCheck) -> Option<String> {
    [&answer.system, &answer.meta, &answer.lane]
        .into_iter()
        .find(|facts| request.tracked.is_tracked(&facts.path) == Some(true))
        .map(|facts| {
            format!(
                "`{}` is tracked by git; the generated prefix, its sidecar and the agent lane \
                 are produced and never committed — `git rm --cached` it and let the boot \
                 directory's own `.gitignore` cover it",
                path_to_slash(
                    facts
                        .path
                        .strip_prefix(request.workspace_root)
                        .unwrap_or(&facts.path)
                )
            )
        })
}

/// Read one owned path, refusing anything that is not a regular file before
/// opening it.
pub(super) fn read_owned(path: &Path) -> Result<Option<Vec<u8>>, WorkspaceError> {
    crate::safe_file::preflight_absent_or_regular(path)
        .and_then(|()| crate::safe_file::read_optional(path))
        .map_err(|error| WorkspaceError::SystemPromptCapture {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
}

/// Parse the sidecar. A protocol tag this build does not know is
/// `freshness_unverifiable` rather than a refusal: an old consumer need not
/// read a new artifact, and the reverse is just as true (`#SYS-SIDECAR`).
fn read_sidecar(path: &Path) -> Result<Result<wire::SystemPromptMeta, String>, WorkspaceError> {
    let bytes = read_owned(path)?.unwrap_or_default();
    let probe: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(Err(format!(
                "`{}` is not JSON: {error}",
                layout::SYSTEM_META_JSON
            )));
        }
    };
    let protocol = probe.get("protocol").and_then(serde_json::Value::as_str);
    if protocol != Some(SIDECAR_PROTOCOL) {
        return Ok(Err(format!(
            "`{}` carries protocol `{}`, which this build does not read; it knows `{}`",
            layout::SYSTEM_META_JSON,
            protocol.unwrap_or("<absent>"),
            SIDECAR_PROTOCOL
        )));
    }
    Ok(serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "`{}` carries protocol `{SIDECAR_PROTOCOL}` and does not parse as one: {error}",
            layout::SYSTEM_META_JSON
        )
    }))
}

/// Rebuilding the witnessed input set from what is on disk now — out of
/// line along the observation-versus-verdict seam.
#[path = "check/inputs.rs"]
mod inputs;
use inputs::{input_changes, recapture};

#[path = "check/warnings.rs"]
mod warnings;
pub use warnings::{host_mismatch_warning, prompt_warnings};

#[cfg(test)]
#[path = "check/tests.rs"]
pub(super) mod tests;
