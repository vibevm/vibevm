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

mod derive_floor;
mod domain_types;
mod empty_policy;
// `wire-diff` reads the registry through this loader too (one loader,
// one truth), so the module is crate-visible to it.
pub(crate) mod format_id;
mod layout;
mod open_vocabulary;
mod optional_shapes;
mod ordered_maps;
mod output_tree;
mod postproc;
mod reader_projection;
mod shared_module;
mod snake_case;
mod strictness;
mod vocabulary;
mod write;

use crate::repo_root;
use format_id::emit_format_id;
use layout::{
    module_tree, schema_module_dir, schemas_under, specmap_generated_dir, specmap_schema_dir,
    vibe_wire_generated_dir,
};
use output_tree::StagedOutputTree;
use postproc::{StrictnessSource, rewrite_generated};
use strictness::Strictness;
use vocabulary::{Vocabularies, vocabularies_path};
use write::write_generated;

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
    let mut groups: Vec<GenerationGroup> = Vec::new();
    for (schema_root, out_dir, owner) in homes {
        if !schema_root.exists() {
            bail!("schema directory not found at {}", schema_root.display());
        }
        let schemas = schemas_under(&schema_root)?;
        if !schemas.is_empty() {
            groups.push(GenerationGroup {
                schema_root,
                live_out_dir: out_dir,
                owner,
                schemas,
            });
        }
    }

    let total: usize = groups.iter().map(|group| group.schemas.len()).sum();
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

    // Prepare every output first, beside its live tree and on the same
    // volume. From this point until the install loop, every writer sees only
    // a `.new-*` tree; a failed generator or an interrupted process cannot
    // erase the committed live output. Publication starts only after ALL
    // output trees have been generated successfully.
    let mut staged: Vec<StagedOutputTree> = Vec::with_capacity(groups.len());
    for group in &groups {
        staged.push(StagedOutputTree::prepare(&group.live_out_dir)?);
    }

    for (group, output) in groups.iter().zip(&staged) {
        generate_into(
            &binary,
            &root,
            group,
            output.fresh(),
            &mut vocabularies,
            &strictness,
        )?;
    }

    for (group, output) in groups.iter().zip(staged) {
        output.install().with_context(|| {
            format!(
                "publishing complete generated tree {}",
                group.live_out_dir.display()
            )
        })?;
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

/// One schema home and the live output tree it owns.
struct GenerationGroup {
    schema_root: PathBuf,
    live_out_dir: PathBuf,
    owner: FormatOwner,
    schemas: Vec<PathBuf>,
}

/// Regenerate one group into a fresh sibling of its live output tree, each
/// schema in its own submodule mirroring its path relative to `schema_root`
/// (`<rel_dir>/<stem>.jtd.json` → `<rel_dir>/<stem>/`), then synthesise a
/// `mod.rs` on every level of the tree, each listing its direct submodules
/// (alphabetically sorted). A fresh empty tree keeps `check-codegen` exact:
/// removed or rerouted schemas cannot leave stale modules, while the live
/// tree remains intact until every group succeeds. Each schema is first
/// resolved through the shared vocabularies (`vocabulary.rs`): the generator
/// reads the resolved document, never the authored file.
fn generate_into(
    binary: &Path,
    root: &Path,
    group: &GenerationGroup,
    out_dir: &Path,
    vocabularies: &mut Vocabularies,
    strictness: &Strictness,
) -> Result<()> {
    // jtd-codegen 0.4.1 writes a single `mod.rs` per `--rust-out` and
    // overwrites whatever is there. To keep several schemas in one tree
    // without each stomping the others, give every schema its own
    // subdirectory and synthesise a `mod.rs` per level re-exporting them.
    //
    // The host home runs THREE phases, not one loop: substitution puts
    // a shared fragment into every schema that names it, so a single
    // pass would emit the fragment's type once per module — distinct
    // Rust types that merely look alike. Phase 1 resolves every schema
    // once and KEEPS the closure each resolution places (the engine
    // home shares the phase, taking only the resolved copies from it).
    // Phase 2 — host home only, and only when anything is shared —
    // emits the shared module every re-export points at, with each
    // fragment's reader policy computed from all registered consumers
    // and a strict/permissive mixture refused.
    // Phase 3 emits each schema's module as always, then replaces its
    // copies of the closure's blocks with re-exports of the shared
    // types (the engine home gets none of this: our wire policy has no
    // standing over a vendored package's public Rust API, exactly as
    // it gets none of the post-processing).
    let mut resolved: Vec<(PathBuf, vocabulary::Resolved)> = Vec::new();
    for schema in &group.schemas {
        let resolution = vocabularies.resolve(schema)?;
        resolved.push((schema.clone(), resolution));
    }
    if group.owner == FormatOwner::Ours {
        reader_projection::validate_policies(root, &resolved)?;
    }

    let mut shared: Option<shared_module::SharedModule> = None;
    let mut rewire_stats: Vec<shared_module::RewireStats> = Vec::new();
    if group.owner == FormatOwner::Ours && resolved.iter().any(|(_, r)| !r.vocabularies.is_empty())
    {
        let shared_strictness = shared_module::guard_shared_strictness(root, &resolved)?;
        let shared_doc = vocabularies.shared_schema()?;
        let shared_file = shared_module::emit_shared_module(
            binary,
            out_dir,
            &shared_doc,
            &vocabularies_path(root),
            &shared_strictness,
        )?;
        shared = Some(shared_module::SharedModule::load(&shared_file)?);
    }

    let mut leaves: Vec<PathBuf> = Vec::new();
    if shared.is_some() {
        leaves.push(out_dir.join(shared_module::SHARED_MODULE));
    }
    for (schema, resolution) in &resolved {
        let sub_out = schema_module_dir(&group.schema_root, out_dir, schema)?;
        std::fs::create_dir_all(&sub_out)
            .with_context(|| format!("creating per-schema dir {}", sub_out.display()))?;
        eprintln!("  - {} → {}/", schema.display(), sub_out.display());
        let status = Command::new(binary)
            .arg("--rust-out")
            .arg(&sub_out)
            .arg(&resolution.doc)
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
        // into it (the full rule lives in `postproc`'s docs). The
        // replacement then swaps the closure's blocks for re-exports of
        // the shared module's types — byte-checked, in place.
        if group.owner == FormatOwner::Ours {
            let module_file = sub_out.join("mod.rs");
            reader_projection::strip_reflexive_root_alias(&module_file, &resolution.doc, schema)?;
            rewrite_generated(
                &module_file,
                &resolution.doc,
                schema,
                StrictnessSource::Registry {
                    registry: strictness,
                    projections: &resolution.projections,
                },
            )?;
            reader_projection::rewrite_consumer(
                &module_file,
                &resolution.doc,
                schema,
                &resolution.projections,
            )?;
            if let Some(module) = &shared {
                rewire_stats.push(module.rewire(&module_file, schema, &resolution.vocabularies)?);
            }
            reader_projection::append_consumer_adapter(
                &module_file,
                &resolution.doc,
                schema,
                &resolution.projections,
            )?;
            reader_projection::format_consumer(&module_file, out_dir, &resolution.projections)?;
        }
        leaves.push(sub_out);
    }
    if let Some(module) = &shared {
        shared_module::check_counter(&rewire_stats, &module.names())?;
    }

    // The vibe-wire generated tree also carries `format_id`, emitted from
    // `formats/REGISTRY.toml` (PROP-044 §4.1) rather than a JTD schema — so it
    // is its own emission branch here, not an entry in the schema-home
    // routing (which keys on the `*.jtd.json` suffix; the registry is TOML).
    // Registered as a leaf of the top level only, so the synthesised `mod.rs`
    // lists it alphabetically alongside the schema submodules, and a freshly
    // placed file under `generated/` can never be swept away by the next
    // codegen run.
    if group.live_out_dir == vibe_wire_generated_dir(root) {
        emit_format_id(root, out_dir)?;
        leaves.push(out_dir.join("format_id"));
    }

    // The header names the tree's actual schema home(s), repo-relative —
    // the two generated trees read their schemas from different places.
    let mut sources: Vec<String> = group
        .schemas
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
    // direct children (sorted by construction — `module_tree`). The root
    // of a home that carries a shared module says so: a reader meeting a
    // `pub use` inside a generated file must not mistake it for a hand
    // edit, and the header is where that reader looks first.
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
        top.push_str("// this file by hand will be overwritten on the next codegen run.\n");
        if dir == out_dir && shared.is_some() {
            top.push_str("//\n");
            top.push_str("// The `shared` submodule is the one home of the vocabulary\n");
            top.push_str("// fragments (`formats/vocabularies.json`) the schemas pull in;\n");
            top.push_str("// every other submodule re-exports its types (`pub use`) instead\n");
            top.push_str("// of carrying its own copies — one type per name, emitted once.\n");
        }
        top.push('\n');
        for name in children {
            top.push_str(&format!("pub mod {name};\n"));
        }
        let top_path = dir.join("mod.rs");
        write_generated(&top_path, &top)?;
    }

    eprintln!(
        "xtask codegen: {} ({} submodule{}).",
        group.live_out_dir.display(),
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
             (`vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md`), \
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
