//! The sidecar — `vibe-system-prompt/1`, the witness beside the prefix.
//!
//! Two different facts live in it and are never conflated.
//! `prefix_sha256` is the digest of the bytes actually composed: it proves
//! byte identity and nothing else. The **input witness** is the ordered list
//! of observed inputs together with the selected node, the entrypoint, the
//! recipe, the lane spelling and the plan's identity: it proves
//! *consistency* between an output and a set of observations. Neither proves
//! authenticity (`#SYS-TWO-IDENTITIES`).
//!
//! **Four things it must not hold**, each for a stated reason. A build time
//! or any timestamp; an absolute checkout, `HOME` or cache path; and the
//! generating host's operating system — all three because the sidecar is a
//! pure function of its inputs, which is what makes a second `vibe install`
//! with nothing changed produce byte-identical files and no diff. And **no
//! content digest for any continuation row**: a hash of `90-user.xml` here
//! would demand a rebuild on every edit of a user boot snippet, destroying
//! the one property dynamic linking exists for. Continuation bytes are
//! witnessed per request, in `vibe prompt prepare`'s response
//! (`#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
//!
//! The `host` member is the *declared* one, read from the node's own
//! manifest like `entrypoint` or `recipe` — never an observation of the
//! machine that composed the bytes (`#SYS-HOST-AND-TARGET`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SIDECAR");

use vibe_core::manifest::{HostChoice, LaneForm, TargetOs};
use vibe_wire::generated::system_prompt_meta as wire;

use crate::WorkspaceError;

use super::compose::{ComposedPrefix, PrefixSettings};
use super::markers::sha256_hex;
use super::{
    CapturedPrefixInputs, ContinuationBand, ContinuationPlan, InputRole, LaneSpelling,
    compose_error,
};

/// The protocol tag version 1 of the sidecar carries.
pub const SIDECAR_PROTOCOL: &str = "vibe-system-prompt/1";

/// What a sidecar needs beyond the inputs and the composed bytes.
pub struct SidecarFacts<'a> {
    /// The continuation plan **after** the declared host resolved its `os:`
    /// rows — so a fixed-host sidecar records an empty gated band, which is
    /// the fact `prepare` acts on.
    pub plan: &'a ContinuationPlan,
    /// The generated `INDEX.md` bytes this generation publishes, whose
    /// digest fixes the plan's identity and order. `None` when this
    /// generation published no INDEX.
    pub index_bytes: Option<&'a [u8]>,
}

/// Build the sidecar document.
///
/// Pure, like the composer: every member is a function of the witnessed
/// inputs, the declared settings and the composed bytes.
pub fn compose_sidecar(
    inputs: &CapturedPrefixInputs,
    settings: &PrefixSettings,
    prefix: &ComposedPrefix,
    facts: SidecarFacts<'_>,
) -> Result<wire::SystemPromptMeta, WorkspaceError> {
    Ok(wire::SystemPromptMeta {
        protocol: SIDECAR_PROTOCOL.to_owned(),
        layer: wire::Layer::Project,
        node: inputs.node_rel.clone(),
        node_id: settings.node_id.clone(),
        recipe: settings.recipe,
        entrypoint: inputs
            .entrypoint
            .as_ref()
            .map(|input| input.relative.clone()),
        lane: match &inputs.lane {
            Some(lane) => lane_spelling(lane.spelling),
            None => wire::LaneSpelling::None,
        },
        lane_form: match settings.lane_form {
            LaneForm::Verbatim => wire::LaneForm::Verbatim,
        },
        host: host(settings.host),
        target: settings.target.platforms().into_iter().map(os).collect(),
        inputs: inputs
            .inputs()
            .map(|input| {
                Ok(wire::Input {
                    role: role(input.role),
                    path: input.relative.clone(),
                    sha256: input.sha256.clone(),
                    len: fits("an input's length", input.bytes.len())?,
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?,
        prefix_sha256: prefix.sha256.clone(),
        prefix_len: fits("the prefix length", prefix.bytes.len())?,
        segments: prefix
            .segments
            .iter()
            .map(|segment| {
                Ok(wire::Segment {
                    id: segment.id.as_str().to_owned(),
                    offset: fits("a segment offset", segment.offset)?,
                    len: fits("a segment length", segment.len)?,
                    cache_boundary: segment.cache_boundary,
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?,
        index_sha256: facts.index_bytes.map(sha256_hex),
        continuation: facts
            .plan
            .rows
            .iter()
            .map(|row| wire::ContinuationRow {
                path: row.path.clone(),
                band: match row.band {
                    ContinuationBand::Unconditional => wire::Band::Unconditional,
                    ContinuationBand::Gated(_) => wire::Band::Gated,
                },
                when: row.band.when().map(ToString::to_string),
            })
            .collect(),
    })
}

/// Render the sidecar's bytes: pretty JSON with one trailing newline, the
/// convention every generated sidecar in the tree already follows.
pub fn render_sidecar_bytes(meta: &wire::SystemPromptMeta) -> Result<Vec<u8>, WorkspaceError> {
    let mut bytes = serde_json::to_vec_pretty(meta).map_err(|error| {
        compose_error(
            "sidecar",
            format!("the sidecar does not serialise: {error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// A length or offset the wire format carries as `uint32`. A prefix past
/// four gigabytes is not a thing, and a silent truncation would make a
/// witness lie, so the impossible case is a named refusal.
fn fits(what: &str, value: usize) -> Result<u32, WorkspaceError> {
    u32::try_from(value).map_err(|_| {
        compose_error(
            "sidecar",
            format!("{what} is {value} bytes, past what the sidecar format can carry"),
        )
    })
}

fn role(role: InputRole) -> wire::Role {
    match role {
        InputRole::Template => wire::Role::Template,
        InputRole::Entrypoint => wire::Role::Entrypoint,
        InputRole::Lane => wire::Role::Lane,
    }
}

fn lane_spelling(spelling: LaneSpelling) -> wire::LaneSpelling {
    match spelling {
        LaneSpelling::StaticMd => wire::LaneSpelling::StaticMd,
        LaneSpelling::StaticXml => wire::LaneSpelling::StaticXml,
    }
}

fn host(host: HostChoice) -> wire::Host {
    match host {
        HostChoice::Portable => wire::Host::Portable,
        HostChoice::Fixed(TargetOs::Linux) => wire::Host::Linux,
        HostChoice::Fixed(TargetOs::Macos) => wire::Host::Macos,
        HostChoice::Fixed(TargetOs::Windows) => wire::Host::Windows,
    }
}

fn os(os: TargetOs) -> wire::Os {
    match os {
        TargetOs::Linux => wire::Os::Linux,
        TargetOs::Macos => wire::Os::Macos,
        TargetOs::Windows => wire::Os::Windows,
    }
}

#[cfg(test)]
#[path = "meta/tests.rs"]
mod tests;
