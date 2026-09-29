//! Binary-owned native clap observation, without argument parsing or handlers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#REGISTRATION-OBSERVATION");

use clap::CommandFactory;
use std::collections::{BTreeMap, BTreeSet};
use vibe_wire::generated::capability::e1::coverage::{Coverage, Surface, SurfaceKind};

#[derive(Debug, PartialEq, Eq)]
struct Registration {
    token: String,
    parent_path: String,
    aliases: BTreeSet<String>,
}

fn native_tree(command: &clap::Command, parent: &str, out: &mut BTreeMap<String, Registration>) {
    let mut siblings = BTreeSet::new();
    for child in command.get_subcommands() {
        let aliases: BTreeSet<_> = child.get_all_aliases().map(str::to_owned).collect();
        for name in std::iter::once(child.get_name()).chain(aliases.iter().map(String::as_str)) {
            assert!(
                siblings.insert(name.to_owned()),
                "duplicate native sibling name/alias {parent} {name}"
            );
        }
        let path = format!("{parent} {}", child.get_name());
        assert!(
            out.insert(
                path.clone(),
                Registration {
                    token: child.get_name().to_owned(),
                    parent_path: parent.to_owned(),
                    aliases,
                }
            )
            .is_none(),
            "duplicate native path {path}"
        );
        native_tree(child, &path, out);
    }
}

fn inventory_path(
    row: &Surface,
    rows: &BTreeMap<&str, &Surface>,
    root: &str,
) -> Result<String, String> {
    let mut tokens = vec![row.name.as_str()];
    let mut parent = row.parent_surface_id.as_deref();
    let mut visited = BTreeSet::new();
    while let Some(id) = parent {
        if !visited.insert(id) {
            return Err(format!("cyclic CLI parent {id}"));
        }
        let row = rows
            .get(id)
            .ok_or_else(|| format!("missing CLI parent {id}"))?;
        tokens.push(row.name.as_str());
        parent = row.parent_surface_id.as_deref();
    }
    tokens.reverse();
    Ok(format!("{root} {}", tokens.join(" ")))
}

fn compare(
    coverage: &Coverage,
    native: &BTreeMap<String, Registration>,
    root: &str,
) -> Result<(), String> {
    let rows: Vec<_> = coverage
        .surfaces
        .iter()
        .filter(|row| row.kind == SurfaceKind::Cli)
        .collect();
    let by_id: BTreeMap<_, _> = rows
        .iter()
        .map(|row| (row.surface_id.as_str(), *row))
        .collect();
    let mut declared = BTreeMap::new();
    for row in rows.into_iter().filter(|row| row.registered) {
        let path = inventory_path(row, &by_id, root)?;
        let aliases: BTreeSet<_> = row.aliases.iter().cloned().collect();
        if aliases.len() != row.aliases.len() {
            return Err(format!("duplicate CLI alias {path}"));
        }
        let parent_path = match row.parent_surface_id.as_deref() {
            Some(id) => inventory_path(by_id.get(id).ok_or("CLI parent absent")?, &by_id, root)?,
            None => root.to_owned(),
        };
        if declared
            .insert(
                path.clone(),
                Registration {
                    token: row.name.clone(),
                    parent_path,
                    aliases,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate CLI inventory path {path}"));
        }
    }
    if native.is_empty() || declared.is_empty() {
        return Err("empty CLI registration denominator".into());
    }
    if declared != *native {
        return Err(format!(
            "native CLI registration mismatch: declared={declared:?}; actual={native:?}"
        ));
    }
    Ok(())
}

#[test]
fn matches_native_clap_inventory() {
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
    let command = super::Cli::command();
    let mut native = BTreeMap::new();
    native_tree(&command, command.get_name(), &mut native);
    compare(&coverage, &native, command.get_name())
        .expect("all native CLI declarations mapped exactly");
    let first = coverage
        .surfaces
        .iter()
        .position(|row| row.kind == SurfaceKind::Cli && row.registered)
        .expect("nonzero CLI inventory");
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
            compare(&counterexample, &native, command.get_name()).is_err(),
            "{defect} cannot match native CLI"
        );
    }
}
