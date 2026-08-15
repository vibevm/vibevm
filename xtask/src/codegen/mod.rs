//! `cargo xtask codegen` / `check-codegen` — regenerate the Rust types
//! under each owning crate's `src/generated/` from the JTD schemas, and
//! the CI drift check over the result. Schemas live in two homes: the
//! host wire contracts under `schemas/` at the repo root, and the
//! specmap schema inside the `core-ai-native` package (the traceability
//! engine owns its own data model, schema included). A schema's generated
//! module mirrors its path relative to its home, so a schema directory
//! (which carries its epoch — PROP-044 §4.6) becomes a module path
//! instead of being flattened away.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

mod format_id;

use crate::repo_root;
use format_id::emit_format_id;

/// Locate the jtd-codegen binary. Prefer the project-local copy under
/// `tools/jtd-codegen/`; fall back to PATH if the local copy is
/// absent so contributors who chose a system-wide install still work.
fn find_jtd_codegen(root: &Path) -> Result<PathBuf> {
    let exe = if cfg!(windows) {
        "jtd-codegen.exe"
    } else {
        "jtd-codegen"
    };
    let local = root.join("tools").join("jtd-codegen").join(exe);
    if local.exists() {
        return Ok(local);
    }
    // Fall back to PATH lookup.
    let probe = Command::new(exe).arg("--version").output();
    match probe {
        Ok(out) if out.status.success() => Ok(PathBuf::from(exe)),
        _ => bail!(
            "jtd-codegen not found. Looked at:\n  \
             1. {} (project-local, preferred)\n  \
             2. `{exe}` on PATH (fallback)\n\n\
             Install per `tools/jtd-codegen/README.md`. PROP-000 §16 \
             pins the JTD + codegen toolchain as project-local; the PATH \
             fallback is a courtesy for contributors who already have \
             it installed system-wide.",
            local.display()
        ),
    }
}

/// The authored specmap engine package: both the `specmap` schema and the
/// generated types live inside `core-ai-native` (vendored stack copies catch
/// up at release events, never from here).
const SPECMAP_ENGINE_SLOT: &str = "packages/org.vibevm.ai-native/core-ai-native/v0.8.0";

/// The engine package's own `schemas/` — the second schema home the
/// codegen scans besides the host `schemas/` at the repo root.
fn specmap_schema_dir(root: &Path) -> PathBuf {
    root.join(SPECMAP_ENGINE_SLOT).join("schemas")
}

/// The engine crate's generated tree — the output home for every schema
/// under the engine's own `schemas/` (`specmap` owns its data model there,
/// so the traceability engine carries its types and can relocate without an
/// engine → `vibe-wire` edge; Traceability Relocation Plan, Phase 1). One
/// function so schema-home routing and `check-codegen`'s diff set agree on
/// the exact path, never divergent literals.
fn specmap_generated_dir(root: &Path) -> PathBuf {
    root.join(SPECMAP_ENGINE_SLOT)
        .join("crates/core-ai-native-specmap/src/generated")
}

/// The vibe-wire generated tree — the home of every host wire type and, since
/// `FormatId` routes wire I/O through `wire::publish/load` (PROP-044 §4.1, gate
/// G1), the home of the generated format-id enum too. One function so every
/// site that needs the path (schema-home routing, the `format_id` emission
/// branch, `check-codegen`'s diff set) compares against exactly one literal,
/// never a divergent copy.
fn vibe_wire_generated_dir(root: &Path) -> PathBuf {
    root.join("crates/vibe-wire/src/generated")
}

/// `foo.jtd.json` → `foo`.
fn schema_stem(schema: &Path) -> Result<String> {
    schema
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".jtd.json"))
        .map(str::to_string)
        .with_context(|| format!("schema name not `*.jtd.json`: {}", schema.display()))
}

/// Every `*.jtd.json` anywhere under `dir`, depth-first — subdirectories
/// carry epochs (PROP-044 §4.6), so schemas live nested and the scan must
/// reach them. The result is sorted: the walk order a filesystem hands back
/// is not guaranteed, and everything downstream (grouping, emission,
/// `check-codegen`) must stay byte-stable across platforms.
fn schemas_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut schemas: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(dir) {
        let path = entry
            .with_context(|| format!("reading {}", dir.display()))?
            .into_path();
        if !path.is_file() {
            continue;
        }
        let is_schema = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".jtd.json"))
            .unwrap_or(false);
        if is_schema {
            schemas.push(path);
        }
    }
    schemas.sort();
    Ok(schemas)
}

/// Where a schema's generated module lives: the schema's path relative to
/// its schema home, mirrored under the output tree —
/// `<root>/<rel_dir>/<stem>.jtd.json` → `<out_dir>/<rel_dir>/<stem>/`.
/// Mirroring rather than flattening is contract, not taste: the schema's
/// directory carries its epoch (PROP-044 §4.6 — a heavy break mints the new
/// world as a new path), so `index/e1/entry` and a future `index/e2/entry`
/// must stay distinct modules.
fn schema_module_dir(schema_root: &Path, out_dir: &Path, schema: &Path) -> Result<PathBuf> {
    let rel = schema.strip_prefix(schema_root).with_context(|| {
        format!(
            "schema {} is not under its schema home {}",
            schema.display(),
            schema_root.display()
        )
    })?;
    let rel_dir = rel.parent().unwrap_or(Path::new(""));
    Ok(out_dir.join(rel_dir).join(schema_stem(schema)?))
}

/// The `mod.rs` tree for an output directory: every directory from `out_dir`
/// down to each generated leaf gets a `mod.rs` listing its direct submodule
/// children. Built by walking up from every leaf so intermediate directories
/// (the epoch dirs between root and leaf) are registered even though no
/// schema of their own sits there. `BTreeMap` / `BTreeSet` keep both the
/// directory order and each child list sorted — determinism across platforms
/// (the filesystem walk order is not guaranteed).
fn module_tree(out_dir: &Path, leaves: &[PathBuf]) -> BTreeMap<PathBuf, BTreeSet<String>> {
    let mut tree: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    for leaf in leaves {
        let mut child = leaf.clone();
        while child != *out_dir {
            let (Some(parent), Some(name)) = (child.parent(), child.file_name()) else {
                break;
            };
            tree.entry(parent.to_path_buf())
                .or_default()
                .insert(name.to_string_lossy().into_owned());
            child = parent.to_path_buf();
        }
    }
    tree
}

pub(crate) fn run_codegen() -> Result<()> {
    let root = repo_root()?;

    let binary = find_jtd_codegen(&root)?;

    // Schema home → owning generated tree, iterated in this fixed order
    // (deterministic across platforms). Routing keys on the HOME a schema
    // came from, never on the schema's name: a second schema in the engine
    // home lands in the engine's tree, not vibe-wire's.
    let homes = [
        (root.join("schemas"), vibe_wire_generated_dir(&root)),
        (specmap_schema_dir(&root), specmap_generated_dir(&root)),
    ];

    // Both homes are committed; a missing one is a broken checkout,
    // not an empty state.
    let mut groups: Vec<(PathBuf, PathBuf, Vec<PathBuf>)> = Vec::new();
    for (schema_root, out_dir) in homes {
        if !schema_root.exists() {
            bail!("schema directory not found at {}", schema_root.display());
        }
        let schemas = schemas_under(&schema_root)?;
        if !schemas.is_empty() {
            groups.push((schema_root, out_dir, schemas));
        }
    }

    let total: usize = groups.iter().map(|g| g.2.len()).sum();
    if total == 0 {
        eprintln!("no `*.jtd.json` schemas found — nothing to do.");
        return Ok(());
    }

    eprintln!(
        "xtask codegen: {} schema{} → {} generated tree{}",
        total,
        if total == 1 { "" } else { "s" },
        groups.len(),
        if groups.len() == 1 { "" } else { "s" },
    );

    for (schema_root, out_dir, schemas) in &groups {
        generate_into(&binary, &root, schema_root, out_dir, schemas)?;
    }
    Ok(())
}

/// Wipe `out_dir` (preserving a `.gitkeep`) and regenerate `schemas` into it,
/// each into its own submodule mirroring its path relative to `schema_root`
/// (`<rel_dir>/<stem>.jtd.json` → `<rel_dir>/<stem>/`), then synthesise a
/// `mod.rs` on every level of the tree, each listing its direct submodules
/// (alphabetically sorted). Wiping first keeps `check-codegen` exact: what's
/// on disk is exactly what the generator would produce from *only* the
/// schemas routed to this dir, so a removed or rerouted schema cannot leave
/// a stale submodule behind.
fn generate_into(
    binary: &Path,
    root: &Path,
    schema_root: &Path,
    out_dir: &Path,
    schemas: &[PathBuf],
) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;

    for entry in
        std::fs::read_dir(out_dir).with_context(|| format!("scanning {}", out_dir.display()))?
    {
        let entry = entry.context("reading entry under out_dir")?;
        let path = entry.path();
        // Preserve a `.gitkeep` if present so an empty (no-schema) state still
        // leaves a tracked path; everything else is codegen output.
        if path.file_name().and_then(|n| n.to_str()) == Some(".gitkeep") {
            continue;
        }
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("removing stale {}", path.display()))?;
        } else {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing stale {}", path.display()))?;
        }
    }

    // jtd-codegen 0.4.1 writes a single `mod.rs` per `--rust-out` and
    // overwrites whatever is there. To keep several schemas in one tree
    // without each stomping the others, give every schema its own
    // subdirectory and synthesise a `mod.rs` per level re-exporting them.
    let mut leaves: Vec<PathBuf> = Vec::new();
    for schema in schemas {
        let sub_out = schema_module_dir(schema_root, out_dir, schema)?;
        std::fs::create_dir_all(&sub_out)
            .with_context(|| format!("creating per-schema dir {}", sub_out.display()))?;
        eprintln!("  - {} → {}/", schema.display(), sub_out.display());
        let status = Command::new(binary)
            .arg("--rust-out")
            .arg(&sub_out)
            .arg(schema)
            .status()
            .with_context(|| format!("spawning {}", binary.display()))?;
        if !status.success() {
            bail!(
                "jtd-codegen failed for `{}` (exit code {:?})",
                schema.display(),
                status.code()
            );
        }
        leaves.push(sub_out);
    }

    // The vibe-wire generated tree also carries `format_id`, emitted from
    // `formats/REGISTRY.toml` (PROP-044 §4.1) rather than a JTD schema — so it
    // is its own emission branch here, not an entry in the schema-home
    // routing (which keys on the `*.jtd.json` suffix; the registry is TOML).
    // Registered as a leaf of the top level only, so the synthesised `mod.rs`
    // lists it alphabetically alongside the schema submodules, and a freshly
    // placed file under `generated/` can never be swept away by the next
    // codegen run.
    if out_dir == vibe_wire_generated_dir(root) {
        emit_format_id(root, out_dir)?;
        leaves.push(out_dir.join("format_id"));
    }

    // The header names the tree's actual schema home(s), repo-relative —
    // the two generated trees read their schemas from different places.
    let mut sources: Vec<String> = schemas
        .iter()
        .filter_map(|s| s.parent())
        .map(|d| {
            let rel = d.strip_prefix(root).unwrap_or(d);
            format!("`{}/`", rel.display().to_string().replace('\\', "/"))
        })
        .collect();
    sources.sort();
    sources.dedup();
    let sources = sources.join(" / ");

    // One `mod.rs` per directory on every root→leaf path, each listing its
    // direct children (sorted by construction — `module_tree`).
    let tree = module_tree(out_dir, &leaves);
    let declared: usize = tree.values().map(BTreeSet::len).sum();
    for (dir, children) in &tree {
        let mut top = String::new();
        top.push_str("// Generated by `cargo xtask codegen`. DO NOT EDIT.\n");
        top.push_str("//\n");
        top.push_str("// Each submodule is generated by `jtd-codegen` from the matching\n");
        top.push_str(&format!(
            "// `*.jtd.json` schema under {sources}. Editing\n"
        ));
        top.push_str("// this file by hand will be overwritten on the next codegen run.\n\n");
        for name in children {
            top.push_str(&format!("pub mod {name};\n"));
        }
        let top_path = dir.join("mod.rs");
        std::fs::write(&top_path, top)
            .with_context(|| format!("writing {}", top_path.display()))?;
    }

    eprintln!(
        "xtask codegen: {} ({} submodule{}).",
        out_dir.display(),
        declared,
        if declared == 1 { "" } else { "s" }
    );
    Ok(())
}

pub(crate) fn run_check_codegen() -> Result<()> {
    run_codegen()?;
    let root = repo_root()?;
    // Diff every generated tree codegen may write, so drift in any owning
    // crate is caught (schema-home routing fans the engine home out to the
    // engine crate `core-ai-native-specmap`, the rest to vibe-wire).
    let out_dirs = [vibe_wire_generated_dir(&root), specmap_generated_dir(&root)];
    let mut cmd = Command::new("git");
    cmd.arg("diff").arg("--exit-code").arg("--");
    for dir in &out_dirs {
        cmd.arg(dir);
    }
    let status = cmd
        .current_dir(&root)
        .status()
        .context("spawning git diff")?;
    if !status.success() {
        bail!(
            "generated code under {} is out of date relative to the JTD \
             schemas and `formats/REGISTRY.toml`. Run `cargo xtask codegen` \
             and commit the result.",
            out_dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(" / ")
        );
    }
    eprintln!("xtask check-codegen: clean.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Nested schemas must be found at any depth, and the result must be
    /// sorted — the walk order a filesystem gives is not guaranteed.
    #[test]
    fn schemas_under_finds_nested_schemas_in_sorted_order() -> Result<()> {
        let dir = tempdir()?;
        std::fs::write(dir.path().join("a.jtd.json"), "{}")?;
        std::fs::create_dir_all(dir.path().join("sub").join("deep"))?;
        std::fs::write(dir.path().join("sub").join("deep").join("b.jtd.json"), "{}")?;

        let found = schemas_under(dir.path())?;
        assert_eq!(
            found,
            vec![
                dir.path().join("a.jtd.json"),
                dir.path().join("sub").join("deep").join("b.jtd.json"),
            ]
        );
        Ok(())
    }

    /// Only `*.jtd.json` FILES count: other extensions, backup tails, and a
    /// directory merely named like a schema must not be picked up.
    #[test]
    fn schemas_under_skips_non_schema_entries() -> Result<()> {
        let dir = tempdir()?;
        std::fs::write(dir.path().join("x.json"), "{}")?;
        std::fs::write(dir.path().join("y.jtd.json.bak"), "{}")?;
        std::fs::create_dir_all(dir.path().join("z.jtd.json"))?;

        assert_eq!(schemas_under(dir.path())?, Vec::<PathBuf>::new());
        Ok(())
    }

    /// The output path mirrors the schema's path relative to its home:
    /// root-level schemas keep today's flat layout, nested ones carry
    /// their directory (the epoch — PROP-044 §4.6) into the module path.
    #[test]
    fn schema_module_dir_mirrors_schema_path() -> Result<()> {
        let (root, out) = (Path::new("schemas"), Path::new("generated"));
        assert_eq!(
            schema_module_dir(root, out, &root.join("init_report.jtd.json"))?,
            out.join("init_report")
        );
        assert_eq!(
            schema_module_dir(root, out, &root.join("journal").join("journal.jtd.json"))?,
            out.join("journal").join("journal")
        );
        assert_eq!(
            schema_module_dir(
                root,
                out,
                &root.join("index").join("e1").join("entry.jtd.json")
            )?,
            out.join("index").join("e1").join("entry")
        );
        Ok(())
    }

    /// The `mod.rs` tree registers every directory from the output root down
    /// to each leaf — intermediates included — each with its direct children
    /// sorted, so `format_id` stays a child of the top level only.
    #[test]
    fn module_tree_registers_every_level_with_sorted_children() {
        let out = Path::new("generated");
        let leaves = vec![
            out.join("format_id"),
            out.join("init_report"),
            out.join("index").join("e1").join("entry"),
            out.join("index").join("e1").join("by_name"),
            out.join("journal").join("e1").join("journal"),
        ];
        let tree = module_tree(out, &leaves);

        let children_of = |dir: &Path| -> Vec<String> {
            tree.get(dir)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        };
        assert_eq!(
            children_of(out),
            vec!["format_id", "index", "init_report", "journal"]
        );
        assert_eq!(children_of(&out.join("index")), vec!["e1"]);
        assert_eq!(
            children_of(&out.join("index").join("e1")),
            vec!["by_name", "entry"]
        );
        assert_eq!(
            children_of(&out.join("journal").join("e1")),
            vec!["journal"]
        );
        // Exactly the directories on some root→leaf path carry a `mod.rs`.
        assert_eq!(tree.len(), 5);
    }
}
