//! The crate's dependency DAG, proved structurally (R4 architecture §1,
//! R4-TRANSFORM-PLAN-ABI §1).
//!
//! Its own cell since R4.2, split from `plan_fence_tests` at that file's
//! budget seam — and along a real responsibility line: every other fence in
//! this module reads SOURCE, while this one reads MANIFESTS. It is also the
//! single home for a fact `registry_fence_tests` used to restate: the exact
//! runtime and dev dependency sets of `vibe-spec`, and the absence of any
//! reverse edge from the two crates below it.

use std::collections::BTreeSet;

/// Parse one crate manifest structurally.
fn manifest(source: &str) -> toml::Table {
    toml::from_str(source).expect("crate manifest parses as TOML")
}

/// The dependency names of one section, structurally.
fn section_names(table: &toml::Table, section: &str) -> BTreeSet<String> {
    table
        .get(section)
        .and_then(toml::Value::as_table)
        .map(|dependencies| dependencies.keys().cloned().collect())
        .unwrap_or_default()
}

/// The DAG proof, parsed with `toml` rather than substring-scanned:
/// `vibe-spec` gains exactly `vibe-core` and `vibe-extension-registry` as
/// new lower-crate runtime dependencies plus the R4.2 `toml` value-tower
/// edge, the dev set is exactly `tempfile`, `syn`, `toml` (the fence's own
/// tooling), the registry depends on core, and neither lower crate gains a
/// reverse edge in any section.
///
/// **Why `toml` is a RUNTIME edge since R4.2.** ABI §5.3 gives `vibe-spec`
/// the lowering, and ABI §3 requires it lossless: TOML datetime and the TOML
/// number tower are not JSON values, generic JSON is forbidden and a
/// render/parse round trip may not enter identity — so reading an effective
/// configuration means naming `toml::Value`'s variants
/// (`toml_datetime::Offset` distinguishes `Z` from a signed minute offset
/// only that way). T10B recorded the missing edge as an interim refusal
/// rather than crossing it; this is that closure. Exactly one cell may use
/// it: `config_lowering.rs`, admitted by name in `CONFIG_LOWERING_RULES`
/// above while every other family still bans the identifier.
#[test]
fn the_dependency_dag_gains_exactly_the_two_intended_lower_edges() {
    let own = manifest(include_str!("../../../Cargo.toml"));
    let dependencies = section_names(&own, "dependencies");
    let expected = BTreeSet::from([
        "base64".to_owned(),
        "quick-xml".to_owned(),
        "serde".to_owned(),
        "serde_json".to_owned(),
        "sha2".to_owned(),
        "specmark".to_owned(),
        "thiserror".to_owned(),
        "toml".to_owned(),
        "vibe-core".to_owned(),
        "vibe-extension-registry".to_owned(),
        "vibe-specdoc".to_owned(),
        "vibe-wire".to_owned(),
    ]);
    assert_eq!(
        dependencies, expected,
        "the runtime dependency set must be the frozen prior set plus \
         exactly vibe-core, vibe-extension-registry and the R4.2 toml \
         value-tower edge"
    );
    let dev_dependencies = section_names(&own, "dev-dependencies");
    assert_eq!(
        dev_dependencies,
        BTreeSet::from(["syn".to_owned(), "tempfile".to_owned(), "toml".to_owned(),]),
        "the dev set is exactly the fence's dev-only tooling"
    );

    // No reverse edge: neither lower crate names vibe-spec in any section.
    let core = manifest(include_str!("../../../../vibe-core/Cargo.toml"));
    let registry = manifest(include_str!(
        "../../../../vibe-extension-registry/Cargo.toml"
    ));
    for (name, lower) in [("vibe-core", &core), ("vibe-extension-registry", &registry)] {
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let names = section_names(lower, section);
            assert!(
                !names.contains("vibe-spec"),
                "{name} must never gain a reverse edge to vibe-spec ({section})"
            );
        }
    }
    // The chain: the registry's one workspace edge is vibe-core.
    assert!(
        section_names(&registry, "dependencies").contains("vibe-core"),
        "vibe-extension-registry depends on vibe-core"
    );
}
