//! One read-only interpretation of explicit lower-layer grants and relations.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#LAYERED-PREFIX");
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use vibe_core::manifest::{LayerCategory, LayerName, LayerPolicy, LayerRelationKind};

mod owners;
mod source;

pub struct LayerInput<'a> {
    pub layer: LayerName,
    pub root: Option<&'a Path>,
    pub enabled: bool,
    pub available: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Relation {
    pub id: String,
    pub layer: LayerName,
    pub source: String,
    pub source_owner: String,
    pub source_version: String,
    pub target_owner: String,
    pub target_version: Option<String>,
    pub kind: LayerRelationKind,
    pub target_layer: LayerName,
    pub target: String,
    pub target_source: Option<String>,
    pub state: String,
    pub reason: String,
    #[serde(skip)]
    pub source_text: String,
}
impl Relation {
    /// One qualified provenance line shared by plain and interactive views.
    pub fn display_line(&self) -> String {
        format!(
            "relation {} {:?}: {}@{} {} -> {:?}:{}@{}:{} — {} ({})",
            self.id,
            self.kind,
            self.source_owner,
            self.source_version,
            self.source,
            self.target_layer,
            self.target_owner,
            self.target_version.as_deref().unwrap_or("unknown"),
            self.target,
            self.state,
            self.reason
        )
    }
}
#[derive(Debug, Default)]
pub struct Resolution {
    pub relations: Vec<Relation>,
    pub denied_layers: BTreeSet<LayerName>,
    pub diagnostics: Vec<(LayerName, String)>,
}
impl Resolution {
    /// A late notice does not remove any earlier shared prefix bytes.
    pub fn notice(&self, layer: LayerName) -> String {
        let mut text = String::new();
        for r in self
            .relations
            .iter()
            .filter(|r| r.layer == layer && matches!(r.state.as_str(), "active" | "superseded"))
        {
            text.push_str(&format!("Declared layer relation {}: {:?} {} -> {:?}:{} ({}) is {}. {} Earlier prefix bytes remain present; only this explicitly granted precedence applies.\n",
                r.id, r.kind, r.source, r.target_layer, r.target, r.target_source.as_deref().unwrap_or("unknown"), r.state, r.reason));
            if r.state == "active" {
                text.push_str(&format!(
                    "Declared effective source unit {}:\n{}\n",
                    r.source, r.source_text
                ));
            }
        }
        text
    }
}
struct Target {
    source: String,
    policy: LayerPolicy,
    category: LayerCategory,
    version: String,
}
/// Inputs are ordered project, user, override. Optional-layer errors deny that
/// layer, never grant authority by omission or prevent the base project boot.
pub fn resolve(inputs: &[LayerInput<'_>]) -> Resolution {
    let mut result = Resolution::default();
    let mut targets = BTreeMap::<(LayerName, String, String), Target>::new();
    let mut roots = BTreeMap::new();
    for input in inputs {
        let Some(root) = input.root else { continue };
        if !input.available {
            continue;
        }
        if !input.enabled {
            // Observe admitted owner declarations only: no anchor reads,
            // compilation, grants or relation activation for a disabled layer.
            match owners::collect(root) {
                Ok((_, owners)) => {
                    for owner in owners {
                        let Some(decl) = owner.manifest.layers else {
                            continue;
                        };
                        for relation in decl.relations {
                            let target_owner = relation.target_owner.unwrap_or_else(|| {
                                if owner.is_root {
                                    roots
                                        .get(&relation.target_layer)
                                        .cloned()
                                        .unwrap_or_default()
                                } else {
                                    owner.coordinate.clone()
                                }
                            });
                            let target = targets.get(&(
                                relation.target_layer,
                                target_owner.clone(),
                                relation.target.clone(),
                            ));
                            result.relations.push(Relation {
                            id: relation.id, layer: input.layer, source: relation.source,
                            source_owner: owner.coordinate.clone(), source_version: owner.version.clone(),
                            target_owner, target_version: target.map(|t| t.version.clone()),
                            kind: relation.kind, target_layer: relation.target_layer, target: relation.target,
                            target_source: target.map(|t| t.source.clone()),
                            state: "disabled".into(), reason: "source layer disabled; declarations observed without activating source units".into(), source_text: String::new(),
                        });
                        }
                    }
                }
                Err(error) => result.diagnostics.push((
                    input.layer,
                    format!("disabled declaration inventory unavailable: {error}"),
                )),
            }
            continue;
        }
        let (workspace, owners) = match owners::collect(root) {
            Ok(value) => value,
            Err(error) => {
                result.denied_layers.insert(input.layer);
                result.diagnostics.push((input.layer, format!("layer declaration admission refused: {error}; restore/install its ordinary project")));
                continue;
            }
        };
        roots.insert(input.layer, owners[0].coordinate.clone());
        let mut pending_targets = Vec::new();
        let mut pending_relations = Vec::new();
        let mut errors = Vec::new();
        let mut replaced = BTreeSet::new();
        for owner in &owners {
            let Some(decl) = &owner.manifest.layers else {
                continue;
            };
            let selected = match owners::selected(owner, &owners) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            for target in &decl.targets {
                match source::admit(
                    owner,
                    &workspace,
                    selected.clone(),
                    &target.source,
                    target.policy == LayerPolicy::Overridable,
                ) {
                    Ok(_) => pending_targets.push((
                        (input.layer, owner.coordinate.clone(), target.id.clone()),
                        Target {
                            source: target.source.clone(),
                            policy: target.policy,
                            category: target.category,
                            version: owner.version.clone(),
                        },
                    )),
                    Err(error) => errors.push(format!(
                        "{} target `{}` source `{}`: {error}",
                        owner.coordinate, target.id, target.source
                    )),
                }
            }
            for relation in &decl.relations {
                let target_owner = relation.target_owner.clone().unwrap_or_else(|| {
                    if owner.is_root {
                        roots
                            .get(&relation.target_layer)
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        owner.coordinate.clone()
                    }
                });
                let key = (
                    relation.target_layer,
                    target_owner.clone(),
                    relation.target.clone(),
                );
                let mut row = Relation {
                    id: relation.id.clone(),
                    layer: input.layer,
                    source: relation.source.clone(),
                    source_owner: owner.coordinate.clone(),
                    source_version: owner.version.clone(),
                    target_owner,
                    target_version: None,
                    kind: relation.kind,
                    target_layer: relation.target_layer,
                    target: relation.target.clone(),
                    target_source: None,
                    state: "active".into(),
                    reason: "explicit lower-layer grant".into(),
                    source_text: String::new(),
                };
                let error = if relation.target_layer >= input.layer {
                    Some("relations must target a strictly lower layer".to_string())
                } else if let Err(error) =
                    source::admit(owner, &workspace, selected.clone(), &relation.source, false).map(
                        |text| {
                            row.source_text = text;
                        },
                    )
                {
                    Some(format!("source `{}`: {error}", relation.source))
                } else if let Some(target) = targets.get(&key) {
                    row.target_source = Some(target.source.clone());
                    row.target_version = Some(target.version.clone());
                    if relation.kind == LayerRelationKind::Replace
                        && (target.policy != LayerPolicy::Overridable
                            || target.category != LayerCategory::Preference)
                    {
                        Some("target is protected; only the lower unit owner may grant preference replacement".into())
                    } else if relation.kind == LayerRelationKind::Replace && !replaced.insert(key) {
                        Some(
                            "competing replacements in one layer; choose one explicit source"
                                .into(),
                        )
                    } else {
                        None
                    }
                } else {
                    Some("target is absent, unmarked, unavailable or disabled; name its explicit target_owner and admitted lower target".into())
                };
                if let Some(error) = error {
                    row.state = "denied".into();
                    row.reason = error.clone();
                    errors.push(format!(
                        "{} relation `{}`: {error}",
                        owner.coordinate, relation.id
                    ));
                }
                pending_relations.push(row);
            }
        }
        if !errors.is_empty() {
            result.denied_layers.insert(input.layer);
            for row in &mut pending_relations {
                if row.state == "active" {
                    row.state = "denied".into();
                    row.reason = "another declaration denied this entire layer".into();
                }
            }
            result
                .diagnostics
                .extend(errors.into_iter().map(|e| (input.layer, e)));
        } else {
            targets.extend(pending_targets);
            for row in pending_relations
                .iter()
                .filter(|r| r.kind == LayerRelationKind::Replace)
            {
                for previous in &mut result.relations {
                    if previous.state == "active"
                        && previous.kind == LayerRelationKind::Replace
                        && previous.target_layer == row.target_layer
                        && previous.target_owner == row.target_owner
                        && previous.target == row.target
                    {
                        previous.state = "superseded".into();
                        previous.reason =
                            format!("higher active {:?} relation `{}` wins", row.layer, row.id);
                    }
                }
            }
        }
        result.relations.extend(pending_relations);
    }
    result
}

#[cfg(test)]
mod tests;
