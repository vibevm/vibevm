//! The static compilation pipeline (PROP-035 §8) — the primitives, composed.
//!
//! `compile_static` runs the phases in the fixed order the spec pins:
//!
//! 1. **parse / topo** — build the `#use` graph from the seed and order it so
//!    every dependency precedes its dependents (§7.2, §8 phase 2);
//! 2. **source-merge** — fold `source` into `contract` (§7.3) — *deferred*: the
//!    `#source` contract→impl resolution lands in a follow-up, noted below;
//! 3. **embed-expand** — splice every `#embed` to a fixed point (§7.1);
//! 4. **emit** — concatenate the nodes in topological order, each wrapped in
//!    open/close markers (§11), so the output is reversible.
//!
//! A `#use` line is *resolved by the ordering* — its target is emitted, once,
//! above — so the line itself is stripped from a node's body on emit; it would
//! otherwise be a dangling directive in the compiled `STATIC.md`. `@spec`
//! in-place references are left in prose (their target is likewise already
//! above). No `#embed` survives (§7.1).
//!
//! This is the algorithmic, LLM-free static compiler (§2) — the reference
//! semantics the structural loader is later checked against.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt::Write as _;

use crate::address::{Authority, SpecAddress, SpecAddressError};
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::embed::{EmbedError, SectionSource, expand_embeds};
use crate::gate::{DuplicateId, first_duplicate};
use crate::merge::fold_source;
use crate::qualify::{RenameEntry, qualify_contribution, read_anchor_id};
use crate::use_graph::{UseGraphError, topo_order_from};

/// Why static compilation failed.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    UseGraph(#[from] UseGraphError),
    #[error(transparent)]
    Embed(#[from] EmbedError),
    #[error("internal: re-parsing topo key `{0}` failed")]
    Address(#[from] SpecAddressError),
    #[error("cannot load {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
    /// The `#source` merge produced a document whose anchor namespace is no
    /// longer unique — a collision the per-fact override did not cancel
    /// (PROP-035 §7.3, clause 3). Fails the build; never a warning.
    #[error("merged {addr}: {dup}")]
    DuplicateId { addr: String, dup: DuplicateId },
    /// A short reference `(#x)` in the compiled closure names a label two or
    /// more nodes define (B-006 rider / PROP-035 §8 phase 5). Per-node
    /// qualification already resolved every within-node reference, so this
    /// fires only on a *cross-node* short link the compiler cannot attribute
    /// without guessing. Fails the build citing the candidate qualified heirs
    /// (B-011: fail with candidates, never a silent pick); the author must cite
    /// one explicitly.
    #[error("ambiguous short link `{label}`: defined by {}", .candidates.join(", "))]
    AmbiguousShortLink {
        label: String,
        candidates: Vec<String>,
    },
}

/// Compile the closure reachable from `seed` into a single static document —
/// the **unqualified** reference semantics (PROP-035 §2) the structural loader
/// is later checked against. See [`compile_static_qualified`] for the per-node
/// origin-qualified compile a `normal` static lane ships.
pub fn compile_static(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<String, CompileError> {
    let (out, _) = compile_static_inner(seed, source, CompileMode::Plain)?;
    Ok(out)
}

/// Compile the closure reachable from `seed` and qualify **every node under its
/// own authoring origin** (PROP-035 §8 phase 5, B-006 rider).
///
/// Unlike [`compile_static`], each emitted node is passed through
/// [`qualify_contribution`] under the origin derived from its topo key
/// (`<group>/<name>`, or the host token) — so a node a `normal` package splices
/// in from *another* package via `#use` is qualified under THAT package's
/// origin, never the entry's. Returns the compiled lane alongside the per-node
/// rename map (`(origin, rename)`, in emit order) for the tombstone.
///
/// A second pass then resolves the cross-node short references the per-node
/// qualify leaves behind: a `(#x)` in node A whose target lives in node B is
/// rewritten to B's qualified heir; a label two or more nodes define is a build
/// error ([`CompileError::AmbiguousShortLink`]) citing the candidates; a label
/// no node defines is left for the loader's two-scope lookup.
pub fn compile_static_qualified(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    compile_static_inner(seed, source, CompileMode::QualifyPerNode)
}

/// Whether [`compile_static_inner`] qualifies each node under its own origin.
#[derive(Clone, Copy)]
enum CompileMode {
    /// Reference semantics — labels emitted as authored (the structural
    /// loader's oracle).
    Plain,
    /// Per-node origin qualification (PROP-035 §8 phase 5, B-006 rider).
    QualifyPerNode,
}

/// The shared phase loop (PROP-035 §8): parse/topo → source-merge → embed →
/// emit. In [`CompileMode::QualifyPerNode`] each node is qualified under its
/// own origin before emission and a second pass resolves cross-node short
/// references; in [`CompileMode::Plain`] the body is emitted as-authored and
/// the rename map is empty. One loop, parameterised by mode — never two copies
/// of the phase body (B-006 rider).
fn compile_static_inner(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: CompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let order = topo_order_from(seed, source)?; // phase 2
    let qualify = matches!(mode, CompileMode::QualifyPerNode);

    let mut out = String::new();
    let mut renames: Vec<(String, RenameEntry)> = Vec::new();
    for key in &order {
        let addr = SpecAddress::parse(key)?;
        let text = source
            .section_text(&addr)
            .map_err(|reason| CompileError::Unresolved {
                addr: key.clone(),
                reason,
            })?;

        // phase 3 — fold source into a contract that declares #source, then
        // re-gate id uniqueness over the merged view (§7.3, clause 3): a
        // duplicate the per-fact override did not cancel fails the build.
        let folded = match first_source_directive(&text) {
            Some(source_addr) => {
                let contract_tree = DocTree::parse(&text);
                let src_text = source.section_text(&source_addr).map_err(|reason| {
                    CompileError::Unresolved {
                        addr: source_addr.to_string(),
                        reason,
                    }
                })?;
                let merged = fold_source(&contract_tree, &DocTree::parse(&src_text));
                if let Some(dup) = first_duplicate(&DocTree::parse(&merged)) {
                    return Err(CompileError::DuplicateId {
                        addr: key.clone(),
                        dup,
                    });
                }
                merged
            }
            None => text,
        };
        // phase 4 — embed over the use/source-resolved body.
        let body = strip_directive_lines(&folded, &[DirectiveKind::Use, DirectiveKind::Source]);
        let expanded = expand_embeds(&body, source)?;

        // B-011 §7.4 (PROP-035 §8 phase 5): rewrite every `@!<Alias>` to the
        // full `@spec://<target>` it denotes. The alias table is parsed from the
        // pre-strip `folded` text, so the `#use … as <Alias>` bindings survive
        // even though the declaration lines themselves are stripped above (they
        // leave the body together with every other `#use` line). The compiled
        // lane is then self-describing without the alias table, and resolvable
        // after any future cleaning — the alias binds to the address, never to
        // compiled text.
        let aliases = Directives::parse(&folded).aliases;
        let emitted = rewrite_at_bang(&expanded, &aliases);

        // B-006 rider (PROP-035 §8 phase 5): in qualified mode each node's
        // emitted body is qualified under ITS OWN authoring origin — derived
        // from the topo key the same way `normal_seed` derives a package
        // coordinate — never the entry's, so a node spliced in from another
        // package keeps its true provenance. Per-node, so a node referencing
        // its own label is resolved within the node; a cross-node short link is
        // left for the second pass below.
        let emitted = if qualify {
            let origin = node_origin(&addr);
            let (qualified, node_renames) = qualify_contribution(&emitted, &origin);
            renames.extend(node_renames.into_iter().map(|r| (origin.clone(), r)));
            qualified
        } else {
            emitted
        };

        writeln!(out, "{}", crate::markers::open(key)).unwrap(); // phase 5
        out.push_str(&emitted);
        if !emitted.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "{}", crate::markers::close(key)).unwrap();
    }

    if qualify {
        // Second pass — resolve the cross-node short references the per-node
        // qualify could not see (B-006 rider).
        out = resolve_cross_node_short_links(&out, &renames)?;
    }
    Ok((out, renames))
}

/// The authoring origin of a closure node — `<group>/<name>` for a package, the
/// host token for the host project (PROP-035 §6). Derived from the node's topo
/// key by the same authority half `normal_seed` builds a coordinate from, so a
/// node compiled from another package's `#use` target is qualified under THAT
/// package's origin, not the entry's (B-006 rider).
fn node_origin(addr: &SpecAddress) -> String {
    match &addr.authority {
        Authority::Host(h) => h.clone(),
        Authority::Package { group, name, .. } => format!("{group}/{name}"),
    }
}

/// The first `#source` address in a document, if it declares one (§7.3).
fn first_source_directive(text: &str) -> Option<SpecAddress> {
    Directives::parse(text)
        .directives
        .into_iter()
        .find(|d| d.kind == DirectiveKind::Source)
        .map(|d| d.address)
}

/// Remove directive lines of the given kinds. `#use` is resolved by the
/// ordering and `#source` by the fold, so both would be leftovers in the
/// compiled output.
fn strip_directive_lines(text: &str, kinds: &[DirectiveKind]) -> String {
    let directives = Directives::parse(text);
    let strip: HashSet<usize> = directives
        .directives
        .iter()
        .filter(|d| kinds.contains(&d.kind))
        .map(|d| d.line)
        .collect();

    let kept: Vec<&str> = text
        .lines()
        .enumerate()
        .filter(|(i, _)| !strip.contains(i))
        .map(|(_, line)| line)
        .collect();
    kept.join("\n")
}

/// Rewrite every `@!<Alias>` in `text` to the full `@spec://<target>` its alias
/// binds to (B-011 §7.4 / PROP-035 §8 phase 5). Fenced code blocks are left
/// untouched (the shared fence mask). An `@!X` whose `X` is not a declared alias
/// is left in place — it is already a `DirectiveError` the scan recorded, and
/// the rewrite must not silently drop prose. The fast path (no aliases) returns
/// the text unchanged, so a directive-free lane is byte-identical.
fn rewrite_at_bang(text: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    if aliases.is_empty() {
        return text.to_string();
    }
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = crate::doctree::fence_mask(&lines);
    let out_lines: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if fenced[i] {
                line.clone()
            } else {
                rewrite_at_bang_line(line, aliases)
            }
        })
        .collect();
    out_lines.join("\n")
}

/// Rewrite `@!<Alias>` occurrences in a single non-fenced line, leaving
/// everything else byte-identical. The identifier boundary reuses
/// [`directives::identifier_run`] so this rewrite and the scanner can never
/// disagree on what counts as a name.
fn rewrite_at_bang_line(line: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize; // first not-yet-flushed byte (exclusive boundary)
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'@' && bytes[i + 1] == b'!' {
            let id = crate::directives::identifier_run(&line[i + 2..]);
            if !id.is_empty()
                && let Some(target) = aliases.get(id)
            {
                out.push_str(&line[last..i]);
                out.push('@');
                out.push_str(&target.without_pin());
                let after = i + 2 + id.len();
                last = after;
                i = after;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    out
}

/// Second pass of per-node qualification (B-006 rider / PROP-035 §8 phase 5):
/// resolve the cross-node short references the per-node qualify left behind.
///
/// After every node's labels are qualified under its own origin, a `(#x)` in
/// node A whose target lives in node B is still bare — node A's qualify pass
/// could not see B's labels. This pass walks the assembled lane (outside fenced
/// code) and rewrites each remaining `(#x)` against the union of every node's
/// definitions: a label one node defines → that node's qualified heir; a label
/// ≥2 nodes define → a build error ([`CompileError::AmbiguousShortLink`]) citing
/// the candidates (B-011: fail with candidates, never a silent pick); a label no
/// node defines → left as written (resolving it is the loader's two-scope
/// lookup, not the compiler's).
fn resolve_cross_node_short_links(
    text: &str,
    renames: &[(String, RenameEntry)],
) -> Result<String, CompileError> {
    // The union map: short label → every (origin, qualified heir) that defines
    // it, across the whole closure. Built from the per-node rename maps.
    let mut defs: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (origin, r) in renames {
        defs.entry(r.original.clone())
            .or_default()
            .push((origin.clone(), r.qualified.clone()));
    }

    // Split on '\n' (not `lines()`) so a trailing newline round-trips; reuse the
    // qualify cell's fence mask so this pass and the per-node pass agree on what
    // is code.
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = crate::doctree::fence_mask(&lines);
    let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if fenced[i] {
            out_lines.push(line.clone());
        } else {
            out_lines.push(rewrite_cross_node_links(line, &defs)?);
        }
    }
    Ok(out_lines.join("\n"))
}

/// Rewrite the remaining `(#x)` short references in one non-fenced line against
/// the union definition map (B-006 rider). Inline-code spans are skipped via the
/// same backtick toggle the qualify cell uses, and the anchor id is read with
/// the qualify cell's [`read_anchor_id`] scanner so the two passes never disagree
/// on what counts as a name. References already qualified by the per-node pass
/// (a `<slug>--<id>` form) are not keys in `defs` and so pass through untouched.
fn rewrite_cross_node_links(
    line: &str,
    defs: &BTreeMap<String, Vec<(String, String)>>,
) -> Result<String, CompileError> {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize; // first not-yet-flushed byte (exclusive boundary)
    let mut i = 0usize;
    let mut in_code = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if !in_code
            && b == b'('
            && bytes.get(i + 1) == Some(&b'#')
            && let Some((id, after_id)) = read_anchor_id(bytes, i + 2)
            && bytes.get(after_id) == Some(&b')')
        {
            match defs.get(id) {
                Some(heirs) if heirs.len() == 1 => {
                    // A unique definer → rewrite to its qualified heir.
                    out.push_str(&line[last..i]);
                    out.push_str("(#");
                    out.push_str(&heirs[0].1);
                    out.push(')');
                    last = after_id + 1;
                    i = after_id + 1;
                    continue;
                }
                Some(heirs) => {
                    // ≥2 definers → ambiguous: fail citing the candidates.
                    let mut candidates: Vec<String> = heirs
                        .iter()
                        .map(|(origin, qualified)| format!("{qualified} ({origin})"))
                        .collect();
                    candidates.sort();
                    return Err(CompileError::AmbiguousShortLink {
                        label: id.to_string(),
                        candidates,
                    });
                }
                None => {} // no definer → leave the reference as written
            }
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    Ok(out)
}

#[cfg(test)]
mod tests;
