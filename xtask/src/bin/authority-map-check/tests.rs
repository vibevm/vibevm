use super::*;

const VALID: &str = "# Authority map\n\n\
| Model ID | Inventory owner | Source IDs | Conformance ID |\n\
| --- | --- | --- | --- |\n\
| model.example | retained-model-owner | source:model | case.model |\n\n\
## Planned families\n\n\
| Family ID | Inventory owner | Source IDs | Conformance ID |\n\
| --- | --- | --- | --- |\n\
| family.example | unbound planned callable | source:family | case.family |\n\n\
### Proposed obligations\n\n\
| Conformance ID | Domain owner | Status | Obligation |\n\
| --- | --- | --- | --- |\n\
| case.model | proposed-existing-seam | PROPOSED UNRUN | Observe retained model with explicit incompleteness. |\n\
| case.family | another-existing-seam | PROPOSED UNRUN | Preserve domain ownership at extraction. |\n";

fn expected() -> Expected {
    Expected {
        models: BTreeMap::from([(
            "model.example".into(),
            Binding {
                owner: "retained-model-owner".into(),
                sources: BTreeSet::from(["source:model".into()]),
            },
        )]),
        families: BTreeMap::from([(
            "family.example".into(),
            Binding {
                owner: "unbound planned callable".into(),
                sources: BTreeSet::from(["source:family".into()]),
            },
        )]),
        sources: BTreeSet::from(["source:model".into(), "source:family".into()]),
    }
}

fn check(text: &str) -> Result<usize> {
    check_document(&vibe_specdoc::from_markdown(text)?, &expected())
}

#[test]
fn canonical_tables_join_nested_sections_and_independent_proposed_owners() {
    assert_eq!(check(VALID).unwrap(), 2);
}

#[test]
fn refuses_missing_and_orphan_model_or_family() {
    let missing = VALID.replace(
        "| model.example | retained-model-owner | source:model | case.model |\n",
        "",
    );
    assert!(
        check(&missing)
            .unwrap_err()
            .to_string()
            .contains("model denominator mismatch")
    );
    for (before, after, error) in [
        ("model.example", "model.orphan", "unknown model ID"),
        ("family.example", "family.orphan", "unknown family ID"),
    ] {
        assert!(
            check(&VALID.replace(before, after))
                .unwrap_err()
                .to_string()
                .contains(error)
        );
    }
}

#[test]
fn refuses_unknown_and_substituted_source_bindings() {
    for (replacement, error) in [
        ("source:unknown", "unknown Source ID"),
        ("source:family", "source binding mismatch"),
        ("source:model;source:model", "duplicate Source ID"),
    ] {
        let clone = VALID.replacen("source:model", replacement, 1);
        assert!(check(&clone).unwrap_err().to_string().contains(error));
    }
}

#[test]
fn refuses_inventory_owner_substitution() {
    let clone = VALID.replace("retained-model-owner", "different-owner");
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("inventory owner mismatch")
    );
}

#[test]
fn refuses_missing_and_orphan_conformance_references() {
    let clone = VALID.replacen("case.model", "case.missing", 1);
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("unknown Conformance ID")
    );
    let clone = format!(
        "{VALID}| case.orphan | existing-owner | PROPOSED UNRUN | Separate obligation. |\n"
    );
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("orphan Conformance IDs")
    );
}

#[test]
fn refuses_duplicate_identities_and_tables() {
    for row in [
        "| model.example | retained-model-owner | source:model | case.model |\n",
        "| family.example | unbound planned callable | source:family | case.family |\n",
        "| case.model | proposed-existing-seam | PROPOSED UNRUN | Observe retained model with explicit incompleteness. |\n",
    ] {
        let clone = VALID.replace(row, &format!("{row}{row}"));
        assert!(check(&clone).unwrap_err().to_string().contains("duplicate"));
    }
    let duplicate_table = "\n| Model ID | Inventory owner | Source IDs | Conformance ID |\n| --- | --- | --- | --- |\n| model.example | retained-model-owner | source:model | case.model |\n";
    assert!(
        check(&format!("{VALID}{duplicate_table}"))
            .unwrap_err()
            .to_string()
            .contains("expected exactly one")
    );
}

#[test]
fn refuses_executed_status_blank_obligation_and_ragged_row() {
    let clone = VALID.replacen("PROPOSED UNRUN", "PASSED", 1);
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("invalid conformance status")
    );
    let clone = VALID.replace("Observe retained model with explicit incompleteness.", "");
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("blank obligation")
    );
    let clone = VALID.replace("source:model | case.model |", "source:model |");
    assert!(
        check(&clone)
            .unwrap_err()
            .to_string()
            .contains("expected four")
    );
}
