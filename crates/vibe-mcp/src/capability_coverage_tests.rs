//! Default factory/descriptor metadata only; no context or tool execution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#REGISTRATION-OBSERVATION");

use std::collections::BTreeSet;
use vibe_wire::generated::capability::e1::coverage::{Coverage, SurfaceKind};

fn names(coverage: &Coverage) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for row in coverage
        .surfaces
        .iter()
        .filter(|row| row.kind == SurfaceKind::Mcp && row.registered)
    {
        if !row.aliases.is_empty() || row.parent_surface_id.is_some() {
            return Err(format!(
                "default MCP descriptor has no aliases/parent mapping: {}",
                row.name
            ));
        }
        if !names.insert(row.name.clone()) {
            return Err(format!("duplicate MCP inventory name {}", row.name));
        }
    }
    if names.is_empty() {
        return Err("empty MCP registration denominator".into());
    }
    Ok(names)
}

#[test]
fn matches_native_mcp_inventory() {
    let path = std::env::var_os("VIBE_CAPABILITY_COVERAGE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../formats/capability-coverage/coverage-e1.json")
        });
    let coverage: Coverage =
        serde_json::from_slice(&std::fs::read(path).expect("selected coverage bytes"))
            .expect("generated Coverage reader");
    coverage
        .validate_capability_coverage()
        .expect("own pure coverage invariants");
    let descriptors: Vec<_> = crate::default_tools()
        .into_iter()
        .map(|tool| tool.descriptor().name)
        .collect();
    let native: BTreeSet<_> = descriptors.iter().cloned().collect();
    assert!(!native.is_empty(), "native MCP registrations are nonzero");
    assert_eq!(
        native.len(),
        descriptors.len(),
        "factory duplicate descriptors cannot be collapsed"
    );
    assert_eq!(
        names(&coverage).expect("unique registered MCP rows"),
        native
    );
    let first = coverage
        .surfaces
        .iter()
        .position(|row| row.kind == SurfaceKind::Mcp && row.registered)
        .expect("MCP row");
    for defect in [
        "missing",
        "orphan",
        "alias",
        "duplicate",
        "hidden-as-unregistered",
    ] {
        let mut counterexample = coverage.clone();
        match defect {
            "missing" => {
                counterexample.surfaces.remove(first);
            }
            "orphan" => counterexample.surfaces[first].name = "coverage-orphan".into(),
            "alias" => counterexample.surfaces[first]
                .aliases
                .push("coverage-wrong-alias".into()),
            "duplicate" => counterexample
                .surfaces
                .push(counterexample.surfaces[first].clone()),
            _ => counterexample.surfaces[first].registered = false,
        }
        assert!(
            match names(&counterexample) {
                Ok(names) => names != native,
                Err(_) => true,
            },
            "{defect} cannot match native MCP"
        );
    }
}
