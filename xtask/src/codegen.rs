//! `cargo xtask codegen` / `check-codegen` — regenerate the Rust types
//! under each owning crate's `src/generated/` from the JTD schemas under
//! `schemas/`, and the CI drift check over the result.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::repo_root;

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

/// Per-schema output routing: a schema's generated types live in the crate
/// that owns them. Most wire contracts live in `vibe-wire` (the shared
/// wire-format crate); `specmap` owns its own data model in `specmap-core`, so
/// the traceability engine carries its types and can relocate without a
/// `specmap-core → vibe-wire` edge (Traceability Relocation Plan, Phase 1).
fn generated_dir_for(stem: &str, root: &Path) -> PathBuf {
    match stem {
        "specmap" => {
            root.join("packages/org.vibevm/rust-ai-native/v0.2.0/crates/specmap-core/src/generated")
        }
        _ => root.join("crates/vibe-wire/src/generated"),
    }
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

pub(crate) fn run_codegen() -> Result<()> {
    let root = repo_root()?;
    let schemas_dir = root.join("schemas");

    if !schemas_dir.exists() {
        bail!(
            "`schemas/` directory not found at {}",
            schemas_dir.display()
        );
    }

    let binary = find_jtd_codegen(&root)?;

    // Find every `*.jtd.json` under `schemas/`.
    let schemas: Vec<PathBuf> = std::fs::read_dir(&schemas_dir)
        .with_context(|| format!("reading {}", schemas_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".jtd.json"))
                    .unwrap_or(false)
        })
        .collect();

    if schemas.is_empty() {
        eprintln!(
            "no `*.jtd.json` schemas under `{}` — nothing to do.",
            schemas_dir.display()
        );
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
        generate_into(&binary, out_dir, group)?;
    }
    Ok(())
}

/// Wipe `out_dir` (preserving a `.gitkeep`) and regenerate `schemas` into it,
/// each into its own `<stem>/` submodule, then synthesise the top-level
/// `mod.rs` re-exporting the (alphabetically sorted) submodules. Wiping first
/// keeps `check-codegen` exact: what's on disk is exactly what the generator
/// would produce from *only* the schemas routed to this dir, so a removed or
/// rerouted schema cannot leave a stale submodule behind.
fn generate_into(binary: &Path, out_dir: &Path, schemas: &[PathBuf]) -> Result<()> {
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

    // Module names sorted for determinism so `check-codegen` stays stable
    // across platforms (filesystem read order is not guaranteed).
    module_names.sort();
    let mut top = String::new();
    top.push_str("// Generated by `cargo xtask codegen`. DO NOT EDIT.\n");
    top.push_str("//\n");
    top.push_str("// Each submodule is generated by `jtd-codegen` from the matching\n");
    top.push_str("// `*.jtd.json` schema under `schemas/` at the repo root. Editing\n");
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
    // crate is caught (the routing fans `specmap` out to specmap-core, the
    // rest to vibe-wire).
    let out_dirs = [
        root.join("crates/vibe-wire/src/generated"),
        root.join("packages/org.vibevm/rust-ai-native/v0.2.0/crates/specmap-core/src/generated"),
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
            "generated code under {} is out of date relative to `schemas/`. \
             Run `cargo xtask codegen` and commit the result.",
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
