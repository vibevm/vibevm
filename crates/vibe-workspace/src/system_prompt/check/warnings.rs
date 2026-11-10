//! The observations `check` reports that are **not** freshness.
//!
//! Out of the verdict on purpose, and out of line along that seam. None of
//! them can make a published prefix wrong — a lane whose bodies have drifted
//! is *expected*, because `vibe agent` recomposes it at launch — so folding
//! any of them into `stale` would make the one assertion the band model
//! rests on fail (`#SYS-NO-FROZEN-INDEX`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use vibe_core::layout;
use vibe_core::manifest::{HostChoice, TargetOs};

use crate::WorkspaceError;
use crate::boot_artifacts;

use super::super::constraints::delivery_constraints;
use super::super::markers::verify_lane_markers;
use super::super::verify::ImportInertness;
use super::{CheckRequest, PromptCheck, read_owned};

/// The one-line host mismatch: this session's own operating system against a
/// **fixed** `[system_prompt] host`.
///
/// **It warns and it never refuses**, which is the whole decision
/// (`#SYS-HOST-AND-TARGET`). A fixed host is a deliberate project
/// declaration, and refusing would leave a cross-platform worker unable to
/// read its own project's instructions — so the sentence says what the
/// prefix carries and that it is served anyway.
///
/// Silent in three cases, each for its own reason. `host = "portable"` and
/// an absent `host` are the same value and carry **no** host decision, so
/// there is nothing to differ from. An equal pair is the expected case. And
/// a session OS this build does not recognise is `None`: inventing a verdict
/// about an unknown platform is worse than saying nothing, exactly as an
/// unknown session gate is never resolved to `true` (`#SYS-SESSION-TARGET`).
///
/// ```
/// use vibe_core::manifest::{HostChoice, TargetOs};
/// use vibe_workspace::system_prompt::host_mismatch_warning;
///
/// // Portable never warns, whatever the session is.
/// assert_eq!(
///     host_mismatch_warning(HostChoice::Portable, Some(TargetOs::Windows)),
///     None
/// );
/// // A fixed host that matches is silent too.
/// let fixed = HostChoice::Fixed(TargetOs::Linux);
/// assert_eq!(host_mismatch_warning(fixed, Some(TargetOs::Linux)), None);
/// // An unrecognised session platform is not a verdict.
/// assert_eq!(host_mismatch_warning(fixed, None), None);
/// // A difference names both operating systems and says it continues.
/// let warning = host_mismatch_warning(fixed, Some(TargetOs::Macos)).unwrap();
/// assert!(warning.contains("linux") && warning.contains("macos"));
/// assert!(warning.contains("rather than refused"));
/// ```
pub fn host_mismatch_warning(host: HostChoice, session_os: Option<TargetOs>) -> Option<String> {
    let declared = host.fixed()?;
    let session = session_os?;
    (declared != session).then(|| {
        format!(
            "`[system_prompt] host` declares `{declared}` and this session runs on `{session}`, \
             so the prefix bakes in the `os:{declared}` continuation rows and carries none for \
             `{session}`; it is served as declared rather than refused"
        )
    })
}

/// The observations that are not freshness: the host mismatch, marker
/// integrity, the import census, the Codex budget and the lane's size
/// against a stated window.
///
/// Out of the verdict on purpose. None of them can make a published prefix
/// wrong — a lane whose bodies have drifted is *expected*, because `vibe
/// agent` recomposes it at launch — so folding any of them into `stale`
/// would make the one assertion the band model rests on fail.
pub fn prompt_warnings(
    request: &CheckRequest<'_>,
    answer: &PromptCheck,
) -> Result<Vec<String>, WorkspaceError> {
    let mut warnings = Vec::new();
    // First, because it is about the whole prefix rather than about one
    // artifact, and because it is the one observation here that is a fact
    // about the *reader* rather than about the repository.
    warnings.extend(host_mismatch_warning(
        request
            .manifest
            .system_prompt
            .as_ref()
            .map(|section| section.host)
            .unwrap_or_default(),
        request.session_os,
    ));
    if let Some(bytes) = read_owned(&answer.lane.path)? {
        match verify_lane_markers(&bytes) {
            Ok(segments) => {
                let inertness =
                    ImportInertness::of(layout::AGENT_LANE_MD, &String::from_utf8_lossy(&bytes));
                if !inertness.is_inert() {
                    warnings.push(format!(
                        "the agent lane carries {} line-initial `@` token(s) outside a fence; \
                         the `import` delivery tier stays refused in version 1 for exactly that \
                         reason",
                        inertness.line_initial()
                    ));
                }
                if let Some(window) = request.model_window.filter(|window| bytes.len() > *window) {
                    warnings.push(format!(
                        "the agent lane is {} bytes against a stated {window}-byte context \
                         window, so it does not fit",
                        bytes.len()
                    ));
                }
                warnings.push(format!(
                    "the agent lane delivers {} segment(s), {} bytes",
                    segments.len(),
                    bytes.len()
                ));
            }
            Err(error) => warnings.push(format!("the agent lane's markers do not verify: {error}")),
        }
    }
    warnings.extend(entrypoint_warnings(request)?);
    // The one-delivery-mode constraints of `HARNESS-DELIVERY.md` §4, which
    // that section places "at install and in `vibe prompt check`". They are
    // observations rather than freshness for the same reason as the rest of
    // this cell: a divergent instruction file does not make a published
    // prefix wrong, and folding it into `stale` would send a caller to
    // `vibe prompt build`, which is not the remedy.
    warnings.extend(delivery_constraints(request.node_dir, request.manifest)?);
    Ok(warnings)
}

/// The entrypoint's own two: how many managed blocks it carries, and what a
/// Codex truncation at the declared limit would cost.
fn entrypoint_warnings(request: &CheckRequest<'_>) -> Result<Vec<String>, WorkspaceError> {
    let mut warnings = Vec::new();
    let Some(name) = request
        .manifest
        .system_prompt
        .as_ref()
        .map(|section| section.entrypoint)
        .unwrap_or_default()
        .file_name()
    else {
        return Ok(warnings);
    };
    let Some(bytes) = read_owned(&request.node_dir.join(name))? else {
        return Ok(warnings);
    };
    let text = String::from_utf8_lossy(&bytes);
    let opens = text
        .lines()
        .filter(|line| line.trim() == boot_artifacts::BLOCK_OPEN)
        .count();
    if opens != 1 {
        warnings.push(format!(
            "`{name}` carries {opens} `{}` marker line(s); exactly one managed block is the \
             only shape composition can splice",
            boot_artifacts::BLOCK_OPEN
        ));
    }
    let limit = request
        .manifest
        .system_prompt
        .as_ref()
        .map_or(boot_artifacts::PROJECT_DOC_MAX_BYTES, |section| {
            section.codex_doc_max_bytes()
        });
    if let Some(message) = boot_artifacts::CodexBudget::whole_file(name, bytes.len(), limit) {
        warnings.push(message);
    }
    Ok(warnings)
}
