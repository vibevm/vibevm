use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

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
pub(super) fn emit_format_id(root: &Path, out_dir: &Path) -> Result<()> {
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
