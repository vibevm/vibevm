//! BUILTIN producer oracle — QUALIFY SPELLING (`x-corpus-producer-oracles`),
//! deliberately NOT a conversion gate.
//!
//! What THIS corpus's builtin `qualify` wrote, and only that. A plugin may
//! legally return a verifier-valid closure whose anchors the builtin would
//! have spelled otherwise, and the R6.3 decoder must accept it — so nothing
//! here may become a decode rule. The mandatory gates live in
//! `compiler_ir_conversion_gates.rs` and `compiler_ir_domain_invariants.rs`;
//! the CLOSE ORDER oracle in `compiler_ir_producer_laws.rs`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::{
    AbsorptionState, ClosureIr, CompileMode, ContributionAbsorption, DocumentAddress, Ir,
};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn typed<T: DeserializeOwned + Serialize>(value: &serde_json::Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

fn closures() -> Vec<(String, ClosureIr)> {
    valid_names()
        .iter()
        .filter_map(|name| match typed::<Ir>(&raw(name)) {
            Ir::ClosureArtifact(arm) => Some((name.clone(), arm.closure)),
            _ => None,
        })
        .collect()
}

// ── ORACLE · QUALIFY SPELLING ────────────────────────────────────────────────

/// `compiler/qualify.rs::node_qualification_origin`: a graph node qualifies
/// under its bare origin ONLY for a `boot/` / `contract/` / empty doc path.
fn qualification_origin(origin: &str, doc_path: &str) -> String {
    if doc_path.starts_with("boot/") || doc_path.starts_with("contract/") || doc_path.is_empty() {
        origin.to_string()
    } else {
        format!("{origin}/{}", doc_path.replace('/', "."))
    }
}

/// `qualify.rs::origin_slug` (lib): first whitespace token, lowercased, `.`
/// to `-`, the `/` joiner to `--`.
fn origin_slug(origin: &str) -> String {
    origin
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .replace('.', "-")
        .replace('/', "--")
}

/// The node ids qualify VISITED, in visit order (`qualify.rs:214-217`): the
/// applied plan's occurrences, skipping an absorbed or already-seen node.
fn visited_nodes(closure: &ClosureIr) -> Vec<u32> {
    let AbsorptionState::Applied(state) = &closure.absorption else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for entry in &state.plan.contributions {
        let ContributionAbsorption::Normal(normal) = entry else {
            continue;
        };
        for occurrence in &normal.occurrences {
            if !occurrence.absorbed && seen.insert(occurrence.node) {
                out.push(occurrence.node);
            }
        }
    }
    out
}

/// What the builtin qualify wrote, and ONLY that. Nothing is claimed about a
/// node qualify skipped: an absorbed or already-seen occurrence keeps whatever
/// its author wrote, and an author may perfectly well have written something
/// slug-shaped. A reverse assertion there would be a false law.
fn qualify_violations(closure: &ClosureIr) -> Vec<String> {
    if !matches!(closure.context.mode, CompileMode::QualifyPerNode) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut audit: Vec<(String, String, String)> = Vec::new();
    for index in visited_nodes(closure) {
        let Some(node) = closure.nodes.get(index as usize) else {
            out.push(format!("visited node {index} is out of the arena"));
            continue;
        };
        let DocumentAddress::Spec(spec) = &node.address else {
            continue;
        };
        let slug = origin_slug(&qualification_origin(&node.origin, &spec.address.doc_path));
        let prefix = format!("{slug}--");
        for anchor in node.tree.anchors.keys() {
            if !anchor.starts_with(&prefix) {
                out.push(format!(
                    "visited node {index}: anchor `{anchor}` is not qualified under `{slug}`"
                ));
            }
        }
        // `rename_audit` takes each visited node's renames in DOCUMENT order,
        // which is the arena order the parser minted.
        for entry in node.tree.nodes.iter().skip(1) {
            let Some(anchor) = entry.id.as_deref() else {
                continue;
            };
            if let Some(original) = anchor.strip_prefix(&prefix) {
                audit.push((
                    node.origin.clone(),
                    original.to_string(),
                    anchor.to_string(),
                ));
            }
        }
    }
    let actual: Vec<(String, String, String)> = closure
        .renames
        .iter()
        .map(|entry| {
            (
                entry.origin.clone(),
                entry.rename.original.clone(),
                entry.rename.qualified.clone(),
            )
        })
        .collect();
    if actual != audit {
        out.push(format!(
            "rename audit is {actual:?}, builtin qualify would write {audit:?}"
        ));
    }
    out
}

#[test]
fn the_corpus_anchors_and_rename_audit_are_builtin_qualify_output() {
    let mut checked = 0;
    for (name, closure) in closures() {
        assert_eq!(qualify_violations(&closure), Vec::<String>::new(), "{name}");
        if !matches!(closure.context.mode, CompileMode::QualifyPerNode) {
            continue;
        }
        checked += 1;
        // The doc-path component is what keeps two documents of ONE package
        // from minting the same bare label.
        for node in &closure.nodes {
            let DocumentAddress::Spec(spec) = &node.address else {
                continue;
            };
            let slug = origin_slug(&qualification_origin(&node.origin, &spec.address.doc_path));
            assert_ne!(
                slug,
                origin_slug(&node.origin),
                "{name}: a `manual/` document never qualifies under its bare origin"
            );
        }
        assert_eq!(
            visited_nodes(&closure).len(),
            closure.nodes.len() - 1,
            "{name}: exactly one node was absorbed before qualify ran"
        );
        assert_eq!(closure.renames.len(), 3, "{name}: the audit is pinned");
    }
    assert_eq!(
        checked, 1,
        "exactly one qualified closure carries the oracle"
    );
}

#[test]
fn an_author_written_slug_on_an_absorbed_node_is_accepted() {
    // The false reverse this replaces: qualify never visited the absorbed
    // twin, so it has NO opinion on how that node's author spelled its
    // anchors — a slug-looking authored label is perfectly legal input.
    let mut document = raw("closure_artifact.json");
    let looks_like = "org-demo--lib--manual-part-md--api";
    let node = &mut document["closure"]["nodes"][1];
    node["tree"]["nodes"][1]["id"] = serde_json::json!(looks_like);
    node["tree"]["lines"][0] = serde_json::json!(format!("# Part {{#{looks_like}}}"));
    node["tree"]["anchors"] = serde_json::json!({ looks_like: 1 });

    let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
        panic!("the mutated document is still a closure");
    };
    assert_eq!(
        qualify_violations(&arm.closure),
        Vec::<String>::new(),
        "an absorbed node's authored spelling is never the oracle's business"
    );
}

#[test]
fn a_visited_node_or_a_wrong_audit_is_red() {
    // A VISITED node carrying the origin-only slug — the spelling the repaired
    // corpus replaced, which lets a sibling document's `#root` collide.
    let mut document = raw("closure_artifact.json");
    let flat = "org-demo--lib--root";
    let node = &mut document["closure"]["nodes"][2];
    let old = node["tree"]["nodes"][1]["id"].as_str().unwrap().to_string();
    node["tree"]["nodes"][1]["id"] = serde_json::json!(flat);
    let anchors = node["tree"]["anchors"].as_object_mut().unwrap();
    anchors.remove(&old);
    anchors.insert(flat.to_string(), serde_json::json!(1));
    let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
        panic!("the mutated document is still a closure");
    };
    assert!(
        qualify_violations(&arm.closure)
            .iter()
            .any(|entry| entry.contains("is not qualified under")),
        "the origin-only slug on a visited node must be red"
    );

    // A rename audit that does not match what qualify would have written:
    // reordered, and with one entry dropped.
    for mutate in [
        |renames: &mut Vec<serde_json::Value>| renames.reverse(),
        |renames: &mut Vec<serde_json::Value>| {
            renames.pop();
        },
    ] {
        let mut document = raw("closure_artifact.json");
        let mut renames = document["closure"]["renames"].as_array().unwrap().clone();
        mutate(&mut renames);
        document["closure"]["renames"] = serde_json::Value::Array(renames);
        let Ir::ClosureArtifact(arm) = typed::<Ir>(&document) else {
            panic!("the mutated document is still a closure");
        };
        assert!(
            qualify_violations(&arm.closure)
                .iter()
                .any(|entry| entry.contains("rename audit is")),
            "a rename audit that is not qualify's own must be red"
        );
    }
}
