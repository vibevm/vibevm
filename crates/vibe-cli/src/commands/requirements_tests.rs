//! The `vibe requirements` grammar reds, the lifecycle join's truth
//! table, and the surface fence (PROP-054 `##FACT-QUERY-CONTRACT` /
//! `##REF-REQUIREMENTS-SURFACES`).
//!
//! The fence lives in its own cell for a reason a whole-file fence
//! cannot survive: a test that `include_str!`s the file it is written
//! inside would match its own needles and pass for the wrong reason.

use std::collections::BTreeMap;

use clap::Parser;
use specmark::verifies;
use vibe_wire::generated::lifecycle_state::{LifecycleState, StateRun};

use super::joined_run_id;
use crate::cli::{Cli, Command};

const RUN_ID: &str = "0123456789abcdef0123456789abcdef";

/// The production source of the two files that make up this surface,
/// stripped of comments and doc comments. The fence judges CODE: a
/// needle that only ever appears in prose explaining why it is absent
/// would otherwise turn a true fence red.
fn surface_code() -> String {
    let mut code = String::new();
    for source in [
        include_str!("requirements.rs"),
        include_str!("../cli/requirements.rs"),
    ] {
        for line in source.lines() {
            if !line.trim_start().starts_with("//") {
                code.push_str(line);
                code.push('\n');
            }
        }
    }
    code
}

fn parse(argv: &[&str]) -> Cli {
    Cli::try_parse_from(argv).unwrap_or_else(|error| panic!("parse `{argv:?}`: {error}"))
}

fn requirements_args(cli: Cli) -> crate::cli::RequirementsArgs {
    let Command::Requirements(args) = cli.command else {
        panic!("argv did not parse to `requirements`");
    };
    args
}

/// The defaults an answer must be reconstructible from: no prefix, the
/// library's own row bound, no enrichment, the current directory.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-REQUIREMENTS-SURFACES")]
fn the_bare_verb_carries_the_librarys_defaults() {
    let args = requirements_args(parse(&["vibe", "requirements"]));
    assert!(args.address_prefix.is_none());
    assert_eq!(args.limit, 100, "the default row bound is 100");
    assert_eq!(
        args.limit,
        vibe_requirements::RequirementsQuery::default().limit(),
        "the CLI default IS the library's default, not a second copy of it",
    );
    assert!(!args.relations, "enrichment is opt-in");
    assert_eq!(args.path.to_string_lossy(), ".");
}

/// The exact four flags, and the two global ones, reaching the command.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-REQUIREMENTS-SURFACES")]
fn the_exact_flags_parse_together() {
    let cli = parse(&[
        "vibe",
        "--json",
        "requirements",
        "--address-prefix",
        "spec://org.example/demo",
        "--limit",
        "7",
        "--relations",
        "--path",
        "sub",
    ]);
    assert!(cli.json, "--json reaches the root");
    let args = requirements_args(cli);
    assert_eq!(
        args.address_prefix.as_deref(),
        Some("spec://org.example/demo")
    );
    assert_eq!(args.limit, 7);
    assert!(args.relations);
    assert_eq!(args.path.to_string_lossy(), "sub");

    let cli = parse(&["vibe", "--quiet", "requirements"]);
    assert!(cli.quiet, "--quiet reaches the root");
}

/// Unknown options and non-numeric limits never reach a filesystem:
/// clap refuses them at parse time. The spellings are exact — a
/// `--prefix` or `--relation` is not this grammar.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#REF-REQUIREMENTS-SURFACES")]
fn unknown_options_and_wrong_types_refuse_at_parse_time() {
    for argv in [
        vec!["vibe", "requirements", "--evidence"],
        vec![
            "vibe",
            "requirements",
            "--prefix",
            "spec://org.example/demo",
        ],
        vec!["vibe", "requirements", "--relation"],
        vec!["vibe", "requirements", "--limit", "many"],
        vec!["vibe", "requirements", "--limit", "-1"],
        // No positional argument exists on this verb.
        vec!["vibe", "requirements", "spec://org.example/demo"],
    ] {
        assert!(
            Cli::try_parse_from(&argv).is_err(),
            "`{argv:?}` must be refused by the grammar",
        );
    }
}

/// The RANGE and the PREFIX shape are the library's law, not clap's:
/// the grammar accepts the values and the one constructor refuses them.
/// That is what keeps the CLI and the MCP tool on a single grammar.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_range_and_prefix_laws_belong_to_the_one_constructor() {
    for (limit, prefix) in [
        (0_u32, None),
        (257, None),
        (100, Some("req-one")),
        (100, Some("spec:/org.example/demo")),
    ] {
        let mut argv = vec![
            "vibe".to_string(),
            "requirements".to_string(),
            "--limit".to_string(),
            limit.to_string(),
        ];
        if let Some(prefix) = prefix {
            argv.push("--address-prefix".to_string());
            argv.push(prefix.to_string());
        }
        let args = requirements_args(parse(&argv.iter().map(String::as_str).collect::<Vec<_>>()));
        assert_eq!(args.limit, limit, "clap carries the value through");
        assert!(
            vibe_requirements::RequirementsQuery::try_new(
                args.address_prefix.as_deref(),
                args.limit,
                args.relations,
            )
            .is_err(),
            "the constructor refuses limit={limit} prefix={prefix:?}",
        );
    }
}

fn state(selected: Option<&str>, run_id: Option<&str>) -> LifecycleState {
    LifecycleState {
        execution: BTreeMap::new(),
        run: StateRun {
            chain: vec!["validate".to_string()],
            requested: "validate".to_string(),
            started: "2026-01-01T00:00:00Z".to_string(),
            compile_trace: false,
            run_id: run_id.map(str::to_string),
            selected: selected.map(str::to_string),
            slot_continuation: None,
        },
        schema: 1,
    }
}

/// The join key is carried for exactly one node — the one this answer is
/// about. A sibling member's run, a legacy header with no node identity
/// and a header with no id all answer the same way: no key.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_run_id_joins_only_the_same_selected_node() {
    assert_eq!(
        joined_run_id(&state(Some("."), Some(RUN_ID)), "."),
        Some(RUN_ID.to_string()),
        "the same node's run is the join key",
    );
    assert_eq!(
        joined_run_id(&state(Some("members/tool"), Some(RUN_ID)), "."),
        None,
        "a sibling member's run is a different node's evidence",
    );
    assert_eq!(
        joined_run_id(&state(Some("."), Some(RUN_ID)), "members/tool"),
        None,
        "…and the mismatch refuses in both directions",
    );
    assert_eq!(
        joined_run_id(&state(None, Some(RUN_ID)), "."),
        None,
        "a header with no node identity cannot claim this node",
    );
    assert_eq!(
        joined_run_id(&state(Some("."), None), "."),
        None,
        "a matching node with no run id has no key to offer",
    );
}

/// The composition fence: exactly one call to the one query, one shared
/// text projection, and one relation-provider injection site. A second
/// of any of them is a second answer to the same question.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_surface_calls_the_one_query_exactly_once() {
    let code = surface_code();
    for (needle, what) in [
        ("vibe_requirements::query(", "the one report constructor"),
        (
            "vibe_requirements::text::render(",
            "the shared text projection",
        ),
        ("SpecmapRelationProvider", "the relation-provider injection"),
        ("LifecycleStateStore::peek(", "the read-only state peek"),
        (
            "Workspace::discover_selected(",
            "the one read-only discovery",
        ),
    ] {
        assert_eq!(
            code.matches(needle).count(),
            1,
            "{what} must appear exactly once in the surface: `{needle}`",
        );
    }
}

/// «Pass `None` when `relations = false`» has no behavioural signature
/// at this layer: the library short-circuits on the effective query and
/// would never call an injected provider anyway, so a surface that
/// injected unconditionally would still answer `not-requested`. The law
/// is therefore proven structurally — the injection site is decided
/// before the query is asked, it is conditioned on the effective query,
/// and its false arm hands the library `None`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_relation_provider_is_injected_only_when_the_query_asked() {
    let code = surface_code();
    let site = code
        .find("SpecmapRelationProvider")
        .expect("the injection site");
    let call = code
        .find("vibe_requirements::query(")
        .expect("the one query call");
    assert!(
        site < call,
        "the provider is decided before the query is asked",
    );
    let region = &code[site..call];
    assert!(
        region.contains("query.relations()"),
        "the injection is conditioned on the effective query: {region}",
    );
    assert!(
        region.contains("None"),
        "…and its false arm hands the library `None`: {region}",
    );
}

/// The surface assembles nothing. Every report member, every source
/// read and every adoption join belongs below the query; a name from
/// that layer appearing here means the composition crept into the
/// surface.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_surface_assembles_no_report_member_and_reads_no_source() {
    let code = surface_code();
    for needle in [
        "RequirementsObservation",
        "RequirementRow",
        "RequirementRelation",
        "RelationSource",
        "SourceResult",
        "vibe_facts",
        "join_adoption",
        "load_with_witnesses",
        "specmap_core",
        "observation_id",
        "source_digest",
        // No filesystem of its own: the surface names paths, never
        // reads or writes them.
        "std::fs",
        "fs::read",
        "fs::write",
        "create_dir",
        "File::",
    ] {
        assert!(
            !code.contains(needle),
            "`{needle}` belongs below the query, never in the surface",
        );
    }
}

/// The read-only and credential-free fence. `vibe-cli` legitimately
/// carries an LLM edge for the create phase, so the fence is scoped to
/// THIS surface: the requirements path is algorithmic, network-free and
/// never begins, leases or writes lifecycle state.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FACT-QUERY-CONTRACT")]
fn the_surface_has_no_llm_credential_or_write_edge() {
    let code = surface_code();
    for needle in [
        "vibe_llm",
        "LlmProvider",
        "api_key",
        "token",
        "reqwest",
        "http",
        "model",
        // Nothing on this path may begin, adopt, lease or checkpoint a
        // run, nor sync or materialise anything.
        "LifecycleStateStore::begin",
        "LifecycleLease",
        "acquire",
        "checkpoint",
        "materialise",
        "materialize",
        "sync",
    ] {
        assert!(
            !code.contains(needle),
            "`{needle}` has no place on a read-only, credential-free surface",
        );
    }
}
