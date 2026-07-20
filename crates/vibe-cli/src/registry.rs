//! The cell-selection registry — R-001 ("flag at the seam, never in
//! the veins", GUIDE-RUST §3): the **only** module in the binary
//! allowed to read selection flags and construct seam cells from them.
//! An explicit `match` is chosen over distributed registration
//! deliberately — one `match` is the system's table of contents.
//!
//! Two tiers, never confused: cargo features answer "is the code in
//! the binary"; the runtime flags here answer "is the cell selected".
//!
//! Enforced by `cargo xtask conform check` (R-001): constructing any
//! `#[cell]`-manifested type (`NaiveDepSolver`, the provider pair,
//! `LocalRegistry`, …) anywhere else in `vibe-cli` is a finding.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order");

use std::path::PathBuf;

use vibe_publish::DirectGitCreator;
use vibe_registry::{LocalRegistry, MultiRegistryResolver, RegistryError};
use vibe_resolver::sat::Sat;
use vibe_resolver::{
    DepSolver, EmbeddedPrecedence, EmbeddedProvider, LocalRegistryProvider, MultiRegistryProvider,
    NaiveDepSolver, ResolvoDepSolver,
};

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
    /// `solver` — which `DepSolver` cell solves. `resolvo` (CDCL SAT,
    /// PROP-017) is the default since 2026-06-14; `naive` and `sat`
    /// remain selectable fallbacks. The flag is the seam point.
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
        default: "resolvo",
        birth: "2026-06-10",
        sunset: "none — resolvo is the default since 2026-06-14 (PROP-017, \
                 it dominates naive on the differential oracle); naive and \
                 sat stay as selectable fallbacks via the `solver` flag",
    },
    FlagInfo {
        name: "provider",
        default: "multi-registry",
        birth: "2026-06-10",
        sunset: "none — the three provider cells are all permanent \
                 (--registry <path> vs configured registries vs the \
                 embedded source-install registry, PROP-030)",
    },
];

/// Interpret the parsed CLI state into selection flags. The only
/// place flag values are decided.
pub fn selection_flags(
    provider: ProviderCell,
    solver_override: Option<&'static str>,
) -> SelectionFlags {
    let provider = match provider {
        // `--registry <path>` is an explicit operator choice.
        ProviderCell::Local => Selected {
            value: "local-registry",
            provenance: Provenance::Cli,
        },
        ProviderCell::Multi => Selected {
            value: "multi-registry",
            provenance: Provenance::BuiltIn,
        },
        // Ambient default derived from a source install (PROP-030); an
        // explicit `--prefer-embedded` re-stamps this as Cli in a later slice.
        ProviderCell::Embedded => Selected {
            value: "embedded",
            provenance: Provenance::BuiltIn,
        },
    };
    SelectionFlags {
        solver: Selected {
            value: solver_override.unwrap_or("resolvo"),
            provenance: if solver_override.is_some() {
                Provenance::Cli
            } else {
                Provenance::BuiltIn
            },
        },
        provider,
    }
}

/// Which DepProvider cell an install invocation selected — decided by the
/// resolver's shape at the composition root and read here (R-001) to stamp
/// the `provider` flag. Separate from [`ProviderResource`] (which carries the
/// borrowed registries) so the flag decision needs no lifetimes.
#[derive(Debug, Clone, Copy)]
pub enum ProviderCell {
    Local,
    Multi,
    Embedded,
}

/// The provider resource matching the selected `provider` cell. The
/// caller owns the underlying registry value; the registry module owns
/// the cell choice.
pub enum ProviderResource<'a> {
    Local(&'a LocalRegistry),
    Multi(&'a MultiRegistryResolver),
    /// PROP-030: the embedded local-directory registry, composed with an
    /// optional declared multi-registry walk, at the origin-selected
    /// precedence.
    Embedded {
        embedded: &'a LocalRegistry,
        declared: Option<&'a MultiRegistryResolver>,
        precedence: EmbeddedPrecedence,
        /// PROP-030 §3.1: `--embedded-short-circuit` — stop version
        /// enumeration at the embedded registry for any coordinate it
        /// serves, sparing the declared walk's network round-trip.
        short_circuit: bool,
    },
}

/// Construct the `Registry/local` cell for `--registry <dir>` — the
/// Registry-seam construction site (R-001). The caller resolves and
/// canonicalises the path (a CLI concern); this module turns it into
/// the selected cell and commands thread the instance in. No flag is
/// read here: Registry selection is config-driven (`--registry` /
/// `[[registry]]` decide), and the `provider` flag above mirrors the
/// same decision for the DepProvider seam.
pub fn local_registry(root: PathBuf) -> Result<LocalRegistry, RegistryError> {
    LocalRegistry::new(root)
}

/// Construct the `RepoCreator/direct` cell for `vibe registry publish
/// --repo-url <url>` — the publish-seam construction site (R-001). The
/// host adapters (`github` / `gitverse`) are selected inside vibe-publish
/// by `creator_for_url`; the direct adapter is the one the CLI builds
/// from an explicit flag, so its construction lives here with the other
/// cell-selection sites and the publish command threads the instance in.
pub fn direct_git_creator(repo_url: String) -> DirectGitCreator {
    DirectGitCreator::new(repo_url)
}

/// Construct the selected `DepSolver` cell over the selected
/// `DepProvider` cell — the single seam-construction point.
pub fn dep_solver<'a>(
    flags: &SelectionFlags,
    resource: ProviderResource<'a>,
) -> Box<dyn DepSolver + 'a> {
    // recorded provenance: flags.solver / flags.provider carry it.
    match (flags.solver.value, flags.provider.value, resource) {
        ("resolvo", "local-registry", ProviderResource::Local(r)) => {
            Box::new(ResolvoDepSolver::new(LocalRegistryProvider::new(r)))
        }
        ("resolvo", "multi-registry", ProviderResource::Multi(m)) => {
            Box::new(ResolvoDepSolver::new(MultiRegistryProvider::new(m)))
        }
        ("naive", "local-registry", ProviderResource::Local(r)) => {
            Box::new(NaiveDepSolver::new(LocalRegistryProvider::new(r)))
        }
        ("naive", "multi-registry", ProviderResource::Multi(m)) => {
            Box::new(NaiveDepSolver::new(MultiRegistryProvider::new(m)))
        }
        ("sat", "local-registry", ProviderResource::Local(r)) => {
            Box::new(Sat::new(LocalRegistryProvider::new(r)))
        }
        ("sat", "multi-registry", ProviderResource::Multi(m)) => {
            Box::new(Sat::new(MultiRegistryProvider::new(m)))
        }
        (
            "resolvo",
            "embedded",
            ProviderResource::Embedded {
                embedded,
                declared,
                precedence,
                short_circuit,
            },
        ) => Box::new(ResolvoDepSolver::new(EmbeddedProvider::new(
            LocalRegistryProvider::new(embedded),
            declared.map(MultiRegistryProvider::new),
            precedence,
            short_circuit,
        ))),
        (
            "naive",
            "embedded",
            ProviderResource::Embedded {
                embedded,
                declared,
                precedence,
                short_circuit,
            },
        ) => Box::new(NaiveDepSolver::new(EmbeddedProvider::new(
            LocalRegistryProvider::new(embedded),
            declared.map(MultiRegistryProvider::new),
            precedence,
            short_circuit,
        ))),
        (
            "sat",
            "embedded",
            ProviderResource::Embedded {
                embedded,
                declared,
                precedence,
                short_circuit,
            },
        ) => Box::new(Sat::new(EmbeddedProvider::new(
            LocalRegistryProvider::new(embedded),
            declared.map(MultiRegistryProvider::new),
            precedence,
            short_circuit,
        ))),
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
    fn provider_flag_follows_the_selected_cell() {
        let local = selection_flags(ProviderCell::Local, None);
        assert_eq!(local.provider.value, "local-registry");
        assert_eq!(local.provider.provenance, Provenance::Cli);

        let multi = selection_flags(ProviderCell::Multi, None);
        assert_eq!(multi.provider.value, "multi-registry");
        assert_eq!(multi.provider.provenance, Provenance::BuiltIn);
        assert_eq!(multi.solver.value, "resolvo");
        assert_eq!(multi.solver.provenance, Provenance::BuiltIn);

        let embedded = selection_flags(ProviderCell::Embedded, None);
        assert_eq!(embedded.provider.value, "embedded");
        assert_eq!(embedded.provider.provenance, Provenance::BuiltIn);
    }

    #[test]
    fn solver_override_carries_cli_provenance() {
        let overridden = selection_flags(ProviderCell::Multi, Some("naive"));
        assert_eq!(overridden.solver.value, "naive");
        assert_eq!(overridden.solver.provenance, Provenance::Cli);

        let default = selection_flags(ProviderCell::Multi, None);
        assert_eq!(default.solver.value, "resolvo");
        assert_eq!(default.solver.provenance, Provenance::BuiltIn);
    }

    #[test]
    fn flag_table_covers_every_selection_field() {
        let names: Vec<&str> = FLAGS.iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["solver", "provider"]);
        assert!(FLAGS.iter().all(|f| !f.sunset.is_empty()));
    }
}
