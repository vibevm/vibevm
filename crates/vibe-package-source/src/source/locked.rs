//! Explicit operation intent and workspace observations for held sources.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
impl InstallResolver {
    /// Permit a deliberate version update while retaining every held source.
    /// ```
    /// # fn update(source: vibe_package_source::InstallResolver) {
    /// let _same_sources = source.with_locked_updates();
    /// # }
    /// ```
    pub fn with_locked_updates(self) -> Self {
        match self {
            Self::Multi(held, solver) => Self::Multi(Box::new(held.with_locked_updates()), solver),
            Self::LockedLocal {
                local,
                held,
                solver,
            } => Self::LockedLocal {
                local,
                held: Box::new(held.with_locked_updates()),
                solver,
            },
            Self::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                short_circuit,
                solver,
            } => Self::Embedded {
                locals,
                project_local_count,
                declared: declared.map(|held| Box::new(held.with_locked_updates())),
                precedence,
                short_circuit,
                solver,
            },
            other => other,
        }
    }

    /// Choose the named held coordinates' sources again through the current
    /// registry configuration (`vibe update --reselect-source`).
    /// ```
    /// # fn update(source: vibe_package_source::InstallResolver) {
    /// let _source = source.with_reselected_sources(&["org.example/demo".to_string()]);
    /// # }
    /// ```
    pub fn with_reselected_sources(self, keys: &[String]) -> Self {
        match self {
            Self::Multi(held, solver) => Self::Multi(
                Box::new(held.with_reselected_sources(keys.iter().cloned())),
                solver,
            ),
            Self::LockedLocal {
                local,
                held,
                solver,
            } => Self::LockedLocal {
                local,
                held: Box::new(held.with_reselected_sources(keys.iter().cloned())),
                solver,
            },
            Self::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                short_circuit,
                solver,
            } => Self::Embedded {
                locals,
                project_local_count,
                declared: declared
                    .map(|held| Box::new(held.with_reselected_sources(keys.iter().cloned()))),
                precedence,
                short_circuit,
                solver,
            },
            other => other,
        }
    }

    /// Held replay acquires from the locked source, never a store entry
    /// (`vibe reinstall --force`).
    /// ```
    /// # fn force(source: vibe_package_source::InstallResolver) {
    /// let _source = source.with_forced_refetch();
    /// # }
    /// ```
    pub fn with_forced_refetch(self) -> Self {
        match self {
            Self::Multi(held, solver) => Self::Multi(Box::new(held.with_forced_refetch()), solver),
            Self::LockedLocal {
                local,
                held,
                solver,
            } => Self::LockedLocal {
                local,
                held: Box::new(held.with_forced_refetch()),
                solver,
            },
            Self::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                short_circuit,
                solver,
            } => Self::Embedded {
                locals,
                project_local_count,
                declared: declared.map(|held| Box::new(held.with_forced_refetch())),
                precedence,
                short_circuit,
                solver,
            },
            other => other,
        }
    }

    /// Bind workspace-owned checkout addresses separately from selected-member paths.
    /// ```
    /// # fn bind(source: vibe_package_source::InstallResolver) {
    /// let _source = source.with_locked_checkouts(std::collections::HashMap::new());
    /// # }
    /// ```
    pub fn with_locked_checkouts(
        self,
        paths: std::collections::HashMap<String, std::path::PathBuf>,
    ) -> Self {
        match self {
            Self::Multi(held, solver) => {
                Self::Multi(Box::new(held.with_locked_checkouts(paths)), solver)
            }
            Self::LockedLocal {
                local,
                held,
                solver,
            } => Self::LockedLocal {
                local,
                held: Box::new(held.with_locked_checkouts(paths)),
                solver,
            },
            Self::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                short_circuit,
                solver,
            } => Self::Embedded {
                locals,
                project_local_count,
                declared: declared.map(|held| Box::new(held.with_locked_checkouts(paths))),
                precedence,
                short_circuit,
                solver,
            },
            other => other,
        }
    }

    /// Refresh the named held coordinates from their locked in-workspace
    /// directory instead of replaying the held hash or a store entry: an
    /// in-workspace `file://` source is a mutable working tree (PROP-011
    /// §2.6, PROP-054 `#MUTABLE-GETS-A-GATE`), so its edit must reach the
    /// slot. The source itself is never reselected.
    /// ```
    /// # fn refresh(source: vibe_package_source::InstallResolver) {
    /// let _source = source.with_mutable_locked_sources(&["org.example/demo".to_string()]);
    /// # }
    /// ```
    pub fn with_mutable_locked_sources(self, keys: &[String]) -> Self {
        self.map_held(|held| held.with_mutable_locked_sources(keys.iter().cloned()))
    }

    fn map_held(
        self,
        apply: impl FnOnce(vibe_registry::MultiRegistryResolver) -> vibe_registry::MultiRegistryResolver,
    ) -> Self {
        match self {
            Self::Multi(held, solver) => Self::Multi(Box::new(apply(*held)), solver),
            Self::LockedLocal {
                local,
                held,
                solver,
            } => Self::LockedLocal {
                local,
                held: Box::new(apply(*held)),
                solver,
            },
            Self::Embedded {
                locals,
                project_local_count,
                declared,
                precedence,
                short_circuit,
                solver,
            } => Self::Embedded {
                locals,
                project_local_count,
                declared: declared.map(|held| Box::new(apply(*held))),
                precedence,
                short_circuit,
                solver,
            },
            other => other,
        }
    }
}
