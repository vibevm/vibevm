//! The typed transform-plan value family (R4-TRANSFORM-PLAN-ABI §2):
//! configuration wrapper, stage, provider, implementation, seed, entry and
//! the plan itself.
//!
//! Every member is an owned semantic value. A provider holds exactly the
//! typed identity components digests frame — never a resolved root; an
//! implementation is a builtin candidate whose name/epoch authority is the
//! T5 behavior registry (tests may mint candidates; the workspace adapter
//! never supplies an epoch); a seed carries no order, because
//! [`TransformPlan::build`] is the only authority that assigns dense
//! zero-based order. The whole family stays `pub(crate)` with private
//! fields until T10's workspace adapter becomes the first cross-crate
//! consumer.

use vibe_core::manifest::ExtensionKey;
use vibe_core::{ContentHash, PackageKind};
use vibe_extension_registry::{
    CompiledSelector, DependencyProviderId, ExtensionProvider, HostIdentity,
};

use super::config::{ConfigDigest, ConfigTable};
use super::plan_digest::{ImplementationDigest, PlanDigest};

/// The effective configuration of one transform, wrapped as plan identity.
///
/// `None` at the seed layer means no effective config was authored;
/// `Some(TransformConfig::new(empty_table))` means an authored activation
/// cleared the value — two different identities that stay distinct through
/// the plan digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformConfig(ConfigTable);

impl TransformConfig {
    /// Wrap one already-neutral effective-configuration table.
    pub(crate) fn new(table: ConfigTable) -> Self {
        Self(table)
    }

    /// The wrapped neutral table, lent read-only for digesting.
    pub(crate) fn as_table(&self) -> &ConfigTable {
        &self.0
    }
}

/// The one staged tier a transform runs at (PROP-054 §7.2).
///
/// The closed four-stage set: `compile:source`, `compile:document`,
/// `compile:lane`, `compile:emitted`. Selectors are legal at source/document
/// only; the stage byte in the plan digest is its frozen ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransformStage {
    /// One document's raw text before parsing.
    Source,
    /// One document's parsed tree before it enters the closure.
    Document,
    /// The assembled lane as an ordered node list before serialisation.
    Lane,
    /// The serialised artifact bytes as the last word before writing.
    Emitted,
}

/// The typed provider identity of one transform's declaring owner.
///
/// Opaque: an owned struct over a private kind, holding exactly the semantic
/// components the plan digest frames — a dependency's typed coordinate,
/// exact version, kind and content hash, or a host's typed identity with
/// optional kind/hash. The resolved provider root is deliberately absent —
/// it is filesystem state, not identity, and [`From<&ExtensionProvider>`]
/// drops it on purpose so two materialisations of one package are one
/// provider here. No field or variant is constructible outside this module,
/// so the value stays authored solely by the one root-dropping conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformProvider {
    kind: ProviderKind,
}

/// The private provider discrimination: an enum no other module can name,
/// let alone construct.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderKind {
    /// An installed dependency provider.
    Dependency {
        id: DependencyProviderId,
        version: String,
        kind: PackageKind,
        content_hash: ContentHash,
    },
    /// The selected host (or, for a package's own lane, that package in the
    /// host seat).
    Host {
        id: HostIdentity,
        version: String,
        kind: Option<PackageKind>,
        content_hash: Option<ContentHash>,
    },
}

/// The semantic components of one provider, lent read-only.
///
/// The lending view the digest framer and the refusal law consume: it
/// exposes the typed members without granting field writes, and it keeps
/// the dependency/host discriminant explicit for framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderComponents<'provider> {
    Dependency {
        id: &'provider DependencyProviderId,
        version: &'provider str,
        kind: PackageKind,
        content_hash: &'provider ContentHash,
    },
    Host {
        identity: &'provider HostIdentity,
        version: &'provider str,
        kind: Option<PackageKind>,
        content_hash: Option<&'provider ContentHash>,
    },
}

impl TransformProvider {
    /// The typed semantic members, lent read-only.
    pub(crate) fn components(&self) -> ProviderComponents<'_> {
        match &self.kind {
            ProviderKind::Dependency {
                id,
                version,
                kind,
                content_hash,
            } => ProviderComponents::Dependency {
                id,
                version,
                kind: *kind,
                content_hash,
            },
            ProviderKind::Host {
                id,
                version,
                kind,
                content_hash,
            } => ProviderComponents::Host {
                identity: id,
                version,
                kind: *kind,
                content_hash: content_hash.as_ref(),
            },
        }
    }
}

impl From<&ExtensionProvider> for TransformProvider {
    /// Convert one registry provider into plan identity.
    ///
    /// This is the one conversion across the seam: it clones exactly the
    /// typed semantic members and deliberately drops the resolved `root`,
    /// which is what makes root exclusion executable rather than
    /// aspirational. The workspace adapter (T10) lowers rows through it.
    fn from(provider: &ExtensionProvider) -> Self {
        match provider {
            ExtensionProvider::Dependency(dependency) => Self {
                kind: ProviderKind::Dependency {
                    id: dependency.id.clone(),
                    version: dependency.version.clone(),
                    kind: dependency.kind,
                    content_hash: dependency.content_hash.clone(),
                },
            },
            ExtensionProvider::Host(host) => Self {
                kind: ProviderKind::Host {
                    id: host.identity.clone(),
                    version: host.version.clone(),
                    kind: host.kind,
                    content_hash: host.content_hash.clone(),
                },
            },
        }
    }
}

/// The implementation identity of one transform.
///
/// Opaque: an owned struct over private fields — a builtin's registry-owned
/// candidate name plus a nonzero behavior epoch that moves when observable
/// behavior moves. No field is constructible outside this module; the one
/// candidate constructor is private to the transform module, so tests mint
/// candidates, T5's behavior registry becomes the production authority, and
/// the workspace can never supply an epoch — not today, and not the day a
/// later atom reexports the type. R5 adds the `native` discriminant beside
/// `builtin` as another private field or kind, not a public variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformImplementation {
    name: String,
    epoch: u32,
}

impl TransformImplementation {
    /// A candidate builtin implementation: exact name and behavior epoch.
    ///
    /// A candidate is not yet identity — [`TransformPlan::build`] still
    /// validates the name against the frozen backend-id grammar and refuses
    /// epoch zero, so no invalid internal or test candidate can enter a
    /// plan. Private to the transform module on purpose: widening its
    /// visibility would let the workspace author an epoch directly.
    pub(super) fn builtin_candidate(name: impl Into<String>, epoch: u32) -> Self {
        Self {
            name: name.into(),
            epoch,
        }
    }

    /// The builtin's exact candidate name.
    pub(crate) fn builtin_name(&self) -> &str {
        &self.name
    }

    /// The builtin's registry-owned behavior epoch.
    pub(crate) fn builtin_epoch(&self) -> u32 {
        self.epoch
    }
}

/// One workspace-adapter input: everything a plan entry is made of, except
/// its order.
///
/// The key is the typed `ExtensionKey`; the printable key enters the digest
/// only through `ExtensionKey::as_str()`. The selector is the kernel's
/// compiled positive selector, legal at source/document only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformSeed {
    key: ExtensionKey,
    provider: TransformProvider,
    stage: TransformStage,
    implementation: TransformImplementation,
    config: Option<TransformConfig>,
    selector: Option<CompiledSelector>,
}

impl TransformSeed {
    /// Author one seed from its full semantic membership.
    ///
    /// The six members are exactly the plan-entry semantics minus order;
    /// validation and canonicalization remain [`TransformPlan::build`]'s
    /// job, not the constructor's.
    pub(crate) fn new(
        key: ExtensionKey,
        provider: TransformProvider,
        stage: TransformStage,
        implementation: TransformImplementation,
        config: Option<TransformConfig>,
        selector: Option<CompiledSelector>,
    ) -> Self {
        Self {
            key,
            provider,
            stage,
            implementation,
            config,
            selector,
        }
    }

    /// The typed declaration key.
    pub(crate) fn key(&self) -> &ExtensionKey {
        &self.key
    }

    /// The typed provider identity.
    pub(crate) fn provider(&self) -> &TransformProvider {
        &self.provider
    }

    /// The staged tier.
    pub(crate) fn stage(&self) -> &TransformStage {
        &self.stage
    }

    /// The implementation identity.
    pub(crate) fn implementation(&self) -> &TransformImplementation {
        &self.implementation
    }

    /// The effective configuration, absent until authored.
    pub(crate) fn config(&self) -> Option<&TransformConfig> {
        self.config.as_ref()
    }

    /// The compiled positive selector, absent when none was authored (or
    /// when a behaviorally unscoped one was canonicalized away).
    pub(crate) fn selector(&self) -> Option<&CompiledSelector> {
        self.selector.as_ref()
    }

    /// Canonicalize the selector for identity: at source/document, a
    /// supplied selector whose two raw dimension accessors are both absent
    /// becomes outer absence (`applies_to` absent and `applies_to = {}`
    /// share one behavioral identity); any present dimension — including a
    /// present empty one — keeps outer presence. Lane/emitted seeds never
    /// reach here with a selector: the refusal law rejects them first.
    fn canonicalized(self) -> Self {
        let selector = match self.selector {
            Some(selector)
                if selector.package_patterns().is_none() && selector.path_patterns().is_none() =>
            {
                None
            }
            other => other,
        };
        Self { selector, ..self }
    }
}

/// One planned transform: its seed, its assigned order, and the child
/// digests the plan digest frames.
///
/// The order is assigned by [`TransformPlan::build`] from the effective-row
/// sequence — dense, zero-based, never caller-authored. The implementation
/// digest always exists (every seed carries an implementation); the config
/// digest exists exactly when effective config was authored, preserving
/// `None` versus authored-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformEntry {
    seed: TransformSeed,
    order: u32,
    config_digest: Option<ConfigDigest>,
    implementation_digest: ImplementationDigest,
}

impl TransformEntry {
    /// The entry's full seed semantics.
    pub(crate) fn seed(&self) -> &TransformSeed {
        &self.seed
    }

    /// The dense zero-based order assigned at build.
    pub(crate) fn order(&self) -> u32 {
        self.order
    }

    /// The config digest, present exactly when config was authored.
    pub(crate) fn config_digest(&self) -> Option<&ConfigDigest> {
        self.config_digest.as_ref()
    }

    /// The implementation digest (always present).
    pub(crate) fn implementation_digest(&self) -> &ImplementationDigest {
        &self.implementation_digest
    }
}

/// One owner-scoped transform plan: the typed, canonically digested value a
/// lane's effective registry rows lower into.
///
/// Built only through [`TransformPlan::build`], which owns the refusal law
/// and the dense order assignment. An empty plan owns no entries, no
/// allocation and no digest — appending it to a schedule must reproduce the
/// exact historical bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformPlan {
    entries: Vec<TransformEntry>,
    digest: Option<PlanDigest>,
}

impl TransformPlan {
    /// The empty plan: no entries, no allocation, no digest.
    pub(crate) const fn empty() -> Self {
        Self {
            entries: Vec::new(),
            digest: None,
        }
    }

    /// Build a plan from the effective-row sequence, or refuse it.
    ///
    /// An empty sequence canonicalizes to [`TransformPlan::empty`]. A
    /// nonempty one is validated whole under the frozen precedence (checked
    /// entry count, then each seed in input order: key scalar, duplicate
    /// key, provider version/host scalar and content-hash recheck,
    /// implementation name/epoch, selector/stage), then each seed is
    /// canonicalized, assigned its dense order, and digested; the plan
    /// digest covers every entry in effective order.
    pub(crate) fn build(
        seeds: Vec<TransformSeed>,
    ) -> Result<Self, super::plan_validate::TransformPlanError> {
        if seeds.is_empty() {
            return Ok(Self::empty());
        }
        super::plan_validate::checked_entry_count(seeds.len())?;
        super::plan_validate::validate_seeds(&seeds)?;
        let mut entries = Vec::with_capacity(seeds.len());
        for (index, seed) in seeds.into_iter().enumerate() {
            let seed = seed.canonicalized();
            let implementation_digest =
                super::plan_digest::implementation_digest(seed.implementation());
            let config_digest = seed
                .config()
                .map(|config| super::config::config_digest(config.as_table()));
            entries.push(TransformEntry {
                seed,
                order: index as u32,
                config_digest,
                implementation_digest,
            });
        }
        let digest = Some(super::plan_digest::plan_digest(&entries));
        Ok(Self { entries, digest })
    }

    /// The plan entries in effective order.
    pub(crate) fn entries(&self) -> &[TransformEntry] {
        &self.entries
    }

    /// The canonical plan digest; `None` exactly when the plan is empty.
    pub(crate) fn digest(&self) -> Option<PlanDigest> {
        self.digest
    }

    /// The number of planned transforms.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no transform is planned.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry vector's capacity — a test-only observation of the
    /// empty-plan law (`empty()` owns no allocation). Allocation capacity is
    /// not production domain API, so the accessor never compiles into a
    /// non-test build.
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.entries.capacity()
    }
}
