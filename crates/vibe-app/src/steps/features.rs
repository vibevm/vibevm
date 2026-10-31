//! Feature resolution (`PROP-065#feature-tree`, `#profiles`): the one
//! deterministic rule set that turns explicit choices over a base set into
//! the closed set of selected features, and records which features were
//! implied by which rule.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#profiles");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl, ProfileDecl};

use crate::error::StepError;

/// `--profile ID`, `--feature ID` and `--no-feature ID`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#features")]
pub struct FeatureRequest {
    pub enable: Vec<String>,
    pub disable: Vec<String>,
    pub profile: Option<String>,
}

impl FeatureRequest {
    /// Whether nothing was named, so recorded or default choices apply.
    pub fn is_empty(&self) -> bool {
        self.enable.is_empty() && self.disable.is_empty() && self.profile.is_none()
    }
}

/// The rule that selected a feature nobody named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree")]
pub enum ImpliedRule {
    /// The ancestor of a selected feature.
    Parent,
    /// Named in a selected feature's `requires`.
    Requires,
    /// A `required` child of a selected parent, or a required root.
    Required,
    /// The default member of an exclusive group with no other selection.
    ExclusiveDefault,
}

impl ImpliedRule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parent => "parent",
            Self::Requires => "requires",
            Self::Required => "required",
            Self::ExclusiveDefault => "exclusive_default",
        }
    }
}

/// One implied feature: the rule and the feature (or group) that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree")]
pub struct Implied {
    pub feature: String,
    pub rule: ImpliedRule,
    pub by: Option<String>,
}

/// The closed set conditions see, in declaration order, and why each
/// implied member is in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree")]
pub struct FeatureResolution {
    pub selected: Vec<String>,
    pub implied: Vec<Implied>,
}

/// Resolve the selection of `option`.
///
/// The base is the terminal's checkboxes, else the profile's features,
/// else (with no flags at all) the recorded set of an update, else the
/// defaults. Explicit choices are `--feature`, the profile and the
/// checkboxes; `--no-feature` deselects a feature with its descendants; an
/// explicit member of an exclusive group replaces the others. The closure
/// then selects ancestors, `requires`, required children and group
/// defaults. A deselection the closure undoes, or an exclusive group left
/// with two members, is a refusal naming the reason.
///
/// ```
/// use vibe_app::{FeatureRequest, resolve_features};
/// use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl};
///
/// let mut option: InstallerOptionDecl = toml::from_str(
///     "id = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"x86_64\"",
/// ).unwrap();
/// option.features = vec![
///     FeatureDecl { id: "gui".into(), name: "GUI".into(), ..Default::default() },
///     FeatureDecl { id: "gtk".into(), name: "GTK".into(), parent: Some("gui".into()), ..Default::default() },
/// ];
/// let request = FeatureRequest { enable: vec!["gtk".into()], ..Default::default() };
/// let resolved = resolve_features(&option, None, &request, None, None).unwrap();
/// assert_eq!(resolved.selected, ["gui", "gtk"]);
/// assert_eq!(resolved.implied[0].feature, "gui");
/// ```
pub fn resolve_features(
    option: &InstallerOptionDecl,
    profile: Option<&ProfileDecl>,
    request: &FeatureRequest,
    previous: Option<&[String]>,
    chosen_by_prompt: Option<&[String]>,
) -> Result<FeatureResolution, StepError> {
    let declared = &option.features;
    let tree = Tree::new(declared);
    let profile_features = profile.map(|profile| profile.features.as_slice());
    for id in request
        .enable
        .iter()
        .chain(&request.disable)
        .chain(profile_features.unwrap_or_default())
        .chain(chosen_by_prompt.unwrap_or_default())
    {
        if !tree.contains(id) {
            return Err(StepError::UnknownFeature {
                option: option.id.clone(),
                feature: id.clone(),
                declared: declared
                    .iter()
                    .map(|feature| feature.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
    }
    let conflict = |reason: String| StepError::FeatureConflict {
        option: option.id.clone(),
        reason,
    };
    if let Some(both) = request
        .enable
        .iter()
        .find(|id| request.disable.contains(id))
    {
        return Err(conflict(format!(
            "`{both}` is both selected and deselected"
        )));
    }
    let explicit: BTreeSet<&str> = request
        .enable
        .iter()
        .chain(profile_features.unwrap_or_default())
        .chain(chosen_by_prompt.unwrap_or_default())
        .map(String::as_str)
        .collect();
    let base: Vec<&str> = match (chosen_by_prompt, profile_features, previous) {
        (Some(chosen), _, _) => chosen.iter().map(String::as_str).collect(),
        (None, Some(features), _) => features.iter().map(String::as_str).collect(),
        (None, None, Some(previous)) if request.is_empty() => previous
            .iter()
            .map(String::as_str)
            .filter(|id| tree.contains(id))
            .collect(),
        _ => declared
            .iter()
            .filter(|feature| feature.default)
            .map(|feature| feature.id.as_str())
            .collect(),
    };
    let mut set: BTreeSet<&str> = base.into_iter().chain(explicit.iter().copied()).collect();
    for id in &request.disable {
        if let Some(below) = explicit
            .iter()
            .find(|selected| tree.is_descendant(selected, id))
        {
            return Err(conflict(format!(
                "`{id}` cannot be deselected: the selected feature `{below}` is beneath it"
            )));
        }
        tree.remove_with_descendants(&mut set, id);
    }
    for (group, members) in &tree.groups {
        let chosen: Vec<&str> = members
            .iter()
            .copied()
            .filter(|member| explicit.contains(member))
            .collect();
        match chosen.as_slice() {
            [] => {}
            [only] => {
                for member in members.iter().filter(|member| *member != only) {
                    tree.remove_with_descendants(&mut set, member);
                }
            }
            several => {
                return Err(conflict(format!(
                    "exclusive group `{group}` has more than one explicit selection: {}",
                    several.join(", ")
                )));
            }
        }
    }
    let implied = tree.close(&mut set);
    for (group, members) in &tree.groups {
        let selected: Vec<&str> = members
            .iter()
            .copied()
            .filter(|member| set.contains(member))
            .collect();
        if selected.len() > 1 {
            return Err(conflict(format!(
                "exclusive group `{group}` would select {}; exactly one member may be selected",
                selected.join(" and ")
            )));
        }
    }
    for id in &request.disable {
        if let Some((rule, by)) = implied.get(id.as_str()) {
            let by = by.unwrap_or_default();
            return Err(conflict(match rule {
                ImpliedRule::Parent => {
                    format!(
                        "`{id}` cannot be deselected: the selected feature `{by}` is beneath it"
                    )
                }
                ImpliedRule::Requires => {
                    format!("`{id}` cannot be deselected: the selected feature `{by}` requires it")
                }
                ImpliedRule::Required if by.is_empty() => {
                    format!("`{id}` is a required feature and is always selected")
                }
                ImpliedRule::Required => format!(
                    "`{id}` cannot be deselected: it is required while its parent `{by}` is selected"
                ),
                ImpliedRule::ExclusiveDefault => format!(
                    "`{id}` cannot be deselected: it is the default of exclusive group `{by}` and no other member is selected"
                ),
            }));
        }
    }
    Ok(FeatureResolution {
        selected: declared
            .iter()
            .filter(|feature| set.contains(feature.id.as_str()))
            .map(|feature| feature.id.clone())
            .collect(),
        implied: declared
            .iter()
            .filter_map(|feature| {
                let (rule, by) = implied.get(feature.id.as_str())?;
                Some(Implied {
                    feature: feature.id.clone(),
                    rule: *rule,
                    by: by.map(str::to_owned),
                })
            })
            .collect(),
    })
}

/// The declared features of an option as a tree with groups.
struct Tree<'a> {
    features: &'a [FeatureDecl],
    groups: BTreeMap<&'a str, Vec<&'a str>>,
}

type Implications<'a> = BTreeMap<&'a str, (ImpliedRule, Option<&'a str>)>;

impl<'a> Tree<'a> {
    fn new(features: &'a [FeatureDecl]) -> Self {
        let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for feature in features {
            if let Some(group) = &feature.exclusive_group {
                groups.entry(group).or_default().push(&feature.id);
            }
        }
        Self { features, groups }
    }

    fn contains(&self, id: &str) -> bool {
        self.features.iter().any(|feature| feature.id == id)
    }

    fn get(&self, id: &str) -> Option<&'a FeatureDecl> {
        self.features.iter().find(|feature| feature.id == id)
    }

    /// Whether `id` lies beneath `ancestor`.
    fn is_descendant(&self, id: &str, ancestor: &str) -> bool {
        let mut current = self.get(id).and_then(|feature| feature.parent.as_deref());
        for _ in 0..=self.features.len() {
            match current {
                Some(parent) if parent == ancestor => return true,
                Some(parent) => current = self.get(parent).and_then(|f| f.parent.as_deref()),
                None => return false,
            }
        }
        false
    }

    fn remove_with_descendants(&self, set: &mut BTreeSet<&'a str>, id: &str) {
        set.remove(id);
        for child in self
            .features
            .iter()
            .filter(|feature| feature.parent.as_deref() == Some(id))
        {
            self.remove_with_descendants(set, &child.id);
        }
    }

    /// Apply the selection rules until nothing changes; return why each
    /// added feature was added.
    fn close(&self, set: &mut BTreeSet<&'a str>) -> Implications<'a> {
        let mut implied = Implications::new();
        let mut add = |set: &mut BTreeSet<&'a str>, id: &'a str, rule, by| {
            if set.insert(id) {
                implied.insert(id, (rule, by));
                true
            } else {
                false
            }
        };
        loop {
            let mut changed = false;
            for feature in self.features {
                let selected = set.contains(feature.id.as_str());
                if selected {
                    if let Some(parent) = feature.parent.as_deref().and_then(|id| self.get(id)) {
                        changed |= add(set, &parent.id, ImpliedRule::Parent, Some(&feature.id));
                    }
                    for required in &feature.requires {
                        if let Some(required) = self.get(required) {
                            changed |=
                                add(set, &required.id, ImpliedRule::Requires, Some(&feature.id));
                        }
                    }
                }
                if feature.required && !selected {
                    match feature.parent.as_deref() {
                        None => changed |= add(set, &feature.id, ImpliedRule::Required, None),
                        Some(parent) if set.contains(parent) => {
                            let parent = self.get(parent).map(|decl| decl.id.as_str());
                            changed |= add(set, &feature.id, ImpliedRule::Required, parent);
                        }
                        Some(_) => {}
                    }
                }
            }
            for (group, members) in &self.groups {
                let active = members.iter().any(|member| {
                    self.get(member)
                        .and_then(|decl| decl.parent.as_deref())
                        .is_none_or(|parent| set.contains(parent))
                });
                if active && !members.iter().any(|member| set.contains(member)) {
                    let default = members
                        .iter()
                        .copied()
                        .find(|member| self.get(member).is_some_and(|decl| decl.default));
                    if let Some(default) = default {
                        changed |= add(set, default, ImpliedRule::ExclusiveDefault, Some(group));
                    }
                }
            }
            if !changed {
                return implied;
            }
        }
    }
}

/// The features in tree order (each parent before its children, siblings
/// in declaration order) with their depth, for an indented multi-select.
pub(crate) fn tree_order(features: &[FeatureDecl]) -> Vec<(usize, &FeatureDecl)> {
    fn visit<'a>(
        features: &'a [FeatureDecl],
        parent: Option<&str>,
        depth: usize,
        out: &mut Vec<(usize, &'a FeatureDecl)>,
    ) {
        for feature in features
            .iter()
            .filter(|feature| feature.parent.as_deref() == parent)
        {
            out.push((depth, feature));
            visit(features, Some(&feature.id), depth + 1, out);
        }
    }
    let mut out = Vec::new();
    visit(features, None, 0, &mut out);
    out
}

#[cfg(test)]
#[path = "features/tests.rs"]
mod tests;
