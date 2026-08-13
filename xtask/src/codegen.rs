//! `cargo xtask codegen` / `check-codegen` — regenerate the Rust types
//! under each owning crate's `src/generated/` from the JTD schemas, and
//! the CI drift check over the result. Schemas live in two homes: the
//! host wire contracts under `schemas/` at the repo root, and the
//! specmap schema inside the `core-ai-native` package (the traceability
//! engine owns its own data model, schema included).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

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

// ── Format registry → `FormatId` enum ──────────────────────────────────────
//
// PROP-044 §4.1 `##M-FORMAT-REGISTRY`: every surface a foreign parser reads is
// inventoried in `formats/REGISTRY.toml`, and the `FormatId` enum is generated
// from it so an unregistered format is inexpressible in the type system. This
// is the emission for that enum. Unlike a JTD schema, the registry has no
// schema stem to route on, so it is emitted from its own branch in
// `generate_into` rather than looked up in `generated_dir_for`.

/// One `[format.<id>]` record, reduced to the fields the generated enum needs.
/// `schema`, `corpus` and `sunset` stay declarative in the TOML for later
/// phases (golden corpora Ф5, the break window Ф5.3) and are not consumed here.
struct FormatEntry {
    /// The registry id, verbatim — what `FormatId::id()` returns.
    id: String,
    /// `cli-init-report` → `CliInitReport`.
    variant: String,
    epoch: u32,
    recoverable: bool,
    /// `none` | `ours` | `many`, validated.
    foreign_parsers: String,
}

/// Parse `formats/REGISTRY.toml` into the reduced entries, in sorted id order
/// (`toml::Value`'s table is a `BTreeMap`, so iteration is deterministic across
/// platforms — `check-codegen` stays byte-stable).
fn load_format_registry(root: &Path) -> Result<Vec<FormatEntry>> {
    let path = root.join("formats/REGISTRY.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parsed: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let table = parsed
        .get("format")
        .and_then(|v| v.as_table())
        .with_context(|| format!("`{}` has no `[format.*]` table", path.display()))?;

    let mut entries: Vec<FormatEntry> = Vec::new();
    for (id, entry) in table {
        let epoch = require_u32(entry, id, "epoch", &path)?;
        let recoverable = require_bool(entry, id, "recoverable", &path)?;
        let foreign_parsers = require_str(entry, id, "foreign_parsers", &path)?;
        if !matches!(foreign_parsers.as_str(), "none" | "ours" | "many") {
            bail!(
                "`{}`: `[format.{id}].foreign_parsers` must be none|ours|many, got `{foreign_parsers}`",
                path.display()
            );
        }
        let variant = pascal_case(id).with_context(|| {
            format!(
                "`{}`: id `{id}` is not a valid enum variant",
                path.display()
            )
        })?;
        entries.push(FormatEntry {
            id: id.clone(),
            variant,
            epoch,
            recoverable,
            foreign_parsers,
        });
    }
    Ok(entries)
}

fn require_str(entry: &toml::Value, id: &str, key: &str, path: &Path) -> Result<String> {
    entry
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .with_context(|| {
            format!(
                "`{}`: `[format.{id}]` missing string `{key}`",
                path.display()
            )
        })
}

fn require_bool(entry: &toml::Value, id: &str, key: &str, path: &Path) -> Result<bool> {
    entry.get(key).and_then(|v| v.as_bool()).with_context(|| {
        format!(
            "`{}`: `[format.{id}]` missing boolean `{key}`",
            path.display()
        )
    })
}

fn require_u32(entry: &toml::Value, id: &str, key: &str, path: &Path) -> Result<u32> {
    let raw = entry
        .get(key)
        .and_then(|v| v.as_integer())
        .with_context(|| {
            format!(
                "`{}`: `[format.{id}]` missing integer `{key}`",
                path.display()
            )
        })?;
    u32::try_from(raw).with_context(|| {
        format!(
            "`{}`: `[format.{id}].{key}` = {raw} is not a valid epoch (expect a u32)",
            path.display()
        )
    })
}

/// `cli-init-report` → `CliInitReport`. Splits on `-`; each segment must be
/// non-empty and start with an ASCII letter (a valid Rust variant name).
fn pascal_case(id: &str) -> Result<String> {
    let mut out = String::new();
    for segment in id.split('-') {
        let mut chars = segment.chars();
        let first = chars
            .next()
            .ok_or_else(|| anyhow!("format id `{id}` has an empty segment"))?;
        if !first.is_ascii_alphabetic() {
            bail!("format id `{id}` segment must start with a letter");
        }
        out.push(first.to_ascii_uppercase());
        out.extend(chars);
    }
    Ok(out)
}

/// Emit `crates/vibe-wire/src/generated/format_id/mod.rs` from the registry.
/// Hand-formatted by string building — this is developer tooling in `xtask`,
/// outside the hand-written-wire ban (Ф4.3 binds product crates, not codegen).
/// The output must be `cargo fmt`-clean: `cargo fmt --all --check` runs over it.
fn emit_format_id(root: &Path, out_dir: &Path) -> Result<()> {
    let entries = load_format_registry(root)?;

    let mut out = String::new();
    out.push_str(
        "// Generated by `cargo xtask codegen` from `formats/REGISTRY.toml`. DO NOT EDIT.\n",
    );
    out.push_str("//\n");
    out.push_str("// `FormatId` enumerates every surface a foreign parser reads, so an\n");
    out.push_str("// unregistered format is inexpressible in the type system (PROP-044 §4.1\n");
    out.push_str("// `##M-FORMAT-REGISTRY`). The `recoverable` / `foreign_parsers` axes\n");
    out.push_str("// define each format's computed policy (PROP-044 §5 `##POLICY-IS-COMPUTED`).\n");
    out.push_str("//\n");
    out.push_str("// Internal identifier, not a wire type: it deliberately carries no\n");
    out.push_str("// Serialize / Deserialize — the hand-written-wire ban of Ф4.3 would\n");
    out.push_str("// otherwise forbid it. Wire I/O routes through it via\n");
    out.push_str("// `wire::publish/load(FormatId, …)` (gate G1, PROP-044 §8).\n\n");
    // Many match arms share a right-hand side today (every epoch is 1); that is
    // the point of codegen, not duplication worth a manual collapse.
    out.push_str("#![allow(clippy::match_same_arms)]\n\n");

    out.push_str("/// Every registered data format a foreign parser may read (PROP-044 §4.1).\n");
    out.push_str("///\n");
    out.push_str("/// Variants are generated from the `[format.*]` sections of\n");
    out.push_str("/// `formats/REGISTRY.toml`; the `format_id_completeness` test asserts the\n");
    out.push_str("/// two stay in lockstep in both directions.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum FormatId {\n");
    for e in &entries {
        out.push_str(&format!("    {},\n", e.variant));
    }
    out.push_str("}\n\n");

    out.push_str("/// How many independent parsers read a format — the second policy axis\n");
    out.push_str("/// (PROP-044 §5 `##POLICY-IS-COMPUTED`).\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    out.push_str("pub enum ForeignParsers {\n");
    out.push_str("    /// Read only by our own code; never a published surface.\n");
    out.push_str("    None,\n");
    out.push_str("    /// Read by our own code and no independent parser.\n");
    out.push_str("    Ours,\n");
    out.push_str("    /// Read by independent parsers (scripts, agents, foreign clients).\n");
    out.push_str("    Many,\n");
    out.push_str("}\n\n");

    out.push_str("impl FormatId {\n");
    out.push_str("    /// Every variant, in registry (sorted-id) order.\n");
    out.push_str("    pub const ALL: &[FormatId] = &[\n");
    for e in &entries {
        out.push_str(&format!("        FormatId::{},\n", e.variant));
    }
    out.push_str("    ];\n\n");

    out.push_str("    /// The registry id, verbatim (e.g. `cli-init-report`).\n");
    out.push_str("    pub fn id(self) -> &'static str {\n");
    out.push_str("        match self {\n");
    for e in &entries {
        out.push_str(&format!(
            "            FormatId::{} => \"{}\",\n",
            e.variant, e.id
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n\n");

    out.push_str("    /// The epoch this format lives in (PROP-044 §4.6 `##M-EPOCHS`).\n");
    out.push_str("    pub fn epoch(self) -> u32 {\n");
    out.push_str("        match self {\n");
    for e in &entries {
        out.push_str(&format!(
            "            FormatId::{} => {},\n",
            e.variant, e.epoch
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n\n");

    out.push_str("    /// Whether the format is rebuildable without a human (PROP-044 §5).\n");
    out.push_str("    pub fn recoverable(self) -> bool {\n");
    out.push_str("        match self {\n");
    for e in &entries {
        out.push_str(&format!(
            "            FormatId::{} => {},\n",
            e.variant, e.recoverable
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n\n");

    out.push_str("    /// How many independent parsers read this format (PROP-044 §5).\n");
    out.push_str("    pub fn foreign_parsers(self) -> ForeignParsers {\n");
    out.push_str("        match self {\n");
    for e in &entries {
        let fp = match e.foreign_parsers.as_str() {
            "none" => "ForeignParsers::None",
            "ours" => "ForeignParsers::Ours",
            "many" => "ForeignParsers::Many",
            // Validated in `load_format_registry`; unreachable here means the
            // validator and this mapping drifted apart.
            _ => unreachable!("foreign_parsers validated in load_format_registry"),
        };
        out.push_str(&format!("            FormatId::{} => {},\n", e.variant, fp));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    let sub_out = out_dir.join("format_id");
    std::fs::create_dir_all(&sub_out).with_context(|| format!("creating {}", sub_out.display()))?;
    let target = sub_out.join("mod.rs");
    std::fs::write(&target, out).with_context(|| format!("writing {}", target.display()))?;
    eprintln!("  - formats/REGISTRY.toml → {}/", sub_out.display());
    Ok(())
}
