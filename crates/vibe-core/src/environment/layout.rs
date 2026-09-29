//! Five injected root roles; layout descriptions never discover or admit roots.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT");
use super::*;
use crate::layout as names;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootRole {
    Configuration,
    ImmutableStore,
    Profile,
    MutableState,
    DisposableCache,
}
impl RootRole {
    pub const ALL: [Self; 5] = [
        Self::Configuration,
        Self::ImmutableStore,
        Self::Profile,
        Self::MutableState,
        Self::DisposableCache,
    ];
    fn index(self) -> usize {
        match self {
            Self::Configuration => 0,
            Self::ImmutableStore => 1,
            Self::Profile => 2,
            Self::MutableState => 3,
            Self::DisposableCache => 4,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootBindingReferences {
    pub binding: Reference,
    pub producer: Reference,
    pub physical_evidence: Option<Reference>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimVerification {
    Unverified,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredRootBinding {
    instance: TargetInstanceId,
    role: RootRole,
    references: RootBindingReferences,
    principal: Reference,
    path: AbsoluteTargetPath,
}
impl DeclaredRootBinding {
    pub fn new(
        instance: TargetInstanceId,
        role: RootRole,
        references: RootBindingReferences,
        principal: Reference,
        path: AbsoluteTargetPath,
    ) -> Self {
        Self {
            instance,
            role,
            references,
            principal,
            path,
        }
    }
    pub fn instance(&self) -> &TargetInstanceId {
        &self.instance
    }
    pub fn role(&self) -> RootRole {
        self.role
    }
    pub fn references(&self) -> &RootBindingReferences {
        &self.references
    }
    pub fn principal(&self) -> &Reference {
        &self.principal
    }
    pub fn path(&self) -> &AbsoluteTargetPath {
        &self.path
    }
    pub fn evidence_verification(&self) -> ClaimVerification {
        ClaimVerification::Unverified
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootSlot {
    Bound(DeclaredRootBinding),
    MissingRequired,
    Inapplicable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplicitRootChoice {
    Required(Option<DeclaredRootBinding>),
    Inapplicable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSpec {
    pub references: RootBindingReferences,
    pub path: AbsoluteTargetPath,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheSelection {
    Unused,
    Requested(Option<RootSpec>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLayoutInputs {
    pub instance: TargetInstanceId,
    pub principal: Reference,
    pub policy: TargetPathPolicy,
    pub mode: TargetMode,
    pub settings: Option<RootSpec>,
    pub store_references: RootBindingReferences,
    pub state_references: RootBindingReferences,
    pub profile: Option<RootSpec>,
    pub cache: CacheSelection,
    pub home: Option<AbsoluteTargetPath>,
    pub config_file_override: Option<AbsoluteTargetPath>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentLayout {
    instance: TargetInstanceId,
    principal: Reference,
    policy: TargetPathPolicy,
    roots: [RootSlot; 5],
    home: Option<AbsoluteTargetPath>,
    config_file_override: Option<AbsoluteTargetPath>,
}
impl EnvironmentLayout {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT",
        r = 1
    )]
    pub fn explicit(
        instance: TargetInstanceId,
        principal: Reference,
        policy: TargetPathPolicy,
        choices: [ExplicitRootChoice; 5],
        home: Option<AbsoluteTargetPath>,
        config_file_override: Option<AbsoluteTargetPath>,
    ) -> Result<Self, EnvironmentError> {
        let roots = choices.map(|c| match c {
            ExplicitRootChoice::Required(Some(r)) => RootSlot::Bound(r),
            ExplicitRootChoice::Required(None) => RootSlot::MissingRequired,
            ExplicitRootChoice::Inapplicable => RootSlot::Inapplicable,
        });
        for (role, slot) in RootRole::ALL.into_iter().zip(&roots) {
            if let RootSlot::Bound(r) = slot
                && (r.role() != role
                    || r.instance() != &instance
                    || r.principal() != &principal
                    || r.path().policy() != policy)
            {
                return Err(invalid(
                    "layout.roots",
                    "root role, instance, principal or target policy does not match layout",
                ));
            }
        }
        if home
            .as_ref()
            .into_iter()
            .chain(config_file_override.as_ref())
            .any(|p| p.policy() != policy)
        {
            return Err(invalid(
                "layout.paths",
                "home/config override uses a different target policy",
            ));
        }
        Ok(Self {
            instance,
            principal,
            policy,
            roots,
            home,
            config_file_override,
        })
    }
    /// ONE selected defaults table. Binding IDs are supplied, never formed by
    /// hashing/encoding opaque instance text or adding it to a directory path.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT-DEFAULTS",
        r = 1
    )]
    pub fn settings_anchored(input: SettingsLayoutInputs) -> Result<Self, EnvironmentError> {
        let bind = |role: RootRole, spec: RootSpec| {
            DeclaredRootBinding::new(
                input.instance.clone(),
                role,
                spec.references,
                input.principal.clone(),
                spec.path,
            )
        };
        let (config, store, state) = if let Some(s) = &input.settings {
            let store_path = s
                .path
                .appended_component(names::IMMUTABLE_STORE_DIR.as_bytes().to_vec())?;
            let state_path = s
                .path
                .appended_component(names::MUTABLE_STATE_DIR.as_bytes().to_vec())?;
            (
                Some(bind(RootRole::Configuration, s.clone())),
                Some(bind(
                    RootRole::ImmutableStore,
                    RootSpec {
                        references: input.store_references.clone(),
                        path: store_path,
                    },
                )),
                Some(bind(
                    RootRole::MutableState,
                    RootSpec {
                        references: input.state_references.clone(),
                        path: state_path,
                    },
                )),
            )
        } else {
            (None, None, None)
        };
        let profile = if input.mode == TargetMode::WorkspaceProjection {
            if input.profile.is_some() {
                return Err(invalid(
                    "layout.profile",
                    "workspace settings-table profile is inapplicable",
                ));
            }
            ExplicitRootChoice::Inapplicable
        } else {
            ExplicitRootChoice::Required(input.profile.clone().map(|s| bind(RootRole::Profile, s)))
        };
        let cache = match input.cache.clone() {
            CacheSelection::Unused => ExplicitRootChoice::Inapplicable,
            CacheSelection::Requested(spec) => {
                ExplicitRootChoice::Required(spec.map(|s| bind(RootRole::DisposableCache, s)))
            }
        };
        Self::explicit(
            input.instance,
            input.principal,
            input.policy,
            [
                ExplicitRootChoice::Required(config),
                ExplicitRootChoice::Required(store),
                profile,
                ExplicitRootChoice::Required(state),
                cache,
            ],
            input.home,
            input.config_file_override,
        )
    }
    pub fn instance(&self) -> &TargetInstanceId {
        &self.instance
    }
    pub fn principal(&self) -> &Reference {
        &self.principal
    }
    pub fn policy(&self) -> TargetPathPolicy {
        self.policy
    }
    pub fn slot(&self, role: RootRole) -> &RootSlot {
        &self.roots[role.index()]
    }
    pub fn missing_required(&self) -> Vec<RootRole> {
        RootRole::ALL
            .into_iter()
            .filter(|role| matches!(self.slot(*role), RootSlot::MissingRequired))
            .collect()
    }
    pub fn home(&self) -> Option<&AbsoluteTargetPath> {
        self.home.as_ref()
    }
    pub fn config_file_override(&self) -> Option<&AbsoluteTargetPath> {
        self.config_file_override.as_ref()
    }
    /// Checks only config.toml, settings.toml, registry.toml and the supplied
    /// override. It does not cover the whole configuration root or arbitrary
    /// credentials. Even LexicallySeparate is not physical isolation.
    pub fn protected_configuration_paths_cache_relation(
        &self,
        comparison: CaseComparison,
    ) -> ConfigurationCacheRelation {
        let cache = match self.slot(RootRole::DisposableCache) {
            RootSlot::Inapplicable => return ConfigurationCacheRelation::CacheInapplicable,
            RootSlot::MissingRequired => return ConfigurationCacheRelation::Unresolved,
            RootSlot::Bound(r) => r,
        };
        let RootSlot::Bound(config) = self.slot(RootRole::Configuration) else {
            return ConfigurationCacheRelation::Unresolved;
        };
        let mut protected = Vec::new();
        for name in [
            names::USER_CONFIG_FILE,
            names::PREFERENCES_FILE,
            names::REGISTRY_CONFIG_FILE,
        ] {
            let Ok(path) = config.path().appended_component(name.as_bytes().to_vec()) else {
                return ConfigurationCacheRelation::Unresolved;
            };
            protected.push(path);
        }
        protected.extend(self.config_file_override.clone());
        let mut unknown = false;
        for path in protected {
            match cache.path().lexical_relation(&path, comparison) {
                LexicalRelation::Separate => {}
                LexicalRelation::Unresolved => unknown = true,
                _ => return ConfigurationCacheRelation::LexicalOverlap,
            }
        }
        if unknown {
            ConfigurationCacheRelation::Unresolved
        } else {
            ConfigurationCacheRelation::LexicallySeparate
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationCacheRelation {
    CacheInapplicable,
    LexicallySeparate,
    LexicalOverlap,
    Unresolved,
}
