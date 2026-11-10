//! Rebuilding one node's witnessed input set from the bytes on disk now.
//!
//! The same three roles, in the same order, through the same projection the
//! write path uses — because a second derivation of the prospective
//! entrypoint is how a published block and its own witness come to disagree
//! (`#SYS-PROSPECTIVE-ENTRYPOINT`). Nothing here compiles a lane: the lane
//! segment's bytes are the **committed** `STATIC.*` the caller read
//! (`#SYS-READ-ONLY-CHECK`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-READ-ONLY-CHECK");

use vibe_core::layout;
use vibe_core::manifest::{SpecFormat, WhenCondition};
use vibe_wire::generated::system_prompt_meta as wire;

use crate::boot_artifacts::{self, RouterFacts};
use crate::{WorkspaceError, path_to_slash};

use super::super::compose::PrefixSettings;
use super::super::markers::segment_hash12;
use super::super::{
    CapturedInput, CapturedLane, CapturedPrefixInputs, ContinuationBand, ContinuationPlan,
    ContinuationRow, InputRole, LaneSpelling, compose_error,
};
use super::{CheckRequest, read_owned};

/// Rebuild the witnessed input set from what is on disk now — the same three
/// roles, in the same order, through the same projection the write path uses.
pub(super) fn recapture(
    request: &CheckRequest<'_>,
    meta: &wire::SystemPromptMeta,
    template: &[u8],
    lane_bytes: Option<&[u8]>,
    settings: &PrefixSettings,
) -> Result<CapturedPrefixInputs, WorkspaceError> {
    let spec_format = match meta.lane {
        wire::LaneSpelling::StaticXml => SpecFormat::Xml,
        _ => SpecFormat::Mixed,
    };
    let router = RouterFacts {
        node_id: settings.node_id.clone(),
        static_hash12: lane_bytes.map(segment_hash12),
        spec_format,
    };
    let entrypoint = match request
        .manifest
        .system_prompt
        .as_ref()
        .map(|section| section.entrypoint)
        .unwrap_or_default()
        .file_name()
    {
        Some(name) => {
            let path = request.node_dir.join(name);
            let bytes = read_owned(&path)?.ok_or_else(|| {
                compose_error(
                    "entrypoint",
                    format!("the selected system-prompt entrypoint `{name}` does not exist"),
                )
            })?;
            let text = String::from_utf8(bytes).map_err(|error| {
                compose_error("entrypoint", format!("`{name}` is not UTF-8: {error}"))
            })?;
            let projected = boot_artifacts::project_managed_block(Some(&text), &router)
                .map_err(|reason| WorkspaceError::MalformedRedirectBlock { path, reason })?;
            Some(CapturedInput::new(
                InputRole::Entrypoint,
                name.to_owned(),
                projected.into_bytes(),
            ))
        }
        None => None,
    };
    let lane = lane_bytes.map(|bytes| {
        let spelling = LaneSpelling::of(spec_format);
        CapturedLane {
            spelling,
            input: CapturedInput::new(
                InputRole::Lane,
                path_to_slash(&layout::current_boot_dir().join(spelling.file_name())),
                bytes.to_vec(),
            ),
        }
    });
    Ok(CapturedPrefixInputs {
        node_rel: request.node_rel.to_owned(),
        template: CapturedInput::new(
            InputRole::Template,
            path_to_slash(&layout::current_boot_system_template()),
            template.to_vec(),
        ),
        entrypoint,
        lane,
        plan: sidecar_plan(meta)?,
    })
}

/// The plan the sidecar recorded, back as a [`ContinuationPlan`].
fn sidecar_plan(meta: &wire::SystemPromptMeta) -> Result<ContinuationPlan, WorkspaceError> {
    Ok(ContinuationPlan {
        rows: meta
            .continuation
            .iter()
            .map(|row| {
                let band = match (&row.band, &row.when) {
                    (wire::Band::Unconditional, _) => ContinuationBand::Unconditional,
                    (wire::Band::Gated, Some(when)) => {
                        ContinuationBand::Gated(when.parse::<WhenCondition>().map_err(|error| {
                            compose_error(
                                "sidecar",
                                format!("the gated row `{}` carries `{when}`: {error}", row.path),
                            )
                        })?)
                    }
                    (wire::Band::Gated, None) => {
                        return Err(compose_error(
                            "sidecar",
                            format!("the gated row `{}` carries no condition", row.path),
                        ));
                    }
                };
                Ok(ContinuationRow {
                    path: row.path.clone(),
                    band,
                })
            })
            .collect::<Result<Vec<_>, WorkspaceError>>()?,
    })
}

/// Each input witness against the bytes observed now, by role.
pub(super) fn input_changes(
    meta: &wire::SystemPromptMeta,
    captured: &CapturedPrefixInputs,
) -> Vec<String> {
    let mut reasons = Vec::new();
    for input in captured.inputs() {
        let role = wire_role(input.role);
        match meta.inputs.iter().find(|witness| witness.role == role) {
            Some(witness) if witness.sha256 != input.sha256 => reasons.push(format!(
                "the {} input `{}` changed since the prefix was generated",
                input.role.as_str(),
                input.relative
            )),
            None => reasons.push(format!(
                "the {} input `{}` is present now and the sidecar witnesses none",
                input.role.as_str(),
                input.relative
            )),
            Some(_) => {}
        }
    }
    for witness in &meta.inputs {
        if !captured
            .inputs()
            .any(|input| wire_role(input.role) == witness.role)
        {
            reasons.push(format!(
                "the sidecar witnesses a `{}` input that no longer contributes",
                witness.path
            ));
        }
    }
    reasons
}

fn wire_role(role: InputRole) -> wire::Role {
    match role {
        InputRole::Template => wire::Role::Template,
        InputRole::Entrypoint => wire::Role::Entrypoint,
        InputRole::Lane => wire::Role::Lane,
    }
}
