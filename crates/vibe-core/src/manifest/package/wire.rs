//! Wire (serde) intermediates for `[requires]` — the TOML table form of
//! PROP-002 §2.4.1 / PROP-007 §2.6 / PROP-009 §2.4, reached only through
//! `Requires`'s `Serialize` / `Deserialize` (`into` / `try_from`).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#git-source");

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::capability_ref::CapabilityRef;
use crate::error::{Error, Result};
use crate::manifest::SpecFormat;
use crate::manifest::project::AuthKind;
use crate::manifest::purl::Purl;
use crate::package_ref::{Group, PackageKind, PackageRef, VersionSpec};

use super::capabilities::{AccessLevel, Requires, link_key};
use super::deps::{inline_to_git_dep, inline_to_path_dep, inline_to_var_dep};
use super::{
    Authorship, GitPackageDep, GitRefKind, LinkType, Materialization, PackageFormat, PackageMeta,
    PathPackageDep, PublishPosture, VarRegistryDep, is_false,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageMetaWire {
    name: String,
    group: Group,
    kind: PackageKind,
    version: semver::Version,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    epoch: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spec_format: Option<SpecFormat>,
    #[serde(default, skip_serializing_if = "is_false")]
    frozen: bool,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, rename = "abstract", skip_serializing_if = "Option::is_none")]
    abstract_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authorship: Option<Authorship>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    describes: Option<Purl>,
    #[serde(default, skip_serializing_if = "PublishPosture::is_default")]
    publish: PublishPosture,
    #[serde(default, skip_serializing_if = "Materialization::is_default")]
    materialization: Materialization,
    #[serde(default, skip_serializing_if = "is_false")]
    bridge: bool,
    #[serde(default, skip_serializing_if = "PackageFormat::is_default")]
    format: PackageFormat,
}

impl Serialize for PackageMeta {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PackageMetaWire::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PackageMeta {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PackageMetaWire::deserialize(deserializer)?.into())
    }
}

impl From<PackageMeta> for PackageMetaWire {
    fn from(value: PackageMeta) -> Self {
        Self {
            name: value.name,
            group: value.group,
            kind: value.kind,
            version: value.version,
            epoch: value.epoch,
            spec_format: value.spec_format,
            frozen: value.frozen,
            authors: value.authors,
            license: value.license,
            description: value.description,
            title: value.title,
            abstract_text: value.abstract_text,
            authorship: value.authorship,
            lang: value.lang,
            homepage: value.homepage,
            keywords: value.keywords,
            describes: value.describes,
            publish: value.publish,
            materialization: value.materialization,
            bridge: value.bridge,
            format: value.format,
        }
    }
}

impl From<PackageMetaWire> for PackageMeta {
    fn from(value: PackageMetaWire) -> Self {
        Self {
            name: value.name,
            group: value.group,
            kind: value.kind,
            version: value.version,
            epoch: value.epoch,
            spec_format: value.spec_format,
            frozen: value.frozen,
            authors: value.authors,
            license: value.license,
            description: value.description,
            title: value.title,
            abstract_text: value.abstract_text,
            authorship: value.authorship,
            lang: value.lang,
            homepage: value.homepage,
            keywords: value.keywords,
            describes: value.describes,
            publish: value.publish,
            materialization: value.materialization,
            bridge: value.bridge,
            format: value.format,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::manifest::package) struct RequiresWire {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    packages: BTreeMap<String, RequiresPackageEntryWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<CapabilityRef>,
    /// `[requires.remaps]` — a pkgref key per remap package, with the
    /// bare constraint string every such requirement takes. A remap
    /// package carries no features, no link type and no visibility edge,
    /// so the inline form has nothing to say here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    remaps: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum RequiresPackageEntryWire {
    /// Bare constraint string: `"^0.3"`, `"=1.0"`, `"*"`.
    Constraint(String),
    /// Inline-table: registry-resolved with options OR git-source.
    Inline(InlinePackageDepWire),
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(in crate::manifest::package) struct InlinePackageDepWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) version: Option<VersionFieldWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) git: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) auth: Option<AuthKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::manifest::package) token_env: Option<String>,
    /// Inclusion type (PROP-009 §2.4). Valid on every source kind; lifted
    /// into `Requires::links` by the `TryFrom` conversion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link: Option<LinkType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    access: Option<AccessLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    friend: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
}

/// The `version` field of an inline `[requires.packages]` entry — either a
/// concrete constraint string or a `[workspace.versions]` placeholder
/// reference (`version.var = "core"`).
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::manifest::package) enum VersionFieldWire {
    /// `version = "^0.3"` — a concrete constraint.
    Constraint(String),
    /// `version.var = "core"` — a `[workspace.versions]` placeholder.
    Var { var: String },
}

mod requires;
