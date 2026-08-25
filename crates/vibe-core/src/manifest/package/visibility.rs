specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#model");

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::manifest::document::{BootSection, Manifest, OriginSection, WorkspaceSection};
use crate::manifest::extension::{
    ExtensionDeclWire, ExtensionsControlWire, validate_extension_declarations,
};
use crate::manifest::i18n::I18nDecl;
use crate::manifest::project::{
    ActiveSection, LlmSection, MirrorSection, OverrideSection, ProjectSection, RegistrySection,
};

use super::{
    BinaryDecl, BootSnippet, Compatibility, ConditionalTarget, ConflictsList, FeaturesTable,
    HooksDecl, McpServerDecl, Obsoletes, PackageMeta, Provides, Recommends, Requires, RequiresAny,
    SkillDecl, Suggests,
};

/// Per-edge seepage level: how far the target travels toward consumers.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_core::manifest::AccessLevel;
///
/// let levels: BTreeMap<String, AccessLevel> =
///     toml::from_str("level = \"friends-only\"").unwrap();
/// assert_eq!(levels["level"], AccessLevel::FriendsOnly);
/// assert!(toml::to_string(&levels).unwrap().contains("friends-only"));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessLevel {
    /// The default: the target seeps to every consumer above.
    #[default]
    Public,
    /// The edge is traversed only in the declaring node's dev world.
    Private,
    /// The target seeps only to consumers that befriend the declarant.
    FriendsOnly,
}

/// The role-blind `[visibility]` section.
///
/// ```
/// use vibe_core::manifest::VisibilityMeta;
///
/// let meta: VisibilityMeta = toml::from_str("allow-friends = []").unwrap();
/// assert_eq!(meta.allow_friends, Some(Vec::new()));
/// assert!(!meta.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct VisibilityMeta {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub friends: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unfriend: Vec<String>,
    /// Absent is open, an empty list is sealed, and a list names the circle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_friends: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_concept_warnings: Vec<String>,
}

impl VisibilityMeta {
    /// Whether the section carries no declaration.
    ///
    /// ```
    /// use vibe_core::manifest::VisibilityMeta;
    ///
    /// assert!(VisibilityMeta::default().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.friends.is_empty()
            && self.unfriend.is_empty()
            && self.allow_friends.is_none()
            && self.ignore_concept_warnings.is_empty()
    }
}

/// One `[override]` entry: the sanctioned path-scoped break-in.
///
/// ```
/// use vibe_core::manifest::{AccessLevel, OverrideEntry};
///
/// let entry: OverrideEntry = toml::from_str("access = \"private\"").unwrap();
/// assert_eq!(entry.access, Some(AccessLevel::Private));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct OverrideEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friend: Option<bool>,
    /// `true` kills the targeted edge on chains through the declarant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
    /// A node-targeted replacement for its sealed-circle declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_friends: Option<AllowFriendsOverride>,
}

/// `allow-friends = "*"` or a replacement permits list.
///
/// ```
/// use vibe_core::manifest::{AllowFriendsOverride, OverrideEntry};
///
/// let entry: OverrideEntry = toml::from_str("allow-friends = \"*\"").unwrap();
/// assert_eq!(entry.allow_friends, Some(AllowFriendsOverride::Everyone("*".into())));
/// assert!(toml::from_str::<OverrideEntry>("allow-friends = \"somebody\"").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AllowFriendsOverride {
    Everyone(String),
    List(Vec<String>),
}

impl<'de> Deserialize<'de> for AllowFriendsOverride {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Everyone(String),
            List(Vec<String>),
        }

        match Wire::deserialize(deserializer)? {
            Wire::Everyone(value) if value == "*" => Ok(Self::Everyone(value)),
            Wire::Everyone(value) => Err(serde::de::Error::custom(format!(
                "allow-friends string must be `*`, got `{value}`"
            ))),
            Wire::List(values) => Ok(Self::List(values)),
        }
    }
}

/// The `[override]` table keyed by an edge or node coordinate.
///
/// ```
/// use vibe_core::manifest::{OverrideTable, OverrideTarget};
///
/// let table: OverrideTable = toml::from_str(
///     "\"org.x/a -> org.x/b\" = { access = \"public\" }",
/// ).unwrap();
/// assert!(matches!(table.targets().unwrap()[0].0, OverrideTarget::Edge { .. }));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverrideTable(pub BTreeMap<String, OverrideEntry>);

/// A validated `[override]` key.
///
/// ```
/// use vibe_core::manifest::OverrideTarget;
///
/// let target = OverrideTarget::Node("org.x/api".into());
/// assert!(matches!(target, OverrideTarget::Node(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideTarget {
    Edge { from: String, to: String },
    Node(String),
}

impl OverrideTable {
    /// Parse and validate every key and its edge/node field combination.
    ///
    /// ```
    /// use vibe_core::manifest::OverrideTable;
    ///
    /// let table: OverrideTable = toml::from_str(
    ///     "\"org.x/api\" = { allow-friends = \"*\" }",
    /// ).unwrap();
    /// assert_eq!(table.targets().unwrap().len(), 1);
    /// ```
    pub fn targets(&self) -> Result<Vec<(OverrideTarget, &OverrideEntry)>, String> {
        self.0
            .iter()
            .map(|(key, entry)| {
                let target = parse_override_target(key)?;
                match &target {
                    OverrideTarget::Node(_) if edge_fields_present(entry) => Err(format!(
                        "override node `{key}` may set only `allow-friends`"
                    )),
                    OverrideTarget::Edge { .. } if entry.allow_friends.is_some() => Err(format!(
                        "override edge `{key}` cannot set node field `allow-friends`"
                    )),
                    _ => {
                        validate_allow_friends(entry)?;
                        Ok((target, entry))
                    }
                }
            })
            .collect()
    }
}

fn parse_override_target(key: &str) -> Result<OverrideTarget, String> {
    let pieces: Vec<&str> = key.split("->").collect();
    match pieces.as_slice() {
        [node] => {
            let node = node.trim();
            validate_coordinate(node, key)?;
            Ok(OverrideTarget::Node(node.to_string()))
        }
        [from, to] => {
            let from = from.trim();
            let to = to.trim();
            validate_coordinate(from, key)?;
            validate_coordinate(to, key)?;
            Ok(OverrideTarget::Edge {
                from: from.to_string(),
                to: to.to_string(),
            })
        }
        _ => Err(format!("invalid override key `{key}`: expected one `->`")),
    }
}

fn validate_coordinate(coordinate: &str, key: &str) -> Result<(), String> {
    if coordinate_is_valid(coordinate) {
        Ok(())
    } else {
        Err(format!(
            "invalid override key `{key}`: `{coordinate}` is not a `<group>/<name>` coordinate"
        ))
    }
}

fn coordinate_is_valid(coordinate: &str) -> bool {
    !coordinate.chars().any(char::is_whitespace)
        && coordinate.split_once('/').is_some_and(|(group, name)| {
            !group.is_empty() && !name.is_empty() && !name.contains('/')
        })
}

fn edge_fields_present(entry: &OverrideEntry) -> bool {
    entry.access.is_some() || entry.friend.is_some() || entry.exclude.is_some()
}

fn validate_allow_friends(entry: &OverrideEntry) -> Result<(), String> {
    match &entry.allow_friends {
        Some(AllowFriendsOverride::Everyone(value)) if value != "*" => {
            Err(format!("allow-friends string must be `*`, got `{value}`"))
        }
        _ => Ok(()),
    }
}

pub(crate) fn visibility_is_empty(value: &Option<VisibilityMeta>) -> bool {
    value.as_ref().is_none_or(VisibilityMeta::is_empty)
}

pub(crate) fn validate_visibility(meta: &VisibilityMeta) -> Result<(), String> {
    for (field, values) in [("friends", &meta.friends), ("unfriend", &meta.unfriend)] {
        for value in values {
            if !coordinate_is_valid(value) {
                return Err(format!("invalid visibility.{field} coordinate `{value}`"));
            }
        }
    }
    if let Some(circle) = &meta.allow_friends {
        for value in circle {
            if let Some(group) = value.strip_suffix("/*") {
                if group.is_empty() || group.contains('/') || group.chars().any(char::is_whitespace)
                {
                    return Err(format!("invalid allow-friends pattern `{value}`"));
                }
            } else if !coordinate_is_valid(value) {
                return Err(format!("invalid allow-friends coordinate `{value}`"));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum OverrideWire {
    Registry(Vec<OverrideSection>),
    Visibility(OverrideTable),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<ProjectSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    package: Option<PackageMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workspace: Option<WorkspaceSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<OriginSection>,
    #[serde(default, skip_serializing_if = "Requires::is_empty")]
    requires: Requires,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    requires_any: Vec<RequiresAny>,
    #[serde(default, skip_serializing_if = "Provides::is_empty")]
    provides: Provides,
    #[serde(default, skip_serializing_if = "Obsoletes::is_empty")]
    obsoletes: Obsoletes,
    #[serde(default, skip_serializing_if = "ConflictsList::is_empty")]
    conflicts: ConflictsList,
    #[serde(default, skip_serializing_if = "Recommends::is_empty")]
    recommends: Recommends,
    #[serde(default, skip_serializing_if = "Suggests::is_empty")]
    suggests: Suggests,
    #[serde(default, rename = "skill", skip_serializing_if = "Vec::is_empty")]
    skills: Vec<SkillDecl>,
    #[serde(default, rename = "binary", skip_serializing_if = "Vec::is_empty")]
    binaries: Vec<BinaryDecl>,
    #[serde(default, rename = "mcp_server", skip_serializing_if = "Vec::is_empty")]
    mcp_servers: Vec<McpServerDecl>,
    #[serde(default, skip_serializing_if = "HooksDecl::is_empty")]
    hooks: HooksDecl,
    #[serde(default, rename = "extension", skip_serializing_if = "Vec::is_empty")]
    extensions: Vec<ExtensionDeclWire>,
    #[serde(
        default,
        rename = "extensions",
        skip_serializing_if = "ExtensionsControlWire::is_empty"
    )]
    extension_controls: ExtensionsControlWire,
    #[serde(default, skip_serializing_if = "Compatibility::is_empty")]
    compatibility: Compatibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boot_snippet: Option<BootSnippet>,
    #[serde(default, skip_serializing_if = "FeaturesTable::is_empty")]
    features: FeaturesTable,
    #[serde(default, rename = "target", skip_serializing_if = "BTreeMap::is_empty")]
    conditional_deps: BTreeMap<String, ConditionalTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<ActiveSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    llm: Option<LlmSection>,
    #[serde(default, rename = "registry", skip_serializing_if = "Vec::is_empty")]
    registries: Vec<RegistrySection>,
    #[serde(default, rename = "mirror", skip_serializing_if = "Vec::is_empty")]
    mirrors: Vec<MirrorSection>,
    #[serde(default, rename = "override", skip_serializing_if = "Option::is_none")]
    override_wire: Option<OverrideWire>,
    #[serde(default, skip_serializing_if = "visibility_is_empty")]
    visibility: Option<VisibilityMeta>,
    #[serde(default, skip_serializing_if = "I18nDecl::is_default")]
    i18n: I18nDecl,
    #[serde(default, skip_serializing_if = "BootSection::is_empty")]
    boot: BootSection,
}

impl TryFrom<ManifestWire> for Manifest {
    type Error = String;

    fn try_from(wire: ManifestWire) -> Result<Self, Self::Error> {
        if let Some(meta) = &wire.visibility {
            validate_visibility(meta)?;
        }
        let (overrides, override_table) = match wire.override_wire {
            Some(OverrideWire::Registry(entries)) => (entries, None),
            Some(OverrideWire::Visibility(table)) => {
                table.targets()?;
                (Vec::new(), Some(table))
            }
            None => (Vec::new(), None),
        };
        let extensions = wire
            .extensions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            project: wire.project,
            package: wire.package,
            workspace: wire.workspace,
            origin: wire.origin,
            requires: wire.requires,
            requires_any: wire.requires_any,
            provides: wire.provides,
            obsoletes: wire.obsoletes,
            conflicts: wire.conflicts,
            recommends: wire.recommends,
            suggests: wire.suggests,
            skills: wire.skills,
            binaries: wire.binaries,
            mcp_servers: wire.mcp_servers,
            hooks: wire.hooks,
            extensions,
            extension_controls: wire.extension_controls.into(),
            compatibility: wire.compatibility,
            boot_snippet: wire.boot_snippet,
            features: wire.features,
            conditional_deps: wire.conditional_deps,
            active: wire.active,
            llm: wire.llm,
            registries: wire.registries,
            mirrors: wire.mirrors,
            overrides,
            visibility: wire.visibility,
            override_table,
            i18n: wire.i18n,
            boot: wire.boot,
        })
    }
}

impl TryFrom<Manifest> for ManifestWire {
    type Error = String;

    fn try_from(manifest: Manifest) -> Result<Self, Self::Error> {
        let has_project = manifest.project.is_some();
        let has_package = manifest.package.is_some();
        let override_wire = match (manifest.overrides.is_empty(), manifest.override_table) {
            (false, Some(_)) => {
                return Err("manifest cannot serialize registry [[override]] and visibility [override] together".into());
            }
            (false, None) => Some(OverrideWire::Registry(manifest.overrides)),
            (true, Some(table)) => Some(OverrideWire::Visibility(table)),
            (true, None) => None,
        };
        validate_extension_declarations(&manifest.extensions, has_project, has_package)?;
        let extensions = manifest
            .extensions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            project: manifest.project,
            package: manifest.package,
            workspace: manifest.workspace,
            origin: manifest.origin,
            requires: manifest.requires,
            requires_any: manifest.requires_any,
            provides: manifest.provides,
            obsoletes: manifest.obsoletes,
            conflicts: manifest.conflicts,
            recommends: manifest.recommends,
            suggests: manifest.suggests,
            skills: manifest.skills,
            binaries: manifest.binaries,
            mcp_servers: manifest.mcp_servers,
            hooks: manifest.hooks,
            extensions,
            extension_controls: manifest.extension_controls.into(),
            compatibility: manifest.compatibility,
            boot_snippet: manifest.boot_snippet,
            features: manifest.features,
            conditional_deps: manifest.conditional_deps,
            active: manifest.active,
            llm: manifest.llm,
            registries: manifest.registries,
            mirrors: manifest.mirrors,
            override_wire,
            visibility: manifest.visibility,
            i18n: manifest.i18n,
            boot: manifest.boot,
        })
    }
}
