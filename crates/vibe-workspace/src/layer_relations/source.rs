//! Address admission delegates lookup and anchors to the ordinary spec owners.
use super::owners::Owner;
use std::{collections::BTreeMap, path::Path};
use vibe_spec::{Authority, DocTree, FileResolver, SelectedPackage, SelfCoordinate, SpecAddress};

pub(super) fn admit(
    owner: &Owner,
    workspace: &Path,
    selected: BTreeMap<(String, String), SelectedPackage>,
    source: &str,
    grant: bool,
) -> Result<String, String> {
    let address = SpecAddress::parse(source).map_err(|e| e.to_string())?;
    if address.pinned_r.is_some() {
        return Err("revision-qualified relation sources are not supported; use an exact anchor/version URI".into());
    }
    if address.anchor.is_empty() {
        return Err("explicit source anchor required".into());
    }
    let (group, name) = owner
        .coordinate
        .split_once('/')
        .ok_or("invalid declaring coordinate")?;
    let self_coord =
        SelfCoordinate::new((group != "__host__").then(|| group.to_owned()), name.into());
    let own =
        matches!(&address.authority,Authority::Package{group:g,name:n,..} if g==group&&n==name);
    if grant && !own {
        return Err("only the unit's own declaring owner may grant replacement; reference its target_owner instead".into());
    }
    let owning_root = match &address.authority {
        Authority::Package { .. } if own => owner.root.clone(),
        Authority::Package { group, name, .. } => selected
            .get(&(group.clone(), name.clone()))
            .ok_or("source is not own or an admitted declared dependency")?
            .root
            .clone(),
        _ => return Err("source must name an ordinary package coordinate".into()),
    };
    let resolver = FileResolver::with_selected_world(&owner.root, workspace, selected, self_coord);
    let file = resolver.resolve_file(&address).map_err(|e| e.to_string())?;
    let relative = file
        .strip_prefix(&owning_root)
        .map_err(|_| "source escaped selected owner")?;
    if relative.starts_with(vibe_core::layout::current_boot_dir().join("local")) {
        return Err("composed boot/local is not a declaration authority".into());
    }
    let safe = vibe_safefs::Project::open(&owning_root).map_err(|e| e.to_string())?;
    let snapshot = safe
        .read_file_snapshot_bounded(
            relative.to_str().ok_or("source path is not UTF-8")?,
            16 * 1024 * 1024,
        )
        .map_err(|e| e.to_string())?
        .ok_or("declared source absent")?;
    let raw = std::str::from_utf8(&snapshot.bytes).map_err(|e| e.to_string())?;
    let (text, _) = vibe_specdoc::project_spec_text(&file, raw).map_err(|e| e.to_string())?;
    let tree = DocTree::parse(&text);
    let node = tree
        .resolve_unique_path(&address.anchor)
        .map_err(|e| e.to_string())?
        .ok_or("source anchor absent; declare one exact ordinary spec unit")?;
    Ok(tree.text(node))
}
