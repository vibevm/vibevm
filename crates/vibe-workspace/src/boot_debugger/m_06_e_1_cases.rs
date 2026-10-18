use super::hypothetical::{comparison::*, *};
use super::measurement::Count;
use std::collections::BTreeMap;
use vibe_core::manifest::Manifest;

fn world(names: &[&str], missing: Vec<String>) -> FrozenWorld {
    let host = HostCapture {
        id: format!("host-{names:?}"),
        manifest: Manifest::parse_str(
            "[project]\nname='fixture'\nversion='1.0.0'\n[boot]\ndefault_link='static'\n",
        )
        .unwrap(),
        files: BTreeMap::new(),
    };
    let packages = names.iter().map(|n| SelectedPackage {
        manifest: Manifest::parse_str(&format!("[package]\ngroup='org.example'\nname='{n}'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n[[boot_snippet.fragment]]\nsource='dynamic.md'\nwhen='os:linux'\n")).unwrap(),
        requires: if *n == "shared" { vec![] } else { vec![("org.example".parse().unwrap(), "shared".into())] },
        files: BTreeMap::from([("boot.md".into(), format!("# {n}\n\nBODY {n}\n").into_bytes()), ("dynamic.md".into(), format!("potential {n}\n").into_bytes())]),
        source_identity: format!("registry/{n}/source"),
    }).collect();
    FrozenWorld::from_resolution(
        host,
        packages,
        "catalog".into(),
        "request-selected-provider-and-config".into(),
        missing,
    )
    .unwrap()
}
fn compare(b: FrozenWorld, c: FrozenWorld) -> Comparison {
    let s = PreparedScenario::new(b, c, "os:linux".into()).unwrap();
    s.evaluate(
        s.identity(),
        s.baseline().identity(),
        s.candidate().identity(),
        s.candidate().catalog(),
        &BTreeMap::new(),
    )
    .unwrap()
    .comparison
    .unwrap()
}
fn exact(n: &Count) -> u64 {
    let Count::Exact { value } = n else {
        panic!("{n:?}")
    };
    *value
}
fn delta(c: &Comparison, index: usize) -> i64 {
    let SignedCount::Exact { value } = c.marginal[index].value else {
        panic!("{:?}", c.marginal[index])
    };
    value
}
#[test]
fn shared_transitive_membership_and_both_graph_links() {
    let c = compare(
        world(&["app", "shared"], vec![]),
        world(&["app", "added", "shared"], vec![]),
    );
    let added = c
        .candidate
        .package_costs
        .iter()
        .find(|p| p.coordinate == "org.example/added")
        .unwrap();
    assert!(delta(&c, 0) > 0);
    assert!((delta(&c, 0) as u64) < exact(&added.static_raw_inclusive));
    assert_eq!(delta(&c, 0) as u64, exact(&added.static_raw_direct));
    assert_eq!(c.candidate.packages.len(), 3);
    assert_eq!(
        added.inclusive_members,
        vec!["org.example/added", "org.example/shared"]
    );
    assert!(c.baseline.compiler.is_some() && c.candidate.compiler.is_some());
    for d in &c.marginal {
        assert_eq!(d.baseline.world, c.baseline.world);
        assert_eq!(d.candidate.world, c.candidate.world);
        assert!(!d.baseline.declarations.is_empty() && !d.candidate.declarations.is_empty());
        assert_eq!(d.candidate.catalog, c.candidate.catalog);
        assert!(d.recipe.contains("candidate-minus-baseline"));
    }
    assert!(
        c.candidate
            .static_measurement
            .as_ref()
            .unwrap()
            .snippets
            .iter()
            .all(|s| !s.occurrences.is_empty())
    );
}
#[test]
fn independent_experiments_are_non_additive() {
    let a = compare(world(&[], vec![]), world(&["app", "shared"], vec![]));
    let b = compare(world(&[], vec![]), world(&["added", "shared"], vec![]));
    let joint = compare(
        world(&[], vec![]),
        world(&["app", "added", "shared"], vec![]),
    );
    assert!(delta(&a, 0) + delta(&b, 0) > delta(&joint, 0));
    assert!(
        Comparison::sum_independent(&[a, b])
            .unwrap_err()
            .contains("non-additive")
    );
}
#[test]
fn unavailable_transitive_and_static_dynamic_signed_removal() {
    let c = compare(
        world(&["app", "shared"], vec![]),
        world(&["shared"], vec![]),
    );
    assert!(delta(&c, 0) < 0 && delta(&c, 2) < 0);
    assert_eq!(c.marginal[0].lane, "static");
    assert_eq!(c.marginal[2].lane, "dynamic_potential");
    assert!(matches!(
        c.candidate.runtime_reads,
        Count::Unavailable { .. }
    ));
    let partial = compare(world(&["shared"], vec![]), world(&["app"], vec![]));
    assert!(
        partial
            .candidate
            .missing
            .iter()
            .any(|m| m.contains("transitive selected source capture"))
    );
    assert!(
        partial
            .marginal
            .iter()
            .all(|d| matches!(d.value, SignedCount::Unavailable { .. }))
    );
}
#[test]
fn duplicate_shared_membership_refuses() {
    let w = world(&["shared"], vec![]);
    let encoded = serde_json::to_value(compare(w.clone(), w)).unwrap();
    assert_eq!(encoded["marginal"][0]["value"]["value"], 0);
    // Constructor rejects duplicate membership before it can become double cost.
    let single = world(&["shared"], vec![]);
    let result = FrozenWorld::from_resolution(
        HostCapture {
            id: "x".into(),
            manifest: single_host(),
            files: BTreeMap::new(),
        },
        vec![
            SelectedPackage {
                manifest: Manifest::parse_str(
                    "[package]\ngroup='org.example'\nname='shared'\nkind='flow'\nversion='1.0.0'\n"
                )
                .unwrap(),
                requires: vec![],
                files: BTreeMap::new(),
                source_identity: "s".into()
            };
            2
        ],
        single.catalog().into(),
        "r".into(),
        vec![],
    );
    assert!(result.err().unwrap().contains("duplicate selected"));
}
fn single_host() -> Manifest {
    Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n").unwrap()
}
