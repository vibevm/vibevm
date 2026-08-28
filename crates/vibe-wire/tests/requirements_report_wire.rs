//! The R7.5 requirements report root (PROP-054 §14.7
//! `##REF-REQUIREMENTS-WIRE`, `##FACT-QUERY-CONTRACT`).
//!
//! REGISTRY: the format is inventoried, so it is expressible as a
//! `FormatId` and its corpus has a home. ROUND-TRIP AND SEMANTICS:
//! four authored corpora — base, enriched relations, partial/
//! unavailable provider, truncation — ride the generated root
//! byte-identically, are read for their identity, states and ORDER,
//! and validate green through the hand-written cell. SCHEMA PARITY:
//! the schema's `x-relational-laws` label set equals the validator's
//! implemented list, and its caps equal the constants the validator
//! enforces. VOCABULARY PARITY: the relation verb and provenance sets
//! are the specmap engine's own closed sets, and the fact stage/state
//! sets are the PROP-043 markup vocabulary. FENCES: no verdict word,
//! no prose-carrying member, and a source-body canary that appears in
//! no emitted byte.

use std::collections::BTreeSet;
use std::path::PathBuf;

#[path = "wire_support/mod.rs"]
mod support;
use support::{read_json, repo_root};

use vibe_wire::behaviour::requirements_report::{
    ADDRESS_CAP_BYTES, IMPLEMENTED_LAWS, LIMIT_MAX, validate,
};
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::requirements_report::{
    AdoptionObservationPresence, AuthoringObservationPresence, RelationSourceProvenance,
    RelationSourceState, RequirementSourceKind, RequirementsReport, SourceResultState,
};

fn corpus_dir() -> PathBuf {
    repo_root().join("formats/corpora/requirements/e1")
}

/// Parse one corpus through the generated root, prove the bytes
/// survive, and prove the value satisfies every relational law.
fn corpus(name: &str) -> RequirementsReport {
    let authored = read_json(&format!("formats/corpora/requirements/e1/{name}"));
    let report: RequirementsReport =
        serde_json::from_value(authored.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        authored,
        "{name} loses data on generated round-trip"
    );
    validate(&report).unwrap_or_else(|e| panic!("{name} violates a relational law: {e}"));
    report
}

fn enum_at(document: &serde_json::Value, pointer: &str) -> Vec<String> {
    document
        .pointer(pointer)
        .and_then(|value| value.as_array())
        .unwrap_or_else(|| panic!("{pointer} is an enum array"))
        .iter()
        .map(|value| value.as_str().expect("an enum value").to_string())
        .collect()
}

/// The format id is NEUTRAL. One generated root is returned by the
/// shared library, by the CLI projection and by the MCP tool, so a
/// `cli-*` id would be false on two of the three the day it shipped —
/// which is exactly the legacy misnaming `cli-lifecycle-report`
/// carries and this new surface must not repeat.
#[test]
fn the_format_is_inventoried_under_a_surface_neutral_id() {
    let registry = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&registry).unwrap();
    let formats = parsed["format"].as_table().expect("a [format.*] table");
    assert!(
        !formats.contains_key("cli-requirements-report"),
        "the requirements root is not a CLI-only format"
    );
    let row = &formats["requirements-report"];
    assert_eq!(
        row["schema"].as_str(),
        Some("schemas/requirements_report.jtd.json")
    );
    assert_eq!(
        row["corpus"].as_str(),
        Some("formats/corpora/requirements/e1")
    );
    assert_eq!(
        row["foreign_parsers"].as_str(),
        Some("many"),
        "an orchestrator's `curl | jq` is a foreign parser; the reader stays permissive"
    );
    assert!(row["recoverable"].as_bool().unwrap());
    assert!(
        FormatId::ALL
            .iter()
            .any(|id| id.id() == "requirements-report"),
        "the registry row must have reached the generated FormatId"
    );
    // The `cli-*` family's own count comment stays honest: this row is
    // not one of them.
    let cli_rows = formats.keys().filter(|id| id.starts_with("cli-")).count();
    assert_eq!(
        cli_rows, 13,
        "the CLI report family did not grow; the requirements root lives in its own section"
    );
    assert!(
        corpus_dir().is_dir(),
        "the registry names a corpus home that exists"
    );
}

/// The version of `core-ai-native` the LOCKFILE selected, and the
/// materialised slot that version was installed into. Reading a
/// hardcoded `v0.8.0` under `vibepacks/` would pin this parity to a
/// workspace copy the project no longer resolves.
fn lock_selected_specmap_schema() -> PathBuf {
    let lock = std::fs::read_to_string(repo_root().join("vibe.lock")).expect("vibe.lock readable");
    let parsed: toml::Value = toml::from_str(&lock).expect("vibe.lock parses");
    let version = parsed["package"]
        .as_array()
        .expect("vibe.lock has [[package]] entries")
        .iter()
        .find(|entry| {
            entry.get("name").and_then(|v| v.as_str()) == Some("core-ai-native")
                && entry.get("group").and_then(|v| v.as_str()) == Some("org.vibevm.ai-native")
        })
        .and_then(|entry| entry.get("version"))
        .and_then(|v| v.as_str())
        .expect("the lock selects a core-ai-native version");
    let slot = repo_root()
        .join("vibevm/vibedeps/org.vibevm.ai-native.core-ai-native")
        .join(version)
        .join("schemas/specmap.jtd.json");
    assert!(
        slot.is_file(),
        "the lock-selected specmap schema is materialised at {}",
        slot.display()
    );
    slot
}

#[test]
fn the_base_corpus_keeps_four_observation_axes_apart() {
    let report = corpus("report_base.json");
    assert_eq!(report.requirements, 1);
    assert!(!report.truncated);
    assert!(!report.query.relations);
    assert_eq!(report.query.limit, 100);
    assert!(report.query.address_prefix.is_none());

    // Relations were not requested, so every source says so and no row
    // carries an edge — the provider was never called (`Q5`).
    assert!(
        report
            .relation_sources
            .iter()
            .all(|source| source.state == RelationSourceState::NotRequested
                && source.provenance == RelationSourceProvenance::None)
    );
    assert!(report.rows.iter().all(|row| row.relations.is_empty()));

    // Two AVAILABLE base sources, one host and one package, and every
    // row binds to one of them by (kind, package).
    assert_eq!(
        report
            .sources
            .iter()
            .map(|result| (
                result.source.package.as_str(),
                result.state.clone(),
                result.digest.is_some()
            ))
            .collect::<Vec<_>>(),
        [
            ("org.demo/host", SourceResultState::Available, true),
            ("org.vendor/tool", SourceResultState::Available, true),
        ]
    );

    // Host and package rows, in address order, with distinct
    // authoring and adoption observations on each.
    let axes: Vec<(
        &str,
        &RequirementSourceKind,
        bool,
        &AdoptionObservationPresence,
    )> = report
        .rows
        .iter()
        .map(|row| {
            (
                row.address.as_str(),
                &row.source.kind,
                matches!(row.authoring.presence, AuthoringObservationPresence::Marked),
                &row.adoption.presence,
            )
        })
        .collect();
    assert_eq!(axes.len(), 5);
    assert_eq!(axes[0].1, &RequirementSourceKind::Host);
    assert_eq!(axes[0].3, &AdoptionObservationPresence::NotApplicable);
    assert!(axes[0].2, "the first host row is marked");
    assert!(!axes[1].2, "an addressed fact with no marker still appears");

    // The three absent-adoption words are all present and distinct —
    // this is the corpus half of «four words, not a boolean».
    let adoptions: BTreeSet<&str> = report
        .rows
        .iter()
        .map(|row| match row.adoption.presence {
            AdoptionObservationPresence::NotApplicable => "not-applicable",
            AdoptionObservationPresence::Absent => "absent",
            AdoptionObservationPresence::Indeterminate => "indeterminate",
            AdoptionObservationPresence::Recorded => "recorded",
        })
        .collect();
    assert_eq!(
        adoptions,
        ["absent", "indeterminate", "not-applicable", "recorded"]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn the_relations_corpus_carries_current_and_carried_provenance_in_edge_order() {
    let report = corpus("report_relations.json");
    assert!(report.query.relations);
    assert_eq!(
        report
            .relation_sources
            .iter()
            .map(|source| (
                source.package.as_str(),
                source.state.clone(),
                source.provenance.clone()
            ))
            .collect::<Vec<_>>(),
        [
            (
                "org.demo/host",
                RelationSourceState::Current,
                RelationSourceProvenance::FreshProjectMap
            ),
            (
                "org.vendor/tool",
                RelationSourceState::Carried,
                RelationSourceProvenance::CarriedPackageMap
            ),
        ]
    );
    // Edges carry coordinates, never bodies, and are sorted.
    let host = &report.rows[0];
    assert_eq!(host.relations.len(), 2);
    assert_eq!(host.relations[0].line, 42);
    assert_eq!(host.relations[0].symbol, "demo::build::compile");
    assert_eq!(host.relations[0].file, "crates/demo/src/build.rs");

    // A `verifies` EDGE does not become a verification verdict: the
    // report carries no evidence member of any kind.
    let rendered = serde_json::to_value(&report).unwrap();
    for absent in ["verification", "evidence", "status"] {
        assert!(
            rendered.get(absent).is_none(),
            "the requirements root carries no `{absent}` member; evidence is a separate root"
        );
    }
}

/// The base source layer carries what `relation_sources` cannot say:
/// that an authored source was missing, unparseable, or existed only
/// as adoption-registry orphans. Only an `available` result owns rows.
#[test]
fn the_partial_corpus_types_every_source_and_enrichment_loss_without_failing() {
    let report = corpus("report_partial.json");
    let sources: Vec<(&str, SourceResultState, bool, bool, Option<u32>)> = report
        .sources
        .iter()
        .map(|result| {
            (
                result.source.package.as_str(),
                result.state.clone(),
                result.digest.is_some(),
                result.reason_code.is_some(),
                result.adoption_entries,
            )
        })
        .collect();
    assert_eq!(
        sources,
        [
            (
                "org.demo/host",
                SourceResultState::Available,
                true,
                false,
                None
            ),
            (
                "org.other/absent",
                SourceResultState::Orphaned,
                false,
                true,
                Some(2)
            ),
            (
                "org.stale/unreadable",
                SourceResultState::Unavailable,
                false,
                true,
                None
            ),
            (
                "org.vendor/tool",
                SourceResultState::Invalid,
                true,
                true,
                None
            ),
        ],
        "a malformed source is named as itself, and an orphan is a SOURCE observation"
    );
    // Every row belongs to the one available source; the orphan
    // contributed no fabricated `unmarked` fact.
    assert_eq!(report.rows.len(), 2);
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.source.package == "org.demo/host")
    );

    let relations: Vec<(&str, RelationSourceState, bool)> = report
        .relation_sources
        .iter()
        .map(|source| {
            (
                source.package.as_str(),
                source.state.clone(),
                source.reason_code.is_some(),
            )
        })
        .collect();
    assert_eq!(
        relations,
        [
            ("org.demo/host", RelationSourceState::Stale, true),
            ("org.other/absent", RelationSourceState::Unavailable, true),
            ("org.vendor/tool", RelationSourceState::Invalid, true),
        ]
    );
    assert_eq!(
        report.rows[0].relations.len(),
        1,
        "a stale source may still carry the edges it did produce"
    );
    assert!(
        report.rows[1].relations.is_empty(),
        "a row with no edges is still answered for"
    );
    assert!(
        report.observation.lifecycle_run_id.is_none(),
        "a project that never ran a phase still answers"
    );
}

#[test]
fn the_truncated_corpus_reaches_its_own_bound_inside_its_own_prefix() {
    let report = corpus("report_truncated.json");
    assert!(report.truncated);
    assert_eq!(report.query.limit, 2);
    assert_eq!(report.rows.len(), 2);
    let prefix = report.query.address_prefix.as_deref().unwrap();
    assert!(prefix.starts_with("spec://"));
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.address.starts_with(prefix))
    );
    // The hard maximum itself is arithmetic, not shape: the validator's
    // boundary arms own 256/257, and this corpus owns the wire form.
    assert!(report.query.limit <= LIMIT_MAX);
}

#[test]
fn law_labels_and_caps_match_the_schema() {
    let schema = read_json("schemas/requirements_report.jtd.json");
    let documented: BTreeSet<String> = schema["metadata"]["x-relational-laws"]
        .as_array()
        .expect("x-relational-laws is an array")
        .iter()
        .map(|law| {
            law.as_str()
                .expect("every law is a string")
                .split_once(':')
                .expect("every law is `label: sentence`")
                .0
                .to_string()
        })
        .collect();
    let implemented: BTreeSet<String> = IMPLEMENTED_LAWS.iter().map(|l| (*l).to_string()).collect();
    assert_eq!(
        documented, implemented,
        "law parity drift between the schema and behaviour::requirements_report"
    );
    assert_eq!(
        schema["metadata"]["x-diagnostic-cap-bytes"].as_u64(),
        Some(vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES as u64)
    );
    assert_eq!(
        schema["metadata"]["x-address-cap-bytes"].as_u64(),
        Some(ADDRESS_CAP_BYTES as u64)
    );
    assert_eq!(
        schema["metadata"]["x-limit-max"].as_u64(),
        Some(u64::from(LIMIT_MAX))
    );
}

#[test]
fn the_relation_vocabularies_are_the_specmap_engine_s_own_closed_sets() {
    let schema = read_json("schemas/requirements_report.jtd.json");
    let specmap: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(lock_selected_specmap_schema()).expect("readable"),
    )
    .expect("the lock-selected specmap schema parses");
    for (ours, theirs) in [
        (
            "/definitions/requirement_relation/properties/verb/enum",
            "/definitions/edge/properties/verb/enum",
        ),
        (
            "/definitions/requirement_relation/properties/provenance/enum",
            "/definitions/edge/properties/provenance/enum",
        ),
    ] {
        assert_eq!(
            enum_at(&schema, ours),
            enum_at(&specmap, theirs),
            "{ours} must be the engine's own closed set, verbatim"
        );
    }
    // The fact-status vocabulary is the PROP-043 markup domain, in the
    // progress order the markup fixes.
    assert_eq!(
        enum_at(&schema, "/definitions/fact_status/properties/stage/enum"),
        ["unknown", "idea", "spec", "impl", "test", "doc", "freeze"]
    );
    assert_eq!(
        enum_at(&schema, "/definitions/fact_status/properties/state/enum"),
        ["hold", "plan", "work", "done", "void"]
    );
    // Every enum site on this root is CLOSED — a requirements answer
    // with an unknown word is a reader error, not a newer writer.
    let mut open_sites = Vec::new();
    collect_enum_policies(&schema, &mut open_sites);
    assert!(
        open_sites.iter().all(|policy| policy == "closed"),
        "every requirements vocabulary is closed: {open_sites:?}"
    );
    assert_eq!(open_sites.len(), 10, "ten closed vocabularies on this root");
    // The base source layer's own closed set, which the correction
    // added: four states, four different instructions.
    assert_eq!(
        enum_at(&schema, "/definitions/source_result/properties/state/enum"),
        ["available", "unavailable", "invalid", "orphaned"]
    );
}

/// The `x-vocabulary` policy of every enum site in a document.
fn collect_enum_policies(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(fields) => {
            if fields.contains_key("enum") {
                out.push(
                    fields
                        .get("metadata")
                        .and_then(|metadata| metadata.get("x-vocabulary"))
                        .and_then(|policy| policy.as_str())
                        .unwrap_or("<missing>")
                        .to_string(),
                );
            }
            for field in fields.values() {
                collect_enum_policies(field, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_enum_policies(item, out);
            }
        }
        _ => {}
    }
}
