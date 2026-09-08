use super::*;

#[test]
fn prepared_export_converts_without_a_second_plan_or_inventory() {
    let source = tempfile::tempdir().unwrap();
    crate::init_contract(source.path()).unwrap();
    std::fs::write(
        source.path().join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::create_dir(source.path().join("src")).unwrap();
    std::fs::write(source.path().join("src/lib.rs"), "pub fn value() {}\n").unwrap();
    let contract_path = source.path().join("vibevm/scrape/contract.toml");
    let contract = std::fs::read_to_string(&contract_path)
        .unwrap()
        .replace("modified = \"refuse\"", "modified = \"delete\"")
        .replace("tests = \"required\"", "tests = \"skip\"");
    std::fs::write(contract_path, contract).unwrap();
    let output_parent = tempfile::tempdir().unwrap();
    let prepared = crate::prepare(crate::model::ScrapeRequest {
        root: source.path().to_path_buf(),
        contract: None,
        mode: ScrapeMode::Export {
            output: output_parent.path().join("release"),
        },
    })
    .unwrap();
    assert!(
        prepared.plan.blockers.is_empty(),
        "{:?}",
        prepared.plan.blockers
    );
    assert!(
        prepared.health.blockers.is_empty(),
        "{:?}",
        prepared.health.blockers
    );
    let wire_plan = prepared.plan.to_wire().unwrap();
    let refused_report = tx::TransactionReport {
        project_key: tx::ProjectKey(format!("sha256:{}", "1".repeat(64))),
        transaction_id: tx::TransactionId("TX000001".into()),
        plan_id: tx::Digest(prepared.plan.plan_id.clone()),
        mode: tx::TransactionMode::Export,
        outcome: tx::Outcome::Refused,
        assurance: tx::Assurance::Full,
        cleanup: tx::Cleanup::Complete,
        before_tree: Some(tx::Digest(prepared.inventory.tree_digest.clone())),
        after_tree: None,
        snapshots: Vec::new(),
        verification: Vec::new(),
        planned_mutations: Vec::new(),
        actual_mutations: Vec::new(),
        events: Vec::new(),
    };
    let refused_wire =
        super::super::report::report_to_wire_plan(&refused_report, &wire_plan).unwrap();
    assert!(refused_wire.deleted_artifacts.is_empty());
    assert!(refused_wire.rewrites.is_empty());
    assert!(refused_wire.relocations.is_empty());
    assert!(refused_wire.residuals.is_empty());
    let transaction = prepared_transaction(prepared).unwrap();
    super::super::validate::prepared(&transaction).unwrap();
    let tx::PreparedMode::Export(plan) = transaction.mode else {
        panic!("export adapter changed mode")
    };
    assert!(
        plan.entries
            .iter()
            .any(|entry| entry.target_path == "src/lib.rs")
    );
    assert!(
        !plan
            .entries
            .iter()
            .any(|entry| entry.target_path.starts_with("vibevm"))
    );

    let prepared = crate::prepare(crate::model::ScrapeRequest {
        root: source.path().to_path_buf(),
        contract: None,
        mode: ScrapeMode::InPlace,
    })
    .unwrap();
    assert!(
        prepared.plan.blockers.is_empty(),
        "{:?}",
        prepared.plan.blockers
    );
    let transaction = prepared_transaction(prepared).unwrap();
    super::super::validate::prepared(&transaction).unwrap();
}
