//! Owned effective-world inputs and the retained registry model.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{ExtensionConfig, ExtensionDecl, ExtensionsControl};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};

use super::collect::CollectionNotice;
use super::selector::{CompiledSelector, SelectorSubject};

/// Versionless typed identity of an installed dependency provider.
///
/// The printable extension key is deliberately not parsed back into this
/// type. Adapters construct it from already validated lockfile coordinates.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyProviderId {
    group: Group,
    name: PackageName,
}

impl DependencyProviderId {
    /// Construct an identity from validated coordinate components.
    #[must_use]
    pub const fn new(group: Group, name: PackageName) -> Self {
        Self { group, name }
    }

    /// The provider's reverse-domain group.
    #[must_use]
    pub const fn group(&self) -> &Group {
        &self.group
    }

    /// The provider's package name.
    #[must_use]
    pub const fn name(&self) -> &PackageName {
        &self.name
    }
}

impl fmt::Display for DependencyProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.group, self.name)
    }
}

/// Identity of the selected host manifest.
///
/// Only a project without a self coordinate uses the reserved `__host__/`
/// prefix. A grouped project and a package-role host share the same typed
/// `group/name` identity; manifest role is cosmetic at this boundary.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#HOST-ACTIVATION")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostIdentity {
    /// A project with no group, named exactly as authored.
    UngroupedProject(String),
    /// A grouped project or package-role host with a validated coordinate.
    Coordinate(DependencyProviderId),
    /// A coordinator that may control dependencies but cannot declare one.
    VirtualWorkspace,
}

impl HostIdentity {
    /// Construct a project identity without normalising its authored name.
    #[must_use]
    pub fn ungrouped_project(name: impl Into<String>) -> Self {
        Self::UngroupedProject(name.into())
    }

    /// Construct a grouped project or package-role host identity.
    #[must_use]
    pub const fn coordinate(id: DependencyProviderId) -> Self {
        Self::Coordinate(id)
    }

    /// Construct a pure virtual workspace identity.
    #[must_use]
    pub const fn virtual_workspace() -> Self {
        Self::VirtualWorkspace
    }
}

impl fmt::Display for HostIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UngroupedProject(name) => write!(formatter, "__host__/{name}"),
            Self::Coordinate(id) => id.fmt(formatter),
            Self::VirtualWorkspace => formatter.write_str("<virtual-workspace>"),
        }
    }
}

/// Metadata carried with every installed dependency's declarations.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyProvider {
    /// Versionless typed provider identity.
    pub id: DependencyProviderId,
    /// Already resolved provider root; collection never reads it.
    pub root: PathBuf,
    /// Exact installed version spelling.
    pub version: String,
    /// Installed package kind.
    pub kind: PackageKind,
    /// Installed content identity.
    pub content_hash: ContentHash,
}

/// Metadata carried with the selected host's direct declarations.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProvider {
    /// Project or package-role host identity.
    pub identity: HostIdentity,
    /// Already resolved host root; collection never reads it.
    pub root: PathBuf,
    /// Exact host version spelling.
    pub version: String,
    /// Package kind for a package-role host, absent for a plain project.
    pub kind: Option<PackageKind>,
    /// Optional precomputed host content identity.
    pub content_hash: Option<ContentHash>,
}

/// One lock-ordered dependency row supplied to the pure collector.
///
/// Consumer controls are intentionally absent: installed packages cannot
/// activate or disable contributions in the selected host's effective world.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyExtensionSource {
    /// Provider metadata retained for later dispatch and observability.
    pub provider: DependencyProvider,
    /// Manifest declaration order, already parsed and validated upstream.
    pub declarations: Vec<ExtensionDecl>,
}

/// The selected host's declarations and its sole consumer-control surface.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostExtensionSource {
    /// Host metadata retained for later dispatch and observability.
    pub provider: HostProvider,
    /// Direct host declarations in manifest array order.
    pub declarations: Vec<ExtensionDecl>,
    /// Ordered activations plus exact disable keys authored by the host.
    pub controls: ExtensionsControl,
}

/// Effective role of the selected manifest for reporting and future preset
/// metadata. This is derived from the existing role tables, never authored as
/// another manifest field.
///
/// ```
/// use vibe_lifecycle::EffectiveManifestKind;
///
/// assert_ne!(
///     EffectiveManifestKind::Project,
///     EffectiveManifestKind::VirtualWorkspace,
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveManifestKind {
    Project,
    Package(PackageKind),
    VirtualWorkspace,
}

/// Complete owned input to pure extension collection.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionWorld {
    /// Installed providers in canonical lockfile order.
    pub installed: Vec<DependencyExtensionSource>,
    /// The one selected host effective world.
    pub host: HostExtensionSource,
    /// Effective stack coordinate selected upstream, if any.
    pub effective_stack: Option<DependencyProviderId>,
}

/// One algorithmic binding injected ahead of authored contributions. Its key
/// is reserved by vibe, while provider attribution remains the package that
/// declared the underlying capability.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
#[derive(Debug, Clone)]
pub struct SyntheticPresetSource {
    pub key: vibe_core::manifest::ExtensionKey,
    pub provider: ExtensionProvider,
    pub declaration: ExtensionDecl,
}

/// Provider metadata retained beside a registry declaration.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionProvider {
    /// A package from the installed lock-ordered world.
    Dependency(DependencyProvider),
    /// The selected host manifest.
    Host(HostProvider),
}

impl ExtensionProvider {
    /// Whether the declaration is eligible as a `[[extensions.use]]` target.
    #[must_use]
    pub const fn is_dependency(&self) -> bool {
        matches!(self, Self::Dependency(_))
    }
}

impl fmt::Display for ExtensionProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dependency(provider) => provider.id.fmt(formatter),
            Self::Host(provider) => provider.identity.fmt(formatter),
        }
    }
}

/// The four fixed effective ordering tiers.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContributionTier {
    /// Active-stack `phase:*` declarations, in lock/declaration order.
    Preset,
    /// Every other installed declaration, in lock/declaration order.
    Dependency,
    /// Direct host declarations, in declaration order.
    HostDeclaration,
    /// Host activations, in `[[extensions.use]]` order.
    HostActivation,
}

/// One retained structural declaration plus its effective control state.
///
/// Disabled, inactive compile, and selector-mismatched declarations remain
/// queryable. Only [`ExtensionRegistry::plan`] filters them from execution.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone)]
pub struct ExtensionRegistryRow {
    pub(super) key: vibe_core::manifest::ExtensionKey,
    pub(super) provider: ExtensionProvider,
    pub(super) declaration: ExtensionDecl,
    pub(super) natural_tier: ContributionTier,
    pub(super) effective_tier: ContributionTier,
    pub(super) provider_ordinal: Option<usize>,
    pub(super) declaration_ordinal: usize,
    pub(super) activation_ordinal: Option<usize>,
    pub(super) active_by_default: bool,
    pub(super) activated: bool,
    pub(super) disabled: bool,
    pub(super) effective_config: Option<ExtensionConfig>,
    pub(super) selector: CompiledSelector,
}

impl ExtensionRegistryRow {
    /// Stable opaque printable declaration identity.
    #[must_use]
    pub const fn key(&self) -> &vibe_core::manifest::ExtensionKey {
        &self.key
    }

    /// Provider metadata carried independently from the opaque key.
    #[must_use]
    pub const fn provider(&self) -> &ExtensionProvider {
        &self.provider
    }

    /// The complete authored declaration, including authored configuration.
    #[must_use]
    pub const fn declaration(&self) -> &ExtensionDecl {
        &self.declaration
    }

    /// Configuration exactly as authored on the declaration.
    #[must_use]
    pub fn authored_config(&self) -> Option<&ExtensionConfig> {
        self.declaration.config.as_ref()
    }

    /// Configuration after whole-value host activation replacement.
    #[must_use]
    pub fn effective_config(&self) -> Option<&ExtensionConfig> {
        self.effective_config.as_ref()
    }

    /// Tier assigned before host controls are applied.
    #[must_use]
    pub const fn natural_tier(&self) -> ContributionTier {
        self.natural_tier
    }

    /// Tier used by the effective plan after activation moves.
    #[must_use]
    pub const fn effective_tier(&self) -> ContributionTier {
        self.effective_tier
    }

    /// Lockfile ordinal for dependencies; absent for the host.
    #[must_use]
    pub const fn provider_ordinal(&self) -> Option<usize> {
        self.provider_ordinal
    }

    /// Declaration ordinal within its provider manifest.
    #[must_use]
    pub const fn declaration_ordinal(&self) -> usize {
        self.declaration_ordinal
    }

    /// `[[extensions.use]]` ordinal when host-activated.
    #[must_use]
    pub const fn activation_ordinal(&self) -> Option<usize> {
        self.activation_ordinal
    }

    /// Whether family policy made this row active before host controls.
    #[must_use]
    pub const fn active_by_default(&self) -> bool {
        self.active_by_default
    }

    /// Whether a host activation targeted this row.
    #[must_use]
    pub const fn is_activated(&self) -> bool {
        self.activated
    }

    /// Whether the exact key appeared in the host disable list.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether controls leave the row active before subject selection.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        (self.active_by_default || self.activated) && !self.disabled
    }
}

/// One all-view row evaluated for a particular selector subject.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone, Copy)]
pub struct RegistryView<'registry> {
    /// Retained declaration and its host-control state.
    pub row: &'registry ExtensionRegistryRow,
    /// Result of the declaration's positive selector for this subject.
    pub selector_matches: bool,
}

impl RegistryView<'_> {
    /// Whether this row belongs to the effective execution plan.
    #[must_use]
    pub const fn is_effective(&self) -> bool {
        self.row.is_enabled() && self.selector_matches
    }
}

/// Retained extension registry and its fixed effective order.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY")]
#[derive(Debug, Clone)]
pub struct ExtensionRegistry {
    pub(super) rows: Vec<ExtensionRegistryRow>,
    pub(super) effective_order: Vec<usize>,
    pub(super) notices: Vec<CollectionNotice>,
}

impl ExtensionRegistry {
    /// Every structural declaration, in collection order and without loss.
    #[must_use]
    pub fn rows(&self) -> &[ExtensionRegistryRow] {
        &self.rows
    }

    /// Non-fatal collection notices, such as ignored compile `auto = true`.
    #[must_use]
    pub fn notices(&self) -> &[CollectionNotice] {
        &self.notices
    }

    /// Evaluate every retained row for a subject without filtering any out.
    #[must_use]
    pub fn all(&self, subject: SelectorSubject<'_>) -> Vec<RegistryView<'_>> {
        self.rows
            .iter()
            .map(|row| RegistryView {
                row,
                selector_matches: row.selector.matches(subject),
            })
            .collect()
    }

    /// Return effective rows at one point in the closed four-tier order.
    ///
    /// The returned references borrow this registry; planning neither consumes
    /// nor rewrites the all-view.
    #[must_use]
    pub fn plan(
        &self,
        point: ExtensionPoint,
        subject: SelectorSubject<'_>,
    ) -> Vec<&ExtensionRegistryRow> {
        self.effective_order
            .iter()
            .map(|index| &self.rows[*index])
            .filter(|row| {
                row.declaration.point == point && row.is_enabled() && row.selector.matches(subject)
            })
            .collect()
    }
}

/// One owned effective contribution, labelled with its lifecycle phase.
///
/// Adapters retain this after the source registry has dropped; dispatch never
/// re-collects or re-sorts it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone)]
pub struct ExecutableContribution {
    /// Canonical lifecycle phase spelling supplied with the planned point.
    pub phase: String,
    /// Complete retained declaration/provider/control row.
    pub row: ExtensionRegistryRow,
}

/// Owned canonical execution plan shared by every future surface adapter.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, Default)]
pub struct ExecutablePlan(Vec<ExecutableContribution>);

impl ExecutablePlan {
    /// Select points in caller-supplied phase order while preserving the
    /// registry's closed tier order within each point.
    #[must_use]
    pub fn from_points<I>(
        registry: &ExtensionRegistry,
        points: I,
        subject: SelectorSubject<'_>,
    ) -> Self
    where
        I: IntoIterator<Item = (String, ExtensionPoint)>,
    {
        let mut rows = Vec::new();
        for (phase, point) in points {
            rows.extend(registry.plan(point, subject).into_iter().map(|row| {
                ExecutableContribution {
                    phase: phase.clone(),
                    row: row.clone(),
                }
            }));
        }
        Self(rows)
    }

    /// Number of contributions selected for one phase spelling.
    #[must_use]
    pub fn count_for(&self, phase: &str) -> usize {
        self.0.iter().filter(|row| row.phase == phase).count()
    }
}

impl Deref for ExecutablePlan {
    type Target = [ExecutableContribution];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
