//! The one-delivery-mode constraints — `HARNESS-DELIVERY.md` §4, C1–C4.
//!
//! §4 says vibe enforces them "at install and in `vibe prompt check`", and
//! this cell is the half a report can carry. Each constraint is named in its
//! own finding, because a reader who meets one is reading the constraint
//! list beside it.
//!
//! **What each constraint has to enforce in version 1, stated once.** C1,
//! C3 and C4 are written as conditions on `import`, and `import` is refused
//! at **deserialisation** (`HarnessDelivery`), so no manifest can declare
//! it. That does not make them vacuous, and the distinction matters:
//!
//! - **C1** has two halves. The *shape* half — every declared tier must
//!   need the same managed block, since one block's bytes reach every
//!   harness — is [`one_block_shape`], total today and *proved* total by a
//!   test over every [`HarnessDelivery`] variant, so the day `import`
//!   parses the test fails rather than the rule. The *premise* half is
//!   reachable now: the per-harness spelling is only honest while the
//!   instruction files that carry a block carry the **same** block, which
//!   is C1's own "unless byte-identity is given up".
//! - **C2** is `vibe agent`'s, not a report's: it injects only for `launch`,
//!   passes no slot flag and writes no carrier otherwise, and does not
//!   refuse a user's own slot flag when it is about to pass none. That lives
//!   in the command layer beside the launch it governs.
//! - **C4** is the proof table's "several instruction files read by one
//!   harness" row. Identical copies are explicitly admitted there — "two
//!   identical routers, same verdict", duplicated bytes being a stated cost
//!   — so the finding fires only when a harness that reads more than one of
//!   them would read two **different** routers. The nested-discovery half of
//!   C4 is `vibe check`'s `redirect_block` cell and is named rather than
//!   re-implemented, because two scans of one question are two answers.
//! - **C3** — the lane's import-inertness — is already reported, as the
//!   `@`-token census in `check/warnings.rs`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-DELIVERY");

use std::path::Path;

use vibe_core::manifest::{HarnessDelivery, Manifest};

use crate::WorkspaceError;
use crate::boot_artifacts::{BlockLocation, REDIRECT_FILES, locate_block};

use super::check::read_owned;

/// Which shape of managed block one delivery tier needs in the entrypoint.
///
/// The three entrypoint files are kept byte-identical, so the block they
/// carry is one text reaching every harness — which is why a *tier* that
/// needs a different shape is a project-wide choice however per-harness its
/// spelling looks (C1).
///
/// ```
/// use vibe_core::manifest::HarnessDelivery;
/// use vibe_workspace::system_prompt::BlockShape;
///
/// // Both tiers version 1 can declare share one block, which is what makes
/// // `launch` a safe default: started plain, the markers are absent and the
/// // same router has the agent read the files.
/// assert_eq!(BlockShape::of(HarnessDelivery::Launch), BlockShape::Router);
/// assert_eq!(BlockShape::of(HarnessDelivery::Read), BlockShape::Router);
/// assert_eq!(BlockShape::Import.as_str(), "import");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockShape {
    /// The router block: five numbered rules over the lane markers. `read`
    /// and `launch` share it byte for byte.
    Router,
    /// The import block: an `@path` line the harness inlines. Reserved and
    /// unreachable from a manifest in version 1.
    Import,
}

impl BlockShape {
    /// The shape one declared tier needs.
    pub fn of(delivery: HarnessDelivery) -> Self {
        match delivery {
            HarnessDelivery::Launch | HarnessDelivery::Read => BlockShape::Router,
        }
    }

    /// The shape's own name, as a refusal spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            BlockShape::Router => "router",
            BlockShape::Import => "import",
        }
    }
}

/// C1's shape half: the one block shape every declared harness needs.
///
/// `declared` is `(harness name, shape)` in declaration order. An empty list
/// is the default world and answers [`BlockShape::Router`].
///
/// ```
/// use vibe_workspace::system_prompt::{BlockShape, one_block_shape};
///
/// let agreed = [("claude", BlockShape::Router), ("codex", BlockShape::Router)];
/// assert_eq!(one_block_shape(&agreed), Ok(BlockShape::Router));
///
/// let mixed = [("claude", BlockShape::Router), ("gemini", BlockShape::Import)];
/// let refusal = one_block_shape(&mixed).expect_err("C1 refuses the mixture");
/// assert!(refusal.contains("C1"), "{refusal}");
/// assert!(refusal.contains("gemini"), "{refusal}");
/// ```
pub fn one_block_shape(declared: &[(&str, BlockShape)]) -> Result<BlockShape, String> {
    let mut chosen: Option<(&str, BlockShape)> = None;
    for (harness, shape) in declared {
        match chosen {
            None => chosen = Some((harness, *shape)),
            Some((first, first_shape)) if first_shape != *shape => {
                return Err(format!(
                    "C1: `[system_prompt.harness.{first}]` needs the {} block and \
                     `[system_prompt.harness.{harness}]` needs the {} one, but \
                     `CLAUDE.md`, `AGENTS.md` and `GEMINI.md` are kept byte-identical, so one \
                     block's bytes reach every harness. The delivery tier is per harness in \
                     spelling and project-wide in effect unless that byte-identity is given \
                     up — pick one tier for the project, or stop keeping the three files \
                     identical",
                    first_shape.as_str(),
                    shape.as_str()
                ));
            }
            Some(_) => {}
        }
    }
    Ok(chosen.map_or(BlockShape::Router, |(_, shape)| shape))
}

/// C1's shape half over one node's own manifest.
pub fn manifest_block_shape(manifest: &Manifest) -> Result<BlockShape, String> {
    let shapes: Vec<(&str, BlockShape)> = HARNESS_NAMES
        .iter()
        .map(|name| {
            (
                *name,
                BlockShape::of(
                    manifest
                        .system_prompt
                        .as_ref()
                        .map_or(HarnessDelivery::Launch, |section| {
                            section.harness.delivery(name)
                        }),
                ),
            )
        })
        .collect();
    one_block_shape(&shapes)
}

/// The harness names `[system_prompt.harness.<name>]` admits. Spelled here
/// rather than derived from the table's fields so a new harness is a
/// deliberate addition to the constraint sweep too.
const HARNESS_NAMES: [&str; 2] = ["claude", "codex"];

/// The constraints a node's files can violate, as rendered findings in
/// constraint order.
///
/// One scan over the node's instruction files answers both reachable halves:
/// C1's premise (the blocks are not the same text) and C4's row (a harness
/// that reads more than one of them reads two different routers). They fire
/// on one condition and say two different things, because one is about the
/// declaration being incoherent and the other about what a single harness
/// ends up with.
pub fn delivery_constraints(
    node_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<String>, WorkspaceError> {
    let mut findings = Vec::new();
    if let Err(refusal) = manifest_block_shape(manifest) {
        findings.push(refusal);
    }
    let mut blocks: Vec<(&str, String)> = Vec::new();
    for name in REDIRECT_FILES {
        let Some(bytes) = read_owned(&node_dir.join(name))? else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        // A malformed pair is `vibe check`'s `redirect_block` error and a
        // refusal on the write path; comparing a defect against a good
        // block would report the same file twice under two names.
        if let BlockLocation::Present { start, end } = locate_block(&text) {
            blocks.push((name, text[start..end].to_owned()));
        }
    }
    let Some((_, body)) = blocks.first() else {
        return Ok(findings);
    };
    // Named as a set rather than against a reference copy: which of them a
    // developer edited is not knowable from the bytes, and naming one as
    // the authority would blame whichever file happens to sort first.
    if blocks.iter().any(|(_, other)| other != body) {
        let carriers: Vec<&str> = blocks.iter().map(|(name, _)| *name).collect();
        let carriers = carriers.join("`, `");
        findings.push(format!(
            "C1: the managed blocks of `{carriers}` are not all the same text, so the \
             project's instruction files are no longer byte-identical and a per-harness \
             `[system_prompt.harness.<name>] delivery` no longer describes what any one \
             harness receives — regenerate them with `vibe install`"
        ));
        findings.push(format!(
            "C4: a harness that reads more than one of `{carriers}` — Gemini CLI and Qwen \
             Code read several names and concatenate them — would read two different \
             routers. Identical copies are admitted and cost only their own bytes; \
             differing ones are a contradiction the agent cannot resolve. The \
             nested-discovery half of C4 is `vibe check`'s `redirect_block` finding"
        ));
    }
    Ok(findings)
}

#[cfg(test)]
#[path = "constraints/tests.rs"]
mod tests;
