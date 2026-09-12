//! The TypeScript half of `cargo xtask codegen`.
//!
//! One schema, `schemas/doc_manifest.jtd.json`, is read by two languages:
//! the Rust pipeline writes the page manifest and the site reads it. The
//! site is a separate package under a separate toolchain, and the cheap
//! answer — hand-writing the interface over there and remembering to
//! change it here — is exactly the drift a wire contract exists to
//! prevent. So the same pinned generator that emits the Rust types emits
//! the TypeScript ones, from the same resolved document, in the same run,
//! and `check-codegen` diffs both (PROP-057 `##SEO-MANIFEST-AND-RESOLVER`;
//! the plan's fork F-14).
//!
//! The output takes none of the eight Rust post-processing passes: those
//! are keyed to the Rust emission shape and encode our wire policy for
//! Rust readers, and a consumer in another language has no standing to
//! receive a half-applied version of it. Nothing is reformatted either —
//! the bytes are the generator's, which is what makes the drift check
//! meaningful, so the site's floor excludes the generated tree from
//! prettier, eslint and the traceability map rather than editing it into
//! shape.
//!
//! It takes exactly ONE pass, and that pass is not taste. The generator
//! emits a closed vocabulary as a TypeScript `enum`, and the consuming
//! package compiles under `erasableSyntaxOnly` — the discipline's floor
//! flag that forbids any syntax the compiler cannot simply delete
//! (`TS1294`). An `enum` is the canonical example: it emits a runtime
//! object the type system also names. The pass rewrites each one into
//! the erasable form that means the same thing — a frozen object plus a
//! union type of its values, under the same name — so the schema's
//! closed vocabularies stay closed and the package keeps the flag. A
//! member the pass does not recognise stops the run rather than being
//! silently left behind.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::vocabulary::Vocabularies;
use super::write::write_generated;

/// A schema that also generates a TypeScript client, and where the file
/// lands. Spelled out rather than derived: a second consumer in a second
/// language is a decision, and the one place to read what that decision
/// was is a table with the destination written in it.
const TARGETS: [TypescriptTarget; 1] = [TypescriptTarget {
    schema: "schemas/doc_manifest.jtd.json",
    out_dir: "vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/generated",
    file: "doc-manifest.ts",
}];

struct TypescriptTarget {
    /// Repo-relative path of the authored schema.
    schema: &'static str,
    /// Repo-relative directory the generated module is written into.
    out_dir: &'static str,
    /// The module's file name — kebab-case, as the consuming package spells its files.
    file: &'static str,
}

/// Rewrite every `export enum Name { Member = "value", … }` into the
/// erasable pair the discipline's `erasableSyntaxOnly` admits:
///
/// ```text
/// export const Name = { Member: "value", … } as const;
/// export type Name = (typeof Name)[keyof typeof Name];
/// ```
///
/// Same name, same members, same closed set of values — and nothing the
/// compiler has to emit code for. A member whose form is not
/// `Ident = "string",` is refused by name: the schemas this runs over
/// have string vocabularies only, and quietly copying through a numeric
/// or computed member would put syntax the floor forbids back in the
/// file the next `tsc` run would then reject with no clue why.
fn erase_enums(source: &str, schema: &str) -> Result<String> {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(at) = rest.find("export enum ") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "export enum ".len()..];
        let (name, after) = match after.split_once(" {") {
            Some(split) => split,
            None => bail!(
                "{schema}: an `export enum` in the generated TypeScript has no \
                 opening brace — the generator's emission shape changed, and \
                 the erasure pass is written against the old one."
            ),
        };
        let (body, after) = match after.split_once("\n}") {
            Some(split) => split,
            None => bail!("{schema}: the `export enum {name}` block is never closed."),
        };
        let mut members = String::new();
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Some((member, value)) = trimmed.trim_end_matches(',').split_once(" = ") else {
                bail!(
                    "{schema}: cannot erase `enum {name}`: the member `{trimmed}` is not \
                     `Member = \"value\"`. Closed vocabularies are string-valued; a member \
                     of another shape is a schema change that this pass must be taught."
                );
            };
            if !(value.starts_with('"') && value.ends_with('"')) {
                bail!(
                    "{schema}: cannot erase `enum {name}`: the member `{member}` has the \
                     non-string value `{value}`."
                );
            }
            members.push_str(&format!("  {member}: {value},\n"));
        }
        out.push_str(&format!(
            "export const {name} = {{\n{members}}} as const;\nexport type {name} = \
             (typeof {name})[keyof typeof {name}];"
        ));
        rest = after.trim_start_matches("\n}");
    }
    out.push_str(rest);
    Ok(out)
}

/// The `@scope` line the generated module opens with, taken from the
/// schema's own `metadata.spec.implements`.
///
/// The consuming package's traceability map holds every exported unit to
/// a spec citation, and a generated module is no exception — a type
/// nobody can trace is a type nobody can argue with. The citation is not
/// invented here: the schema already declares which requirement it
/// implements, and this copies that declaration into the file so the two
/// cannot disagree. A schema that declares none gets no marker, and the
/// consumer's orphan ratchet says so on the next run — which is the
/// right place to find out.
fn scope_marker(schema: &Path) -> Result<String> {
    let text = std::fs::read_to_string(schema)
        .with_context(|| format!("reading schema {}", schema.display()))?;
    let document: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing schema {}", schema.display()))?;
    let implements = document
        .get("metadata")
        .and_then(|metadata| metadata.get("spec"))
        .and_then(|spec| spec.get("implements"))
        .and_then(serde_json::Value::as_str);
    Ok(match implements {
        Some(uri) => format!("/** @scope {uri} */\n\n"),
        None => String::new(),
    })
}

/// Every generated TypeScript file, for `check-codegen`'s diff.
pub(crate) fn typescript_out_files(root: &Path) -> Vec<PathBuf> {
    TARGETS
        .iter()
        .map(|target| root.join(target.out_dir).join(target.file))
        .collect()
}

/// Emit every TypeScript target from its resolved schema.
pub(crate) fn generate_typescript(
    binary: &Path,
    root: &Path,
    vocabularies: &mut Vocabularies,
) -> Result<()> {
    for target in &TARGETS {
        let schema = root.join(target.schema);
        if !schema.exists() {
            bail!(
                "the TypeScript target names {}, which is not in the tree. \
                 Fix: repoint the table in `xtask/src/codegen/typescript.rs` \
                 at the schema's new home, or drop the target if the \
                 consumer is gone.",
                schema.display()
            );
        }
        // The same resolution the Rust path takes: the generator reads
        // the document with its vocabulary fragments placed, never the
        // authored file, so both languages describe one contract.
        let resolved = vocabularies.resolve(&schema)?;

        let staging = tempfile::tempdir().context("creating the TypeScript staging directory")?;
        let status = Command::new(binary)
            .arg("--typescript-out")
            .arg(staging.path())
            .arg(&resolved.doc)
            .status()
            .with_context(|| format!("spawning {}", binary.display()))?;
        if !status.success() {
            bail!(
                "jtd-codegen failed to emit TypeScript for `{}` (exit code {:?})",
                schema.display(),
                status.code()
            );
        }

        // jtd-codegen writes one `index.ts` per `--typescript-out`; the
        // consuming package names its files after what is in them.
        let emitted = staging.path().join("index.ts");
        let body = std::fs::read_to_string(&emitted).with_context(|| {
            format!(
                "reading the emitted TypeScript {} — the generator reported \
                 success but wrote no module",
                emitted.display()
            )
        })?;

        let body = erase_enums(&body, target.schema)?;

        let out_dir = root.join(target.out_dir);
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("creating {}", out_dir.display()))?;
        let out_file = out_dir.join(target.file);
        let header = format!(
            "{}// Generated by `cargo xtask codegen`. DO NOT EDIT.\n\
             //\n\
             // Emitted by `jtd-codegen` from `{}` — the same schema, and the\n\
             // same resolved document, the Rust writer of this manifest is\n\
             // generated from. `cargo xtask check-codegen` fails on any\n\
             // difference between this file and what the generator emits, so\n\
             // an edit here is reverted by the next run rather than kept.\n\
             \n",
            scope_marker(&schema)?,
            target.schema
        );
        write_generated(&out_file, &format!("{header}{body}"))?;
        eprintln!("  - {} → {}", target.schema, out_file.display());
    }
    Ok(())
}
