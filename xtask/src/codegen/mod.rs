//! `cargo xtask codegen` / `check-codegen` — regenerate the Rust types
//! under each owning crate's `src/generated/` from the JTD schemas, and
//! the CI drift check over the result. Schemas live in two homes: the
//! host wire contracts under `schemas/` at the repo root, and the
//! specmap schema inside the `core-ai-native` package (the traceability
//! engine owns its own data model, schema included).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

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

/// Per-schema output routing: a schema's generated types live in the crate
/// that owns them. Most wire contracts live in `vibe-wire` (the shared
/// wire-format crate); `specmap` owns its own data model in the engine crate
/// `core-ai-native-specmap`, so the traceability engine carries its types and
/// can relocate without an engine → `vibe-wire` edge (Traceability Relocation
/// Plan, Phase 1).
fn generated_dir_for(stem: &str, root: &Path) -> PathBuf {
    match stem {
        "specmap" => root
            .join(SPECMAP_ENGINE_SLOT)
            .join("crates/core-ai-native-specmap/src/generated"),
        _ => vibe_wire_generated_dir(root),
    }
}

/// The vibe-wire generated tree — the home of every host wire type and, since
/// `FormatId` routes wire I/O through `wire::publish/load` (PROP-044 §4.1, gate
/// G1), the home of the generated format-id enum too. Factored out so the
/// `generate_into` emission branch compares against exactly the path routing
/// produces, never a divergent literal.
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

/// Every `*.jtd.json` directly under `dir`.
fn schemas_under(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".jtd.json"))
                    .unwrap_or(false)
        })
        .collect())
}

pub(crate) fn run_codegen() -> Result<()> {
    let root = repo_root()?;

    let binary = find_jtd_codegen(&root)?;

    // Both schema homes are committed; a missing one is a broken checkout,
    // not an empty state.
    let mut schemas: Vec<PathBuf> = Vec::new();
    for dir in [root.join("schemas"), specmap_schema_dir(&root)] {
        if !dir.exists() {
            bail!("schema directory not found at {}", dir.display());
        }
        schemas.extend(schemas_under(&dir)?);
    }

    if schemas.is_empty() {
        eprintln!("no `*.jtd.json` schemas found — nothing to do.");
        return Ok(());
    }

    // Group schemas by their owning crate's generated dir, then regenerate
    // each dir from scratch. A `BTreeMap` keeps per-dir processing order
    // deterministic across platforms.
    let mut by_dir: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for schema in schemas {
        let dir = generated_dir_for(&schema_stem(&schema)?, &root);
        by_dir.entry(dir).or_default().push(schema);
    }

    let total: usize = by_dir.values().map(Vec::len).sum();
    eprintln!(
        "xtask codegen: {} schema{} → {} generated tree{}",
        total,
        if total == 1 { "" } else { "s" },
        by_dir.len(),
        if by_dir.len() == 1 { "" } else { "s" },
    );

    for (out_dir, group) in &by_dir {
        generate_into(&binary, &root, out_dir, group)?;
    }
    Ok(())
}

/// Wipe `out_dir` (preserving a `.gitkeep`) and regenerate `schemas` into it,
/// each into its own `<stem>/` submodule, then synthesise the top-level
/// `mod.rs` re-exporting the (alphabetically sorted) submodules. Wiping first
/// keeps `check-codegen` exact: what's on disk is exactly what the generator
/// would produce from *only* the schemas routed to this dir, so a removed or
/// rerouted schema cannot leave a stale submodule behind.
fn generate_into(binary: &Path, root: &Path, out_dir: &Path, schemas: &[PathBuf]) -> Result<()> {
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
    // subdirectory and synthesise a top-level `mod.rs` re-exporting them.
    let mut module_names: Vec<String> = Vec::new();
    for schema in schemas {
        let stem = schema_stem(schema)?;
        let sub_out = out_dir.join(&stem);
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
        module_names.push(stem);
    }

    // The vibe-wire generated tree also carries `format_id`, emitted from
    // `formats/REGISTRY.toml` (PROP-044 §4.1) rather than a JTD schema — so it
    // is its own emission branch here, not a routing entry in
    // `generated_dir_for` (which keys on the `*.jtd.json` suffix; the registry
    // is TOML). Pushed before the sort so the synthesised `mod.rs` lists it
    // alphabetically alongside the schema submodules, and a freshly placed
    // file under `generated/` can never be swept away by the next codegen run.
    if out_dir == vibe_wire_generated_dir(root) {
        emit_format_id(root, out_dir)?;
        module_names.push("format_id".to_string());
    }

    // Module names sorted for determinism so `check-codegen` stays stable
    // across platforms (filesystem read order is not guaranteed).
    module_names.sort();
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
    let mut top = String::new();
    top.push_str("// Generated by `cargo xtask codegen`. DO NOT EDIT.\n");
    top.push_str("//\n");
    top.push_str("// Each submodule is generated by `jtd-codegen` from the matching\n");
    top.push_str(&format!(
        "// `*.jtd.json` schema under {sources}. Editing\n"
    ));
    top.push_str("// this file by hand will be overwritten on the next codegen run.\n\n");
    for name in &module_names {
        top.push_str(&format!("pub mod {name};\n"));
    }
    let top_path = out_dir.join("mod.rs");
    std::fs::write(&top_path, top).with_context(|| format!("writing {}", top_path.display()))?;

    eprintln!(
        "xtask codegen: {} ({} submodule{}).",
        out_dir.display(),
        module_names.len(),
        if module_names.len() == 1 { "" } else { "s" }
    );
    Ok(())
}

pub(crate) fn run_check_codegen() -> Result<()> {
    run_codegen()?;
    let root = repo_root()?;
    // Diff every generated tree codegen may write, so drift in any owning
    // crate is caught (the routing fans `specmap` out to the engine crate
    // `core-ai-native-specmap`, the rest to vibe-wire).
    let out_dirs = [
        vibe_wire_generated_dir(&root),
        root.join(SPECMAP_ENGINE_SLOT)
            .join("crates/core-ai-native-specmap/src/generated"),
    ];
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
