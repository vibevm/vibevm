//! Compose: a pure function from witnessed inputs to the prefix bytes and
//! the segment map.
//!
//! Three segments in one fixed order — the template, the entrypoint export,
//! the compiled static lane (`#SYS-BAND-ORDER`). That is PROP-009
//! `#SESSION-START-ORDER` with the user's own frame prepended, so no
//! authored instruction moves relative to another; it is *also* the
//! cache-optimal order, because a provider cache matches a **prefix**, and
//! the lane is the segment that changes most often while the template
//! changes least. The two constraints point the same way and neither is
//! being traded against the other — recorded so nobody "optimises" it later.
//!
//! **Determinism has named mechanics, and this module is where they live.**
//! The order is fixed above; the encoding is UTF-8 with no BOM; frame lines
//! use LF and the composed bytes end with a newline the composer owns; and a
//! segment **body** is the admitted bytes verbatim, with one LF inserted
//! before the next frame when the body does not already end with one — so an
//! authored CRLF document keeps CRLF inside its own segment while the
//! surrounding structure is identical on every host (`#SYS-SEGMENT-FRAMING`,
//! `#SYS-DETERMINISM`).
//!
//! **Nothing volatile.** No build time, no absolute checkout or `HOME` path,
//! no branch, HEAD or dirty state, no session id, no model name, no token
//! estimate, no environment detection — and no whole-build digest before the
//! content, which would make a late change destroy the whole prefix's match
//! (`#SYS-NO-VOLATILE`). A declared `host` is read from the node's own
//! manifest and is therefore an input, not a detection
//! (`#SYS-HOST-AND-TARGET`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-BAND-ORDER");

use std::ops::Range;

use vibe_core::manifest::{HostChoice, LaneForm, Manifest, TargetSelection};

use crate::WorkspaceError;
use crate::boot_artifacts::{BlockLocation, locate_block};

use super::contract::{CURRENT_RECIPE, SuppliedSource, render_contract};
use super::markers::{marker_field, sha256_hex};
use super::verify::{ImportInertness, refuse_marker_shaped_lines, verify_no_recursion};
use super::{CapturedInput, CapturedPrefixInputs, compose_error};

/// The frame keyword that introduces a prefix segment. Distinct from the
/// lane's `vibe:c1` provenance comments and from `vibe:lane-begin`, and
/// never an exactly standalone `<vibevm>` line — so `locate_block` can never
/// mistake a frame for a managed marker, and a frame can never be mistaken
/// for lane content (`#SYS-SEGMENT-FRAMING`).
pub const SEGMENT_FRAME: &str = "vibe:prompt-segment";

/// Which body a segment carries. Ids are layer-qualified from version 1,
/// although no other layer exists yet, so two layers' sidecars merge without
/// collision (`#SYS-LAYER-CONCATENATION`).
///
/// ```
/// use vibe_workspace::system_prompt::SegmentId;
///
/// assert_eq!(SegmentId::Lane.as_str(), "project.lane");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentId {
    /// The authored template's bytes.
    Template,
    /// The selected entrypoint, with its managed block replaced in place.
    Entrypoint,
    /// The transport contract on its own, used only when the entrypoint
    /// carries no managed block to replace — then the entrypoint's bytes
    /// stay whole and the contract becomes its own trailing segment
    /// (`#SYS-REDIRECT-IN-PLACE`).
    Contract,
    /// The compiled static lane, verbatim and opaque.
    Lane,
}

impl SegmentId {
    /// The layer-qualified id, as a frame line and a sidecar spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            SegmentId::Template => "project.template",
            SegmentId::Entrypoint => "project.entrypoint",
            SegmentId::Contract => "project.contract",
            SegmentId::Lane => "project.lane",
        }
    }
}

/// One segment's body inside the composed bytes. `offset` and `len` slice
/// exactly that body — the frame line is not part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Which body this is.
    pub id: SegmentId,
    /// Byte offset of the body in the composed bytes.
    pub offset: usize,
    /// Byte length of the body.
    pub len: usize,
    /// Whether the **end** of this segment is a recommended cache
    /// breakpoint. Two positions earn one: the end of the whole prefix (the
    /// end of band P), and the position just before the lane — which is what
    /// makes `#SYS-BAND-ORDER`'s argument pay, since a lane change then
    /// costs only the lane's own cache entry.
    pub cache_boundary: bool,
}

/// The composed prefix: its bytes, their digest, the segment map, and the
/// import-inertness census of what it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedPrefix {
    /// The prefix bytes — what `SYSTEM.md` holds.
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`, which proves byte identity and nothing else
    /// (`#SYS-TWO-IDENTITIES`).
    pub sha256: String,
    /// The segments in composition order.
    pub segments: Vec<Segment>,
    /// The `@`-initial lines the prefix carries, reported and never edited
    /// (`#SYS-IMPORT-INERTNESS`).
    pub inertness: ImportInertness,
}

/// The declared tuning a composition needs, read from the node's own
/// manifest and from nowhere else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixSettings {
    /// The node's declared identity, as a lane marker spells it.
    pub node_id: String,
    /// `host` — where the agent runs. A fixed value resolves `os:` rows at
    /// build time; `portable` leaves them gated.
    pub host: HostChoice,
    /// `target` — which operating systems the product is for.
    pub target: TargetSelection,
    /// How the lane enters. `verbatim` is this version's only value.
    pub lane_form: LaneForm,
    /// The transport-contract recipe to render.
    pub recipe: u32,
}

impl PrefixSettings {
    /// Read the settings off a node's manifest.
    ///
    /// Two refusals, both before anything is composed: a node that declares
    /// no `[project]` or `[package]` identity has no node id a marker line
    /// could carry, and an identity that cannot be written into an XML
    /// comment is named rather than sanitised.
    pub fn of(manifest: &Manifest) -> Result<Self, WorkspaceError> {
        let section = manifest.system_prompt.clone().unwrap_or_default();
        let node_id = node_id(manifest)?;
        marker_field("node id", &node_id)?;
        Ok(Self {
            node_id,
            host: section.host,
            target: section.target,
            lane_form: section.lane_form,
            recipe: section.recipe.unwrap_or(CURRENT_RECIPE),
        })
    }
}

/// The node's identity: `<group>/<name>` wherever a group exists, because
/// that is the coordinate the rest of the system addresses a node by, and a
/// bare name would let a nested project's marker match an outer project's
/// lane.
fn node_id(manifest: &Manifest) -> Result<String, WorkspaceError> {
    if let Some(package) = &manifest.package {
        return Ok(format!("{}/{}", package.group, package.name));
    }
    if let Some(project) = &manifest.project {
        return Ok(match &project.group {
            Some(group) => format!("{group}/{}", project.name),
            None => project.name.clone(),
        });
    }
    Err(compose_error(
        "node identity",
        "the node's manifest declares neither `[project]` nor `[package]`, so there is no \
         identity a lane segment marker could carry"
            .to_string(),
    ))
}

/// One body on its way into the composed bytes, with the range of it the
/// composer itself wrote.
struct Part {
    id: SegmentId,
    body: String,
    generated: Option<Range<usize>>,
}

/// Compose the prefix from witnessed inputs and declared settings.
///
/// Pure: no filesystem, no clock, no environment, no child process, no
/// socket — pinned by a test over this module's own sources. The same inputs
/// give the same bytes on every host, which is the claim `#SYS-DETERMINISM`
/// makes and the reason a provider that caches on an exact prefix match can
/// cache these bytes.
pub fn compose_prefix(
    inputs: &CapturedPrefixInputs,
    settings: &PrefixSettings,
) -> Result<ComposedPrefix, WorkspaceError> {
    let supplied: Vec<SuppliedSource<'_>> = inputs
        .inputs()
        .map(|input| SuppliedSource {
            role: input.role,
            relative: input.relative.as_str(),
        })
        .collect();
    let contract = render_contract(settings.recipe, &supplied)?;

    let mut parts = vec![Part {
        id: SegmentId::Template,
        body: text_of(&inputs.template)?,
        generated: None,
    }];
    match &inputs.entrypoint {
        Some(entrypoint) => parts.extend(entrypoint_parts(entrypoint, &contract)?),
        None => parts.push(Part {
            id: SegmentId::Contract,
            body: contract.clone(),
            generated: Some(0..contract.len()),
        }),
    }
    if let Some(lane) = &inputs.lane {
        parts.push(Part {
            id: SegmentId::Lane,
            body: text_of(&lane.input)?,
            generated: None,
        });
    }

    assemble(parts)
}

/// The entrypoint export: the admitted bytes with the vibe-owned managed
/// block's exact byte range replaced by the transport contract, and **every
/// other byte verbatim** (`#SYS-REDIRECT-IN-PLACE`,
/// `#SYS-NO-REWRITE-OF-AUTHORED-TEXT`).
///
/// A node whose entrypoint carries no managed block keeps its bytes whole
/// and gets the contract as its own trailing segment. A malformed marker
/// pair is a named refusal, never a repair of a co-tenant document.
fn entrypoint_parts(
    entrypoint: &CapturedInput,
    contract: &str,
) -> Result<Vec<Part>, WorkspaceError> {
    let text = text_of(entrypoint)?;
    match locate_block(&text) {
        BlockLocation::Malformed(reason) => Err(compose_error(
            "entrypoint export",
            format!(
                "the managed `<vibevm>` block of `{}` is malformed: {reason}",
                entrypoint.relative
            ),
        )),
        BlockLocation::Present { start, end } => {
            let mut body = String::with_capacity(text.len() + contract.len());
            body.push_str(&text[..start]);
            let generated = body.len()..body.len() + contract.len();
            body.push_str(contract);
            body.push_str(&text[end..]);
            Ok(vec![Part {
                id: SegmentId::Entrypoint,
                body,
                generated: Some(generated),
            }])
        }
        BlockLocation::Absent => Ok(vec![
            Part {
                id: SegmentId::Entrypoint,
                body: text,
                generated: None,
            },
            Part {
                id: SegmentId::Contract,
                body: contract.to_owned(),
                generated: Some(0..contract.len()),
            },
        ]),
    }
}

/// Frame, body, one owned LF — then verify and witness.
fn assemble(parts: Vec<Part>) -> Result<ComposedPrefix, WorkspaceError> {
    let mut text = String::new();
    let mut segments: Vec<Segment> = Vec::with_capacity(parts.len());
    let mut generated: Vec<Range<usize>> = Vec::new();
    let mut inertness = ImportInertness::default();
    for part in &parts {
        refuse_marker_shaped_lines(part.id.as_str(), &part.body)?;
        inertness.add(part.id.as_str(), &part.body);
        let frame_start = text.len();
        text.push_str("<!-- ");
        text.push_str(SEGMENT_FRAME);
        text.push(' ');
        text.push_str(part.id.as_str());
        text.push_str(" -->\n");
        generated.push(frame_start..text.len());
        let offset = text.len();
        text.push_str(&part.body);
        if let Some(range) = &part.generated {
            generated.push(offset + range.start..offset + range.end);
        }
        segments.push(Segment {
            id: part.id,
            offset,
            len: part.body.len(),
            cache_boundary: false,
        });
        if !part.body.is_empty() && !part.body.ends_with('\n') {
            text.push('\n');
        }
    }
    mark_cache_boundaries(&mut segments);
    verify_no_recursion(&text, &generated)?;
    let bytes = text.into_bytes();
    Ok(ComposedPrefix {
        sha256: sha256_hex(&bytes),
        bytes,
        segments,
        inertness,
    })
}

/// The end of the prefix is always a recommended breakpoint; so is the
/// position just before the lane, which is the one that makes a lane change
/// cost only the lane.
fn mark_cache_boundaries(segments: &mut [Segment]) {
    if let Some(index) = segments
        .iter()
        .position(|seg| seg.id == SegmentId::Lane)
        .filter(|index| *index > 0)
    {
        segments[index - 1].cache_boundary = true;
    }
    if let Some(last) = segments.last_mut() {
        last.cache_boundary = true;
    }
}

/// The prefix is UTF-8; a body that is not is a refusal naming its role,
/// never a lossy conversion.
fn text_of(input: &CapturedInput) -> Result<String, WorkspaceError> {
    String::from_utf8(input.bytes.clone()).map_err(|error| {
        compose_error(
            "segment body",
            format!(
                "the {} input `{}` is not UTF-8: {error}",
                input.role.as_str(),
                input.relative
            ),
        )
    })
}

#[cfg(test)]
#[path = "compose/tests.rs"]
mod tests;
