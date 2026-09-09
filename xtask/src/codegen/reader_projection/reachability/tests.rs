use super::*;
use serde_json::json;

#[test]
fn unused_direct_and_transitive_pure_aliases_stay_unreachable() {
    let doc = json!({
        "properties": {"live": {"type": "string"}},
        "definitions": {
            "direct": {"ref": "shared"},
            "first": {"ref": "second"},
            "second": {"ref": "shared"}
        }
    });
    assert!(reachable_definition_names(&doc).is_empty());
}

#[test]
fn root_and_public_non_alias_readers_reach_aliases_transitively() {
    let doc = json!({
        "properties": {"root_alias": {"ref": "root_target"}},
        "definitions": {
            "root_target": {"ref": "shared"},
            "public_wrapper": {"properties": {"value": {"ref": "wrapped_alias"}}},
            "wrapped_alias": {"ref": "shared"},
            "unused": {"ref": "shared"}
        }
    });
    assert_eq!(
        reachable_definition_names(&doc),
        BTreeSet::from(["root_target".to_owned(), "wrapped_alias".to_owned()])
    );
}

#[test]
fn reachable_cycles_terminate_and_dangling_refs_invent_no_definition() {
    let doc = json!({
        "properties": {
            "cycle": {"ref": "left"},
            "missing": {"ref": "absent"}
        },
        "definitions": {
            "left": {"ref": "right"},
            "right": {"ref": "left"},
            "dead": {"ref": "dead"}
        }
    });
    assert_eq!(
        reachable_definition_names(&doc),
        BTreeSet::from(["left".to_owned(), "right".to_owned()])
    );
}
