//! Owned effective-world inputs and the retained registry model.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use std::fmt;
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
            // One host-owner codec, shared with `ExtensionKey::for_host` and
            // the mechanism `ProviderOwner::Host`, so a state key, an
            // activation `ref` and a provider pin agree exactly.
            Self::UngroupedProject(name) => vibe_core::HostOwner::new(name.clone()).fmt(formatter),
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
/// The row retains the package's own parsed consumer controls. They are
/// inert data while the package sits in the installed vector of another
/// owner's selected-host world — only `world.host.controls` act there — and
/// become live controls exactly when [`lane_owner_host`] projects the row
/// into the host seat of the package's own lane.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyExtensionSource {
    /// Provider metadata retained for later dispatch and observability.
    pub provider: DependencyProvider,
    /// Manifest declaration order, already parsed and validated upstream.
    pub declarations: Vec<ExtensionDecl>,
    /// The package's own `[extensions]` controls, retained verbatim.
    pub controls: ExtensionsControl,
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

/// Project a dependency into the host seat of its own lane's world.
///
/// The projection retains the exact provider identity, root, version, kind
/// and content hash: identity becomes [`HostIdentity::Coordinate`], and the
/// kind and content hash that are exact for an installed package become
/// `Some` on the host provider. Declarations and the package's retained
/// [`ExtensionsControl`] carry over verbatim, so the one collector — fed this
/// value as the host of a world whose installed rows are the package's
/// dependency closure — applies those controls to that lane alone. Pure and
/// infallible: no filesystem, parsing, or validation happens here.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
#[must_use]
pub fn lane_owner_host(source: &DependencyExtensionSource) -> HostExtensionSource {
    let DependencyExtensionSource {
        provider,
        declarations,
        controls,
    } = source;
    HostExtensionSource {
        provider: HostProvider {
            identity: HostIdentity::coordinate(provider.id.clone()),
            root: provider.root.clone(),
            version: provider.version.clone(),
            kind: Some(provider.kind),
            content_hash: Some(provider.content_hash.clone()),
        },
        declarations: declarations.clone(),
        controls: controls.clone(),
    }
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

    /// The declaration's compiled positive selector, retained read-only.
    ///
    /// The value is already prevalidated by collection; reading its authored
    /// members borrows them without exposing compiled glob internals.
    #[must_use]
    pub const fn compiled_selector(&self) -> &CompiledSelector {
        &self.selector
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

    /// Enabled rows whose point satisfies one predicate, in the closed
    /// four-tier order: the ONE row-iteration seam every public view shares,
    /// so no query can grow a second ordering. Each public query collects its
    /// own `Vec` exactly once over this iterator, so the legacy plan never
    /// materialises an intermediate one.
    fn enabled_rows_where<'registry>(
        &'registry self,
        accepts: impl Fn(ExtensionPoint) -> bool + 'registry,
    ) -> impl Iterator<Item = &'registry ExtensionRegistryRow> + 'registry {
        self.effective_order
            .iter()
            .map(|index| &self.rows[*index])
            .filter(move |row| accepts(row.declaration.point) && row.is_enabled())
    }

    /// Enabled rows at one point in the closed four-tier order.
    fn enabled_rows_at<'registry>(
        &'registry self,
        point: ExtensionPoint,
    ) -> impl Iterator<Item = &'registry ExtensionRegistryRow> + 'registry {
        self.enabled_rows_where(move |candidate| candidate == point)
    }

    /// Return enabled rows at one point in the closed four-tier order
    /// without evaluating selectors.
    ///
    /// Selector-bearing rows are retained: no document subject exists while
    /// a lane's effective rows are lowered into a plan, and filtering on an
    /// unscoped subject there would silently drop them before any document
    /// does. Disabled and inactive rows stay excluded, exactly as execution
    /// planning excludes them. The returned references borrow this registry.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
    #[must_use]
    pub fn enabled_at(&self, point: ExtensionPoint) -> Vec<&ExtensionRegistryRow> {
        self.enabled_rows_at(point).collect()
    }

    /// Return every enabled `compile:*` row in ONE global effective order.
    ///
    /// The order is the registry's single effective order — the closed
    /// four-tier sequence [`enabled_at`](Self::enabled_at) already reads —
    /// restricted to the compile family and to nothing else. Concatenating
    /// the per-point views instead would fabricate a cross-stage order no
    /// manifest ever authored, and a plan digest over it would bless that
    /// invention; there is exactly one authored order and this is it.
    ///
    /// Membership is the whole `compile` family, `compile:pass` included: a
    /// pass-tier row is a compile-point row today, and routing it separately
    /// is the R6 act that splits the pass tier out of the one lowering. Rows
    /// at every other point stay out. Disabled and inactive rows stay
    /// excluded exactly as `enabled_at` excludes them, while selector-bearing
    /// rows are retained: no document subject exists while a lane's rows are
    /// lowered into a plan, and filtering on an unscoped subject there would
    /// silently drop them before any document does. The returned references
    /// borrow this registry.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
    #[must_use]
    pub fn enabled_compile_rows(&self) -> Vec<&ExtensionRegistryRow> {
        self.enabled_rows_where(|point| matches!(point, ExtensionPoint::Compile(_)))
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
        self.enabled_rows_at(point)
            .filter(|row| row.selector.matches(subject))
            .collect()
    }
}
