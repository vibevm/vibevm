//! The cell-selection registry — R-001 ("flag at the seam, never in the
//! veins", GUIDE-RUST §3): the **only** module in this crate allowed to
//! read selection flags and construct seam cells from them. An explicit
//! `match` is chosen over distributed registration deliberately — one
//! `match` is the system's table of contents.
//!
//! Moved verbatim from `vibe-cli/src/registry.rs` (R7.4 A15a) when the ONE
//! production package-source composition moved down: the cells and their
//! construction sites travel together, so the law is preserved by the move.
//! The conform engine's single-registry pin still names the CLI registry
//! file (which keeps the publish seam); THIS crate fences the same law with
//! its own exact constructor-set / source RED in `fence_tests.rs`.
//!
//! Two tiers, never confused: cargo features answer "is the code in the
//! binary"; the runtime flags here answer "is the cell selected".

specmark::scope!(
    "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order"
);

use std::path::PathBuf;

use vibe_registry::{LocalRegistry, MultiRegistryResolver, RegistryError};
use vibe_resolver::sat::SatDepSolver;
use vibe_resolver::{
    EmbeddedDepProvider, EmbeddedPrecedence, LocalCompositeDepProvider, LocalRegistryDepProvider,
    MultiRegistryDepProvider, NaiveDepSolver, ResolvoDepSolver,
};

/// Where a selected value came from. The full chain is
/// CLI > env > project file > built-in (GUIDE-RUST §3); v0 populates
/// the two lanes that exist today and reserves the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provenance {
    BuiltIn,
    Cli,
}

/// One selected flag value with its provenance recorded. The
/// provenance field is registry data: rendered by diagnostics and the
/// flag-matrix tooling, not consumed on the solve path itself.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct Selected {
    pub(crate) value: &'static str,
    pub(crate) provenance: Provenance,
}

/// The selection flags the registry reads. Built once per command
/// invocation from already-parsed surface state; nothing else interprets
/// them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionFlags {
    /// `solver` — which `DepSolver` cell solves. `resolvo` (CDCL SAT,
    /// PROP-017) is the default since 2026-06-14; `naive` and `sat`
    /// remain selectable fallbacks. The flag is the seam point.
    pub(crate) solver: Selected,
    /// `provider` — which `DepProvider` cell feeds the solver:
    /// `local-registry` when an explicit registry path is given, else
    /// `multi-registry`.
    pub(crate) provider: Selected,
}

/// Static registry of the selection flags: name, default, birth,
/// sunset criterion (GUIDE-RUST §3 — "the flag registry is data").
/// Consumed source-level by `cargo xtask conform-lite` and unit tests;
/// the R-060 flag-matrix generator is its Phase 4+ runtime consumer.
#[allow(dead_code)]
pub(crate) struct FlagInfo {
    pub(crate) name: &'static str,
    pub(crate) default: &'static str,
    pub(crate) birth: &'static str,
    pub(crate) sunset: &'static str,
}

#[allow(dead_code)]
pub(crate) const FLAGS: &[FlagInfo] = &[
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

/// Interpret the parsed surface state into selection flags. The only
/// place flag values are decided.
pub(crate) fn selection_flags(
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
pub(crate) enum ProviderCell {
    Local,
    Multi,
    Embedded,
}

/// The provider resource matching the selected `provider` cell. The
/// caller owns the underlying registry value; the registry module owns
/// the cell choice.
pub(crate) enum ProviderResource<'a> {
    Local(&'a LocalRegistry),
    Multi(&'a MultiRegistryResolver),
    /// PROP-030: the local-registry family (project-local `packages/` plus
    /// the vibe-embedded `packages/` of a source install), composed into one
    /// `LocalCompositeDepProvider` and an optional declared multi-registry
    /// walk, at the origin-selected precedence. The Vec is ordered:
    /// project-local first (when discovered), then vibe-embedded.
    Embedded {
        locals: Vec<&'a LocalRegistry>,
        declared: Option<&'a MultiRegistryResolver>,
        precedence: EmbeddedPrecedence,
        /// PROP-030 §3.1: `--embedded-short-circuit` — stop version
        /// enumeration at the embedded registry for any coordinate it
        /// serves, sparing the declared walk's network round-trip.
        short_circuit: bool,
    },
}

/// Construct the `Registry/local` cell for an explicit `<dir>` registry —
/// the Registry-seam construction site (R-001). The caller resolves and
/// canonicalises the path (a surface concern); this module turns it into
/// the selected cell and callers thread the instance in. No flag is
/// read here: Registry selection is config-driven (the explicit path /
/// `[[registry]]` decide), and the `provider` flag above mirrors the
/// same decision for the DepProvider seam.
pub(crate) fn local_registry(root: PathBuf) -> Result<LocalRegistry, RegistryError> {
    LocalRegistry::new(root)
}

/// The embedded DepProvider composite — one construction site (R-001) for
/// the local-family + optional-declared shape all catalog routes share.
fn embedded_dep_provider<'a>(
    locals: Vec<&'a LocalRegistry>,
    declared: Option<&'a MultiRegistryResolver>,
    precedence: EmbeddedPrecedence,
    short_circuit: bool,
) -> EmbeddedDepProvider<'a> {
    EmbeddedDepProvider::new(
        LocalCompositeDepProvider::new(
            locals
                .into_iter()
                .map(LocalRegistryDepProvider::new)
                .collect(),
        ),
        declared.map(MultiRegistryDepProvider::new),
        precedence,
        short_circuit,
    )
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

/// All transaction routes retain the same backend selection and admitted intent.
pub(crate) fn solve_catalog_request(
    flags: &SelectionFlags,
    catalog: &vibe_resolver::resolvo_engine::TransactionCatalog,
    request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    roots: &[vibe_core::PackageRef],
    blocked: &std::collections::BTreeSet<(String, String)>,
    retained: &[vibe_resolver::resolvo_engine::SelectedChoice],
) -> Result<vibe_resolver::resolvo_engine::request::RouteResolution, vibe_resolver::SolveError> {
    use vibe_resolver::resolvo_engine::{
        RecordedResolution,
        request::{RouteProvider, RouteResolution},
    };
    use vibe_resolver::{DepProvider, SolveError};
    let roots = request.all_roots(roots);
    if let Some(configuration) = &request.configuration {
        if flags.solver.value != "resolvo" {
            return Err(SolveError::BackendCapability {
                backend: flags.solver.value.into(),
                capability: "candidate-conditioned consumer configuration".into(),
            });
        }
        let result = ResolvoDepSolver::new(catalog)
            .solve_configuration_routes(
                &roots,
                &configuration.request,
                &configuration.definitions,
                &configuration.defaults,
                blocked,
                retained,
            )
            .map_err(|error| SolveError::Configuration(Box::new(error)))?;
        return RouteResolution::bind(
            catalog,
            request,
            RecordedResolution {
                graph: result.graph,
                choices: result.choices,
            },
            result.manifests,
            result.effects,
        );
    }
    let provider = RouteProvider {
        catalog,
        blocked,
        retained,
    };
    let filtered = vibe_resolver::resolvo_engine::request::LegacyRouteProvider(
        vibe_resolver::resolvo_engine::request::RouteProvider {
            catalog,
            blocked,
            retained,
        },
    );
    let mut recorded = match flags.solver.value {
        "resolvo" => ResolvoDepSolver::new(provider).solve_recorded(&roots)?,
        "naive" => NaiveDepSolver::new(&filtered).solve_recorded(&roots)?,
        "sat" => SatDepSolver::new(&filtered).solve_recorded(&roots)?,
        _ => unreachable!("validated solver selection"),
    };
    if flags.solver.value != "resolvo" {
        filtered.restore_provenance(&mut recorded)?;
    }
    let mut manifests = std::collections::BTreeMap::new();
    let mut effects = std::collections::BTreeMap::new();
    for node in recorded.graph.iter() {
        let manifest = catalog.fetch_manifest(&node.group, &node.name, &node.version)?;
        let feature_request = if node.is_root {
            vibe_resolver::FeatureRequest {
                explicit: request
                    .features
                    .explicit
                    .iter()
                    .filter(|name| manifest.features.features.contains_key(name.as_str()))
                    .cloned()
                    .collect(),
                no_defaults: request.features.no_defaults,
                all: request.features.all,
            }
        } else {
            Default::default()
        };
        let expansion =
            vibe_resolver::features::expand_manifest_features(&manifest, &feature_request)
                .map_err(|e| vibe_resolver::DepProviderError::Other(e.to_string()))?;
        let coordinate = format!("{}/{}@{}", node.group, node.name, node.version);
        manifests.insert(coordinate.clone(), manifest);
        effects.insert(coordinate, expansion);
    }
    RouteResolution::bind(catalog, request, recorded, manifests, effects)
}

// Candidate capture uses the incumbent provider selection and enumeration.
pub(crate) fn catalog_provider(
    resource: ProviderResource<'_>,
) -> Box<dyn vibe_resolver::VersionEnumerator + '_> {
    match resource {
        ProviderResource::Local(r) => Box::new(LocalRegistryDepProvider::new(r)),
        ProviderResource::Multi(m) => Box::new(MultiRegistryDepProvider::new(m)),
        ProviderResource::Embedded {
            locals,
            declared,
            precedence,
            short_circuit,
        } => Box::new(embedded_dep_provider(
            locals,
            declared,
            precedence,
            short_circuit,
        )),
    }
}
