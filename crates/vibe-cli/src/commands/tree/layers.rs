//! Opt-in layered view; the legacy project tree document remains unchanged.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#LAYERS-IN-TREE");
use super::{build, model::PackageTree, plain};
use crate::output;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub(super) struct Layer {
    pub(super) id: &'static str,
    pub(super) enabled: bool,
    pub(super) state: &'static str,
    pub(super) root: Option<String>,
    pub(super) tree: Option<PackageTree>,
    pub(super) diagnostics: Vec<String>,
    pub(super) relations: Vec<vibe_workspace::layer_relations::Relation>,
}
#[derive(Serialize)]
struct LayeredTree<'a> {
    ok: bool,
    command: &'static str,
    schema_version: u32,
    project: &'a PackageTree,
    layers: Vec<Layer>,
}
fn observe(id: &'static str, root: Option<PathBuf>, enabled: bool) -> Layer {
    let mut layer = Layer {
        id,
        enabled,
        state: "absent",
        root: root.as_deref().map(vibe_core::machine_json_path),
        tree: None,
        diagnostics: Vec::new(),
        relations: Vec::new(),
    };
    let Some(root) = root else {
        return layer;
    };
    match vibe_workspace::user_project::status(&root) {
        vibe_workspace::user_project::Observation::Absent
        | vibe_workspace::user_project::Observation::Empty => {}
        vibe_workspace::user_project::Observation::Unavailable(_) => {
            layer.state = "unavailable";
            layer
                .diagnostics
                .push("layer project unavailable or unsafe".into());
        }
        vibe_workspace::user_project::Observation::Initialized { .. } => {
            let built = vibe_core::manifest::Manifest::read(root.join("vibe.toml"))
                .map_err(anyhow::Error::from)
                .and_then(|manifest| super::super::reinstall::effective_rederive_format(&manifest))
                .and_then(|format| build::build_tree_bound(&root, format));
            match built {
                Ok(tree) => {
                    layer.state = "available";
                    layer.tree = Some(tree);
                }
                Err(_) => {
                    layer.state = "unavailable";
                    layer
                        .diagnostics
                        .push("layer tree unavailable; install its requirements explicitly".into());
                }
            }
        }
    }
    layer
}
pub(super) fn collect(root: &Path) -> Vec<Layer> {
    let control = super::super::user_override::user_enabled(root);
    let personal = match vibe_core::user_project::admit_root(vibe_core::settings::settings_dir()) {
        Ok(root) => observe(
            "user",
            Some(root),
            control.as_ref().copied().unwrap_or(false),
        ),
        Err(_) => Layer {
            id: "user",
            enabled: true,
            state: "unavailable",
            root: None,
            tree: None,
            diagnostics: vec!["settings root unavailable".into()],
            relations: Vec::new(),
        },
    };
    let override_layer = match super::super::user_override::observed_root(root) {
        Ok(Some((root, enabled))) => observe("override", Some(root), enabled),
        Ok(None) => observe("override", None, false),
        Err(_) => Layer {
            id: "override",
            enabled: false,
            state: "unavailable",
            root: None,
            tree: None,
            diagnostics: vec!["override binding unavailable".into()],
            relations: Vec::new(),
        },
    };
    let mut layers = vec![
        Layer {
            id: "project",
            enabled: true,
            state: "available",
            root: Some(vibe_core::machine_json_path(root)),
            tree: None,
            diagnostics: Vec::new(),
            relations: Vec::new(),
        },
        personal,
        override_layer,
    ];
    if let Err(error) = control {
        layers[1].state = "unavailable";
        layers[1]
            .diagnostics
            .push(format!("invalid personal layer control: {error}"));
    }
    let resolved = super::super::layer_selection::relations(root);
    for layer in &mut layers {
        let name = match layer.id {
            "project" => vibe_core::manifest::LayerName::Project,
            "user" => vibe_core::manifest::LayerName::User,
            _ => vibe_core::manifest::LayerName::Override,
        };
        if resolved.denied_layers.contains(&name) {
            layer.state = "denied";
        }
        layer.relations = resolved
            .relations
            .iter()
            .filter(|r| r.layer == name)
            .cloned()
            .collect();
        layer.diagnostics.extend(
            resolved
                .diagnostics
                .iter()
                .filter(|(l, _)| *l == name)
                .map(|(_, d)| d.clone()),
        );
    }
    layers
}

pub(super) fn layer_name(id: &str) -> &'static str {
    match id {
        "project" => "project",
        "user" => "personal",
        _ => "override",
    }
}

fn layer_body(layer: &Layer, tree: Option<&PackageTree>) -> String {
    let mut status = format!(
        "  {} · {}",
        if layer.enabled { "enabled" } else { "disabled" },
        layer.state
    );
    if let Some(root) = &layer.root {
        status.push_str(&format!(" · {}", output::language::short_path(root)));
    }
    let mut lines = vec![status];
    if let Some(tree) = tree {
        lines.push(format!("  {}", plain::layer_summary(tree)));
        lines.extend(
            plain::rows(tree)
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| format!("  {line}")),
        );
    }
    lines.extend(
        layer
            .relations
            .iter()
            .map(|relation| format!("  {}", relation.display_line())),
    );
    lines.extend(layer.diagnostics.iter().map(|line| format!("  {line}")));
    lines.join("\n")
}

pub(super) fn run(
    ctx: &output::Context,
    root: &Path,
    tree: &PackageTree,
    interactive: bool,
) -> Result<()> {
    let layers = collect(root);
    if interactive {
        return super::tui::layers::run(root, tree, layers);
    }
    let report = LayeredTree {
        ok: true,
        command: "tree:layers",
        schema_version: 2,
        project: tree,
        layers,
    };
    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    for layer in &report.layers {
        if ctx.is_quiet() {
            ctx.summary(&format!(
                "{} layer · {} · {}",
                layer_name(layer.id),
                if layer.enabled { "enabled" } else { "disabled" },
                layer.state
            ));
            continue;
        }
        let layer_tree = if layer.id == "project" {
            Some(tree)
        } else {
            layer.tree.as_ref()
        };
        ctx.rule(&format!("{} layer", layer_name(layer.id)));
        ctx.block(&layer_body(layer, layer_tree));
    }
    if !ctx.is_quiet() {
        ctx.block(&format!("  {}", plain::legend(tree)));
    }
    Ok(())
}
