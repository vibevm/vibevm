//! Follow-up correction REDs (central A2b acceptance): C1 run-id hex,
//! C2 one-epoch host + lock-shape, C3 typed duplicates, C6 provider
//! contract, C7 text visibility.

use std::cell::Cell;

use vibe_wire::generated::requirements_report::RelationSourceState;

use crate::tests_provider::{Fake, edge, project as two_sources, relation_of};
use crate::tests_query::{
    RUN_ID, ctx, ctx_with_run, find_source, make_slot, project, source_state, write_lock,
};
use crate::{ProviderOutcome, QueryError, RequirementsQuery, query};

#[test]
fn the_run_id_is_lowercase_hex_not_lowercase_alphanumeric() {
    // C1: `g`..`z` refuse exactly like uppercase and wrong lengths.
    let root = project("# Rules\n\n@fact:A a.\n");
    for bad in [
        "g".repeat(32).as_str(), // g-past-hex
        "z".repeat(32).as_str(),
        &"A".repeat(32),                   // uppercase hex
        "0123456789abcdef0123456789abcde", // 31 chars
        &format!("{RUN_ID}0"),             // 33 chars
    ] {
        let error = query(
            &RequirementsQuery::default(),
            &ctx_with_run(root.path(), Some(bad)),
            None,
        )
        .unwrap_err();
        assert!(
            matches!(error, QueryError::InvalidRunId { .. }),
            "`{bad}` must refuse as a run id: {error:?}"
        );
    }
    // The boundary stays green: 32 chars of [0-9a-f].
    query(
        &RequirementsQuery::default(),
        &ctx_with_run(root.path(), Some(RUN_ID)),
        None,
    )
    .unwrap();
}

#[test]
fn a_lock_shaped_directory_is_malformed_scope_not_an_absent_lock() {
    let root = project("# Rules\n\n@fact:A a.\n");
    std::fs::create_dir_all(root.path().join("vibe.lock")).unwrap();
    let error = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap_err();
    assert!(
        matches!(error, QueryError::LockNotFile { .. }),
        "a directory wearing the lock's name must abort: {error:?}"
    );
}

#[test]
fn the_host_comes_from_the_one_epoch_no_second_manifest_read() {
    // C2's fence: enumeration derives the host coordinate from the
    // selected-workspace epoch; the second-read path (`host_package`)
    // must be absent from the product source.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sources.rs"),
    )
    .unwrap();
    assert!(
        !source.contains("host_package"),
        "a second selected-manifest read reappeared in enumeration"
    );
    // And a selected MEMBER answers from its own manifest through the
    // same epoch (the workspace root's lock is still the package
    // universe).
    let ws = tempfile::TempDir::new().unwrap();
    std::fs::write(
        ws.path().join("vibe.toml"),
        "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n[workspace]\nmembers = [\"pkg\"]\n",
    )
    .unwrap();
    std::fs::create_dir_all(ws.path().join("pkg")).unwrap();
    std::fs::write(
        ws.path().join("pkg").join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"member\"\nkind = \"feat\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let specs = ws
        .path()
        .join("pkg")
        .join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(specs.join("RULE.md"), "# M\n\n@fact:M m.\n").unwrap();
    let report = query(
        &RequirementsQuery::default(),
        &ctx(&ws.path().join("pkg")),
        None,
    )
    .unwrap();
    assert_eq!(report.observation.selected, "pkg");
    let host = find_source(&report, "org.example/member");
    assert_eq!(host.state, source_state("available"));
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].address, "spec://org.example/member/RULE#M");
}

#[test]
fn a_duplicate_fact_row_is_a_typed_invariant_never_a_silent_dedup() {
    // C3: the merge refuses adjacent duplicate addresses. Ordinary
    // construction cannot reach this (the scanner refuses per-source
    // duplicates and sources are unique by coordinate), so the law is
    // exercised at the helper it lives in.
    use crate::rows;
    use vibe_wire::generated::requirements_report::{
        AuthoringObservation, AuthoringObservationPresence, RequirementRow, RequirementSource,
        RequirementSourceKind,
    };
    let row = |address: &str| {
        RequirementRow {
        address: address.to_string(),
        source: RequirementSource {
            kind: RequirementSourceKind::Host,
            package: "org.example/demo".to_string(),
        },
        authoring: AuthoringObservation {
            presence: AuthoringObservationPresence::Unmarked,
            status: None,
        },
        adoption: vibe_wire::generated::requirements_report::AdoptionObservation {
            presence:
                vibe_wire::generated::requirements_report::AdoptionObservationPresence::NotApplicable,
            status: None,
        },
        relations: Vec::new(),
    }
    };
    let clean = vec![row("spec://org.example/demo/RULE#A")];
    rows::refuse_duplicate_addresses(&clean).unwrap();
    let duplicated = vec![
        row("spec://org.example/demo/RULE#A"),
        row("spec://org.example/demo/RULE#A"),
    ];
    let error = rows::refuse_duplicate_addresses(&duplicated).unwrap_err();
    assert!(
        matches!(error, QueryError::Invariant(ref message) if message.contains("duplicate full fact address")),
        "{error:?}"
    );
}

#[test]
fn text_render_shows_every_relation_state_even_without_a_reason() {
    // C7: successful enrichment may not disappear from human output.
    let root = project("# Rules\n\n@fact:A a.\n");
    write_lock(root.path(), false);
    make_slot(root.path(), "# P\n\n@fact:P p.\n");
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    let text = crate::text::render(&report);
    assert!(
        text.contains("relations org.example/demo: unavailable (relation-provider-missing)"),
        "the host relation state must be visible with its reason: {text}"
    );
    assert!(
        text.contains("relations org.example/pkg: unavailable (relation-provider-missing)"),
        "the package relation state must be visible with its reason: {text}"
    );
    // With a provider answering Available, current/carried appear.
    struct Available;
    impl crate::RelationProvider for Available {
        fn relations(
            &self,
            request: &crate::RelationRequest<'_>,
        ) -> Result<Vec<(String, crate::ProviderOutcome)>, crate::ProviderError> {
            Ok(request
                .sources
                .iter()
                .map(|source| {
                    (
                        source.package.to_string(),
                        crate::ProviderOutcome::Available { edges: vec![] },
                    )
                })
                .collect())
        }
    }
    let report = query(&q, &ctx(root.path()), Some(&Available)).unwrap();
    let text = crate::text::render(&report);
    assert!(
        text.contains("relations org.example/demo: current"),
        "successful host enrichment must be visible: {text}"
    );
    assert!(
        text.contains("relations org.example/pkg: carried"),
        "successful package enrichment must be visible: {text}"
    );
}

#[test]
fn edges_arrive_sorted_and_an_exact_duplicate_is_a_typed_invalid() {
    let root = two_sources();
    // Sorted order still holds for distinct keys…
    let sorted = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            let edges = vec![edge("b::two"), edge("a::two"), edge("a::one")];
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Available { edges },
            )])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&sorted)).unwrap();
    let row = report
        .rows
        .iter()
        .find(|row| row.address == "spec://org.example/pkg/RULE#P")
        .unwrap();
    let symbols: Vec<&str> = row.relations.iter().map(|e| e.symbol.as_str()).collect();
    assert_eq!(symbols, ["a::one", "a::two", "b::two"], "sorted");

    // …but an exact duplicate (address, verb, symbol, file, line) is a
    // shape failure of that package's answer — never silently deduped.
    let duplicate = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            let mut edges = vec![edge("a::one"), edge("a::one")];
            edges.push(edge("a::two"));
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Available { edges },
            )])
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&duplicate)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-edge-duplicate"));
    assert!(
        report.rows.iter().all(|row| row.relations.is_empty()),
        "the duplicated edges were not attached"
    );
}

#[test]
fn a_blank_loss_reason_is_a_typed_invalid() {
    let root = two_sources();
    let fake = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Stale {
                    reason: "  \t ".to_string(),
                },
            )])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-reason-blank"));
}

#[test]
fn a_host_result_cannot_attach_a_package_row_edge() {
    let root = two_sources();
    // The edge's address IS globally requested (the package row
    // exists), but it sits outside the HOST outcome's own namespace.
    let fake = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            Ok(vec![(
                "org.example/demo".to_string(),
                ProviderOutcome::Available {
                    edges: vec![edge("x::t")],
                },
            )])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/demo");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-edge-out-of-scope"));
    assert!(
        report.rows.iter().all(|row| row.relations.is_empty()),
        "a cross-namespace edge must not attach"
    );
}

#[test]
fn a_wire_malformed_edge_is_a_typed_invalid_not_a_query_error() {
    let root = two_sources();
    let zero_line = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            let (address, mut e) = edge("x::t");
            e.line = 0;
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Available {
                    edges: vec![(address, e)],
                },
            )])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&zero_line)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-edge-malformed"));

    let bad_path = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            let (address, mut e) = edge("x::t");
            e.file = "../escape.rs".to_string();
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Available {
                    edges: vec![(address, e)],
                },
            )])
        },
    };
    let report = query(&q, &ctx(root.path()), Some(&bad_path)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-edge-malformed"));
}

#[test]
fn a_rootless_source_may_only_answer_unavailable() {
    let root = two_sources();
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = "1.0.0".parse().unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(root.path(), &group, "pkg", &version);
    std::fs::remove_dir_all(slot.parent().unwrap()).unwrap();
    // Stale is data-bearing: with no slot there is nothing to prove
    // stale — kind-impossible, exactly like Available.
    let fake = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            Ok(vec![(
                "org.example/pkg".to_string(),
                ProviderOutcome::Stale {
                    reason: "nothing to read".to_string(),
                },
            )])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&fake)).unwrap();
    let (state, _, reason) = relation_of(&report, "org.example/pkg");
    assert_eq!(state, RelationSourceState::Invalid);
    assert_eq!(reason.as_deref(), Some("provider-kind-impossible"));
}

#[test]
fn an_extra_package_poisons_sources_the_answer_omitted_too() {
    let root = two_sources();
    // The provider answers ONLY the host, plus one ghost package: the
    // poisoned answer must be invalid for the host AND for the package
    // the answer never mentioned.
    let extra = Fake {
        calls: Cell::new(0),
        answer: |_: &crate::RelationRequest<'_>| {
            Ok(vec![
                (
                    "org.example/demo".to_string(),
                    ProviderOutcome::Available { edges: vec![] },
                ),
                (
                    "org.example/ghost".to_string(),
                    ProviderOutcome::Available { edges: vec![] },
                ),
            ])
        },
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&extra)).unwrap();
    for package in ["org.example/demo", "org.example/pkg"] {
        let (state, _, reason) = relation_of(&report, package);
        assert_eq!(state, RelationSourceState::Invalid, "{package}");
        assert_eq!(
            reason.as_deref(),
            Some("provider-answer-invalid"),
            "{package}"
        );
    }
}

// --- A2c: lock source authority carried into the provider request ---

use std::cell::RefCell;

struct Capture(RefCell<Vec<(String, Option<String>)>>);

impl crate::RelationProvider for Capture {
    fn relations(
        &self,
        request: &crate::RelationRequest<'_>,
    ) -> Result<Vec<(String, crate::ProviderOutcome)>, crate::ProviderError> {
        for source in request.sources {
            self.0.borrow_mut().push((
                source.package.to_string(),
                source.expected_content_hash.map(str::to_string),
            ));
        }
        Ok(Vec::new())
    }
}

fn two_package_lock(root: &std::path::Path, ghost_hash: &str) {
    std::fs::write(
        root.join("vibe.lock"),
        format!(
            "[meta]
generated_by = \"f\"
generated_at = \"2026-01-01T00:00:00Z\"
             schema_version = 6

[[package]]
kind = \"feat\"
name = \"pkg\"
             group = \"org.example\"
version = \"1.0.0\"
             source_url = \"https://example.invalid/pkg.git\"
             content_hash = \"sha256:{}\"

             [[package]]
kind = \"feat\"
name = \"ghost\"
             group = \"org.example\"
version = \"2.0.0\"
             source_url = \"https://example.invalid/ghost.git\"
             content_hash = \"sha256:{ghost_hash}\"
",
            "a".repeat(64),
        ),
    )
    .unwrap();
}

#[test]
fn the_provider_sees_the_locks_exact_content_hash_per_the_matrix() {
    // Host + materialised locked package from the shared fixture; the
    // custom lock adds a second, NEVER-materialised locked package; the
    // registry adds an entry for a coordinate the lock never named.
    let root = two_sources();
    two_package_lock(root.path(), &"b".repeat(64));
    let mut registry = vibe_facts::Registry::load(root.path()).unwrap();
    registry
        .upsert(
            root.path(),
            vibe_facts::FactEntry {
                address: "spec://org.example/never/RULE#X".to_string(),
                origin: vibe_facts::FactOrigin::Package,
                package: Some("org.example/never".to_string()),
                status: None,
                comment: None,
            },
        )
        .unwrap();

    let capture = Capture(RefCell::new(Vec::new()));
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &ctx(root.path()), Some(&capture)).unwrap();
    assert_eq!(report.rows.len(), 2, "base rows still return");

    let observed: std::collections::BTreeMap<String, Option<String>> =
        capture.0.borrow().iter().cloned().collect();
    // Host: no lock row speaks for it.
    assert_eq!(observed.get("org.example/demo"), Some(&None));
    // Materialised locked package: the EXACT authored hash, verbatim.
    let pkg_hash = format!("sha256:{}", "a".repeat(64));
    assert_eq!(
        observed.get("org.example/pkg"),
        Some(&Some(pkg_hash.clone()))
    );
    // Locked but NEVER materialised: the authority exists even when the
    // materialisation does not.
    let ghost_hash = format!("sha256:{}", "b".repeat(64));
    assert_eq!(observed.get("org.example/ghost"), Some(&Some(ghost_hash)));
    // Registry-only orphan: the lock never named it.
    assert_eq!(observed.get("org.example/never"), Some(&None));
    let plain_before = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    // And the observed value IS the lock's authored bytes — re-author
    // the ghost hash and the provider observes the new value, with no
    // lock read of its own (its only input is the request).
    two_package_lock(root.path(), &"c".repeat(64));
    let capture = Capture(RefCell::new(Vec::new()));
    query(&q, &ctx(root.path()), Some(&capture)).unwrap();
    let observed: std::collections::BTreeMap<String, Option<String>> =
        capture.0.borrow().iter().cloned().collect();
    assert_eq!(
        observed.get("org.example/ghost"),
        Some(&Some(format!("sha256:{}", "c".repeat(64)))),
        "the request carries the lock's CURRENT authority, verbatim"
    );
    // The authority is provider-request metadata only. With relations
    // unrequested, changing ONLY that hash cannot move any wire member or
    // digest (the report carries no version/hash field of its own).
    let plain_after = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert_eq!(plain_after, plain_before);
}
