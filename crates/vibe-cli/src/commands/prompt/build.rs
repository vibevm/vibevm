//! `vibe prompt build`, and the summary shared by command rows and trailers.
//!
//! Failed standalone builds leave their account to the refusal (PROP-076
//! `#SYS-OPERATIONS-PATH-AND-FLAGS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS");

use anyhow::Result;
use specmark::spec;
use vibe_wire::generated::prompt_build as wire;
use vibe_workspace::boot_artifacts::{PromptBuild, PromptPaths, build_node_prompt};

use crate::output;
use crate::output::language::{self, Block, Glyph, Meta};

use super::Selected;

/// The protocol tag `vibe-prompt-build/1`.
pub(crate) const PROTOCOL: &str = "vibe-prompt-build/1";

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) fn run(ctx: &output::Context, selected: &Selected) -> Result<()> {
    let built = build_node_prompt(&selected.workspace, &selected.node_rel);
    let report = report(&selected.node_rel, &built);
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        PromptSummary::of_build(&built).finish(ctx);
    }
    // Rendering never changes the build verdict, including a failed build.
    built.map(|_| ()).map_err(Into::into)
}

/// Lower one build outcome into its registered report.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) fn report(
    node_rel: &str,
    built: &Result<PromptBuild, vibe_workspace::WorkspaceError>,
) -> wire::PromptBuildReport {
    let summary = PromptSummary::of_build(built).text();
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

/// How one build left the system prompt, said once for every surface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) enum PromptState {
    /// Both artifacts were published with these exact byte lengths.
    Rebuilt { system_len: usize, lane_len: usize },
    /// The existing artifacts already matched their inputs.
    Unchanged,
    /// The authored template disappeared and its artifacts were retired.
    Removed,
    /// The project has no authored template.
    OptedOut,
    /// The build refused with this explanation.
    Failed(String),
}

/// One build's outcome and the boot directory it wrote, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) struct PromptSummary {
    /// The outcome shared by every rendering surface.
    pub(crate) state: PromptState,
    /// The artifact directory, present for rebuilt and unchanged outcomes.
    pub(crate) boot_dir: Option<std::path::PathBuf>,
}

impl PromptSummary {
    /// Lower a build result without losing its failure or artifact lengths.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS"
    )]
    pub(crate) fn of_build(built: &Result<PromptBuild, vibe_workspace::WorkspaceError>) -> Self {
        let (state, paths) = match built {
            Err(error) => (PromptState::Failed(error.to_string()), None),
            Ok(PromptBuild::OptedOut) => (PromptState::OptedOut, None),
            Ok(PromptBuild::Removed) => (PromptState::Removed, None),
            Ok(PromptBuild::Rebuilt(paths)) => (
                PromptState::Rebuilt {
                    system_len: paths.system_len,
                    lane_len: paths.lane_len,
                },
                Some(paths),
            ),
            Ok(PromptBuild::Unchanged(paths)) => (PromptState::Unchanged, Some(paths)),
        };
        Self {
            state,
            boot_dir: paths.and_then(|paths| paths.system.parent().map(ToOwned::to_owned)),
        }
    }

    /// The vocabulary shared by the row, JSON text and trailer.
    fn parts(&self) -> (Glyph, &'static str, Vec<String>) {
        match &self.state {
            PromptState::Rebuilt {
                system_len,
                lane_len,
            } => (
                Glyph::Done,
                "rebuilt",
                vec![
                    format!("SYSTEM.md {system_len} bytes"),
                    format!("AGENT-LANE.md {lane_len} bytes"),
                ],
            ),
            PromptState::Unchanged => (
                Glyph::Done,
                "unchanged",
                vec![vibe_workspace::path_to_slash(
                    &vibe_core::layout::current_boot_dir().join(vibe_core::layout::SYSTEM_MD),
                )],
            ),
            PromptState::Removed => (
                Glyph::Done,
                "removed",
                vec![format!(
                    "{} is gone",
                    vibe_workspace::path_to_slash(
                        &vibe_core::layout::current_boot_system_template()
                    ),
                )],
            ),
            PromptState::OptedOut => (
                Glyph::Skipped,
                "not opted in",
                vec![format!(
                    "no {}",
                    vibe_workspace::path_to_slash(
                        &vibe_core::layout::current_boot_system_template()
                    ),
                )],
            ),
            PromptState::Failed(error) => (Glyph::Failed, "build failed", vec![error.clone()]),
        }
    }

    /// The step row (`language::Block`), rendered at `width`.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS"
    )]
    pub(crate) fn row_at(&self, width: usize) -> String {
        let (glyph, word, items) = self.parts();
        let meta = items
            .into_iter()
            .map(|item| {
                if matches!(self.state, PromptState::Failed(_)) {
                    Meta::Alert(item)
                } else {
                    Meta::note(item)
                }
            })
            .collect();
        Block::new(glyph, format!("system prompt  {word}"))
            .with_meta(meta)
            .render_at(width)
    }

    /// The row's words without the glyph: the JSON summary and trailer what.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS"
    )]
    pub(crate) fn text(&self) -> String {
        let (_, word, items) = self.parts();
        let mut text = format!("system prompt {word}");
        for item in items {
            text.push_str(" · ");
            text.push_str(&item);
        }
        text
    }

    /// End a standalone build: trailer, opted-out row and hint, or no failure summary.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS"
    )]
    pub(crate) fn finish(&self, ctx: &output::Context) {
        match &self.state {
            PromptState::Rebuilt { .. } | PromptState::Unchanged => {
                let what = if matches!(self.state, PromptState::Unchanged) {
                    "system prompt unchanged".to_owned()
                } else {
                    self.text()
                };
                if let Some(boot_dir) = &self.boot_dir {
                    ctx.trailer_at(&what, &format!("{}/", boot_dir.display()));
                }
            }
            PromptState::Removed => ctx.trailer(&self.text()),
            PromptState::OptedOut => {
                ctx.block(&self.row_at(language::viewport()));
                ctx.hint("vibe prompt init opts this project in");
            }
            PromptState::Failed(_) => {}
        }
    }
}

/// The step row for agent launch, drawn from the same summary as every build.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) fn summary_line(built: &Result<PromptBuild, vibe_workspace::WorkspaceError>) -> String {
    PromptSummary::of_build(built).row_at(language::viewport())
}

/// A length the report carries as `uint32`. Saturation keeps a report printable
/// even when an artifact exceeds four gigabytes.
fn clamp(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// The two artifacts of a successful build, for a caller that only needs paths.
#[allow(dead_code)]
pub(crate) fn paths(built: &PromptBuild) -> Option<&PromptPaths> {
    built.paths()
}

/// The three owned artifacts before regeneration, so an install can distinguish
/// rebuilt, unchanged and removed without changing the publication's return type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Snapshot {
    system: Option<Vec<u8>>,
    meta: Option<Vec<u8>>,
    lane: Option<Vec<u8>>,
}

impl Snapshot {
    /// Read only the generated artifacts for the node at `project_root`.
    pub(crate) fn of(project_root: &std::path::Path) -> Self {
        let boot = project_root.join(vibe_core::layout::current_boot_dir());
        let read = |name: &str| std::fs::read(boot.join(name)).ok();
        Self {
            system: read(vibe_core::layout::SYSTEM_MD),
            meta: read(vibe_core::layout::SYSTEM_META_JSON),
            lane: read(vibe_core::layout::AGENT_LANE_MD),
        }
    }

    /// Describe what changed; say nothing when neither snapshot has a prefix.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS"
    )]
    pub(crate) fn summary(&self, project_root: &std::path::Path) -> Option<PromptSummary> {
        let after = Self::of(project_root);
        let state = match (&after.system, &after.lane) {
            (Some(_), Some(_)) if *self == after => PromptState::Unchanged,
            (Some(system), Some(lane)) => PromptState::Rebuilt {
                system_len: system.len(),
                lane_len: lane.len(),
            },
            _ if self.system.is_some() || self.lane.is_some() || self.meta.is_some() => {
                PromptState::Removed
            }
            _ => return None,
        };
        Some(PromptSummary {
            state,
            boot_dir: Some(project_root.join(vibe_core::layout::current_boot_dir())),
        })
    }
}

#[cfg(test)]
#[path = "build/tests.rs"]
mod tests;
