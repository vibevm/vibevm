//! Source-rank integration over genuine retained local metadata.
use std::collections::BTreeSet;
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_registry::{LocalRegistry, source_candidates::*};
use vibe_resolver::{
    source_candidates::{EdgeId, EdgeOwner},
    source_edges::*,
    *,
};
struct Forced<'a> {
    inner: LocalRegistryDepProvider<'a>,
    deny: String,
}
impl DepProvider for Forced<'_> {
    fn resolve_version(&self, r: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.inner.resolve_version(r)
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.inner.fetch_manifest(g, n, v)
    }
}
impl VersionEnumerator for Forced<'_> {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        self.inner.list_versions(g, n)
    }
}
impl SourceCandidateProvider for Forced<'_> {
    fn has_locked_coordinate(&self, _: &Group, _: &str) -> bool {
        false
    }
    fn source_choice_mode(&self) -> SourceChoiceMode {
        SourceChoiceMode::CandidateRequired
    }
    fn retain_default_candidate(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<RetainedCandidateObservation, DepProviderError> {
        self.inner.retain_default_candidate(g, n, v)
    }
    fn respects_locked_selection(
        &self,
        _: &RetainedSourceCandidate,
        _: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, DepProviderError> {
        Ok(false)
    }
    fn source_inventory(
        &self,
        g: &Group,
        n: &str,
        s: CandidateInventoryScope,
        l: &SourceInventoryLimits,
    ) -> Result<SourceCandidateInventory, DepProviderError> {
        let mut inventory = self.inner.source_inventory(g, n, s, l)?;
        if n == self.deny {
            for row in &mut inventory.candidates {
                row.default_eligible = false;
            }
        }
        Ok(inventory)
    }
}
fn seed(root: &std::path::Path, name: &str, extra: &str) {
    let path = root.join("org.test").join(name).join("v1.0.0");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("vibe.toml"),
        format!(
            "[package]\ngroup='org.test'\nname='{name}'\nversion='1.0.0'\nkind='flow'\n{extra}"
        ),
    )
    .unwrap();
}
fn roots(names: &[&str]) -> Vec<RootRequirement> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| RootRequirement {
            id: EdgeId {
                owner: EdgeOwner::Root {
                    consumer: "fixture".into(),
                },
                site: RequirementSite::ExplicitRequest(index),
            },
            requirement: OriginalRequirement {
                site: RequirementSite::ExplicitRequest(index),
                activation: RequirementActivation::Mandatory,
                alternatives: vec![RequirementAlternative {
                    reference: PackageRef::parse(&format!("org.test/{name}")).unwrap(),
                    registry_pin: None,
                }],
            },
        })
        .collect()
}
fn solve(
    provider: &impl SourceCandidateProvider,
    names: &[&str],
    limits: SourceSolveLimits,
) -> Result<SourceResolvedGraph, SolveError> {
    let roots = roots(names);
    let blocked = BTreeSet::new();
    ResolvoDepSolver::new(Borrowed(provider)).solve_source_aware(&SourceSolveRequest {
        roots: &roots,
        held_roots: &[],
        authoritative_locked: &[],
        activated: &[],
        limits,
        configuration: None,
        blocked: &blocked,
        recommends: true,
    })
}
struct Borrowed<'a, P>(&'a P);
impl<P: SourceCandidateProvider> DepProvider for Borrowed<'_, P> {
    fn resolve_version(&self, r: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.0.resolve_version(r)
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0.fetch_manifest(g, n, v)
    }
}
impl<P: SourceCandidateProvider> VersionEnumerator for Borrowed<'_, P> {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        self.0.list_versions(g, n)
    }
}
impl<P: SourceCandidateProvider> SourceCandidateProvider for Borrowed<'_, P> {
    fn has_locked_coordinate(&self, g: &Group, n: &str) -> bool {
        self.0.has_locked_coordinate(g, n)
    }
    fn source_choice_mode(&self) -> SourceChoiceMode {
        self.0.source_choice_mode()
    }
    fn retain_default_candidate(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<RetainedCandidateObservation, DepProviderError> {
        self.0.retain_default_candidate(g, n, v)
    }
    fn validate_registry_pin(
        &self,
        g: &Group,
        n: &str,
        p: &vibe_core::manifest::RegistryPin,
    ) -> Result<(), DepProviderError> {
        self.0.validate_registry_pin(g, n, p)
    }
    fn respects_locked_selection(
        &self,
        c: &RetainedSourceCandidate,
        l: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, DepProviderError> {
        self.0.respects_locked_selection(c, l)
    }
    fn source_inventory(
        &self,
        g: &Group,
        n: &str,
        s: CandidateInventoryScope,
        l: &SourceInventoryLimits,
    ) -> Result<SourceCandidateInventory, DepProviderError> {
        self.0.source_inventory(g, n, s, l)
    }
}
#[test]
fn real_retained_cycle_has_rooted_support() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "[requires.packages]\n'org.test/b'='*'");
    seed(dir.path(), "b", "[requires.packages]\n'org.test/a'='*'");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Forced {
        inner: LocalRegistryDepProvider::new(&registry),
        deny: String::new(),
    };
    let result = solve(&provider, &["a"], SourceSolveLimits::standard_operation()).unwrap();
    assert_eq!(result.selected.len(), 2);
    assert_eq!(result.active_edges.len(), 3);
}
#[test]
fn unpinned_nondefault_candidate_cannot_leak_into_selected_graph() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Forced {
        inner: LocalRegistryDepProvider::new(&registry),
        deny: "a".into(),
    };
    assert!(matches!(
        solve(&provider, &["a"], SourceSolveLimits::standard_operation()),
        Err(SolveError::Unsatisfiable { .. })
    ));
}
#[test]
fn rank_budget_is_not_unsatisfiability() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Forced {
        inner: LocalRegistryDepProvider::new(&registry),
        deny: String::new(),
    };
    let mut limits = SourceSolveLimits::standard_operation();
    limits.ranked_candidates = 0;
    assert!(matches!(
        solve(&provider, &["a"], limits),
        Err(SolveError::SourceModelLimit { .. })
    ));
}
#[test]
fn selected_before_obsoletes_retains_original_metadata() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "[requires.packages]\n'org.test/x'='*'");
    seed(dir.path(), "b", "[obsoletes]\npackages=['org.test/a']");
    seed(dir.path(), "x", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Forced {
        inner: LocalRegistryDepProvider::new(&registry),
        deny: String::new(),
    };
    let result = solve(
        &provider,
        &["a", "b"],
        SourceSolveLimits::standard_operation(),
    )
    .unwrap();
    assert_eq!(result.selected_before_projection.len(), 3);
    assert_eq!(result.selected.len(), 2);
    assert!(result.selected.keys().any(|(_, name)| name == "x"));
}

#[test]
fn inactive_original_pin_does_not_annotate_independently_required_target() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        "a",
        "[requires.packages]\n'org.test/runtime'={version='*',optional=true,registry='inactive'}",
    );
    seed(dir.path(), "runtime", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Forced {
        inner: LocalRegistryDepProvider::new(&registry),
        deny: String::new(),
    };
    let result = solve(
        &provider,
        &["a", "runtime"],
        SourceSolveLimits::standard_operation(),
    )
    .unwrap();
    let runtime = result
        .selected
        .iter()
        .find(|((_, name), _)| name == "runtime")
        .unwrap()
        .1;
    assert!(
        runtime
            .observations
            .iter()
            .any(|row| row.decision == SourceCandidateDecision::Selected)
    );
    assert!(
        !runtime
            .observations
            .iter()
            .any(|row| matches!(row.decision, SourceCandidateDecision::ExcludedByPin { .. }))
    );
}
/// Every observed source of the named coordinate was refused by hosting
/// exclusion; `keep` retains the inner (admitted) candidates beside it.
struct Excluding<'a> {
    inner: Forced<'a>,
    keep: bool,
}
impl DepProvider for Excluding<'_> {
    fn resolve_version(&self, r: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.inner.resolve_version(r)
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.inner.fetch_manifest(g, n, v)
    }
}
impl VersionEnumerator for Excluding<'_> {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        self.inner.list_versions(g, n)
    }
}
impl SourceCandidateProvider for Excluding<'_> {
    fn has_locked_coordinate(&self, g: &Group, n: &str) -> bool {
        self.inner.has_locked_coordinate(g, n)
    }
    fn source_choice_mode(&self) -> SourceChoiceMode {
        self.inner.source_choice_mode()
    }
    fn retain_default_candidate(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<RetainedCandidateObservation, DepProviderError> {
        self.inner.retain_default_candidate(g, n, v)
    }
    fn respects_locked_selection(
        &self,
        c: &RetainedSourceCandidate,
        l: &vibe_core::manifest::LockedPackage,
    ) -> Result<bool, DepProviderError> {
        self.inner.respects_locked_selection(c, l)
    }
    fn source_inventory(
        &self,
        g: &Group,
        n: &str,
        s: CandidateInventoryScope,
        l: &SourceInventoryLimits,
    ) -> Result<SourceCandidateInventory, DepProviderError> {
        let mut inventory = self.inner.source_inventory(g, n, s, l)?;
        if !self.keep {
            inventory.candidates.clear();
            inventory.observations.clear();
        }
        inventory.observations.push(SourceCandidateObservation {
            binding: vibe_core::manifest::CanonicalSourceBinding::Registry {
                registry: vibe_core::manifest::RegistryPin::new("excluded".into()).unwrap(),
                source_url: vibe_core::SourceUrl::new("http://127.0.0.1:9/org".to_string()),
            },
            observation: SourceCandidateObservationKind::Unknown,
            decision: SourceCandidateDecision::ExcludedHost {
                endpoint: "http://127.0.0.1:9".into(),
            },
        });
        Ok(inventory)
    }
}
#[test]
fn requirement_served_only_by_excluded_hosting_is_a_typed_refusal() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    let provider = Excluding {
        inner: Forced {
            inner: LocalRegistryDepProvider::new(&registry),
            deny: String::new(),
        },
        keep: false,
    };
    let error = solve(&provider, &["a"], SourceSolveLimits::standard_operation()).unwrap_err();
    match error {
        SolveError::Provider(DepProviderError::SourcePolicy(
            vibe_core::manifest::SourcePolicyError::ExcludedOnly { package, endpoints },
        )) => {
            assert_eq!(package, "org.test/a");
            assert_eq!(endpoints, ["http://127.0.0.1:9"]);
        }
        other => panic!("expected ExcludedOnly, got {other:?}"),
    }
}
#[test]
fn admitted_entry_beside_an_exclusion_keeps_its_unsatisfiability_proof() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "a", "");
    let registry = LocalRegistry::new(dir.path()).unwrap();
    // The admitted candidate exists but is not default-eligible: a genuine
    // model conflict, never relabelled as an exclusion.
    let provider = Excluding {
        inner: Forced {
            inner: LocalRegistryDepProvider::new(&registry),
            deny: "a".into(),
        },
        keep: true,
    };
    assert!(matches!(
        solve(&provider, &["a"], SourceSolveLimits::standard_operation()),
        Err(SolveError::Unsatisfiable { .. })
    ));
}
