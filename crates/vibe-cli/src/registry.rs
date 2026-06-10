//! The cell-selection registry — R-001 ("flag at the seam, never in
//! the veins", GUIDE-RUST §3): the **only** module in the binary
//! allowed to read selection flags and construct seam cells from them.
//! An explicit `match` is chosen over distributed registration
//! deliberately — one `match` is the system's table of contents.
//!
//! Two tiers, never confused: cargo features answer "is the code in
//! the binary"; the runtime flags here answer "is the cell selected".
//!
//! Enforced by `cargo xtask conform-lite` (flag-reads-outside-registry):
//! constructing `NaiveDepSolver` / `LocalRegistryProvider` /
//! `MultiRegistryProvider` anywhere else in `vibe-cli` is a finding.

use vibe_registry::{LocalRegistry, MultiRegistryResolver};
use vibe_resolver::{DepSolver, LocalRegistryProvider, MultiRegistryProvider, NaiveDepSolver};

/// Where a selected value came from. The full chain is
/// CLI > env > project file > built-in (GUIDE-RUST §3); v0 populates
/// the two lanes that exist today and reserves the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    BuiltIn,
    Cli,
}

/// One selected flag value with its provenance recorded. The
/// provenance field is registry data: rendered by diagnostics and the
/// flag-matrix tooling, not consumed on the solve path itself.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Selected {
    pub value: &'static str,
    pub provenance: Provenance,
}

/// The selection flags the registry reads. Built once per command
/// invocation from already-parsed CLI state; nothing else interprets
/// them.
#[derive(Debug, Clone, Copy)]
pub struct SelectionFlags {
    /// `solver` — which `DepSolver` cell solves. Today only `naive`
    /// exists (DBT-0011 tracks the SAT upgrade); the flag is born so
    /// the seam point is real before the second cell lands.
    pub solver: Selected,
    /// `provider` — which `DepProvider` cell feeds the solver:
    /// `local-registry` when `--registry <path>` is given, else
    /// `multi-registry`.
    pub provider: Selected,
}

/// Static registry of the selection flags: name, default, birth,
/// sunset criterion (GUIDE-RUST §3 — "the flag registry is data").
/// Consumed source-level by `cargo xtask conform-lite` and unit tests;
/// the R-060 flag-matrix generator is its Phase 4+ runtime consumer.
#[allow(dead_code)]
pub struct FlagInfo {
    pub name: &'static str,
    pub default: &'static str,
    pub birth: &'static str,
    pub sunset: &'static str,
}

#[allow(dead_code)]
pub const FLAGS: &[FlagInfo] = &[
    FlagInfo {
        name: "solver",
        default: "naive",
        birth: "2026-06-10",
        sunset: "when SatDepSolver lands and survives its oracle window, \
                 naive demotes to the explicit fallback (PROP-003 §2.1)",
    },
    FlagInfo {
        name: "provider",
        default: "multi-registry",
        birth: "2026-06-10",
        sunset: "none — the two provider cells are both permanent \
                 (--registry <path> vs configured registries)",
    },
];

/// Interpret the parsed CLI state into selection flags. The only
/// place flag values are decided.
pub fn selection_flags(registry_path_given: bool) -> SelectionFlags {
    SelectionFlags {
        solver: Selected {
            value: "naive",
            provenance: Provenance::BuiltIn,
        },
        provider: if registry_path_given {
            Selected {
                value: "local-registry",
                provenance: Provenance::Cli,
            }
        } else {
            Selected {
                value: "multi-registry",
                provenance: Provenance::BuiltIn,
            }
        },
    }
}

/// The provider resource matching the selected `provider` cell. The
/// caller owns the underlying registry value; the registry module owns
/// the cell choice.
pub enum ProviderResource<'a> {
    Local(&'a LocalRegistry),
    Multi(&'a MultiRegistryResolver),
}

/// Construct the selected `DepSolver` cell over the selected
/// `DepProvider` cell — the single seam-construction point.
pub fn dep_solver<'a>(
    flags: &SelectionFlags,
    resource: ProviderResource<'a>,
) -> Box<dyn DepSolver + 'a> {
    // recorded provenance: flags.solver / flags.provider carry it.
    match (flags.solver.value, flags.provider.value, resource) {
        ("naive", "local-registry", ProviderResource::Local(r)) => {
            Box::new(NaiveDepSolver::new(LocalRegistryProvider::new(r)))
        }
        ("naive", "multi-registry", ProviderResource::Multi(m)) => {
            Box::new(NaiveDepSolver::new(MultiRegistryProvider::new(m)))
        }
        (solver, provider, _) => unreachable!(
            "selection_flags is the only producer of flag values and never \
             emits solver `{solver}` / provider `{provider}` with a \
             mismatched resource"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_flag_follows_registry_path() {
        let local = selection_flags(true);
        assert_eq!(local.provider.value, "local-registry");
        assert_eq!(local.provider.provenance, Provenance::Cli);

        let multi = selection_flags(false);
        assert_eq!(multi.provider.value, "multi-registry");
        assert_eq!(multi.provider.provenance, Provenance::BuiltIn);
        assert_eq!(multi.solver.value, "naive");
        assert_eq!(multi.solver.provenance, Provenance::BuiltIn);
    }

    #[test]
    fn flag_table_covers_every_selection_field() {
        let names: Vec<&str> = FLAGS.iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["solver", "provider"]);
        assert!(FLAGS.iter().all(|f| !f.sunset.is_empty()));
    }
}
