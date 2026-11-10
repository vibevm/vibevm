//! `[[application.ecosystem]]` — the adapter by which a runtime package
//! installs, lists, removes and self-upgrades the packages of its **own**
//! ecosystem (PROP-075 `#ECOSYSTEM-ADAPTER`, `#ECOSYSTEM-RECOMMENDS`,
//! `#ECOSYSTEM-MIGRATION`).
//!
//! Nothing in this module knows one ecosystem word. An `id` is an opaque
//! token vibe only ever echoes, and an `item` is an opaque string vibe
//! never parses, splits, quotes, escapes, validates or reorders: the
//! items reach the `install` and `remove` verbs as **separate argv
//! elements** appended to the step's own argv and are never interpolated
//! into a command string, which is why those two verbs must declare
//! `kind = "recipe"` — a `recipe` step is a script invoked with
//! arguments, while a `shell` step is a command *string* into which a
//! list could only be spliced.
//!
//! Each verb is exactly one [`StepDecl`] of the existing grammar,
//! restricted to the kinds `recipe` and `shell` for the reason
//! `#MIGRATION-DECLARATION` gives a migration side: an adapter runs a
//! command inside an installed runtime and must not install, download,
//! delegate or change the user's environment.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-MIGRATION");

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{ApplicationDecl, FeatureDecl, FeatureOrigin, MigrationDecl, StepDecl};

/// The reserved id namespace of a synthesised migration entry
/// (`#ECOSYSTEM-MIGRATION`). An authored `[[application.migration]]` id in
/// this namespace is a validation refusal, so a synthesised entry can
/// never be shadowed.
pub const ECOSYSTEM_MIGRATION_PREFIX: &str = "ecosystem:";

/// One of the four verbs of an adapter. `install` is required; the other
/// three are optional, and a command that asks for an undeclared verb is
/// told which the package declares.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EcosystemVerb {
    Install,
    List,
    Remove,
    SelfUpgrade,
}

impl EcosystemVerb {
    /// The stable spelling, which is also the TOML key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::List => "list",
            Self::Remove => "remove",
            Self::SelfUpgrade => "self_upgrade",
        }
    }

    /// Whether the verb is handed the item list as argv. Exactly the two
    /// verbs that must be `recipe` steps.
    pub fn takes_items(self) -> bool {
        matches!(self, Self::Install | Self::Remove)
    }

    /// Every verb in declaration order.
    pub fn all() -> [Self; 4] {
        [Self::Install, Self::List, Self::Remove, Self::SelfUpgrade]
    }
}

/// One `[[application.ecosystem]]` row.
///
/// ```
/// use vibe_core::manifest::{EcosystemDecl, EcosystemVerb};
///
/// let row: EcosystemDecl = toml::from_str(
///     "id = \"npm\"\nname = \"npm packages\"\n\
///      [install]\nid = \"install\"\ntitle = \"Install them\"\nkind = \"recipe\"\nscript = \"eco/install.sh\"\n",
/// )
/// .unwrap();
/// assert_eq!(row.id, "npm");
/// assert!(row.migration);
/// assert_eq!(row.verb(EcosystemVerb::Install).unwrap().kind.name(), "recipe");
/// assert!(row.verb(EcosystemVerb::List).is_none());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemDecl {
    /// A stable token, **opaque to vibe**: never interpreted, only echoed.
    pub id: String,
    /// What the ecosystem is called, in one human phrase.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether vibe synthesises the `ecosystem:<id>` migration entry from
    /// this adapter's `list` and `install` verbs (`#ECOSYSTEM-MIGRATION`).
    /// `false` for a runtime whose ecosystem state should not travel
    /// between versions.
    #[serde(default = "migration_default", skip_serializing_if = "is_default")]
    pub migration: bool,
    /// The required verb: install the items it is handed as argv.
    pub install: StepDecl,
    /// Observe what one installed version carries; its answer is a file,
    /// never stdout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<StepDecl>,
    /// Uninstall the items it is handed as argv.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<StepDecl>,
    /// Upgrade the bundled manager itself; takes no items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_upgrade: Option<StepDecl>,
    /// `[[application.ecosystem.recommends]]` — the author's recommended
    /// items, each shorthand for one installer feature
    /// (`#ECOSYSTEM-RECOMMENDS`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommends: Vec<EcosystemRecommendDecl>,
}

fn migration_default() -> bool {
    true
}

fn is_default(value: &bool) -> bool {
    *value
}

impl EcosystemDecl {
    /// The declaration of one verb, or `None` when the package declares
    /// it not.
    pub fn verb(&self, verb: EcosystemVerb) -> Option<&StepDecl> {
        match verb {
            EcosystemVerb::Install => Some(&self.install),
            EcosystemVerb::List => self.list.as_ref(),
            EcosystemVerb::Remove => self.remove.as_ref(),
            EcosystemVerb::SelfUpgrade => self.self_upgrade.as_ref(),
        }
    }

    /// The verbs this adapter declares, in verb order.
    pub fn declared_verbs(&self) -> Vec<EcosystemVerb> {
        EcosystemVerb::all()
            .into_iter()
            .filter(|verb| self.verb(*verb).is_some())
            .collect()
    }

    /// The id of this adapter's synthesised migration entry.
    pub fn migration_id(&self) -> String {
        format!("{ECOSYSTEM_MIGRATION_PREFIX}{}", self.id)
    }

    /// The `[[application.migration]]`-shaped entry this adapter stands
    /// for (`#ECOSYSTEM-MIGRATION`): the `list` verb run by the **old**
    /// version as the export, the `install` verb run by the **new** one as
    /// the import, `default = true`. `None` when the adapter declares no
    /// `list` verb — there is then nothing to observe — or suppresses the
    /// synthesis with `migration = false`.
    pub fn migration_entry(&self) -> Option<MigrationDecl> {
        let list = self.list.as_ref()?;
        self.migration.then(|| MigrationDecl {
            id: self.migration_id(),
            name: self.name.clone(),
            description: self.description.clone(),
            default: true,
            export: list.clone(),
            import: self.install.clone(),
        })
    }
}

/// One `[[application.ecosystem.recommends]]` row: shorthand for one
/// installer feature whose selection adds `item` to the adapter's list.
///
/// ```
/// use vibe_core::manifest::EcosystemRecommendDecl;
///
/// let row: EcosystemRecommendDecl = toml::from_str(
///     "ecosystem = \"npm\"\nid = \"corepack\"\nitem = \"corepack@latest\"\ndefault = true\n",
/// )
/// .unwrap();
/// assert_eq!(row.feature_name(), "corepack@latest");
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcosystemRecommendDecl {
    /// The adapter this item belongs to; an id the package declares.
    pub ecosystem: String,
    /// A stable token, the row's own name and the synthesised feature's id.
    pub id: String,
    /// The opaque item string, in exactly one place in the manifest.
    pub item: String,
    /// Whether the synthesised feature is selected by default.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl EcosystemRecommendDecl {
    /// The human name of the synthesised feature: the row's own `name`,
    /// else its `item` — the item string is what a user recognises.
    pub fn feature_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.item)
    }

    /// The feature row this recommendation is shorthand for. Ordinary in
    /// every respect, which is what keeps the fixed point of PROP-065
    /// `#feature-tree` unchanged for a package that declares no adapter.
    pub fn feature(&self) -> FeatureDecl {
        FeatureDecl {
            id: self.id.clone(),
            name: self.feature_name().to_owned(),
            description: self.reason.clone(),
            default: self.default,
            parent: None,
            required: false,
            requires: Vec::new(),
            exclusive_group: None,
            origin: FeatureOrigin::EcosystemRecommends,
        }
    }
}

impl ApplicationDecl {
    /// One declared adapter by id.
    pub fn ecosystem_adapter(&self, id: &str) -> Option<&EcosystemDecl> {
        self.ecosystem.iter().find(|row| row.id == id)
    }

    /// Every recommendation of the package, in declaration order: the
    /// adapters in their order, each adapter's rows in theirs.
    pub fn ecosystem_recommends(&self) -> impl Iterator<Item = &EcosystemRecommendDecl> {
        self.ecosystem.iter().flat_map(|row| row.recommends.iter())
    }

    /// The recommendations **routed to** the adapter `id`, in the package's
    /// own declaration order.
    ///
    /// A row's `ecosystem` field is what routes its item, not the table it
    /// was authored inside: the fact requires the field and requires an
    /// unknown id to be refused by name, so a row naming a *different*
    /// declared adapter belongs to that one (`#ECOSYSTEM-RECOMMENDS`, and
    /// `#P7B-AS-BUILT` (3), which settled it). Reading
    /// `EcosystemDecl::recommends` directly would therefore send such a
    /// row's item to the wrong `install` verb.
    pub fn ecosystem_recommends_of(&self, id: &str) -> Vec<&EcosystemRecommendDecl> {
        self.ecosystem_recommends()
            .filter(|row| row.ecosystem == id)
            .collect()
    }

    /// The feature rows the recommendations are shorthand for, in
    /// declaration order. They are synthesised into **every** installer
    /// option of the package, because a recommendation is a property of
    /// the package and not of one platform's archive.
    pub fn synthesised_features(&self) -> Vec<FeatureDecl> {
        self.ecosystem_recommends()
            .map(EcosystemRecommendDecl::feature)
            .collect()
    }

    /// The migration entries of this application: the authored ones, then
    /// one synthesised per adapter that declares a `list` verb and does
    /// not suppress it (`#ECOSYSTEM-MIGRATION`).
    pub fn migration_entries(&self) -> Vec<Cow<'_, MigrationDecl>> {
        self.migration
            .iter()
            .map(Cow::Borrowed)
            .chain(
                self.ecosystem
                    .iter()
                    .filter_map(EcosystemDecl::migration_entry)
                    .map(Cow::Owned),
            )
            .collect()
    }
}
