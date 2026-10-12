//! Trace one logical unit through a prepared selected slot observation.

use anyhow::{Context, Result, bail};
use specmap_core::generated::specmap::{CanonicalNode, MaterializedLocator, Specmap};
use vibe_workspace::source_query::{
    MaterializedVibeMedia, WitnessedSourceWorld, canonical_node_of_output,
    native_xml_range_of_output,
};

use crate::Explain;

/// Enrich a carried source unit with a schema-4 semantic hash and a native
/// locator from the selected held materialization. Legacy `file`, `line` and
/// `content_hash` remain source/projection decorations with their old meaning.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-070#CANONICAL-NODE-LOCATION"
)]
pub fn explain_selected_materialized(
    world: &WitnessedSourceWorld,
    group: &str,
    name: &str,
    map: &mut Specmap,
    target: &str,
    json: bool,
) -> Result<Explain> {
    if !matches!(map.schema, 3 | 4) {
        bail!("unsupported carried specmap schema");
    }
    let prefix = format!("spec://{group}/{name}/");
    if !target.starts_with(&prefix) {
        bail!("target is outside the selected package");
    }
    let matching = map
        .specUnits
        .iter()
        .enumerate()
        .filter(|(_, unit)| unit.uri == target)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matching.len() > 1 {
        bail!("target has duplicate carried units");
    }
    let index = matching
        .first()
        .copied()
        .context("target has no carried unit")?;
    let unit = &mut map.specUnits[index];
    if format!("{prefix}{}#{}", unit.docPath, unit.anchor) != target {
        bail!("carried unit address differs from its logical stem and anchor");
    }
    let (output, recorded) = world.selected_document_output(group, name, &unit.docPath)?;
    let semantic = canonical_node_of_output(&recorded, &unit.anchor)?
        .context("anchor absent from selected materialized SpecDoc")?;
    let text = std::str::from_utf8(&recorded.snapshot.bytes)
        .context("selected materialized output is not UTF-8")?;
    let range = match recorded.media {
        Some(MaterializedVibeMedia::Xml) => {
            native_xml_range_of_output(&recorded, &unit.anchor).map_err(|error| error.to_string())
        }
        Some(MaterializedVibeMedia::Markdown) => {
            specmap_core::mdspec::native_anchor_range(text, &unit.anchor)
        }
        None => bail!("selected output has no declared Vibe media"),
    }
    .map_err(anyhow::Error::msg)?
    .context("anchor has no native materialized range")?;
    let absolute = recorded.slot_root.join(&output);
    let relative = absolute
        .strip_prefix(world.workspace_root())
        .context("selected slot escaped consumer workspace")?;
    unit.canonicalNode = Some(Box::new(CanonicalNode {
        recipe: semantic.recipe.to_owned(),
        hash: semantic.hash,
    }));
    unit.materializedLocator = Some(Box::new(MaterializedLocator {
        path: vibe_core::machine_json_path(relative),
        startLine: range.0,
        endLine: range.1,
    }));
    map.schema = 4;
    if json {
        let mut value = specmap_core::explain::explain_json(map, target)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "source".into(),
                serde_json::json!({
                    "from": "prepared-selected-materialization",
                    "carried_file_line": "source-or-projection-decoration",
                    "content_hash": "source-or-projection-span"
                }),
            );
        }
        Ok(Explain::Json(value))
    } else {
        let body = specmap_core::explain::explain_text(map, target)?;
        Ok(Explain::Text(format!(
            "note: selected materialized locator and canonical IR hash are shown separately; carried file:line and content hash remain source/projection decorations\n{body}"
        )))
    }
}

#[cfg(test)]
#[path = "selected_materialized/m_01_c_3_cases.rs"]
mod m_01_c_3_cases;
