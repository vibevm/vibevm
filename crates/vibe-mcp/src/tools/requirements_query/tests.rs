//! The `requirements_query` argument law, composition and fences.
//!
//! The library owns the report; these cells own exactly what this surface
//! adds — the strict grammar and its zero-mutation promise, the trusted
//! selected-node and lifecycle-join composition, the provider injection
//! law, and the one-query / no-LLM fences.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use vibe_wire::generated::requirements_report::{RelationSourceProvenance, RelationSourceState};

use super::*;

/// The generated root, aliased for readability. This surface never
/// CONSTRUCTS one — it deserialises exactly what the library produced,
/// which is the law `vibe-requirements`' own cross-crate fence keeps over
/// this tree.
type Report = vibe_wire::generated::requirements_report::RequirementsReport;

/// One authored, addressed, status-carrying fact — and one sentence of
/// prose the bounded projection must never carry across.
const ONE_FACT: &str = "# Rules\n\n@fact:FIRST The one authored sentence. @status:impl/done\n";
const PROSE: &str = "The one authored sentence";
const ADDRESS: &str = "spec://org.example/demo/RULE#FIRST";
const HOST: &str = "org.example/demo";
const RUN_ID: &str = "0123456789abcdef0123456789abcdef";

/// A minimal standalone project: host `org.example/demo`, one spec
/// document, no `.vibe`, no lock, no registry, no specmap config.
fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("RULE.md"), ONE_FACT).unwrap();
    root
}

fn call(root: &Path, args: Value) -> Result<ToolOutput, ToolError> {
    RequirementsQueryMcpTool.run(&args, &ServerContext::new(root))
}

fn report_of(output: &ToolOutput) -> Report {
    serde_json::from_value(output.structured().clone()).expect("the generated root round-trips")
}

/// Every file under `root`, project-relative — the oracle a refusal may
/// not move.
fn tree(root: &Path) -> BTreeSet<String> {
    fn walk(base: &Path, at: &Path, into: &mut BTreeSet<String>) {
        for entry in fs::read_dir(at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, into);
            } else {
                into.insert(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut snapshot = BTreeSet::new();
    walk(root, root, &mut snapshot);
    snapshot
}

/// A durable lifecycle header naming `selected`, with no execution rows —
/// the shape `LifecycleStateStore::peek` reads and validates.
fn write_state(root: &Path, selected: &str) {
    let vibe = root.join(".vibe");
    fs::create_dir_all(&vibe).unwrap();
    fs::write(
        vibe.join("lifecycle.toml"),
        format!(
            "schema = 1\n\n[execution]\n\n[run]\nchain = [\"validate\"]\n\
             requested = \"validate\"\nstarted = \"2026-01-01T00:00:00Z\"\n\
             run_id = \"{RUN_ID}\"\nselected = \"{selected}\"\n"
        ),
    )
    .unwrap();
}

// --- the descriptor ----------------------------------------------------

/// The advertised grammar is exactly the three optional members of R7
/// architecture §6.2 — and, load-bearingly, NOT a path: a `path` member
/// would let any caller point this server's read at another tree.
#[test]
fn the_descriptor_is_three_optional_members_and_no_path() {
    let descriptor = RequirementsQueryMcpTool.descriptor();
    assert_eq!(descriptor.name, "requirements_query");

    let schema = &descriptor.input_schema;
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    // No `required` member at all: `{}` is a complete call.
    assert!(schema["required"].is_null(), "no argument may be required");

    let properties = schema["properties"].as_object().expect("properties object");
    let names: BTreeSet<&str> = properties.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        BTreeSet::from(["address_prefix", "limit", "relations"]),
        "exactly the three optional members — no path, provider, model, \
         lifecycle, sync or write option",
    );
    // Each one documents itself, including its default/cap.
    for name in names {
        let description = properties[name]["description"].as_str().unwrap_or_default();
        assert!(
            description.contains("Default:"),
            "`{name}` must document its default"
        );
    }
}

/// Descriptor and runtime read the SAME defaults and caps. Stated as an
/// equality against the library's own value rather than against retyped
/// literals: a change to `RequirementsQuery::default()` or to the wire
/// owner's bounds must land in both or in neither.
#[test]
fn the_descriptor_defaults_and_caps_are_the_library_and_wire_owners_own() {
    let defaults = RequirementsQuery::default();
    let schema = RequirementsQueryMcpTool.descriptor().input_schema;

    assert_eq!(schema["properties"]["limit"]["default"], defaults.limit());
    assert_eq!(schema["properties"]["limit"]["default"], default_limit());
    assert_eq!(schema["properties"]["limit"]["minimum"], LIMIT_MIN);
    assert_eq!(schema["properties"]["limit"]["maximum"], LIMIT_MAX);
    assert_eq!(
        schema["properties"]["relations"]["default"],
        defaults.relations()
    );
    assert_eq!(
        schema["properties"]["relations"]["default"],
        default_relations()
    );
    assert_eq!(
        schema["properties"]["address_prefix"]["maxLength"],
        ADDRESS_CAP_BYTES
    );
    // The prefix default is ABSENCE, so the schema states no `default`.
    assert!(defaults.address_prefix().is_none());
    assert!(schema["properties"]["address_prefix"]["default"].is_null());

    // And the runtime's own decoded defaults are those same values.
    let decoded = parse_query(&json!({})).unwrap();
    assert_eq!(decoded, defaults);
}

// --- the argument law --------------------------------------------------

/// The strict refusal matrix, each case proved against the project tree
/// itself: an unacceptable question must not have created `.vibe`, a
/// lock, a state file or any other byte. Type, unknown member, range and
/// prefix all refuse in the same place — before the first directory read.
#[test]
fn every_unacceptable_argument_refuses_before_any_filesystem_or_state_byte() {
    let project = project();
    let before = tree(project.path());

    for arguments in [
        // The retargeting member the grammar deliberately lacks — alone,
        // and smuggled beside a member that IS valid.
        json!({ "path": "/elsewhere" }),
        json!({ "limit": 100, "path": "." }),
        // Every other option the contract refuses to own.
        json!({ "provider": "some-provider" }),
        json!({ "model": "some-model" }),
        json!({ "sync": true }),
        json!({ "lifecycle_evidence": true }),
        // Wrong types.
        json!({ "limit": "100" }),
        json!({ "limit": 1.5 }),
        json!({ "relations": "true" }),
        json!({ "address_prefix": 7 }),
        // Range: the inclusive 1..=256 bound, from both sides.
        json!({ "limit": 0 }),
        json!({ "limit": 257 }),
        json!({ "limit": -1 }),
        // Prefix grammar: a bare id is not a `spec://` URI, and an unsafe
        // prefix is refused even though it wears the scheme.
        json!({ "address_prefix": "req-one" }),
        json!({ "address_prefix": "spec://org.example\\demo" }),
        // Not an object at all.
        json!("relations"),
        json!([]),
        json!(7),
    ] {
        let error = call(project.path(), arguments.clone())
            .err()
            .unwrap_or_else(|| panic!("`{arguments}` must refuse"));
        assert!(
            matches!(error, ToolError::InvalidArguments(_)),
            "`{arguments}` refuses as an argument error, got: {error}"
        );
        assert_eq!(
            tree(project.path()),
            before,
            "`{arguments}` moved a byte in the project tree"
        );
        assert!(
            !project.path().join(".vibe").exists(),
            "`{arguments}` created `.vibe`"
        );
    }
}

/// The accepted half of the same law: omitted arguments, an explicit
/// empty object, a null `arguments` and both range endpoints all answer.
#[test]
fn the_accepted_grammar_covers_absence_null_and_both_range_endpoints() {
    let project = project();
    for arguments in [
        Value::Null,
        json!({}),
        json!({ "limit": LIMIT_MIN }),
        json!({ "limit": LIMIT_MAX }),
        json!({ "address_prefix": "spec://org.example/demo" }),
        json!({ "relations": false }),
    ] {
        let output = call(project.path(), arguments.clone())
            .unwrap_or_else(|error| panic!("`{arguments}` must answer, got: {error}"));
        assert!(!output.is_error(), "`{arguments}` is a success");
    }
    // A limit of 1 over a one-row project is not truncation.
    let report = report_of(&call(project.path(), json!({ "limit": 1 })).unwrap());
    assert!(!report.truncated);
    assert_eq!(report.query.limit, 1);
}

// --- the composition ---------------------------------------------------

/// The one-fact fixture end to end: `structuredContent` is EXACTLY the
/// generated root the library produces for the same question over the
/// same node, and the text channel is exactly the library's own bounded
/// projection — no second assembly, and no prose smuggled into either.
#[test]
fn the_one_fact_fixture_returns_the_exact_generated_root_and_the_bounded_text() {
    let project = project();
    let output = call(project.path(), json!({})).unwrap();
    assert!(!output.is_error());

    let report = report_of(&output);
    // The oracle: the SAME call the library would have answered, at the
    // clock the surface injected (which is excluded from the identity).
    let oracle = vibe_requirements::query(
        &RequirementsQuery::default(),
        &QueryContext {
            selected_root: project.path().to_path_buf(),
            observed_at: report.observation.observed_at,
            lifecycle_run_id: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(
        output.structured(),
        &serde_json::to_value(&oracle).unwrap(),
        "structuredContent is the generated root, member for member",
    );
    assert_eq!(
        output.text(),
        vibe_requirements::text::render(&oracle),
        "the text channel is the library's own projection",
    );

    // The observation itself, stated once so a silent emptying is red.
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].address, ADDRESS);
    assert_eq!(report.sources.len(), 1);
    assert_eq!(report.sources[0].source.package, HOST);
    assert!(report.observation.lifecycle_run_id.is_none());

    // Bounded means bounded: the address and its four observation columns
    // cross, the authored SENTENCE does not.
    assert!(output.text().contains(ADDRESS));
    assert!(output.text().contains("authoring=marked=impl/done"));
    assert!(output.text().contains("adoption=not-applicable"));
    assert!(
        !output.text().contains(PROSE),
        "the bounded projection carried fact prose: {}",
        output.text()
    );
    assert!(
        !output.structured().to_string().contains(PROSE),
        "the structured root carried fact prose",
    );
}

/// `relations = false` — the default — is the honest statement that no
/// map was loaded: an explicit `not-requested/none` row per enumerated
/// source, no edges, and nothing built on disk.
#[test]
fn relations_default_false_is_not_requested_per_source_and_builds_no_map() {
    let project = project();
    let report = report_of(&call(project.path(), json!({})).unwrap());

    assert!(!report.query.relations);
    assert_eq!(report.relation_sources.len(), 1);
    assert_eq!(report.relation_sources[0].package, HOST);
    assert_eq!(
        report.relation_sources[0].state,
        RelationSourceState::NotRequested
    );
    assert_eq!(
        report.relation_sources[0].provenance,
        RelationSourceProvenance::None
    );
    assert!(report.relation_sources[0].reason_code.is_none());
    assert!(report.rows.iter().all(|row| row.relations.is_empty()));
    assert!(!project.path().join("specmap.json").exists());
}

/// `relations = true` over a project carrying no map: the provider is
/// injected and answers a TYPED loss, never a false zero-edge success —
/// and the base fact rows still return, because enrichment may not speak
/// for the layer above it.
#[test]
fn relations_true_without_a_map_is_typed_unavailable_and_rows_still_return() {
    let project = project();
    let report = report_of(&call(project.path(), json!({ "relations": true })).unwrap());

    assert!(report.query.relations);
    assert_eq!(report.relation_sources.len(), 1);
    assert_eq!(
        report.relation_sources[0].state,
        RelationSourceState::Unavailable
    );
    assert_eq!(
        report.relation_sources[0].provenance,
        RelationSourceProvenance::None
    );
    assert!(
        report.relation_sources[0].reason_code.is_some(),
        "a loss state names a bounded machine reason"
    );
    // The base layer is untouched by the enrichment loss.
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].address, ADDRESS);
    assert!(report.rows[0].relations.is_empty());
    assert!(!project.path().join("specmap.json").exists());
}

/// The lifecycle join is by NODE IDENTITY, not by presence. A durable run
/// header naming this selected node contributes its id; one naming a
/// sibling does not — and because the id participates in
/// `observation_id`, borrowing it would mint a different identity, which
/// is exactly what the third assertion pins.
#[test]
fn the_lifecycle_run_id_joins_only_when_the_stored_run_names_this_node() {
    let project = project();

    write_state(project.path(), ".");
    let joined = report_of(&call(project.path(), json!({})).unwrap());
    assert_eq!(
        joined.observation.lifecycle_run_id.as_deref(),
        Some(RUN_ID),
        "the run header names this selected node",
    );

    write_state(project.path(), "members/sibling");
    let excluded = report_of(&call(project.path(), json!({})).unwrap());
    assert!(
        excluded.observation.lifecycle_run_id.is_none(),
        "a sibling's run is not this node's join key",
    );

    assert_ne!(
        joined.observation.observation_id, excluded.observation.observation_id,
        "the run join key participates in the observation identity",
    );
    // Read-only throughout: peeking never began, adopted, leased or wrote.
    assert!(!project.path().join(".vibe/lifecycle.lock").exists());
}

// --- registration and fences -------------------------------------------

/// One registration, at the single point, and no second requirements /
/// evidence tool beside it — `lifecycle_run({phase:"verify"})` already
/// returns the generated evidence member.
#[test]
fn the_tool_is_registered_exactly_once_at_the_single_default_tools_point() {
    let names: Vec<String> = crate::tools::default_tools()
        .iter()
        .map(|tool| tool.descriptor().name)
        .collect();
    assert_eq!(
        names
            .iter()
            .filter(|name| name.as_str() == "requirements_query")
            .count(),
        1,
        "registered exactly once: {names:?}"
    );
    assert!(
        !names.iter().any(|name| name.contains("lifecycle_evidence")),
        "no second evidence surface: {names:?}"
    );
    assert!(
        !names.iter().any(|name| name.contains("requirements_sync")),
        "no write/sync sibling: {names:?}"
    );
}

/// The surface fence: ONE call into the query library, no write or lease
/// vocabulary, no LLM/network vocabulary, and a manifest whose new edge is
/// the requirements library rather than a provider or an HTTP client.
#[test]
fn the_surface_calls_one_query_and_the_dependency_floor_holds() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest.join("src/tools/requirements_query.rs")).unwrap();

    assert_eq!(
        source.matches("vibe_requirements::query(").count(),
        1,
        "the surface calls the one query exactly once",
    );
    for forbidden in [
        // Read-only: no writer, no lease, no state mutation.
        "fs::write",
        "fs::create_dir_all",
        "File::create",
        "fs::copy",
        "fs::remove",
        "LifecycleStateStore::begin",
        "peek_with_lease",
        "lease_default_lifecycle",
        "retain_lease",
        // Algorithmic: no model, no transport, no secret.
        "vibe_llm",
        "InferenceBackend",
        "reqwest",
        "api_key",
        "Bearer",
        "http://",
    ] {
        assert!(
            !source.contains(forbidden),
            "the surface reached for `{forbidden}`"
        );
    }

    let cargo = fs::read_to_string(manifest.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("vibe-requirements.workspace = true"));
    for forbidden in ["vibe-llm", "reqwest", "hyper", "tokio"] {
        assert!(
            !cargo.contains(forbidden),
            "a direct `{forbidden}` edge appeared on the MCP crate"
        );
    }
}
