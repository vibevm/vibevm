//! Explicit package-owned B1/B2 definition, PROP-003#M05F1-PROFILE.
use super::*;

pub const PROFILE: &str = "constrained-r1";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Bound {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
    ExactlyOne,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PositiveGuard {
    Member(MemberGuard),
    Group(GroupGuard),
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberGuard {
    member: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupGuard {
    group: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredGroup {
    cardinality: Bound,
    members: Vec<String>,
    defaults: Option<Vec<String>>,
    #[serde(default)]
    guards: Vec<PositiveGuard>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredFeatures {
    profile: String,
    #[serde(default)]
    optional: Vec<String>,
    effects: BTreeMap<String, Vec<TypedEffect>>,
    groups: BTreeMap<String, AuthoredGroup>,
}
impl FeatureWire for AuthoredFeatures {
    fn is_empty(&self) -> bool {
        false
    }
}

fn locals(values: Vec<String>) -> Result<BTreeSet<LocalFeatureRef>, String> {
    let count = values.len();
    let set: BTreeSet<_> = values
        .into_iter()
        .map(LocalFeatureRef::new)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if set.len() != count {
        return Err("duplicate constrained member".into());
    }
    Ok(set)
}

/// Select only the explicit profile. Legacy and B3 readers are unchanged.
/// No selection or default authorization is manufactured by this decoder.
pub fn read(text: &str) -> Result<Option<CheckedFeatureManifest>, String> {
    let value: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    let Some(profile) = value.get("features").and_then(|f| f.get("profile")) else {
        return Ok(None);
    };
    // An array is an ordinary legacy feature, including this spelling.
    if profile.is_array() {
        return Ok(None);
    }
    if profile.as_str() != Some(PROFILE) {
        return Err("unknown authored feature profile".into());
    }
    let wire: ManifestWire<AuthoredFeatures> = toml::from_str(text).map_err(|e| e.to_string())?;
    let authored = wire.features.clone();
    if authored.profile != PROFILE {
        return Err("unknown authored feature profile".into());
    }
    validate_typed(&authored.effects)?;
    let legacy = TypedFeatures {
        features: authored.effects,
        groups: BTreeMap::new(),
    }
    .legacy();
    let manifest = wire.into_manifest_with_features(legacy)?;
    let mut optional = BTreeSet::new();
    for name in authored.optional {
        if !optional.insert(DependencyDeclarationId::new(name).map_err(|e| e.to_string())?) {
            return Err("duplicate optional dependency".into());
        }
    }
    let declarations = FeatureDeclarations::from_requires(&manifest.requires, &optional)
        .map_err(|e| e.to_string())?;
    let program = FeatureProgram::lower_legacy(&manifest.features, &declarations)
        .map_err(|e| e.to_string())?;
    let mut constraints = Constraints::default();
    for (name, group) in authored.groups {
        let guards = group
            .guards
            .into_iter()
            .map(|g| match g {
                PositiveGuard::Member(g) => Guard::parse("member", &g.member),
                PositiveGuard::Group(g) => Guard::parse("group", &g.group),
            })
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        constraints.groups.insert(
            LocalFeatureRef::new(name).map_err(|e| e.to_string())?,
            ConstrainedGroup {
                cardinality: match group.cardinality {
                    Bound::ZeroOrMore => Cardinality::ZeroOrMore,
                    Bound::OneOrMore => Cardinality::OneOrMore,
                    Bound::ZeroOrOne => Cardinality::ZeroOrOne,
                    Bound::ExactlyOne => Cardinality::ExactlyOne,
                },
                members: locals(group.members)?,
                defaults: group.defaults.map(locals).transpose()?,
                guards,
            },
        );
    }
    constraints
        .order(&program, &declarations)
        .map_err(|e| e.to_string())?;
    manifest.validate().map_err(|e| e.to_string())?;
    Ok(Some(CheckedFeatureManifest {
        manifest,
        declarations,
        program,
        constraints,
        selection: Selection::Omitted,
    }))
}
