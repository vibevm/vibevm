//! Capture: from a node and its computed boot to a fully witnessed input
//! set, writing nothing and composing nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-CAPTURE-REUSE");

use std::path::Path;

use vibe_core::layout;
use vibe_core::manifest::{Manifest, SpecFormat};

use crate::boot::EffectiveBoot;
use crate::boot_artifacts;
use crate::{WorkspaceError, path_to_slash};

use super::{
    CapturedInput, CapturedLane, CapturedPrefixInputs, ContinuationPlan, InputRole, LaneSpelling,
    PrefixReader,
};

/// One node's capture request: what this generation is publishing, and for
/// which node.
///
/// The lane arrives as bytes rather than being recompiled here, and that is
/// deliberate. The lane has two publishing seams — the builtin one and the
/// owner-managed one — chosen once per regeneration, and a read-only caller
/// gets the same bytes from the in-memory analyser instead. Capture must not
/// become a third producer of a lane, nor assume which seam ran
/// (PROP-076 `#SYS-LANE-VERBATIM`).
pub struct PrefixCapture<'a> {
    /// The node's canonical workspace-relative path (`.` for the root).
    pub node_rel: &'a str,
    /// The node's absolute directory.
    pub node_dir: &'a Path,
    /// The node's own manifest — the authority for `[system_prompt]`.
    pub manifest: &'a Manifest,
    /// The target format this generation publishes the lane under.
    pub spec_format: SpecFormat,
    /// The node's computed effective boot, from which the continuation plan
    /// is derived.
    pub effective: &'a EffectiveBoot,
    /// The lane bytes this generation publishes, or `None` for a node with
    /// no static contributions.
    pub lane: Option<&'a [u8]>,
    /// The router facts this generation's managed-block write uses.
    ///
    /// They arrive from the caller rather than being derived here, because
    /// the entrypoint witness must be the bytes the write will leave: a
    /// second derivation of the node id or of the expected lane digest is
    /// how a published block and its own witness come to disagree
    /// (`#SYS-PROSPECTIVE-ENTRYPOINT`).
    pub router: boot_artifacts::RouterFacts,
}

/// Capture one node's prefix inputs under a freshly held workspace.
///
/// This is the read-only entry point: it holds the workspace, prepares the
/// same node carriers admission and generation prepare, and reads the two
/// authored inputs through them. It writes nothing, compiles nothing and
/// invokes no provider — the lane's bytes and the node's effective boot are
/// the caller's, because capture must not become a second producer of
/// either (`#SYS-LANE-VERBATIM`, `#SYS-READ-ONLY-CHECK`).
///
/// A publication path that already holds prepared sources calls
/// [`capture_prefix_inputs`] with its own reader instead, so one generation
/// never holds the workspace twice.
pub fn capture_node_prefix_inputs(
    workspace: &crate::Workspace,
    node_rel: &str,
    spec_format: SpecFormat,
    effective: &EffectiveBoot,
    lane: Option<&[u8]>,
) -> Result<Option<CapturedPrefixInputs>, WorkspaceError> {
    let node_dir = workspace.node_abs_path(node_rel);
    let manifest = workspace
        .member_by_rel_path(node_rel)
        .map(|member| &member.manifest)
        .unwrap_or(&workspace.root_manifest);
    let resolution = crate::install::read_durable_resolution(&workspace.root)?;
    let sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        workspace,
        &resolution,
        Some(spec_format),
    )?
    .ok_or_else(|| refusal(workspace.root.clone(), "prepared nodes missing".to_string()))?;
    let router = boot_artifacts::RouterFacts {
        node_id: boot_artifacts::router_node_id(
            Some(manifest),
            &crate::install::bootgen::root_self_coordinate(&workspace.root_manifest),
        ),
        static_hash12: lane.map(super::segment_hash12),
        spec_format,
    };
    capture_prefix_inputs(
        PrefixCapture {
            node_rel,
            node_dir: &node_dir,
            manifest,
            spec_format,
            effective,
            lane,
            router,
        },
        &super::bound_reader(&sources, node_rel),
    )
}

/// Capture one node's prefix inputs.
///
/// `Ok(None)` is the ordinary answer for a node that has not opted in: no
/// `SYSTEM-TEMPLATE.md`, no prefix, and — the property that makes the
/// feature free for every project that does not want it — no change to the
/// node's boot whatever (`#SYS-OPT-IN`).
///
/// Three refusals, each naming its path. A declared entrypoint that does not
/// exist refuses rather than contributing a silent empty segment
/// (`#SYS-ENTRYPOINT-ONE`). A malformed `<vibevm>` marker pair refuses
/// through the redirect owner's own projection, which never repairs a
/// co-tenant document (`#SYS-REDIRECT-IN-PLACE`). Both `STATIC` spellings
/// present propagates `resolve_static_path`'s refusal unchanged rather than
/// picking a side (`#SYS-LANE-SPELLING`).
pub fn capture_prefix_inputs(
    request: PrefixCapture<'_>,
    reader: &dyn PrefixReader,
) -> Result<Option<CapturedPrefixInputs>, WorkspaceError> {
    let template_rel = path_to_slash(&layout::current_boot_system_template());
    let Some(template) = reader.read(&template_rel)? else {
        return Ok(None);
    };

    let entrypoint = capture_entrypoint(&request, reader)?;
    let lane = capture_lane(&request)?;
    let plan = ContinuationPlan::of(request.effective);
    refuse_reserved_continuation_rows(&request, &plan)?;

    Ok(Some(CapturedPrefixInputs {
        node_rel: request.node_rel.to_owned(),
        template: CapturedInput::new(InputRole::Template, template_rel, template),
        entrypoint,
        lane,
        plan,
    }))
}

/// The selected entrypoint's **prospective** export: its bytes as this
/// generation will leave them, with the managed block already replaced by
/// the redirect owner's projection.
///
/// Witnessing the file as it stands would make a newly published prefix
/// stale the instant it was written, because the redirect selector runs
/// after the INDEX and STATIC roll-forwards commit
/// (`#SYS-PROSPECTIVE-ENTRYPOINT`).
fn capture_entrypoint(
    request: &PrefixCapture<'_>,
    reader: &dyn PrefixReader,
) -> Result<Option<CapturedInput>, WorkspaceError> {
    let Some(name) = request
        .manifest
        .system_prompt
        .as_ref()
        .map(|section| section.entrypoint)
        .unwrap_or_default()
        .file_name()
    else {
        // `entrypoint = false` — the node asked for no entrypoint segment.
        return Ok(None);
    };
    let Some(bytes) = reader.read(name)? else {
        return Err(refusal(
            request.node_dir.join(name),
            format!(
                "the selected system-prompt entrypoint `{name}` does not exist \
                 (declare `[system_prompt] entrypoint = false` for no entrypoint segment)"
            ),
        ));
    };
    let existing = String::from_utf8(bytes).map_err(|error| {
        refusal(
            request.node_dir.join(name),
            format!("the selected system-prompt entrypoint is not UTF-8: {error}"),
        )
    })?;
    let projected = boot_artifacts::project_managed_block(Some(&existing), &request.router)
        .map_err(|reason| WorkspaceError::MalformedRedirectBlock {
            path: request.node_dir.join(name),
            reason,
        })?;
    Ok(Some(CapturedInput::new(
        InputRole::Entrypoint,
        name.to_owned(),
        projected.into_bytes(),
    )))
}

/// The compiled lane as one admitted, opaque segment.
///
/// `resolve_static_path` is consulted even though the bytes come from the
/// caller: it is the one owner of the spelling question, and its
/// both-present refusal ("the generator owns one; delete the stray") must
/// reach a capture caller unchanged rather than being decided here
/// (`#SYS-LANE-SPELLING`).
fn capture_lane(request: &PrefixCapture<'_>) -> Result<Option<CapturedLane>, WorkspaceError> {
    boot_artifacts::resolve_static_path(request.node_dir)?;
    let Some(bytes) = request.lane else {
        return Ok(None);
    };
    let spelling = LaneSpelling::of(request.spec_format);
    let relative = path_to_slash(&layout::current_boot_dir().join(spelling.file_name()));
    Ok(Some(CapturedLane {
        spelling,
        input: CapturedInput::new(InputRole::Lane, relative, bytes.to_vec()),
    }))
}

/// The generator-side half of `#SYS-SELF-INCLUSION`: an output listed among
/// its own inputs is a refusal, named, before anything is written.
///
/// The three reserved paths are already excluded from boot enrolment and
/// from declaration admission, so a row naming one of them means a route
/// those gates do not cover. The test is a **file identity**, not a string:
/// a continuation row is checked only when it resolves inside *this* node's
/// own boot directory, and ASCII case aliases are refused because the
/// layout owner recognises them. A row naming a dependency slot's own
/// `STATIC` is untouched — that is another node's artifact and a legitimate
/// unit lane.
fn refuse_reserved_continuation_rows(
    request: &PrefixCapture<'_>,
    plan: &ContinuationPlan,
) -> Result<(), WorkspaceError> {
    let boot_dir = path_to_slash(&layout::current_boot_dir());
    for row in &plan.rows {
        let node_relative = match request.node_rel {
            "." => row.path.as_str(),
            prefix => match row.path.strip_prefix(&format!("{prefix}/")) {
                Some(tail) => tail,
                None => continue,
            },
        };
        if !node_relative.starts_with(&format!("{boot_dir}/")) {
            continue;
        }
        if layout::is_reserved_boot_source(Path::new(node_relative)) {
            return Err(refusal(
                request.node_dir.join(node_relative),
                format!(
                    "boot input `{}` names a reserved system-prompt path — a generated \
                     prefix output or the authored template is never one of the prefix's \
                     own inputs",
                    row.path
                ),
            ));
        }
    }
    Ok(())
}

fn refusal(path: std::path::PathBuf, reason: String) -> WorkspaceError {
    WorkspaceError::SystemPromptCapture { path, reason }
}
