//! `vibe prompt build`, and the one summary line every build ends with.
//!
//! **Whoever builds the artifacts says what it did and where.** One line,
//! from `vibe prompt build` and from `vibe install` alike, and a failed
//! build and a project that has not opted in each say so explicitly rather
//! than staying silent — the one outcome a developer cannot act on is the
//! one nobody printed (`#SYS-OPERATIONS-PATH-AND-FLAGS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS");

use anyhow::Result;
use vibe_wire::generated::prompt_build as wire;
use vibe_workspace::boot_artifacts::{PromptBuild, PromptPaths, build_node_prompt};

use crate::output;

use super::Selected;

/// The protocol tag `vibe-prompt-build/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-build/1";

pub(crate) fn run(ctx: &output::Context, selected: &Selected) -> Result<()> {
    let built = build_node_prompt(&selected.workspace, &selected.node_rel);
    let report = report(&selected.node_rel, &built);
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        ctx.summary(&report.summary);
    }
    // The refusal still refuses: the line above is what happened, and a
    // failed build must not exit zero because it managed to say so.
    built.map(|_| ()).map_err(Into::into)
}

/// Lower one build outcome into its registered report.
pub(crate) fn report(
    node_rel: &str,
    built: &Result<PromptBuild, vibe_workspace::WorkspaceError>,
) -> wire::PromptBuildReport {
    let summary = summary_line(built);
    match built {
        Err(error) => wire::PromptBuildReport {
            protocol: PROTOCOL.to_owned(),
            node: node_rel.to_owned(),
            outcome: wire::Outcome::Failed,
            summary,
            system: None,
            lane: None,
            error: Some(error.to_string()),
        },
        Ok(built) => wire::PromptBuildReport {
            protocol: PROTOCOL.to_owned(),
            node: node_rel.to_owned(),
            outcome: match built {
                PromptBuild::OptedOut => wire::Outcome::OptedOut,
                PromptBuild::Removed => wire::Outcome::Removed,
                PromptBuild::Rebuilt(_) => wire::Outcome::Rebuilt,
                PromptBuild::Unchanged(_) => wire::Outcome::Unchanged,
            },
            summary,
            system: built.paths().map(|paths| wire::Artifact {
                path: paths.system.display().to_string(),
                len: clamp(paths.system_len),
            }),
            lane: built.paths().map(|paths| wire::Artifact {
                path: paths.lane.display().to_string(),
                len: clamp(paths.lane_len),
            }),
            error: None,
        },
    }
}

/// The one line, spelled in exactly one place so `vibe install` and `vibe
/// prompt build` cannot print two different sentences about one outcome.
pub(crate) fn summary_line(built: &Result<PromptBuild, vibe_workspace::WorkspaceError>) -> String {
    match built {
        Err(error) => format!("system prompt: build failed — {error}"),
        Ok(PromptBuild::OptedOut) => format!(
            "system prompt: not generated — this project has not opted in (no `{}`)",
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template())
        ),
        Ok(PromptBuild::Removed) => format!(
            "system prompt: removed — the authored `{}` is gone, so the generated artifacts are \
             too",
            vibe_workspace::path_to_slash(&vibe_core::layout::current_boot_system_template())
        ),
        Ok(PromptBuild::Rebuilt(paths)) => format!(
            "system prompt: rebuilt → {} ({}), lane → {} ({})",
            paths.system.display(),
            bytes(paths.system_len),
            paths.lane.display(),
            bytes(paths.lane_len)
        ),
        Ok(PromptBuild::Unchanged(paths)) => {
            format!("system prompt: unchanged → {}", paths.system.display())
        }
    }
}

/// A byte count as the summary line spells it: the exact number, because a
/// rounded one cannot be compared with `vibe prompt path`'s.
fn bytes(len: usize) -> String {
    format!("{len} bytes")
}

/// A length the report carries as `uint32`. A prefix past four gigabytes is
/// not a thing; saturating here keeps a report printable in the one case
/// where refusing it would hide the build's own outcome.
fn clamp(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// The two artifacts of a successful build, for a caller that only needs the
/// paths.
#[allow(dead_code)]
pub(crate) fn paths(built: &PromptBuild) -> Option<&PromptPaths> {
    built.paths()
}

/// The three owned artifacts' digests before a regeneration, so the summary
/// line an install prints can tell `rebuilt` from `unchanged` without the
/// install path having to carry an outcome of its own.
///
/// It is deliberately a *snapshot of files* rather than a hook inside the
/// publication: an install's job is to publish every node's boot, and
/// threading one node's prompt verdict out through that path would add a
/// return value to the code that regenerates every project's boot — for a
/// line of output. The snapshot costs three reads and says the same thing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Snapshot {
    system: Option<Vec<u8>>,
    meta: Option<Vec<u8>>,
    lane: Option<Vec<u8>>,
}

impl Snapshot {
    /// Read the snapshot for the node at `project_root`. A path that is not
    /// a project at all gives an empty snapshot, and the summary it leads to
    /// is the opted-out line, which is the honest answer.
    pub(crate) fn of(project_root: &std::path::Path) -> Self {
        let boot = project_root.join(vibe_core::layout::current_boot_dir());
        let read = |name: &str| std::fs::read(boot.join(name)).ok();
        Self {
            system: read(vibe_core::layout::SYSTEM_MD),
            meta: read(vibe_core::layout::SYSTEM_META_JSON),
            lane: read(vibe_core::layout::AGENT_LANE_MD),
        }
    }

    /// The summary line for what happened between this snapshot and now.
    ///
    /// `None` is "say nothing": the project neither had nor has a generated
    /// prefix, which is every project that has not opted in and is exactly
    /// the case where a line would be noise rather than information.
    pub(crate) fn summary(&self, project_root: &std::path::Path) -> Option<String> {
        let after = Self::of(project_root);
        let boot = project_root.join(vibe_core::layout::current_boot_dir());
        match (&after.system, &after.lane) {
            (Some(_), Some(_)) if *self == after => Some(format!(
                "system prompt: unchanged → {}",
                boot.join(vibe_core::layout::SYSTEM_MD).display()
            )),
            (Some(system), Some(lane)) => Some(format!(
                "system prompt: rebuilt → {} ({}), lane → {} ({})",
                boot.join(vibe_core::layout::SYSTEM_MD).display(),
                bytes(system.len()),
                boot.join(vibe_core::layout::AGENT_LANE_MD).display(),
                bytes(lane.len())
            )),
            // Present before and absent now: the authored template is gone
            // and this regeneration retired what it owned.
            _ if self.system.is_some() || self.lane.is_some() || self.meta.is_some() => {
                Some(format!(
                    "system prompt: removed — the authored `{}` is gone, so the generated \
                     artifacts are too",
                    vibe_workspace::path_to_slash(
                        &vibe_core::layout::current_boot_system_template()
                    )
                ))
            }
            _ => None,
        }
    }
}
