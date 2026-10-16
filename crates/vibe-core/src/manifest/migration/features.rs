//! Feature edges in the incumbent ordered-token planner.
use super::*;
use crate::manifest::{FeaturesTable, Requires, feature_effects::*, feature_profile};
use std::collections::BTreeSet;

pub const FEATURES_LEGACY_ABSENT: &str = "features-legacy-absent-r1";
pub const FEATURES_LEGACY_EPOCH1: &str = "features-legacy-epoch1-r1";
pub const FEATURES_TYPED_ABSENT: &str = "features-typed-absent-r1";
pub const FEATURES_TYPED_EPOCH1: &str = "features-typed-epoch1-r1";
pub const FEATURES_TO_TYPED: &str = "features-to-typed-r1";
pub const FEATURES_TO_TYPED_EPOCH1: &str = "features-to-typed-epoch1-r1";
pub(super) const EDGES: [(&str, &str, &str); 4] = [
    (
        FEATURES_LEGACY_ABSENT,
        FEATURES_TYPED_ABSENT,
        FEATURES_TO_TYPED,
    ),
    (
        FEATURES_LEGACY_EPOCH1,
        FEATURES_TYPED_EPOCH1,
        FEATURES_TO_TYPED_EPOCH1,
    ),
    (FEATURES_TYPED_ABSENT, FEATURES_TYPED_ABSENT, IDENTITY),
    (FEATURES_TYPED_EPOCH1, FEATURES_TYPED_EPOCH1, IDENTITY),
];
pub(super) fn typed(profile: &str) -> bool {
    matches!(profile, FEATURES_TYPED_ABSENT | FEATURES_TYPED_EPOCH1)
}
pub(super) fn known(profile: &str) -> bool {
    matches!(
        profile,
        FEATURES_LEGACY_ABSENT
            | FEATURES_LEGACY_EPOCH1
            | FEATURES_TYPED_ABSENT
            | FEATURES_TYPED_EPOCH1
            | FEATURES_TO_TYPED
            | FEATURES_TO_TYPED_EPOCH1
    )
}

pub fn read_feature_manifest(
    text: &str,
    profile: &str,
) -> Checked<feature_profile::CheckedFeatureManifest> {
    if !typed(profile) {
        return Err(shape("target_profile"));
    }
    let doc = Document::parse(text).map_err(|_| shape("document"))?;
    check_shape(&doc, profile)?;
    feature_profile::read_typed(text).map_err(|_| {
        refusal(
            MigrationRefusalKind::InvalidCurrentManifest,
            "features",
            "selected typed reader refused feature semantics",
        )
    })
}

pub(super) fn check_shape(doc: &Document<&str>, profile: &str) -> Checked<()> {
    let root = doc.as_table();
    only(
        root,
        &["package", "requires", "requires_any", "features"],
        "document",
    )?;
    let package = table(root.get("package"), "package")?;
    only(
        package,
        &[
            "name",
            "group",
            "kind",
            "version",
            "epoch",
            "description",
            "license",
            "homepage",
            "materialization",
            "authors",
            "keywords",
            "bridge",
            "frozen",
        ],
        "package",
    )?;
    for key in ["name", "group", "kind", "version"] {
        string(package, key, key)?;
    }
    if !matches!(
        string(package, "kind", "package.kind")?,
        "flow" | "feat" | "stack" | "tool" | "mcp" | "lang"
    ) {
        return Err(shape("package.kind"));
    }
    let epoch = package.get("epoch");
    let epoch1 = matches!(profile, FEATURES_LEGACY_EPOCH1 | FEATURES_TYPED_EPOCH1);
    if (epoch1 && epoch.and_then(Item::as_integer) != Some(1)) || (!epoch1 && epoch.is_some()) {
        return Err(shape("package.epoch"));
    }
    for key in ["description", "license", "homepage", "materialization"] {
        if package.contains_key(key) {
            string(package, key, key)?;
        }
    }
    if !matches!(
        package.get("materialization").and_then(Item::as_str),
        None | Some("copy" | "hardlink" | "in-place")
    ) {
        return Err(shape("package.materialization"));
    }
    for key in ["authors", "keywords"] {
        if let Some(item) = package.get(key) {
            if item
                .as_array()
                .is_none_or(|a| a.iter().any(|v| v.as_str().is_none()))
            {
                return Err(shape(key));
            }
        }
    }
    for key in ["bridge", "frozen"] {
        if package.get(key).is_some_and(|v| v.as_bool().is_none()) {
            return Err(shape(key));
        }
    }
    if let Some(item) = root.get("requires") {
        let requires = table(Some(item), "requires")?;
        only(requires, &["packages", "capabilities"], "requires")?;
        if requires.get("capabilities").is_some_and(|v| {
            v.as_array()
                .is_none_or(|a| a.iter().any(|v| v.as_str().is_none()))
        }) {
            return Err(shape("requires.capabilities"));
        }
        if let Some(item) = requires.get("packages") {
            for (key, item) in table(Some(item), "requires.packages")?.iter() {
                qualified_key(key, "requires.packages")?;
                if item.as_str().is_some() {
                    continue;
                }
                let dep = table(Some(item), "requires.packages.entry")?;
                let mut keys = vec![
                    "version",
                    "path",
                    "git",
                    "tag",
                    "branch",
                    "rev",
                    "auth",
                    "token_env",
                    "link",
                    "access",
                    "friend",
                    "exclude",
                ];
                // Recognized unresolved legacy options are questions, never
                // silently accepted by the target or discarded by conversion.
                if !typed(profile) {
                    keys.extend(["features", "optional"]);
                }
                only(dep, &keys, "requires.packages.entry")?;
                for (field, value) in dep.iter() {
                    match field {
                        "version" if value.as_table_like().is_some() => {
                            let v = table(Some(value), "version")?;
                            only(v, &["var"], "version")?;
                            string(v, "var", "version.var")?;
                        }
                        "link"
                            if !matches!(
                                value.as_str(),
                                Some("static" | "dynamic" | "static-transitive" | "static-hard")
                            ) =>
                        {
                            return Err(shape(field));
                        }
                        "access"
                            if !matches!(
                                value.as_str(),
                                Some("public" | "private" | "friends-only")
                            ) =>
                        {
                            return Err(shape(field));
                        }
                        "auth"
                            if !matches!(
                                value.as_str(),
                                Some("none" | "token-env" | "credential-helper" | "ssh")
                            ) =>
                        {
                            return Err(shape(field));
                        }
                        "friend" => {
                            if value.as_bool().is_none() {
                                return Err(shape(field));
                            }
                        }
                        "exclude" => {
                            if value
                                .as_array()
                                .is_none_or(|a| a.iter().any(|v| v.as_str().is_none()))
                            {
                                return Err(shape(field));
                            }
                        }
                        "optional" | "features" => {}
                        _ => {
                            if value.as_str().is_none() {
                                return Err(shape(field));
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(item) = root.get("requires_any") {
        let rows: Vec<&dyn TableLike> = if let Some(rows) = item.as_array_of_tables() {
            rows.iter().map(|t| t as &dyn TableLike).collect()
        } else if let Some(rows) = item.as_array() {
            rows.iter()
                .map(|v| {
                    v.as_inline_table()
                        .map(|t| t as &dyn TableLike)
                        .ok_or_else(|| shape("requires_any"))
                })
                .collect::<Checked<_>>()?
        } else {
            return Err(shape("requires_any"));
        };
        for row in rows {
            only(row, &["one_of"], "requires_any")?;
            if row
                .get("one_of")
                .and_then(Item::as_array)
                .is_none_or(|a| a.iter().any(|v| v.as_str().is_none()))
            {
                return Err(shape("requires_any.one_of"));
            }
        }
    }
    Ok(())
}

fn question(reference: String, choices: Vec<String>) -> MigrationInputRequirement {
    MigrationInputRequirement {
        field: "feature_reference",
        choices,
        authored_reference: Some(reference),
        resource_path: None,
    }
}
fn q(s: &str) -> String {
    Value::from(s).to_string()
}
fn effect_token(effect: &FeatureEffect) -> String {
    match effect {
        FeatureEffect::Local(local) => format!("{{ local = {} }}", q(local.as_str())),
        FeatureEffect::ActivateDependency(dep) => format!("{{ dependency = {} }}", q(dep.as_str())),
        FeatureEffect::TargetFeature {
            dependency,
            feature,
            strength,
        } => format!(
            "{{ dependency = {}, feature = {}, strength = {} }}",
            q(dependency.as_str()),
            q(feature.as_str()),
            q(if *strength == FeatureStrength::Strong {
                "strong"
            } else {
                "weak"
            })
        ),
        _ => unreachable!("unavailable catalogues ask before rendering"),
    }
}

pub(super) fn edits(
    text: &str,
    doc: &Document<&str>,
) -> Checked<Result<Vec<MigrationTokenEdit>, Vec<MigrationInputRequirement>>> {
    let mut questions = Vec::new();
    let root = doc.as_table();
    let mut dependency_ids = BTreeSet::new();
    if let Some(packages) = root
        .get("requires")
        .and_then(Item::as_table_like)
        .and_then(|r| r.get("packages"))
        .and_then(Item::as_table_like)
    {
        for (name, value) in packages.iter() {
            let coordinate = name
                .split_once(':')
                .map_or(name, |(_, coordinate)| coordinate);
            if !dependency_ids.insert(coordinate) {
                questions.push(question(
                    format!("requires.packages[{coordinate}]"),
                    vec![
                        "disambiguate_dependency_declarations".into(),
                        "retain_source_for_manual_resolution".into(),
                    ],
                ));
            }
            if let Some(entry) = value.as_table_like() {
                for option in ["features", "optional"] {
                    if entry.contains_key(option) {
                        questions.push(question(
                            format!("requires.packages[{name}].{option}"),
                            vec![
                                "remove_unsupported_option".into(),
                                "retain_source_for_manual_resolution".into(),
                            ],
                        ));
                    }
                }
            }
        }
    }
    if !questions.is_empty() {
        return Ok(Err(questions));
    }
    let value: toml::Value = toml::from_str(text).map_err(|_| shape("document"))?;
    let requires: Requires = value
        .get("requires")
        .cloned()
        .map(|v| v.try_into())
        .transpose()
        .map_err(|_| shape("requires"))?
        .unwrap_or_default();
    let declarations = match FeatureDeclarations::from_requires(&requires, &BTreeSet::new()) {
        Ok(declarations) => declarations,
        Err(EffectError::Duplicate(id)) => {
            return Ok(Err(vec![question(
                format!("requires.packages[{id}]"),
                vec![
                    "disambiguate_dependency_declarations".into(),
                    "retain_source_for_manual_resolution".into(),
                ],
            )]));
        }
        Err(_) => return Err(shape("requires")),
    };
    let legacy: FeaturesTable = value
        .get("features")
        .cloned()
        .map(|v| v.try_into())
        .transpose()
        .map_err(|_| shape("features"))?
        .unwrap_or_default();
    let Some(features) = root.get("features") else {
        return Ok(Ok(vec![]));
    };
    let features = table(Some(features), "features")?;
    let mut edits = Vec::new();
    let mut owned_members = BTreeSet::new();
    for (group, members) in &legacy.exclusive {
        if LocalFeatureRef::new(group).is_err()
            || members.is_empty()
            || members.iter().any(|m| {
                m.starts_with('_')
                    || !legacy.features.contains_key(m)
                    || !owned_members.insert(m.clone())
            })
        {
            questions.push(question(
                format!("features.exclusive[{group}]"),
                vec![
                    "disambiguate_constrained_membership".into(),
                    "retain_source_for_manual_resolution".into(),
                ],
            ));
        }
    }
    for (name, item) in features.iter() {
        if name == "exclusive" {
            for (_, members) in table(Some(item), "features.exclusive")?.iter() {
                let array = members
                    .as_array()
                    .ok_or_else(|| shape("features.exclusive"))?;
                let span = array.span().ok_or_else(unprovable)?;
                for (pos, token) in [
                    (span.start, "{ cardinality = \"zero-or-one\", members = "),
                    (span.end, " }"),
                ] {
                    edits.push(MigrationTokenEdit {
                        field: "features.exclusive",
                        before_span: pos..pos,
                        before_token: String::new(),
                        after_token: token.into(),
                    });
                }
            }
            continue;
        }
        if LocalFeatureRef::new(name).is_err() {
            questions.push(question(
                format!("features[{name}]"),
                vec![
                    "rename_local_feature".into(),
                    "retain_source_for_manual_resolution".into(),
                ],
            ));
            continue;
        }
        let array = item.as_array().ok_or_else(|| shape("features"))?;
        for (index, value) in array.iter().enumerate() {
            let raw = value.as_str().ok_or_else(|| shape("features"))?;
            let reference = format!("features[{name}][{index}]:{raw}");
            let Ok(mut effect) = FeatureEffect::parse_legacy(raw) else {
                questions.push(question(
                    reference,
                    vec![
                        "qualify_reference".into(),
                        "retain_source_for_manual_resolution".into(),
                    ],
                ));
                continue;
            };
            let dependency = match &mut effect {
                FeatureEffect::ActivateDependency(dep)
                | FeatureEffect::TargetFeature {
                    dependency: dep, ..
                } => Some(dep),
                FeatureEffect::Local(local) if !legacy.features.contains_key(local.as_str()) => {
                    questions.push(question(
                        reference.clone(),
                        vec![
                            "declare_local_feature".into(),
                            "qualify_dependency_reference".into(),
                        ],
                    ));
                    continue;
                }
                FeatureEffect::ExportedSubskill(_) | FeatureEffect::ConfigValue(_) => {
                    questions.push(question(
                        reference.clone(),
                        vec![
                            "supply_authoritative_catalogue_in_a_supported_profile".into(),
                            "retain_source_for_manual_resolution".into(),
                        ],
                    ));
                    continue;
                }
                _ => None,
            };
            if let Some(dep) = dependency {
                if !declarations.dependencies.contains_key(dep) {
                    let candidates: Vec<_> = declarations
                        .dependencies
                        .keys()
                        .filter(|id| {
                            id.as_str()
                                .rsplit_once('/')
                                .is_some_and(|(_, name)| name == dep.as_str())
                        })
                        .cloned()
                        .collect();
                    if candidates.len() == 1 {
                        *dep = candidates[0].clone();
                    } else {
                        let choices = if candidates.is_empty() {
                            vec!["declare_dependency".into(), "qualify_reference".into()]
                        } else {
                            candidates.iter().map(|v| v.as_str().to_owned()).collect()
                        };
                        questions.push(question(reference, choices));
                        continue;
                    }
                }
            }
            let span = value.span().ok_or_else(unprovable)?;
            edits.push(MigrationTokenEdit {
                field: "features",
                before_token: text.get(span.clone()).ok_or_else(unprovable)?.into(),
                before_span: span,
                after_token: effect_token(&effect),
            });
        }
    }
    if !questions.is_empty() {
        return Ok(Err(questions));
    }
    edits.sort_by_key(|edit| (edit.before_span.start, edit.before_span.end));
    Ok(Ok(edits))
}
