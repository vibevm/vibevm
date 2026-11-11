//! The cache-stable system-prompt prefix — capture, for now.
//!
//! The design is `spec://org.vibevm.core/vibevm/common/PROP-076#root`. One
//! sentence carries it: `vibe` generates one byte-stable prefix — the user's
//! template, the selected entrypoint with its managed boot block replaced in
//! place, and the already-compiled static lane — whose bytes are a pure
//! function of its inputs, so a provider that caches on an exact prefix
//! match can cache it.
//!
//! This module holds the first half of that: the **witnessed inputs**. It
//! reads nothing on its own account and writes nothing at all. What it
//! produces is a [`CapturedPrefixInputs`] — the three input bodies with
//! their digests, plus the [`ContinuationPlan`] that fixes the identity and
//! order of the live boot files a runner reads per request.
//!
//! Three properties of this phase are worth stating because later phases
//! depend on them and a convenience could quietly break any of them.
//!
//! **The entrypoint witness is prospective.** The same publication can
//! rewrite the entrypoint's managed block, and the redirect selector runs
//! *after* the INDEX and STATIC roll-forwards commit. So the witness is what
//! this generation will leave in the file, computed through the redirect
//! owner's own projection — never the bytes as they stand
//! (`#SYS-PROSPECTIVE-ENTRYPOINT`).
//!
//! **No INDEX entry's bytes are captured.** `kind = "static"` in a generated
//! INDEX row is read timing, not linkage: the continuation is a *plan* a
//! runner executes against the current files, so its bytes stay late-bound
//! and its digests are request-local observations rather than committed ones
//! (`#SYS-NO-FROZEN-INDEX`, `#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
//!
//! **Nothing user-local is pulled in by reference.** An authored instruction
//! that *mentions* a home directory, a stewardship plan, a custody record or
//! a token path is an instruction to a tool-using agent, not permission to
//! embed that file. Capture opens exactly the inputs its roles name and
//! follows no prose link, `spec://` reference or `#use` marker
//! (`#SYS-NO-HOME-EXPANSION`, `#SYS-CAPTURE-REUSE`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CAPTURE-REUSE");

use vibe_core::manifest::SpecFormat;

use crate::WorkspaceError;

#[path = "system_prompt/capture.rs"]
mod capture;
#[path = "system_prompt/check.rs"]
mod check;
#[path = "system_prompt/compose.rs"]
mod compose;
#[path = "system_prompt/constraints.rs"]
mod constraints;
#[path = "system_prompt/contract.rs"]
mod contract;
#[cfg(test)]
#[path = "system_prompt/docs_example.rs"]
mod docs_example;
#[path = "system_prompt/harness.rs"]
mod harness;
#[cfg(test)]
#[path = "system_prompt/host_repo.rs"]
mod host_repo;
#[path = "system_prompt/lane.rs"]
mod lane;
#[path = "system_prompt/markers.rs"]
mod markers;
#[path = "system_prompt/meta.rs"]
mod meta;
#[path = "system_prompt/plan.rs"]
mod plan;
#[path = "system_prompt/prepare.rs"]
mod prepare;
#[path = "system_prompt/publish.rs"]
mod publish;
#[path = "system_prompt/reader.rs"]
mod reader;
#[path = "system_prompt/verify.rs"]
mod verify;

pub use capture::{PrefixCapture, capture_node_prefix_inputs, capture_prefix_inputs};
pub use check::{
    ArtifactFacts, CheckRequest, CheckVerdict, NotTracked, PromptCheck, TrackedPaths, build_remedy,
    check_node_prompt, host_mismatch_warning, prompt_warnings,
};
pub use compose::{
    ComposedPrefix, PrefixSettings, SEGMENT_FRAME, Segment, SegmentId, compose_prefix,
};
pub use constraints::{BlockShape, delivery_constraints, manifest_block_shape, one_block_shape};
pub use contract::{CURRENT_RECIPE, KNOWN_RECIPES, SuppliedSource, render_contract};
mod supplied_base;
pub use supplied_base::{
    Base, BaseError, Carrier, CarrierDocument, Chosen, Fingerprint, LANE_PLACEHOLDER, Launch,
    License, Mode, PROMPT_BASES_DIR, PROVENANCE_FILE, PROVENANCE_SCHEMA, Provenance, Render,
    Select, Upstream, Verification,
};

pub use harness::{
    CODEX_VERIFIED_VERSION, CarrierVerdict, CodexProfile, EnvAssignment, HarnessKind, LaunchFlags,
    LaunchInputs, OPENCODE_MEASURED_VERSION, QWEN_MEASURED_VERSION, codex_carrier_verdict,
    launch_flags, opencode_carrier_description, qwen_launch_blocker, reported_version,
};
pub use lane::{ComposedLane, LaneSegment, LaneSource, compose_agent_lane};
pub use markers::{
    HASH12_LEN, LANE_BEGIN, LANE_END, LaneMarker, MarkerKind, VerifiedSegment, hash12, marker_line,
    parse_marker_line, segment_hash12, sha256_hex, verify_lane_markers,
};
pub use meta::{SIDECAR_PROTOCOL, SidecarFacts, compose_sidecar, render_sidecar_bytes};
pub use plan::{ContinuationBand, ContinuationPlan, ContinuationRow};
pub use prepare::{PreparedBand, PreparedRequest, UnresolvedRow, prepare_request};
pub use publish::{BOOT_IGNORE_BODY, LaneBodyReader};
pub(crate) use publish::{
    PromptPublication, PromptPublishRequest, bound_lane_reader, prepare_publication,
};
pub use reader::PrefixReader;
pub(crate) use reader::bound_reader;
pub use verify::{
    ImportInertness, ImportSite, refuse_managed_block_markers, refuse_marker_shaped_lines,
    verify_no_recursion,
};

/// Every refusal composition can raise names what it was composing and why.
/// Composition writes nothing, so a refusal leaves every published artifact
/// exactly as it was.
pub(crate) fn compose_error(what: &str, reason: String) -> WorkspaceError {
    WorkspaceError::SystemPromptCompose {
        what: what.to_owned(),
        reason,
    }
}

/// Which role an input plays in the prefix. The three are kept apart because
/// only one of them is authored by the user, and because the sidecar and the
/// segment map address them by name (`#SYS-FILES`).
///
/// ```
/// use vibe_workspace::system_prompt::InputRole;
///
/// assert_eq!(InputRole::Template.as_str(), "template");
/// // Segment ids are layer-qualified from version 1, even though no other
/// // layer exists yet (PROP-076 `#SYS-LAYER-ROLE`).
/// assert_eq!(InputRole::Lane.segment_id(), "project.lane");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputRole {
    /// The authored `SYSTEM-TEMPLATE.md`, whose presence is the whole opt-in.
    Template,
    /// The selected instruction file, with its managed block replaced by
    /// this generation's projection.
    Entrypoint,
    /// The compiled static lane, admitted as opaque bytes.
    Lane,
}

impl InputRole {
    /// The role's own name, as a witness spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            InputRole::Template => "template",
            InputRole::Entrypoint => "entrypoint",
            InputRole::Lane => "lane",
        }
    }

    /// The layer-qualified segment id this role's body carries in the
    /// composed prefix. Qualified from version 1 so two layers' sidecars
    /// merge without collision (`#SYS-LAYER-CONCATENATION`).
    pub fn segment_id(self) -> &'static str {
        match self {
            InputRole::Template => "project.template",
            InputRole::Entrypoint => "project.entrypoint",
            InputRole::Lane => "project.lane",
        }
    }
}

/// One observed input: its role, where it was observed, its bytes, and the
/// SHA-256 of exactly those bytes.
///
/// The digest is of the bytes **as observed**, so no second read can slip
/// different content between the witness and its use. It proves consistency
/// between an output and a set of observations and nothing else — never
/// authenticity (`#SYS-TWO-IDENTITIES`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedInput {
    /// Which of the three roles this body plays.
    pub role: InputRole,
    /// The input's path relative to the selected node, forward-slashed.
    pub relative: String,
    /// The observed body. For the entrypoint this is the **prospective**
    /// content of this generation, not the file as it stands.
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`, 64 lowercase hexadecimal digits.
    pub sha256: String,
}

impl CapturedInput {
    pub(crate) fn new(role: InputRole, relative: String, bytes: Vec<u8>) -> Self {
        let sha256 = crate::vibedeps::sha256_bytes(&bytes);
        Self {
            role,
            relative,
            bytes,
            sha256,
        }
    }
}

/// Which spelling of the compiled lane the node actually carries. Absence is
/// a third answer and is carried by `lane: None` rather than by a variant,
/// because a node with no static contributions is a normal case and not an
/// empty lane (`#SYS-LANE-SPELLING`).
///
/// ```
/// use vibe_core::manifest::SpecFormat;
/// use vibe_workspace::system_prompt::LaneSpelling;
///
/// assert_eq!(LaneSpelling::of(SpecFormat::Xml), LaneSpelling::StaticXml);
/// assert_eq!(LaneSpelling::StaticXml.as_str(), "static-xml");
/// assert_eq!(LaneSpelling::StaticMd.file_name(), "STATIC.md");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneSpelling {
    /// `STATIC.md`.
    StaticMd,
    /// `STATIC.xml`.
    StaticXml,
}

impl LaneSpelling {
    /// The spelling the selected target format publishes.
    pub fn of(spec_format: SpecFormat) -> Self {
        match spec_format {
            SpecFormat::Xml => LaneSpelling::StaticXml,
            _ => LaneSpelling::StaticMd,
        }
    }

    /// The sidecar's own name for this spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            LaneSpelling::StaticMd => "static-md",
            LaneSpelling::StaticXml => "static-xml",
        }
    }

    /// The artifact's basename.
    pub fn file_name(self) -> &'static str {
        match self {
            LaneSpelling::StaticMd => vibe_core::layout::STATIC_MD,
            LaneSpelling::StaticXml => vibe_core::layout::STATIC_XML,
        }
    }
}

/// The compiled lane as one admitted segment, beside the spelling it was
/// published under.
///
/// The bytes are opaque here and stay opaque: the lane is not re-parsed, not
/// re-compiled as a fresh normal source, and no `#use` marker, provenance
/// comment or `spec://` reference inside it is followed
/// (`#SYS-LANE-VERBATIM`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedLane {
    /// Which `STATIC.*` this node publishes.
    pub spelling: LaneSpelling,
    /// The lane's admitted bytes, witnessed like any other input.
    pub input: CapturedInput,
}

/// Everything one node's prefix is built from, witnessed, with nothing
/// written and nothing composed yet.
///
/// A `None` from capture means the node has not opted in — there is no
/// authored template — and in that case nothing about the node's boot
/// changes at all (`#SYS-OPT-IN`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedPrefixInputs {
    /// The selected node's canonical workspace-relative path (`.` for the
    /// absolute root).
    pub node_rel: String,
    /// The authored template. Always present: it is the opt-in.
    pub template: CapturedInput,
    /// The selected entrypoint's prospective export, or `None` when the node
    /// declared `entrypoint = false`.
    pub entrypoint: Option<CapturedInput>,
    /// The compiled static lane, or `None` for a node with no static
    /// contributions.
    pub lane: Option<CapturedLane>,
    /// The live continuation a runner executes after the prefix.
    pub plan: ContinuationPlan,
}

impl CapturedPrefixInputs {
    /// The captured inputs in prefix order — template, entrypoint, lane —
    /// which is the order `#SYS-BAND-ORDER` fixes and the order a composer
    /// and a sidecar both walk.
    pub fn inputs(&self) -> impl Iterator<Item = &CapturedInput> {
        std::iter::once(&self.template)
            .chain(self.entrypoint.as_ref())
            .chain(self.lane.as_ref().map(|lane| &lane.input))
    }
}

#[cfg(test)]
#[path = "system_prompt/tests.rs"]
mod tests;
