//! Native registration consistency only; semantic and runtime proof stays pending.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#COVERAGE-VERIFICATION");

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use vibe_wire::generated::capability::e1::coverage::{Coverage, Surface, SurfaceKind};

const DEFAULT_INVENTORY: &str = "formats/capability-coverage/coverage-e1.json";
const INVENTORY_ENV: &str = "VIBE_CAPABILITY_COVERAGE";
const OWNER_CASES: [(&str, &str, &str, &str); 3] = [
    (
        "vibe-cli",
        "--bin",
        "vibe",
        "cli::capability_coverage_tests::matches_native_clap_inventory",
    ),
    (
        "vibe-mcp",
        "--lib",
        "",
        "capability_coverage_tests::matches_native_mcp_inventory",
    ),
    (
        "vibe-cli",
        "--bin",
        "vibe",
        "commands::tree::tui::search::catalogue::tests::matches_native_tree_inventory",
    ),
];

#[derive(Debug, PartialEq, Eq)]
struct Registration {
    token: String,
    parent_path: String,
    aliases: BTreeSet<String>,
}

fn native_tree(command: &clap::Command) -> Result<BTreeMap<String, Registration>> {
    fn descend(
        command: &clap::Command,
        parent_path: &str,
        rows: &mut BTreeMap<String, Registration>,
    ) -> Result<()> {
        let mut sibling_names = BTreeSet::new();
        for child in command.get_subcommands() {
            let token = child.get_name();
            let aliases: BTreeSet<_> = child.get_all_aliases().map(str::to_owned).collect();
            for name in std::iter::once(token).chain(aliases.iter().map(String::as_str)) {
                if !sibling_names.insert(name.to_owned()) {
                    bail!("duplicate native sibling name/alias below {parent_path}: {name}");
                }
            }
            let path = format!("{parent_path} {token}");
            let row = Registration {
                token: token.to_owned(),
                parent_path: parent_path.to_owned(),
                aliases,
            };
            if rows.insert(path.clone(), row).is_some() {
                bail!("duplicate native command path: {path}");
            }
            descend(child, &path, rows)?;
        }
        Ok(())
    }
    let mut rows = BTreeMap::new();
    // Inspect derive-declared metadata, not build-generated help or argument matches.
    descend(command, command.get_name(), &mut rows)?;
    Ok(rows)
}

fn inventory_path(
    surface: &Surface,
    by_id: &BTreeMap<&str, &Surface>,
    root: &str,
) -> Result<String> {
    let mut tokens = vec![surface.name.as_str()];
    let mut parent = surface.parent_surface_id.as_deref();
    let mut visited = BTreeSet::new();
    while let Some(id) = parent {
        if !visited.insert(id) {
            bail!("cyclic developer registration parent: {id}");
        }
        let row = by_id
            .get(id)
            .context("developer registration parent is absent")?;
        tokens.push(row.name.as_str());
        parent = row.parent_surface_id.as_deref();
    }
    tokens.reverse();
    Ok(format!("{root} {}", tokens.join(" ")))
}

fn compare_developer_tree(
    coverage: &Coverage,
    native: &BTreeMap<String, Registration>,
    root: &str,
) -> Result<()> {
    let rows: Vec<_> = coverage
        .surfaces
        .iter()
        .filter(|row| row.kind == SurfaceKind::DeveloperTool)
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
            bail!("repeated inventory developer alias: {path}");
        }
        let parent_path = match row.parent_surface_id.as_deref() {
            Some(id) => inventory_path(
                by_id.get(id).context("developer parent absent")?,
                &by_id,
                root,
            )?,
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
            bail!("duplicate inventory developer command path: {path}");
        }
    }
    if native.is_empty() || declared.is_empty() {
        bail!("developer registration denominator is empty");
    }
    if declared != *native {
        let missing: Vec<_> = native
            .keys()
            .filter(|path| !declared.contains_key(*path))
            .collect();
        let orphan: Vec<_> = declared
            .keys()
            .filter(|path| !native.contains_key(*path))
            .collect();
        let aliases: Vec<_> = native
            .iter()
            .filter(|(path, row)| declared.get(*path).is_some_and(|other| other != *row))
            .map(|(path, _)| path)
            .collect();
        bail!(
            "developer registration mismatch: missing={missing:?}; orphan={orphan:?}; mapping/aliases={aliases:?}"
        );
    }
    Ok(())
}

fn print_developer_tree(rows: &BTreeMap<String, Registration>) {
    println!("Native xtask declaration metadata (token | parent path | aliases):");
    for row in rows.values() {
        println!("{} | {} | {:?}", row.token, row.parent_path, row.aliases);
    }
}

fn executed_case(selector: &str, success: bool, stdout: &str) -> Result<usize> {
    if !success {
        bail!("owning target failed for {selector}");
    }
    let summaries: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    if summaries.len() != 1 {
        bail!("expected one actual native test summary for {selector}");
    }
    let fields: Vec<_> = summaries[0]
        .strip_prefix("test result: ok. ")
        .context("native test harness did not report an ok result")?
        .split(';')
        .collect();
    let count = |suffix: &str| -> Result<usize> {
        let matches: Vec<_> = fields
            .iter()
            .filter_map(|field| field.trim().strip_suffix(suffix))
            .collect();
        if matches.len() != 1 {
            bail!("native summary lacks one {suffix} count for {selector}");
        }
        Ok(matches[0].trim().parse()?)
    };
    let passed = count("passed")?;
    let failed = count("failed")?;
    if failed != 0 || passed == 0 {
        bail!("no successful nonzero execution for {selector}: passed={passed}; failed={failed}");
    }
    if passed != 1
        || !stdout
            .lines()
            .any(|line| line == format!("test {selector} ... ok"))
    {
        bail!("native harness did not execute exactly the selected owning case {selector}");
    }
    Ok(passed)
}

pub(crate) fn run(root: &Path, selected_inventory: Option<&Path>) -> Result<()> {
    let selected = selected_inventory.unwrap_or(Path::new(DEFAULT_INVENTORY));
    let selected = if selected.is_absolute() {
        selected.to_owned()
    } else {
        root.join(selected)
    };
    let inventory = std::fs::canonicalize(&selected)
        .with_context(|| format!("reading selected coverage inventory {}", selected.display()))?;
    let coverage: Coverage = serde_json::from_slice(&std::fs::read(&inventory)?)?;
    coverage.validate_capability_coverage()?;
    let command = crate::Cli::command();
    let developer = native_tree(&command)?;
    print_developer_tree(&developer);
    compare_developer_tree(&coverage, &developer, command.get_name())?;
    for (package, target_kind, target, selector) in OWNER_CASES {
        let mut child = Command::new("cargo");
        child
            .current_dir(root)
            .env(INVENTORY_ENV, &inventory)
            .args(["test", "--locked", "-p", package, target_kind]);
        if !target.is_empty() {
            child.arg(target);
        }
        let output = child
            .args([selector, "--", "--exact"])
            .output()
            .with_context(|| format!("starting owning registration case {selector}"))?;
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        let stdout = std::str::from_utf8(&output.stdout)
            .context("native test harness stdout is not UTF-8")?;
        executed_case(selector, output.status.success(), stdout).with_context(|| {
            format!("selector {selector}, child exit {:?}", output.status.code())
        })?;
    }
    println!(
        "Registration consistency checked in owning CLI/MCP/TUI targets and native xtask declarations. Nonregistered routes, semantic/model/runtime conformance and whole-feature/API acceptance remain pending."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_result_requires_real_nonzero_selected_case() {
        let selector = "fixture::registration";
        let positive = "running 1 test\ntest fixture::registration ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s\n";
        assert_eq!(executed_case(selector, true, positive).unwrap(), 1);
        assert!(executed_case(selector, false, positive).is_err());
        for invalid in [
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s\n",
            "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 8 filtered out; finished in 0.00s\n",
            "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s\n",
            "test unrelated::case ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s\n",
            "",
        ] {
            assert!(executed_case(selector, true, invalid).is_err());
        }
    }

    #[test]
    fn native_aliases_are_scoped_to_their_parent() {
        let command = clap::Command::new("xtask")
            .subcommand(
                clap::Command::new("one").subcommand(clap::Command::new("inspect").alias("i")),
            )
            .subcommand(
                clap::Command::new("two").subcommand(clap::Command::new("inspect").alias("i")),
            );
        let rows = native_tree(&command).unwrap();
        assert!(rows["xtask one inspect"].aliases.contains("i"));
        assert!(rows["xtask two inspect"].aliases.contains("i"));
    }

    #[test]
    fn observes_native_xtask_registration_table() {
        let rows = native_tree(&crate::Cli::command()).unwrap();
        assert!(rows.contains_key("xtask capability-coverage"));
        print_developer_tree(&rows);
    }
}
