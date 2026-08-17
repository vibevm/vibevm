//! `cargo xtask codegen` / `check-codegen` — regenerate the Rust types
//! under each owning crate's `src/generated/` from the JTD schemas, and
//! the CI drift check over the result. This file is the driver: locating
//! the pinned `jtd-codegen` binary, routing each schema home to the
//! generated tree that owns its output, running the generator per schema
//! through the vocabulary substitution and the post-processing passes,
//! and writing the per-level `mod.rs` the layout rules prescribe. Those
//! rules — where schemas live, which output tree each home owns, what a
//! path segment must look like to become a module name, and the `mod.rs`
//! tree those constraints imply — are `layout`'s half of the mechanism.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

mod domain_types;
mod empty_policy;
mod format_id;
mod layout;
mod open_vocabulary;
mod optional_shapes;
mod ordered_maps;
mod postproc;
mod snake_case;
mod strictness;
mod vocabulary;

use crate::repo_root;
use format_id::emit_format_id;
use layout::{
    module_tree, schema_module_dir, schemas_under, specmap_generated_dir, specmap_schema_dir,
    vibe_wire_generated_dir,
};
use postproc::rewrite_generated;
use strictness::Strictness;
use vocabulary::{Vocabularies, vocabularies_path};

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

pub(crate) fn run_codegen() -> Result<()> {
    let root = repo_root()?;

    let binary = find_jtd_codegen(&root)?;

    // Schema home → owning generated tree → whose formats live there,
    // iterated in this fixed order (deterministic across platforms).
    // Routing keys on the HOME a schema came from, never on the schema's
    // name: a second schema in the engine home lands in the engine's
    // tree, not vibe-wire's. The third column is the same key answering
    // the second question the home decides — whether our transformation
    // layer governs the output — and it is stated here rather than
    // re-derived downstream, so a home added tomorrow has to name its
    // owner instead of defaulting to one silently.
    let homes = [
        (
            root.join("schemas"),
            vibe_wire_generated_dir(&root),
            FormatOwner::Ours,
        ),
        (
            specmap_schema_dir(&root),
            specmap_generated_dir(&root),
            FormatOwner::Foreign,
        ),
    ];

    // Both homes are committed; a missing one is a broken checkout,
    // not an empty state.
    let mut groups: Vec<(PathBuf, PathBuf, FormatOwner, Vec<PathBuf>)> = Vec::new();
    for (schema_root, out_dir, owner) in homes {
        if !schema_root.exists() {
            bail!("schema directory not found at {}", schema_root.display());
        }
        let schemas = schemas_under(&schema_root)?;
        if !schemas.is_empty() {
            groups.push((schema_root, out_dir, owner, schemas));
        }
    }

    let total: usize = groups.iter().map(|g| g.3.len()).sum();
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

    // Shared vocabularies, one home for the whole run (PROP-044 §8 G9):
    // parsed once here, every schema below resolves against it.
    let mut vocabularies = Vocabularies::load(&vocabularies_path(&root))?;

    // Reader strictness, one registry for the whole run: the map from
    // schema path to `foreign_parsers` role (`formats/REGISTRY.toml`,
    // parsed by the one loader `format_id` already owns) is built once
    // here exactly like the vocabularies, and every schema below is
    // ruled on through it — the strictness pass refuses a schema the
    // registry does not claim.
    let strictness = Strictness::load(&root)?;

    for (schema_root, out_dir, owner, schemas) in &groups {
        generate_into(
            &binary,
            &root,
            schema_root,
            out_dir,
            *owner,
            schemas,
            &mut vocabularies,
            &strictness,
        )?;
    }
    Ok(())
}

/// Whose formats a schema home holds — the question Р10 of the
/// change-native plan answers, and the reason the transformation layer
/// is not applied everywhere the generator runs.
///
/// `Ours` is the host home `schemas/`: formats this project owns, whose
/// policy (open vocabularies, canonical ordering, the empty-collection
/// convention) our own layer emits per PROP-044 §4.2. `Foreign` is a
/// vendored package's schema home — its output is that package's public
/// Rust API, and our wire policy has no standing to bind it to our
/// release train. Withholding the passes there costs nothing today: the
/// engine's schema carries no discriminator union, so the boxing pass
/// was already a no-op on it, which is checkable by regenerating and
/// comparing bytes.
///
/// It is a named pair rather than a boolean because the answer is a
/// property of the home the caller already knows literally, and a
/// boolean at a call site says nothing about which way is which.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatOwner {
    Ours,
    Foreign,
}

/// Wipe `out_dir` (preserving a `.gitkeep`) and regenerate `schemas` into it,
/// each into its own submodule mirroring its path relative to `schema_root`
/// (`<rel_dir>/<stem>.jtd.json` → `<rel_dir>/<stem>/`), then synthesise a
/// `mod.rs` on every level of the tree, each listing its direct submodules
/// (alphabetically sorted). Wiping first keeps `check-codegen` exact: what's
/// on disk is exactly what the generator would produce from *only* the
/// schemas routed to this dir, so a removed or rerouted schema cannot leave
/// a stale submodule behind. Each schema is first resolved through the
/// shared vocabularies (`vocabulary.rs`): the generator reads the
/// resolved document, never the authored file.
// The driver's fixed signature: the binary and root, the home-routing
// triple, the schemas, and the run's two once-per-run contexts — every
// argument a distinct responsibility the call site already holds
// literally (the vibe-cli precedent for a signature that will not fold).
#[allow(clippy::too_many_arguments)]
fn generate_into(
    binary: &Path,
    root: &Path,
    schema_root: &Path,
    out_dir: &Path,
    owner: FormatOwner,
    schemas: &[PathBuf],
    vocabularies: &mut Vocabularies,
    strictness: &Strictness,
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
        // The generator reads the vocabulary-resolved document — the
        // authored schema itself for one without vocabularies, a scratch
        // copy otherwise.
        let resolved = vocabularies.resolve(schema)?;
        eprintln!("  - {} → {}/", schema.display(), sub_out.display());
        let status = Command::new(binary)
            .arg("--rust-out")
            .arg(&sub_out)
            .arg(&resolved)
            .status()
            .with_context(|| format!("spawning {}", binary.display()))?;
        if !status.success() {
            bail!(
                "jtd-codegen failed for `{}` (exit code {:?})",
                schema.display(),
                status.code()
            );
        }
        // The generator's output takes its content passes before anything
        // reads it — over our own formats only (`FormatOwner`): first the
        // arms of every discriminator union get their `Box`, then field
        // identifiers become snake_case with the identity renames
        // dropped, then wire maps become ordered `BTreeMap`s, then
        // optional collections collapse per the schema's `x-empty`, then
        // optional scalars and structures lose their `Box` per the
        // schema's `x-default` (and its two Box-free defaults), then
        // the structs of `foreign_parsers = "none"` formats take
        // `#[serde(deny_unknown_fields)]` per the registry's role, then
        // the vocabularies open per the schema's `x-vocabulary`. The pass
        // order is a rule, not a taste: boxing, snake-casing,
        // map-ordering, empty-policy, optional-shapes, and strictness
        // are keyed to the pinned emission shape and run while the file
        // is still that emission; opening then writes hand-rolled impls
        // into it (the full rule lives in `postproc`'s docs).
        if owner == FormatOwner::Ours {
            rewrite_generated(&sub_out.join("mod.rs"), &resolved, schema, strictness)?;
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
            "generated code under {} differs from what this machine's \
             jtd-codegen emits for the current schemas and \
             `formats/REGISTRY.toml`.\n\n\
             Two different things produce that difference, and the recipes \
             are opposites — decide which before committing anything:\n\
             1. The schemas moved and the tree did not. Fix: run \
             `cargo xtask codegen` and commit the result.\n\
             2. The generator is not the pinned build. This check compares \
             the committed tree against the output of whatever binary was \
             found — the project-local copy under `tools/jtd-codegen/` when \
             present, otherwise `jtd-codegen` on PATH — so a different build \
             reads as drift, and recipe 1 would commit ITS emission over \
             ours. Fix: run `jtd-codegen --version`, compare it with the \
             pin's single home \
             (`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md`), \
             and install the pinned build per that recipe before \
             regenerating.\n\n\
             The distinction is load-bearing: eight post-processing passes \
             are keyed to the pinned emission shape (`codegen/postproc.rs`), \
             and the diff itself cannot tell you which cause produced it.",
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
