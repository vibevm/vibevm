use super::*;
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
fn fixture() -> (ChangeManifest, WorkGraph) {
    let node =
        Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n")
            .unwrap();
    let context =
        ChangeContext::for_node(&node, ".", Some(Path::new("docs/custom-specs"))).unwrap();
    let change = ChangeManifest::parse_str(
        TEXT,
        &context,
        &context.directory(&ChangeId::parse(ID).unwrap()),
    )
    .unwrap();
    let graph = WorkGraph::parse_documents(&["schema=1\n[[items]]\nid='work-a'\ntouches=['src/a.rs']\n[[items]]\nid='work-b'\ntouches=['src/b.rs']\n"]).unwrap();
    (change, graph)
}
fn bindings() -> ProductionBindings {
    ProductionBindings {
        resolution: ProductionObservation::new("fixture-held-resolution/1", "exact-world-a")
            .unwrap(),
        declaration: ProductionObservation::new(
            "vibe-execution-declaration-effects-v2",
            "observed-declaration-a",
        )
        .unwrap(),
        target: ProductionObservation::new("selected-node/1", ".").unwrap(),
    }
}
#[test]
fn selected_request_has_two_operations_and_invocation_independent_framing() {
    let (change, graph) = fixture();
    let inputs = [
        NormativeInput {
            path: "intent.md",
            bytes: b"Intent",
        },
        NormativeInput {
            path: "acceptance.md",
            bytes: b"Accept",
        },
    ];
    let plan = SelectedFullPass::select(&change, &inputs, &graph, "work-a").unwrap();
    assert_eq!(
        plan.operations(),
        [
            ChangeOperationRole::Production,
            ChangeOperationRole::CurrentSourceVerification
        ]
    );
    let observed = bindings();
    let producer = plan.bind(&observed);
    // Invocation is deliberately absent from the binding API: the exact
    // retained subject used by create and verify produces the same request.
    for invocation in ["create", "verify", "full-pass"] {
        assert_eq!(producer, plan.bind(&observed), "{invocation}");
    }
    assert_eq!(producer.recipe(), PRODUCTION_REQUEST_RECIPE);
    assert_ne!(producer.digest(), plan.intent().digest());
    assert_ne!(producer.digest(), plan.contribution().digest());
    let mut changed = observed.clone();
    changed.target = ProductionObservation::new("selected-node/1", "members/other").unwrap();
    assert_ne!(producer, plan.bind(&changed));
    changed = observed.clone();
    changed.declaration =
        ProductionObservation::new("vibe-execution-declaration-effects-v2", "changed-effects")
            .unwrap();
    assert_ne!(producer, plan.bind(&changed));
    assert_ne!(
        producer,
        SelectedFullPass::select(&change, &inputs, &graph, "work-b")
            .unwrap()
            .bind(&observed)
    );
    // Independent longhand for the new recipe, not its production helper.
    use sha2::{Digest, Sha256};
    let mut raw = b"sha256:vibe-change-production-request-v1\0epoch=1\0".to_vec();
    let values = [
        ("intent.recipe", plan.intent().recipe()),
        ("intent.digest", plan.intent().digest()),
        ("work.selected", "work-a"),
        ("work.recipe", WORK_CONTRIBUTION_RECIPE),
        ("work.digest", plan.contribution().digest()),
        ("consumer", "org.fixture/alpha"),
        ("selected-node", "."),
        ("resolution.recipe", "fixture-held-resolution/1"),
        ("resolution.value", "exact-world-a"),
        (
            "declaration.recipe",
            "vibe-execution-declaration-effects-v2",
        ),
        ("declaration.value", "observed-declaration-a"),
        ("target.recipe", "selected-node/1"),
        ("target.value", "."),
    ];
    for (label, value) in values {
        raw.extend_from_slice(&(label.len() as u64).to_be_bytes());
        raw.extend_from_slice(label.as_bytes());
        raw.extend_from_slice(&(value.len() as u64).to_be_bytes());
        raw.extend_from_slice(value.as_bytes());
    }
    assert_eq!(
        producer.digest(),
        format!("sha256:{:x}", Sha256::digest(raw))
    );
}
#[test]
fn selection_refuses_missing_work_or_normative_inputs() {
    let (change, graph) = fixture();
    assert_eq!(
        SelectedFullPass::select(&change, &[], &graph, "work-a")
            .unwrap_err()
            .field,
        "intent.inputs"
    );
    let inputs = [
        NormativeInput {
            path: "intent.md",
            bytes: b"a",
        },
        NormativeInput {
            path: "acceptance.md",
            bytes: b"b",
        },
    ];
    assert_eq!(
        SelectedFullPass::select(&change, &inputs, &graph, "work-missing")
            .unwrap_err()
            .field,
        "work.selection"
    );
    for bad in ["", "\n", " "] {
        assert!(ProductionObservation::new("recipe", bad).is_err());
    }
}
