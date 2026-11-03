//! Feature resolution (`PROP-065#feature-tree`, `#profiles`): the one
//! deterministic rule set that turns explicit choices over a base set into
//! the closed set of selected features, and records which features were
//! implied by which rule.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#profiles");

use specmark::spec;
use vibe_core::configuration::{FeatureBase, FeatureInput};
use vibe_core::manifest::{FeatureDecl, InstallerOptionDecl, ProfileDecl};

use crate::error::StepError;

mod adapter;

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
/// defaults. Only `--feature` is a direct addition above that base;
/// `--no-feature` deselects a feature with its descendants; a direct member
/// of an exclusive group replaces the others. The closure
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
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-APP-COMPATIBILITY")]
pub fn resolve_features(
    option: &InstallerOptionDecl,
    profile: Option<&ProfileDecl>,
    request: &FeatureRequest,
    previous: Option<&[String]>,
    chosen_by_prompt: Option<&[String]>,
) -> Result<FeatureResolution, StepError> {
    let profile_features = profile.map(|profile| profile.features.as_slice());
    let known = |id: &str| option.features.iter().any(|feature| feature.id == id);
    // Keep the established input-validation order and app diagnostic shape.
    for id in request
        .enable
        .iter()
        .chain(&request.disable)
        .chain(profile_features.unwrap_or_default())
        .chain(chosen_by_prompt.unwrap_or_default())
    {
        if !known(id) {
            return Err(adapter::unknown(option, id.clone()));
        }
    }
    let base = match (chosen_by_prompt, profile_features, previous) {
        (Some(chosen), _, _) => FeatureBase::Selected(chosen.to_vec()),
        (None, Some(features), _) => FeatureBase::Selected(features.to_vec()),
        (None, None, Some(recorded)) if request.is_empty() => {
            FeatureBase::Selected(recorded.iter().filter(|id| known(id)).cloned().collect())
        }
        _ => FeatureBase::Defaults,
    };
    let model = adapter::model(option)?;
    model
        .resolve(&FeatureInput {
            base,
            additions: request.enable.clone(),
            disabled: request.disable.clone(),
        })
        .map(adapter::result)
        .map_err(|error| adapter::error(option, error))
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

#[cfg(test)]
#[path = "features/legacy_oracle.rs"]
mod legacy_oracle;

#[cfg(test)]
#[path = "features/compatibility.rs"]
mod compatibility;
