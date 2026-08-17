//! The domain-types pass — the seventh content edit the generator's
//! emission takes (the order rule lives in `postproc`'s docs: a pass
//! keyed to the emission shape runs while the file is STILL that
//! emission, and this one is keyed to the `pub type <Name> = …;` alias
//! line, the `pub struct <Name> {` / `pub enum <Name> {` declaration
//! line and the `use <path>::{A, B};` import line, so it runs after arm
//! boxing, field snake_casing, map ordering, the empty-collection
//! policy, the optional-shape pass and reader strictness, and before
//! the vocabularies open — opening writes hand-rolled `impl Serialize` /
//! `impl Deserialize` blocks into the file, text the pinned emission
//! shape does not contain).
//!
//! What it enforces: the RUST TYPE a definition binds to is a policy the
//! schema declares, never a guess. `metadata."x-rust-type"` names one
//! of the two halves of the emitted declaration, and the definition's
//! own JTD form decides which half — so the reading cannot pick the
//! wrong one. A `type` (primitive) form is emitted as an alias, and the
//! annotation is its RIGHT SIDE: `vocab.group` carries
//! `vibe_core::Group` and the emission becomes
//! `pub type Group = vibe_core::Group;` (the annotation cannot name the
//! alias's own name — `pub type Group = Group;` says nothing). An
//! object, enum or discriminator form is emitted as a named type, and
//! the annotation is its NAME: the journal root carries
//! `JournalRecord`, the generator named the root after the schema's
//! stem, and every whole-token occurrence of `Journal` in the file goes
//! over to `JournalRecord` (the annotation cannot name a right side —
//! a structure's "type" is itself). A matching name is a no-op, byte
//! for byte. Any OTHER form under the annotation — `elements`,
//! `values`, `ref`, an empty form — refuses, naming the schema, the
//! definition and the form: the pass rules through forms it can
//! classify from both sides, and a guess would write a shape nothing
//! pins. A definition without the annotation is not its business at
//! all (`Entry` lives without one), and a missing declaration for an
//! annotated definition refuses the same loud way — the emission shape
//! of jtd-codegen is pinned by its version, and its drift must be
//! heard, not absorbed.
//!
//! P23 — the rule this pass carries, paid for in full right here: an
//! `x-rust-type` annotation names a path resolvable WITHOUT the file's
//! own imports (`vibe_core::Group`, `semver::Version`,
//! `chrono::DateTime<chrono::Utc>`, never a bare `Group` or `Utc` that
//! resolves only while the generator's import happens to be in scope —
//! an annotation whose resolvability rides on the generator's import
//! breaks the moment the emission shape shifts, the exact coupling
//! this layer exists to break). The price: a substitution may orphan
//! an import item — the alias line was the last place the generator's
//! own `DateTime` / `FixedOffset` tokens stood — and the pass removes
//! EXACTLY those items, no more. An item is orphaned when its token
//! has no remaining usage: not on any `use` line, not inside a span
//! this pass itself wrote, and not as a `::`-qualified path segment
//! (`chrono::DateTime`, in an annotation or a doc comment, names the
//! path — it is not a reference to the imported name). An emptied
//! `use` line goes away entirely, newline included; an item with a
//! usage left stays (indiscriminate cleanup would be as wrong as
//! none); a `use` line that is neither `use <path>::{A, B};` nor
//! `use <path>;` refuses loudly, naming the file and the line.
//!
//! What the taking-away leaves behind is not a matter of taste either,
//! and the reason is the panel rather than neatness: a generated file
//! is never hand-edited, so whatever this pass writes has to be what
//! `cargo fmt --all --check` — the panel's FIRST step — already
//! accepts, or the tree carries a red nobody is allowed to fix. Two
//! shapes follow from that. A braced import down to its last survivor
//! is written unbraced (`use a::b;`), because rustfmt rewrites
//! `use a::{b};` and the check would fail on the difference. And a line
//! removed from between two blanks takes the second blank with it,
//! because the two were one separator around something that is now
//! gone.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};

mod rulings;
mod variants;

use rulings::{Arm, Keyword, Ruling, declared_in, domain_rulings};
use variants::rename_variants;

/// The pass entry the driver calls: read the domain-type rulings off
/// the document the generator read (`resolved` — the authored schema
/// when it pulls no vocabularies, the scratch copy with the fragments
/// placed otherwise), then stitch the generated Rust to them.
/// Refusals name the AUTHORED schema — the file a human edits — the
/// same cut the sibling schema-reading passes make.
pub(super) fn apply_domain_types(
    src: &str,
    file: &str,
    resolved: &Path,
    schema: &Path,
) -> Result<String> {
    let rulings = domain_rulings(resolved, schema)?;
    apply_rulings(src, file, schema, &rulings)
}

/// One import line of the emission, parsed: where it sits and what it
/// binds.
struct Import {
    /// The line's index in the file, for the rebuild walk.
    line: usize,
    /// The span the line occupies in the substituted body — the usage
    /// count skips it whole.
    span: (usize, usize),
    /// The parsed shape.
    form: UseForm,
}

/// The two import shapes the pinned emission writes.
enum UseForm {
    /// `use <path>::{A, B};` — every braced name binds an item.
    Braced { head: String, items: Vec<String> },
    /// `use <path>;` — the last path segment binds the item.
    Plain { last: String },
}

/// Stitch the rulings into the generated text: validate that every
/// ruling's declaration stands in the file (a missing one means the
/// emission shape moved, and the pass says so rather than skipping),
/// substitute, then take away the import items the substitutions
/// orphaned — exactly those, nothing else.
fn apply_rulings(src: &str, file: &str, schema: &Path, rulings: &[Ruling]) -> Result<String> {
    // Phase 1 — locate every declaration and import, refusing any
    // `use` line the pinned emission does not write.
    let mut imports: BTreeMap<usize, UseForm> = BTreeMap::new();
    let mut aliases: BTreeMap<&str, usize> = BTreeMap::new();
    let mut decls: BTreeMap<&str, (usize, Keyword)> = BTreeMap::new();
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let text = chunk.trim_end_matches(['\r', '\n']).trim();
        if text.starts_with("use ") {
            let Some(form) = parse_use(text) else {
                bail!(
                    "{file}:{}: a `use` line this pass cannot parse:\n\
                     `{text}`\n\
                     The pinned jtd-codegen emission writes imports only as \
                     `use <path>::{{A, B}};` or `use <path>;`, and the \
                     domain-types pass removes exactly the items its \
                     substitutions orphan — reasoning about any other shape \
                     would hide a moved pin behind a green run.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `domain_types.rs` the new shape, then run \
                     `cargo xtask codegen`.",
                    index + 1
                );
            };
            imports.insert(index, form);
            continue;
        }
        if let Some((name, _)) = split_alias(text) {
            aliases.insert(name, index);
        }
        if let Some((name, keyword)) = split_decl(text) {
            decls.insert(name, (index, keyword));
        }
    }
    for ruling in rulings {
        match &ruling.arm {
            None => {}
            Some(Arm::RightSide(annotation)) => {
                if !aliases.contains_key(ruling.emitted.as_str()) {
                    bail!(
                        "schema {}: the definition `{}` carries \
                         `x-rust-type` = `{annotation}` — its `type` form \
                         makes the annotation the alias's right side, but the \
                         generated file declares no `pub type {} = …;`. The \
                         emission shape of jtd-codegen this pass is pinned to \
                         has moved.\n\
                         Fix: restore the pinned jtd-codegen version, or \
                         teach `domain_types.rs` the new shape, then run \
                         `cargo xtask codegen`.",
                        declared_in(schema),
                        ruling.definition,
                        ruling.emitted
                    );
                }
            }
            Some(Arm::Name(annotation, keyword)) => match decls.get(ruling.emitted.as_str()) {
                None => bail!(
                    "schema {}: the definition `{}` carries `x-rust-type` = \
                     `{annotation}` — the annotation names the emitted type, \
                     but the generated file declares no `pub {} {}`. The \
                     emission shape of jtd-codegen this pass is pinned to has \
                     moved.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `domain_types.rs` the new shape, then run \
                     `cargo xtask codegen`.",
                    declared_in(schema),
                    ruling.definition,
                    keyword.as_str(),
                    ruling.emitted
                ),
                Some(&(_, found)) if found != *keyword => bail!(
                    "schema {}: the definition `{}` carries `x-rust-type` = \
                     `{annotation}` and its form rules the `pub {}` keyword, \
                     but the generated file declares `pub {} {}` — the two \
                     sides disagree about the emission, and the pass refuses \
                     to guess which one moved.\n\
                     Fix: restore the pinned jtd-codegen version, or teach \
                     `domain_types.rs` the new shape, then run \
                     `cargo xtask codegen`.",
                    declared_in(schema),
                    ruling.definition,
                    keyword.as_str(),
                    found.as_str(),
                    ruling.emitted
                ),
                _ => {}
            },
        }
    }

    // Phase 1b — variant names, applied inside their own declarations
    // and nowhere else. It runs before the file-wide walk rather than
    // inside it because the two rewrites have different SCOPES: a
    // type's identifier is unique in its file, a variant's belongs to
    // its enum, and mixing them would let a variant rename reach a
    // same-named type elsewhere. Line count is preserved, so the
    // indices phase 1 collected stay valid.
    let src = &rename_variants(src, file, schema, rulings, &decls)?;

    // Phase 2 — substitute. The name arm renames whole tokens on every
    // line except the imports (a renamed type is declared in this file;
    // an import path names an external one); the right-side arm
    // rewrites its alias line; everything else is copied byte for
    // byte, layout and line endings included.
    let renames: BTreeMap<String, String> = rulings
        .iter()
        .filter_map(|ruling| match &ruling.arm {
            Some(Arm::Name(annotation, _)) if *annotation != ruling.emitted => {
                Some((ruling.emitted.clone(), annotation.clone()))
            }
            _ => None,
        })
        .collect();
    let right_sides: BTreeMap<&str, &str> = rulings
        .iter()
        .filter_map(|ruling| match &ruling.arm {
            Some(Arm::RightSide(annotation)) => {
                Some((ruling.emitted.as_str(), annotation.as_str()))
            }
            _ => None,
        })
        .collect();
    let mut body = String::with_capacity(src.len() + src.len() / 8);
    let mut placed: Vec<Import> = Vec::new();
    let mut written: Vec<(usize, usize)> = Vec::new();
    for (index, chunk) in src.split_inclusive('\n').enumerate() {
        let line_body = chunk.trim_end_matches(['\r', '\n']);
        let text = line_body.trim();
        if let Some(form) = imports.remove(&index) {
            let start = body.len();
            body.push_str(chunk);
            placed.push(Import {
                line: index,
                span: (start, body.len()),
                form,
            });
            continue;
        }
        if let Some((name, _)) = split_alias(text)
            && let Some(annotation) = right_sides.get(name)
        {
            let indent = &line_body[..line_body.len() - text.len()];
            body.push_str(indent);
            body.push_str("pub type ");
            body.push_str(name);
            body.push_str(" = ");
            let start = body.len();
            body.push_str(annotation);
            written.push((start, body.len()));
            body.push(';');
            body.push_str(&chunk[line_body.len()..]);
            continue;
        }
        if renames.is_empty() {
            body.push_str(chunk);
        } else {
            let base = body.len();
            let (line, spans) = rename_tokens(line_body, &renames);
            body.push_str(&line);
            body.push_str(&chunk[line_body.len()..]);
            written.extend(spans.iter().map(|(s, e)| (base + s, base + e)));
        }
    }

    // Phase 3 — take away what the substitutions orphaned. Nothing
    // written means nothing orphaned: the imports stand as the
    // generator left them.
    if written.is_empty() {
        return Ok(body);
    }
    let items: BTreeSet<&str> = placed
        .iter()
        .flat_map(|import| match &import.form {
            UseForm::Braced { items, .. } => items.iter().map(String::as_str).collect(),
            UseForm::Plain { last } => vec![last.as_str()],
        })
        .collect();
    let counts = count_usages(&body, &placed, &written, &items);
    let mut out = String::with_capacity(body.len());
    // Whether the last line actually written was blank — the left half
    // of the separator question a removal raises.
    let mut previous_blank = false;
    // Set by a removal whose left neighbour was blank: the blank that
    // FOLLOWS such a line is a second separator around nothing, and it
    // goes with it.
    let mut squash_next_blank = false;
    for (index, chunk) in body.split_inclusive('\n').enumerate() {
        let blank = chunk.trim().is_empty();
        if squash_next_blank {
            squash_next_blank = false;
            if blank {
                continue;
            }
        }
        let Some(import) = placed.iter().find(|import| import.line == index) else {
            out.push_str(chunk);
            previous_blank = blank;
            continue;
        };
        match &import.form {
            UseForm::Braced { head, items } => {
                let kept: Vec<&str> = items
                    .iter()
                    .map(String::as_str)
                    .filter(|item| counts.get(*item).copied().unwrap_or(0) > 0)
                    .collect();
                if kept.is_empty() {
                    // Every item orphaned: the whole line, newline
                    // included, goes.
                    squash_next_blank = previous_blank;
                    continue;
                }
                if kept.len() == items.len() {
                    out.push_str(chunk);
                    previous_blank = false;
                    continue;
                }
                let line_body = chunk.trim_end_matches(['\r', '\n']);
                let text = line_body.trim();
                let indent = &line_body[..line_body.len() - text.len()];
                out.push_str(indent);
                out.push_str("use ");
                out.push_str(head);
                out.push_str("::");
                if let [single] = kept[..] {
                    // One survivor wears no braces: `use a::{b};` is a
                    // shape rustfmt rewrites, and a generated file is
                    // never hand-edited, so the pass emits the form the
                    // panel's first gate already accepts.
                    out.push_str(single);
                } else {
                    out.push('{');
                    out.push_str(&kept.join(", "));
                    out.push('}');
                }
                out.push(';');
                out.push_str(&chunk[line_body.len()..]);
                previous_blank = false;
            }
            UseForm::Plain { last } => {
                if counts.get(last.as_str()).copied().unwrap_or(0) == 0 {
                    squash_next_blank = previous_blank;
                    continue;
                }
                out.push_str(chunk);
                previous_blank = false;
            }
        }
    }
    Ok(out)
}

/// Rewrite every whole-token occurrence of the mapped names on one
/// line, reporting the spans the rewrites wrote — the orphan analysis
/// excludes them, because a name this pass wrote is not a usage of an
/// import item.
fn rename_tokens(line: &str, renames: &BTreeMap<String, String>) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(line.len());
    let mut written = Vec::new();
    let mut run = String::new();
    let mut run_start: Option<usize> = None;
    for (offset, character) in line.char_indices() {
        if character.is_ascii_alphanumeric() || character == '_' {
            run_start.get_or_insert(offset);
            run.push(character);
            continue;
        }
        if run_start.take().is_some() {
            if let Some(new) = renames.get(&run) {
                out.push_str(new);
                written.push((out.len() - new.len(), out.len()));
            } else {
                out.push_str(&run);
            }
            run.clear();
        }
        out.push(character);
    }
    if run_start.is_some() {
        if let Some(new) = renames.get(&run) {
            out.push_str(new);
            written.push((out.len() - new.len(), out.len()));
        } else {
            out.push_str(&run);
        }
    }
    (out, written)
}

/// Count, per import item, the references that keep it alive: whole
/// tokens outside every `use` line, outside the spans this pass wrote,
/// and not as a `::`-qualified path segment — `chrono::DateTime`, in an
/// annotation or a doc comment, names the path, never the import.
fn count_usages(
    body: &str,
    imports: &[Import],
    written: &[(usize, usize)],
    items: &BTreeSet<&str>,
) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut run = String::new();
    let mut run_start: Option<usize> = None;
    for (offset, character) in body.char_indices() {
        if character.is_ascii_alphanumeric() || character == '_' {
            run_start.get_or_insert(offset);
            run.push(character);
            continue;
        }
        if let Some(start) = run_start.take() {
            let alive = items.contains(run.as_str())
                && !body[..start].ends_with(':')
                && !imports
                    .iter()
                    .any(|import| import.span.0 <= start && start < import.span.1)
                && !written.iter().any(|(s, e)| *s <= start && start < *e);
            if alive {
                *counts.entry(run.clone()).or_insert(0) += 1;
            }
            run.clear();
        }
    }
    counts
}

/// Parse one `use` line of the pinned emission — `use <path>::{A, B};`
/// or `use <path>;` — or return `None` when the line is any other
/// shape: glob imports, aliases, nested braces, spaced or multi-word
/// items are not forms the generator writes, and the caller refuses
/// rather than reason around a shape it was not told about.
fn parse_use(text: &str) -> Option<UseForm> {
    let rest = text.strip_prefix("use ")?.strip_suffix(';')?;
    if let Some((head, braced)) = rest.split_once("::{") {
        let items_text = braced.strip_suffix('}')?;
        if !is_path(head) {
            return None;
        }
        let mut items: Vec<String> = Vec::new();
        for item in items_text.split(", ") {
            if !is_ident(item) {
                return None;
            }
            items.push(item.to_string());
        }
        return Some(UseForm::Braced {
            head: head.to_string(),
            items,
        });
    }
    if !rest.contains("::") || !is_path(rest) {
        return None;
    }
    let last = rest.rsplit("::").next()?;
    Some(UseForm::Plain {
        last: last.to_string(),
    })
}

/// A `::`-joined run of identifiers — the head of a braced import or a
/// plain import path.
fn is_path(text: &str) -> bool {
    !text.is_empty() && text.split("::").all(is_ident)
}

/// ASCII identifier shape — the same contract the sibling passes'
/// matchers enforce for the identifiers they rewrite around.
fn is_ident(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Split an alias line of the pinned shape — `pub type <Ident> = <Rhs>;`
/// — into its name and right side, or `None` when the line is not that
/// shape.
fn split_alias(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("pub type ")?.strip_suffix(';')?;
    let (name, rhs) = rest.split_once(" = ")?;
    if !is_ident(name) || rhs.is_empty() {
        return None;
    }
    Some((name, rhs))
}

/// Split a declaration line — `pub struct <Ident> {` or
/// `pub enum <Ident> {` — into its name and keyword, or `None` when
/// the line is not that shape.
///
/// The trailing `trim_end` is load-bearing, not defensive: the brace
/// sits one space after the name, so stripping it leaves that space
/// glued to the identifier, and `is_ident` rejects a space. Without the
/// trim this matcher answers `None` for EVERY declaration the generator
/// writes, the declaration map comes out empty, and the name arm refuses
/// on a file that in fact holds exactly what it was looking for.
fn split_decl(text: &str) -> Option<(&str, Keyword)> {
    let (keyword, rest) = if let Some(rest) = text.strip_prefix("pub struct ") {
        (Keyword::Struct, rest)
    } else {
        let rest = text.strip_prefix("pub enum ")?;
        (Keyword::Enum, rest)
    };
    let name = rest.strip_suffix('{')?.trim_end();
    if !is_ident(name) {
        return None;
    }
    Some((name, keyword))
}

#[cfg(test)]
#[path = "domain_types/tests.rs"]
mod tests;
