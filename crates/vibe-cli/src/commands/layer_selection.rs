//! Shared optional-layer relation observation for transports and views.
use specmark::spec;
use std::path::Path;
use vibe_core::manifest::LayerName;
use vibe_workspace::layer_relations::{LayerInput, Resolution};

pub(crate) fn relations(project: &Path) -> Resolution {
    relations_for_delivery(project, None)
}
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#LAYER-EFFECTIVE-RELATIONS")]
pub(crate) fn relations_for_delivery(
    project: &Path,
    delivered: Option<(bool, bool)>,
) -> Resolution {
    let mut diagnostics = Vec::new();
    let personal = match vibe_core::user_project::admit_root(vibe_core::settings::settings_dir()) {
        Ok(root) => Some(root),
        Err(error) => {
            diagnostics.push((
                LayerName::User,
                format!("personal layer unavailable: {error}"),
            ));
            None
        }
    };
    let user_enabled = match super::user_override::user_enabled(project) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push((
                LayerName::User,
                format!("personal control unavailable: {error}"),
            ));
            false
        }
    };
    let override_layer = match super::user_override::observed_root(project) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push((
                LayerName::Override,
                format!("override binding unavailable: {error}"),
            ));
            None
        }
    };
    // Status is bounded/no-follow; disabled layers never trigger world freshness
    // or source reads merely to decide loading. Safe declarations may be listed.
    let available = |root: Option<&Path>, enabled: bool| {
        root.is_some_and(|root| {
            if !matches!(
                vibe_workspace::user_project::status(root),
                vibe_workspace::user_project::Observation::Initialized { .. }
            ) {
                return false;
            }
            if !enabled {
                return true;
            }
            vibe_workspace::Workspace::load(root)
                .ok()
                .and_then(|workspace| {
                    let format =
                        super::reinstall::effective_rederive_format(&workspace.root_manifest)
                            .ok()?;
                    vibe_workspace::install::verify_boot_graph_with_spec_format(&workspace, format)
                        .ok()
                })
                .is_some_and(|stale| stale.is_empty())
        })
    };
    let override_root = override_layer.as_ref().map(|(root, _)| root.as_path());
    let override_enabled = override_layer.as_ref().is_some_and(|(_, enabled)| *enabled);
    let mut result = vibe_workspace::layer_relations::resolve(&[
        LayerInput {
            layer: LayerName::Project,
            root: Some(project),
            enabled: true,
            available: true,
        },
        LayerInput {
            layer: LayerName::User,
            root: personal.as_deref(),
            enabled: user_enabled,
            available: delivered.map_or_else(
                || available(personal.as_deref(), user_enabled),
                |(user, _)| user,
            ),
        },
        LayerInput {
            layer: LayerName::Override,
            root: override_root,
            enabled: override_enabled,
            available: delivered.map_or_else(
                || available(override_root, override_enabled),
                |(_, over)| over,
            ),
        },
    ]);
    result.diagnostics.extend(diagnostics);
    result
}
