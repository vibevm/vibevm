//! The replacement half of the shared module: swap each copy of a
//! vocabulary fragment's block in a schema module for a re-export of
//! the shared home's type.
//!
//! A BLOCK is one declaration the emission carries — the leading doc
//! comments, the attribute run (`#[derive…]`, `#[serde…]`), the header
//! and body down to the closing `}` at column zero (`;` for a
//! `pub type`) — plus the `impl` blocks that belong to the type:
//! opened vocabularies carry hand-rolled `impl Serialize` / `impl
//! Deserialize` the vocabulary pass wrote in, and those move with the
//! type or the crate would hold two impls of one trait for one type.
//!
//! The stitch is by CONTENT: a block is replaced only after it is
//! byte-identical to the same-named block of the shared module. The
//! fragment's name reaches for its type through the obvious
//! snake_case → PascalCase fold, but the fold only routes the lookup —
//! the generator mints the name, and a layer that trusted its own
//! fold over the bytes would be re-inventing the naming rule it is
//! defending against. Three refusals guard the swap: a name whose
//! block the shared home does not carry; a block that differs by as
//! much as a byte (the first diverging line is named from both
//! sides); and a closure name with no block in the schema module at
//! all. The re-export lines stand where the declarations stood, in
//! the same order, so a diff reads as replacement in place rather
//! than rearrangement — and the imports the departed blocks were
//! last using are taken away exactly when nothing else uses them
//! (`prune_orphan_imports`), leaving none orphaned for clippy to
//! name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::prune_orphan_imports;

/// The re-export every replaced block becomes, pointing at the shared
/// home inside the crate the generated tree lives in.
const RE_EXPORT: &str = "pub use crate::generated::shared::";

/// One top-level declaration block of a generated module: the doc
/// comments, attributes, header and body of one `pub struct` /
/// `pub enum` / `pub type`, plus the `impl` blocks attached to it.
struct Block {
    /// The declared type's name — the generator's minting, read off
    /// the declaration line, never computed here.
    name: String,
    /// The block's line range, `[start, end)` over the file's chunks.
    span: (usize, usize),
    /// The block's bytes exactly as they stand in its file.
    text: String,
}

/// What one schema module's replacement established — the raw material
/// the run-level counter (`check_counter`) totals and refuses on.
#[derive(Debug)]
pub(crate) struct RewireStats {
    /// Declarations the module carried before the replacement.
    pub(super) before: usize,
    /// Declarations after — the locals that stayed.
    pub(super) after: usize,
    /// Blocks replaced by re-exports.
    pub(super) replaced: usize,
    /// The type names the module declared before / after.
    pub(super) names_before: BTreeSet<String>,
    pub(super) names_after: BTreeSet<String>,
}

/// The shared module parsed into blocks — the one home every re-export
/// points at, held for the whole run so each schema module's pass
/// stitches against the same bytes.
pub(crate) struct SharedModule {
    /// The module file's display path, for refusals.
    file: String,
    /// Emitted type name → its block.
    blocks: BTreeMap<String, Block>,
}

impl SharedModule {
    /// Read and parse the shared module the emission phase wrote.
    pub(crate) fn load(file: &Path) -> Result<Self> {
        let src = std::fs::read_to_string(file)
            .with_context(|| format!("reading the shared module {}", file.display()))?;
        let blocks = parse_blocks(&src, &file.display().to_string())?;
        let blocks: BTreeMap<String, Block> = blocks
            .into_iter()
            .map(|block| (block.name.clone(), block))
            .collect();
        Ok(Self {
            file: file.display().to_string(),
            blocks,
        })
    }

    /// The type names the shared home declares — the run-level
    /// counter's "after" side is seeded with them.
    pub(crate) fn names(&self) -> BTreeSet<String> {
        self.blocks.keys().cloned().collect()
    }

    /// Replace this schema module's copies of its closure's fragments
    /// with re-exports, refusing loudly on any divergence between the
    /// copy and the shared block.
    pub(crate) fn rewire(
        &self,
        module: &Path,
        schema: &Path,
        closure: &BTreeSet<String>,
    ) -> Result<RewireStats> {
        let src = std::fs::read_to_string(module)
            .with_context(|| format!("reading the generated {}", module.display()))?;
        let file = module.display().to_string();
        let lines: Vec<&str> = src.split_inclusive('\n').collect();
        let blocks = parse_blocks(&src, &file)?;

        // Which fragment claims which block, by the name fold — the
        // routing only; the byte compare below is the stitch.
        let folds: BTreeMap<String, &str> = closure
            .iter()
            .map(|fragment| (emitted_name(fragment), fragment.as_str()))
            .collect();
        if folds.len() != closure.len() {
            bail!(
                "schema {}: two vocabulary names fold onto one type name \
                 — the vocabulary home's keys are not all snake_case-distinct \
                 under the PascalCase fold, and the replacement refuses to \
                 guess which fragment a block belongs to.\n\
                 Fix: rename the colliding entries in \
                 `formats/vocabularies.json`, then run `cargo xtask codegen`.",
                schema.display()
            );
        }

        // Guard 1 and 2, in declaration order, so the re-exports land
        // where the declarations stood without a second pass.
        let mut targets: Vec<(usize, usize, &str)> = Vec::new();
        let mut claimed: BTreeSet<&str> = BTreeSet::new();
        for block in &blocks {
            let Some(fragment) = folds.get(&block.name) else {
                continue;
            };
            let Some(shared) = self.blocks.get(&block.name) else {
                bail!(
                    "schema {}: the vocabulary `{fragment}` claims the type \
                     `{}`, but the shared module {} declares no such type — \
                     the closure this schema pulls and the home the shared \
                     module was emitted from disagree, which the synthetic \
                     document's construction makes impossible unless one of \
                     the two moved.\n\
                     Fix: this is a defect in the shared-module emission \
                     (`xtask/src/codegen/shared_module.rs`), not in the \
                     schema; rerun `cargo xtask codegen` and file what you \
                     see if it repeats.",
                    schema.display(),
                    block.name,
                    self.file
                );
            };
            if let Some((at, left, right)) = first_divergence(&shared.text, &block.text) {
                bail!(
                    "schema {}: the block `{}` in {} is not byte-identical \
                     to the shared module {}'s — the replacement stitches on \
                     content, and a copy that differs by as much as one \
                     byte is not a copy.\n\
                     First divergence at block line {at}:\n\
                     schema module: {left}\n\
                     shared module: {right}\n\
                     Fix: make the fragment in `formats/vocabularies.json` \
                     and the schema agree through a clean `cargo xtask \
                     codegen` run; if both sides come straight from the \
                     generator, the passes have diverged — that is a defect \
                     in `xtask/src/codegen/shared_module.rs`, not in the \
                     schemas.",
                    schema.display(),
                    block.name,
                    file,
                    self.file
                );
            }
            targets.push((block.span.0, block.span.1, block.name.as_str()));
            claimed.insert(*fragment);
        }
        // The flip side of guard 1: a fragment whose block the module
        // does not declare at all. The generator emits every definition
        // it is handed, so an absent block means the emission moved.
        for fragment in closure {
            if !claimed.contains(fragment.as_str()) {
                let emitted = emitted_name(fragment);
                bail!(
                    "schema {}: the closure pulls the vocabulary `{fragment}` \
                     (type `{emitted}`), but the generated module {} declares \
                     no block for it. The generator emits every definition it \
                     is handed, so an absent block means the pinned emission \
                     shape has moved.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     the block parser in \
                     `xtask/src/codegen/shared_module/rewire.rs` the new \
                     shape, then run `cargo xtask codegen`.",
                    schema.display(),
                    file
                );
            }
        }

        // Rebuild: each target's lines become one re-export line,
        // wearing the replaced block's own first-line ending.
        let mut out = String::with_capacity(src.len());
        let mut index = 0;
        let mut replaced = 0;
        while index < lines.len() {
            if let Some(&(start, end, name)) = targets.iter().find(|target| target.0 == index) {
                let body = lines[start].trim_end_matches(['\r', '\n']);
                let ending = &lines[start][body.len()..];
                out.push_str(RE_EXPORT);
                out.push_str(name);
                out.push(';');
                out.push_str(ending);
                index = end;
                replaced += 1;
                continue;
            }
            out.push_str(lines[index]);
            index += 1;
        }
        let out = prune_orphan_imports(&out, &file)?;

        // The per-module half of the counter: the drop must equal the
        // closure's size — a number the walk above did not produce, so
        // a skipped block cannot satisfy it.
        let names_before: BTreeSet<String> = blocks.iter().map(|b| b.name.clone()).collect();
        let names_after: BTreeSet<String> = blocks
            .iter()
            .map(|b| b.name.clone())
            .filter(|name| !targets.iter().any(|t| t.2 == name.as_str()))
            .collect();
        let stats = RewireStats {
            before: blocks.len(),
            after: blocks.len() - replaced,
            replaced,
            names_before,
            names_after,
        };
        if stats.after != blocks.len() - closure.len() || replaced != closure.len() {
            bail!(
                "schema {}: the replacement wrote {} re-export{} where the \
                 closure pulls {} vocabular{}, leaving {} declarations of \
                 {} — the drop and the closure must agree exactly.\n\
                 Fix: this is a defect in the replacement pass \
                 (`xtask/src/codegen/shared_module/rewire.rs`), not in the \
                 schema; the run refuses to write a half-replaced module.",
                schema.display(),
                replaced,
                if replaced == 1 { "" } else { "s" },
                closure.len(),
                if closure.len() == 1 { "y" } else { "ies" },
                stats.after,
                blocks.len()
            );
        }
        super::super::write::write_generated(module, &out)
            .with_context(|| format!("writing the rewired {file}"))?;
        Ok(stats)
    }
}

/// The type name a vocabulary's name folds to — `version_entry` →
/// `VersionEntry` (`i18n_entry` → `I18nEntry`, the tail kept as
/// authored). The fold ROUTES the lookup only: the generator mints the
/// name, and every replacement is stitched on content afterwards, so
/// a day the fold and the generator disagree lands in the "block not
/// found" refusal rather than in a silently wrong merge.
pub(super) fn emitted_name(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    for segment in fragment.split('_') {
        let mut characters = segment.chars();
        if let Some(first) = characters.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(characters.as_str());
        }
    }
    out
}

/// The first line at which two blocks of text diverge — `None` when
/// they are byte-identical. Returns the 1-based line within the block
/// and both sides' line at that point, trimmed for the refusal text.
fn first_divergence<'a>(shared: &'a str, copy: &'a str) -> Option<(usize, &'a str, &'a str)> {
    let shared_lines: Vec<&str> = shared.split_inclusive('\n').collect();
    let copy_lines: Vec<&str> = copy.split_inclusive('\n').collect();
    for (index, (left, right)) in shared_lines.iter().zip(copy_lines.iter()).enumerate() {
        if left != right {
            return Some((
                index + 1,
                right.trim_end_matches(['\r', '\n']).trim(),
                left.trim_end_matches(['\r', '\n']).trim(),
            ));
        }
    }
    (shared_lines.len() != copy_lines.len()).then(|| {
        let at = shared_lines.len().min(copy_lines.len()) + 1;
        let left = shared_lines.get(at - 1).map_or("<none>", |l| l.trim());
        let right = copy_lines.get(at - 1).map_or("<none>", |l| l.trim());
        (at, right, left)
    })
}

/// Parse a generated module into its declaration blocks, refusing on
/// every shape the pinned emission does not write: an `impl` block
/// with no matching declaration ahead of it, a block that never
/// closes, a duplicate type name.
fn parse_blocks(src: &str, file: &str) -> Result<Vec<Block>> {
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    // A chunk keeps its line ending; `text_of` is the trimmed line.
    fn text_of(chunk: &str) -> &str {
        chunk.trim_end_matches(['\r', '\n']).trim()
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut index = 0;
    while index < lines.len() {
        let text = text_of(lines[index]);
        let Some((name, keyword)) = split_decl(text) else {
            if text.starts_with("impl") {
                bail!(
                    "{file}:{}: an `impl` block with no declaration ahead of \
                     it — the pinned emission attaches every impl to a type \
                     it just declared, and the block parser refuses to guess \
                     which type an orphan belongs to.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     the block parser in \
                     `xtask/src/codegen/shared_module/rewire.rs` the new \
                     shape, then run `cargo xtask codegen`.",
                    index + 1
                );
            }
            index += 1;
            continue;
        };
        if !seen.insert(name.to_string()) {
            bail!(
                "{file}:{}: the type `{name}` is declared twice — the \
                 generator declares each name once, so a second declaration \
                 means the block parser mis-tracked a body.\n\
                 Fix: this is a defect in \
                 `xtask/src/codegen/shared_module/rewire.rs`, not in the \
                 schemas.",
                index + 1
            );
        }
        // The block opens at its declaration; the doc comments and
        // attributes ahead of it belong to it (a blank or a previous
        // item's `}` ends that run naturally).
        let mut start = index;
        while start > 0 {
            let ahead = text_of(lines[start - 1]);
            if ahead.starts_with("///") || ahead.starts_with("#[") {
                start -= 1;
            } else {
                break;
            }
        }
        let mut end = match keyword {
            Keyword::Alias => index + 1,
            Keyword::Struct | Keyword::Enum => close_of(&lines, index, file)? + 1,
        };
        // The impl blocks the emission attached to this type follow it,
        // each separated by one blank; they leave with the type.
        loop {
            let after_blank = end + usize::from(text_of(lines.get(end).unwrap_or(&"")).is_empty());
            let candidate = text_of(lines.get(after_blank).unwrap_or(&""));
            if impl_name(candidate) == Some(name) {
                end = close_of(&lines, after_blank, file)? + 1;
            } else {
                break;
            }
        }
        blocks.push(Block {
            name: name.to_string(),
            span: (start, end),
            text: lines[start..end].concat(),
        });
        index = end;
    }
    Ok(blocks)
}

/// The line index of the `}` at column zero that closes the item
/// opened at `open`, or a refusal when the file ends first.
fn close_of(lines: &[&str], open: usize, file: &str) -> Result<usize> {
    // The match is on the UNINDENTED line: an item's body closes at
    // column zero, and a trimmed match would stop on an indented brace
    // a nested item closed — an opened vocabulary's hand-rolled impl
    // nests a `fn`, and its `    }` is not the impl's end.
    for (index, chunk) in lines.iter().enumerate().skip(open + 1) {
        if chunk.trim_end_matches(['\r', '\n']) == "}" {
            return Ok(index);
        }
    }
    bail!(
        "{file}:{}: the item opened here never closes before end of file — \
         jtd-codegen closes every item it emits, so the file this parser \
         read is not the shape it is pinned to.\n\
         Fix: restore the pinned jtd-codegen version, or teach the block \
         parser in `xtask/src/codegen/shared_module/rewire.rs` the new \
         shape, then run `cargo xtask codegen`.",
        open + 1
    );
}

/// Split a declaration line — `pub struct <Name> {`, `pub enum
/// <Name> {` or `pub type <Name> = <Rhs>;` — into its name and kind,
/// or `None` when the line is not that shape.
fn split_decl(text: &str) -> Option<(&str, Keyword)> {
    if let Some(rest) = text
        .strip_prefix("pub struct ")
        .and_then(|rest| rest.strip_suffix('{'))
    {
        let name = rest.trim_end();
        return is_ident(name).then_some((name, Keyword::Struct));
    }
    if let Some(rest) = text
        .strip_prefix("pub enum ")
        .and_then(|rest| rest.strip_suffix('{'))
    {
        let name = rest.trim_end();
        return is_ident(name).then_some((name, Keyword::Enum));
    }
    if let Some(rest) = text
        .strip_prefix("pub type ")
        .and_then(|rest| rest.strip_suffix(';'))
        && let Some((name, _)) = rest.split_once(" = ")
        && is_ident(name)
    {
        return Some((name, Keyword::Alias));
    }
    None
}

enum Keyword {
    Struct,
    Enum,
    Alias,
}

/// The type an impl header names — `impl Serialize for X {`,
/// `impl<'de> Deserialize<'de> for X {` and the inherent `impl X {` —
/// or `None` when the header is not one of those shapes.
fn impl_name(text: &str) -> Option<&str> {
    let mut rest = text.strip_prefix("impl")?;
    // Generics on the impl itself: skip to their closing `>`.
    if rest.starts_with('<') {
        rest = rest.split_once('>')?.1;
    }
    let rest = rest.trim_end_matches('{').trim_end();
    let name = match rest.rsplit_once(" for ") {
        Some((_, tail)) => tail.trim(),
        None => rest.trim(),
    };
    is_ident(name).then_some(name)
}

/// ASCII identifier shape — the same contract the sibling passes'
/// matchers enforce around the identifiers they rewrite.
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
