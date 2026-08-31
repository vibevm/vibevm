//! The gate registry is pinned to the schema's named `x-conversion-gates`
//! set: an undocumented gate and an unimplemented named gate are both red.

use specmark::verifies;
use std::path::PathBuf;

use super::super::CONVERSION_GATES;

fn schema() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/vocabularies.json")
}

fn ir_vocabulary(doc: &serde_json::Value) -> &serde_json::Value {
    &doc["ir"]
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_gate_registry_is_the_schema_gate_set_in_order() {
    let text = std::fs::read_to_string(schema()).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let gates = ir_vocabulary(&doc)["metadata"]["x-conversion-gates"]
        .as_array()
        .expect("the schema names its conversion gates");
    assert_eq!(
        gates.len(),
        CONVERSION_GATES.len(),
        "the implemented registry covers exactly the schema's gate list"
    );
    for (index, (gate, spec)) in gates.iter().zip(CONVERSION_GATES.iter()).enumerate() {
        let entry = gate.as_str().unwrap();
        assert!(
            entry.contains(spec.probe),
            "gate {index} (`{}`) must be the one `{}` implements",
            spec.label,
            spec.probe
        );
    }
    let mut labels: Vec<_> = CONVERSION_GATES.iter().map(|gate| gate.label).collect();
    labels.sort();
    labels.dedup();
    assert_eq!(
        labels.len(),
        CONVERSION_GATES.len(),
        "gate labels are unique"
    );
}

/// The producer-oracle set is NOT part of the registry: CLOSE ORDER, QUALIFY
/// SPELLING and OPAQUE TAPE bind nothing the decoder owes.
#[test]
fn no_producer_oracle_leaked_into_the_gate_registry() {
    let text = std::fs::read_to_string(schema()).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let oracles = ir_vocabulary(&doc)["metadata"]["x-corpus-producer-oracles"]
        .as_array()
        .unwrap();
    let labels: Vec<String> = CONVERSION_GATES
        .iter()
        .map(|gate| gate.label.to_string())
        .collect();
    for oracle in oracles {
        let oracle = oracle.as_str().unwrap();
        let head: String = oracle
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || *c == ' ')
            .collect();
        assert!(
            !labels.iter().any(|label| head.contains(label.as_str())),
            "the `{head}` oracle is characterization, never a decoder gate"
        );
    }
}
