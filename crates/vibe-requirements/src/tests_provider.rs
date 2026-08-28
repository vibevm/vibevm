//! Provider seam oracles: one call, library-owned provenance, typed
//! loss, shape failures.

use std::cell::Cell;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_wire::generated::requirements_report::{
    RelationSourceProvenance, RelationSourceState, RequirementRelation,
    RequirementRelationProvenance, RequirementRelationVerb,
};

use crate::{
    ProviderOutcome, ProviderSource, QueryContext, RelationProvider, RelationRequest,
    RequirementsQuery, query,
};

pub(crate) fn ctx(root: &Path) -> QueryContext {
    QueryContext {
        selected_root: root.to_path_buf(),
        observed_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        lifecycle_run_id: None,
    }
}

/// Host `org.example/demo` + locked `org.example/pkg` with a slot: two
/// available sources, one row each.
pub(crate) fn project() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let host_specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&host_specs).unwrap();
    fs::write(host_specs.join("RULE.md"), "# Rules\n\n@fact:A a.\n").unwrap();
    fs::write(
        root.path().join("vibe.lock"),
        format!(
            "[meta]\ngenerated_by = \"f\"\ngenerated_at = \"2026-01-01T00:00:00Z\"\n\
             schema_version = 6\n\n[[package]]\nkind = \"feat\"\nname = \"pkg\"\n\
             group = \"org.example\"\nversion = \"1.0.0\"\n\
             source_url = \"https://example.invalid/pkg.git\"\n\
             content_hash = \"sha256:{}\"\n",
            "a".repeat(64)
        ),
    )
    .unwrap();
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = "1.0.0".parse().unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "pkg", &version);
    let specs = slot.join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("RULE.md"), "# P\n\n@fact:P p.\n").unwrap();
    root
}

pub(crate) fn edge(symbol: &str) -> (String, RequirementRelation) {
    (
        "spec://org.example/pkg/RULE#P".to_string(),
        RequirementRelation {
            verb: RequirementRelationVerb::Verifies,
            provenance: RequirementRelationProvenance::Authored,
            symbol: symbol.to_string(),
            file: "crates/x/src/lib.rs".to_string(),
            line: 7,
        },
    )
}

/// A fake provider that counts its calls and answers from a script.
pub(crate) type Answer = Result<Vec<(String, ProviderOutcome)>, crate::provider::ProviderError>;
pub(crate) type AnswerFn = fn(&RelationRequest<'_>) -> Answer;

pub(crate) struct Fake {
    pub(crate) calls: Cell<u32>,
    pub(crate) answer: AnswerFn,
}

impl RelationProvider for Fake {
    fn relations(
        &self,
        request: &RelationRequest<'_>,
    ) -> Result<Vec<(String, ProviderOutcome)>, crate::provider::ProviderError> {
        self.calls.set(self.calls.get() + 1);
        (self.answer)(request)
    }
}

pub(crate) fn relation_of(
    report: &vibe_wire::generated::requirements_report::RequirementsReport,
    package: &str,
) -> (
    RelationSourceState,
    RelationSourceProvenance,
    Option<String>,
) {
    let found = report
        .relation_sources
        .iter()
        .find(|source| source.package == package)
        .unwrap_or_else(|| panic!("no relation source for {package}: {report:?}"));
    (
        found.state.clone(),
        found.provenance.clone(),
        found.reason_code.clone(),
    )
}

#[test]
fn relations_false_never_calls_a_provider_and_names_every_source_not_requested() {
    let root = project();
    let fake = Fake {
        calls: Cell::new(0),
        answer: |request| {
            // An exploding answer: had it been consulted, rows would
            // carry edges and the counter would move.
            Ok(request
                .sources
                .iter()
                .map(|source| {
                    (
                        source.package.to_string(),
                        ProviderOutcome::Available {
                            edges: vec![edge("x::t")],
                        },
                    )
                })
                .collect())
        },
    };
    let report = query(
        &RequirementsQuery::default(),
        &ctx(root.path()),
        Some(&fake),
    )
    .unwrap();
    assert_eq!(fake.calls.get(), 0, "provider counter stays at zero");
    assert_eq!(report.relation_sources.len(), 2);
    for source in &report.relation_sources {
        assert_eq!(source.state, RelationSourceState::NotRequested);
        assert_eq!(source.provenance, RelationSourceProvenance::None);
        assert!(source.reason_code.is_none());
    }
    assert!(report.rows.iter().all(|row| row.relations.is_empty()));
}

#[test]
fn relations_true_calls_once_and_derivatives_come_from_the_base_kind() {
    let root = project();
    let fake = Fake {
        calls: Cell::new(0),
        answer: |request| {
            Ok(request
                .sources
                .iter()
                .map(|source| {
                    (
                        source.package.to_string(),
                        ProviderOutcome::Available { edges: vec![] },
                    )
                })
                .collect())
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    assert_eq!(fake.calls.get(), 1, "exactly one provider call");

    // Host → current/fresh; package → carried/carried — the provider
    // said only `Available` for both.
    let (host_state, host_prov, host_reason) = relation_of(&report, "org.example/demo");
    assert_eq!(host_state, RelationSourceState::Current);
    assert_eq!(host_prov, RelationSourceProvenance::FreshProjectMap);
    assert!(host_reason.is_none());
    let (pkg_state, pkg_prov, _) = relation_of(&report, "org.example/pkg");
    assert_eq!(pkg_state, RelationSourceState::Carried);
    assert_eq!(pkg_prov, RelationSourceProvenance::CarriedPackageMap);

    // Q14: a package that owns a row has a relation-source result even
    // with zero edges — silence about a scanned package is not an answer.
    let row = report
        .rows
        .iter()
        .find(|row| row.address == "spec://org.example/pkg/RULE#P")
        .unwrap();
    assert!(row.relations.is_empty());
}

#[test]
fn missing_or_failed_providers_and_missing_results_are_typed_unavailable() {
    let root = project();
    // No provider at all.
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    for source in &report.relation_sources {
        assert_eq!(source.state, RelationSourceState::Unavailable);
        assert_eq!(source.provenance, RelationSourceProvenance::None);
        assert_eq!(
            source.reason_code.as_deref(),
            Some("relation-provider-missing")
        );
    }
    assert_eq!(report.rows.len(), 2, "base rows still return");

    // A whole-provider failure.
    let failing = Fake {
        calls: Cell::new(0),
        answer: |_| Err(crate::provider::ProviderError("boom".to_string())),
    };
    let report = query(&q, &ctx(root.path()), Some(&failing)).unwrap();
    assert_eq!(failing.calls.get(), 1);
    for source in &report.relation_sources {
        assert_eq!(source.state, RelationSourceState::Unavailable);
        assert_eq!(
            source.reason_code.as_deref(),
            Some("relation-provider-failed")
        );
    }

    // A per-source miss: only the un-answered package degrades.
    let partial = Fake {
        calls: Cell::new(0),
        answer: |request| {
            Ok(request
                .sources
                .iter()
                .take(1)
                .map(|source| {
                    (
                        source.package.to_string(),
                        ProviderOutcome::Available { edges: vec![] },
                    )
                })
                .collect())
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&partial)).unwrap();
    let answered = relation_of(&report, "org.example/demo");
    assert_eq!(answered.0, RelationSourceState::Current);
    let missed = relation_of(&report, "org.example/pkg");
    assert_eq!(missed.0, RelationSourceState::Unavailable);
    assert_eq!(missed.2.as_deref(), Some("provider-result-missing"));
}

#[test]
fn loss_words_keep_their_kind_derived_provenance() {
    let root = project();
    let fake = Fake {
        calls: Cell::new(0),
        answer: |request| {
            Ok(request
                .sources
                .iter()
                .map(|source: &ProviderSource<'_>| {
                    let outcome = if source.package == "org.example/demo" {
                        ProviderOutcome::Stale {
                            reason: "map moved".to_string(),
                        }
                    } else {
                        ProviderOutcome::Invalid {
                            reason: "map did not parse".to_string(),
                        }
                    };
                    (source.package.to_string(), outcome)
                })
                .collect())
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    let (host_state, host_prov, host_reason) = relation_of(&report, "org.example/demo");
    assert_eq!(host_state, RelationSourceState::Stale);
    assert_eq!(host_prov, RelationSourceProvenance::FreshProjectMap);
    assert_eq!(host_reason.as_deref(), Some("map moved"));
    let (pkg_state, pkg_prov, pkg_reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(pkg_state, RelationSourceState::Invalid);
    assert_eq!(pkg_prov, RelationSourceProvenance::CarriedPackageMap);
    assert_eq!(pkg_reason.as_deref(), Some("map did not parse"));
}

#[test]
fn provider_shape_failures_become_fixed_invalid_reasons() {
    let root = project();
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();

    // An edge for an address the query never asked about.
    let out_of_scope = Fake {
        calls: Cell::new(0),
        answer: |_| {
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Available {
                    edges: vec![(
                        "spec://org.example/pkg/RULE#NOPE".to_string(),
                        RequirementRelation {
                            verb: RequirementRelationVerb::Implements,
                            provenance: RequirementRelationProvenance::Generated,
                            symbol: "x::y".to_string(),
                            file: "crates/x/src/lib.rs".to_string(),
                            line: 1,
                        },
                    )],
                },
            )])
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&out_of_scope)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-edge-out-of-scope"));
    assert!(
        report.rows.iter().all(|row| row.relations.is_empty()),
        "the out-of-scope edge was not attached"
    );

    // A duplicate result for one package.
    let duplicate = Fake {
        calls: Cell::new(0),
        answer: |request| {
            let package = request.sources[0].package.to_string();
            Ok(vec![
                (
                    package.clone(),
                    ProviderOutcome::Available { edges: vec![] },
                ),
                (package, ProviderOutcome::Available { edges: vec![] }),
            ])
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&duplicate)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/demo");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-result-invalid"));

    // An extra package this query never enumerated.
    let extra = Fake {
        calls: Cell::new(0),
        answer: |request| {
            let mut results: Vec<(String, ProviderOutcome)> = request
                .sources
                .iter()
                .map(|source| {
                    (
                        source.package.to_string(),
                        ProviderOutcome::Available { edges: vec![] },
                    )
                })
                .collect();
            results.push((
                "org.example/ghost".to_string(),
                ProviderOutcome::Available { edges: vec![] },
            ));
            Ok(results)
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&extra)).unwrap();
    for source in &report.relation_sources {
        assert_eq!(source.state, RelationSourceState::Invalid);
        assert_eq!(
            source.reason_code.as_deref(),
            Some("provider-answer-invalid")
        );
    }

    // An unbounded / control-bearing reason.
    let noisy = Fake {
        calls: Cell::new(0),
        answer: |_| {
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Unavailable {
                    reason: format!("x\n{}", "y".repeat(9000)),
                },
            )])
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&noisy)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-reason-unbounded"));
}

#[test]
fn a_source_with_no_root_here_cannot_carry_a_map() {
    // The locked package has no slot: an `Available` outcome for it is
    // kind-impossible and must not surface as enrichment.
    let root = project();
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = "1.0.0".parse().unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "pkg", &version);
    fs::remove_dir_all(slot.parent().unwrap()).unwrap();
    let fake = Fake {
        calls: Cell::new(0),
        answer: |request| {
            Ok(request
                .sources
                .iter()
                .map(|source| {
                    (
                        source.package.to_string(),
                        ProviderOutcome::Available { edges: vec![] },
                    )
                })
                .collect())
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-kind-impossible"));
}
