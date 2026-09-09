//! Local-definition reachability for transparent compatibility aliases.

use std::collections::BTreeSet;

use serde_json::Value;

pub(super) fn is_pure_ref(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.get("ref").is_some_and(Value::is_string))
}

fn collect_refs(value: &Value, skip_definitions: bool, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object.get("ref").and_then(Value::as_str) {
                names.insert(name.to_owned());
            }
            for (key, child) in object {
                if skip_definitions && key == "definitions" {
                    continue;
                }
                collect_refs(child, false, names);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_refs(child, false, names);
            }
        }
        _ => {}
    }
}

pub(super) fn reachable_definition_names(doc: &Value) -> BTreeSet<String> {
    let Some(definitions) = doc.get("definitions").and_then(Value::as_object) else {
        return BTreeSet::new();
    };
    let mut pending = BTreeSet::new();
    collect_refs(doc, true, &mut pending);
    for definition in definitions.values().filter(|value| !is_pure_ref(value)) {
        collect_refs(definition, false, &mut pending);
    }
    pending.retain(|name| definitions.contains_key(name));
    let mut reached = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if !reached.insert(name.clone()) {
            continue;
        }
        let mut dependencies = BTreeSet::new();
        collect_refs(&definitions[&name], false, &mut dependencies);
        pending.extend(
            dependencies
                .into_iter()
                .filter(|dependency| definitions.contains_key(dependency)),
        );
    }
    reached
}

#[cfg(test)]
#[path = "reachability/tests.rs"]
mod tests;
