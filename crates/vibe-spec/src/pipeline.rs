//! The static compilation pipeline (PROP-035 §8) — the primitives, composed.
//!
//! `compile_static` runs the phases in the fixed order the spec pins:
//!
//! 1. **parse / close** — the scheduler loads the finite explicit-`#use`
//!    worklist; named `parse` lowers each source, then named `close` owns graph
//!    cycles and dependency-before-dependent order (§7.2, §8 phase 2);
//! 2. **source-merge** — fold every declared `#source` into `contract`, in
//!    declaration order (§7.3);
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
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;

use crate::address::{SpecAddress, SpecAddressError};
use crate::compiler::ir::{ClosureContribution, ClosureIr, DocumentAddress};
use crate::directives::{DirectiveKind, Directives};
use crate::embed::{EmbedError, SectionSource, expand_embeds};
use crate::gate::DuplicateId;
use crate::qualify::{RenameEntry, qualify_contribution, read_anchor_id};
use crate::use_graph::UseGraphError;

use fold::fold_source_closure;

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
    /// Two or more sources each declare a top-level section the contract does
    /// **not**, under one anchor — two definitions of one name (B-056). A
    /// section matching a contract anchor is an `:add` sum (one definition,
    /// many contributions); a section the contract never declared is a fresh
    /// definition, and a name has one. The post-merge [`DuplicateId`] gate
    /// cannot see this: by the time it runs, provenance is folded away and the
    /// repeat reads as the accepted `:add` artifact. Caught pre-fold, where each
    /// source's tree is still separate — and only as a fallback after
    /// [`DuplicateId`], so a colliding fact still names the more specific id.
    #[error(
        "merged {addr}: section `{anchor}` is defined by more than one source \
             but not by the contract — a source section matching a contract anchor \
             is an :add sum, but one the contract never declared is a definition, \
             and a name has one definition"
    )]
    DuplicateSourceSection { addr: String, anchor: String },
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

/// The shared phase loop (PROP-035 §8): declared parse → gather → close
/// schedule, then the legacy merge/embed/qualify/absorb/link/assemble/emit tail.
/// In [`CompileMode::QualifyPerNode`] each node is qualified under its own
/// origin before emission and a second pass resolves cross-node short
/// references; in [`CompileMode::Plain`] the body is emitted as-authored and
/// the rename map is empty. One loop, parameterised by mode — never two copies
/// of the phase body (B-006 rider).
fn compile_static_inner(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: CompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let closure = crate::compiler::builtin::compile_closure(seed, source)?;
    compile_static_continuation(closure, source, mode)
}

/// Bridge from the migrated closure level into the still-legacy artifact tail.
///
/// Close owns topology and stores parsed-tree bodies in dependency-first node
/// order. The tail reads that carrier directly; the overlay caches only those
/// use-closure documents. `#source` expansion and `#embed` loading deliberately
/// fall through to their still-owning legacy cells.
fn compile_static_continuation(
    closure: ClosureIr,
    source: &impl SectionSource,
    mode: CompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let qualify = matches!(mode, CompileMode::QualifyPerNode);
    let emission_order = match closure.contributions.as_slice() {
        [ClosureContribution::Normal { emission_order, .. }] => emission_order.clone(),
        _ => unreachable!("one-seed compatibility close returns one normal contribution"),
    };
    let cached_bodies: HashMap<String, String> = closure
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => (address.without_pin(), node.body.clone()),
            DocumentAddress::StaticEntry { .. } => {
                unreachable!("one-seed compatibility close contains only spec addresses")
            }
        })
        .collect();
    let closure_source = ClosureSectionSource {
        cached_bodies: &cached_bodies,
        fallback: source,
    };
    let texts: Vec<(String, SpecAddress, String, String)> = emission_order
        .into_iter()
        .map(|node_id| {
            let node = &closure.nodes[node_id.0];
            let DocumentAddress::Spec(address) = &node.address else {
                unreachable!("one-seed compatibility close contains only spec addresses")
            };
            (
                address.without_pin(),
                address.clone(),
                node.origin.clone(),
                node.body.clone(),
            )
        })
        .collect();

    // §7.4 READ-ONCE over overlapping spans: two closure nodes of ONE doc
    // may nest (a whole-doc / `#root` node beside a section inside it — the
    // `usage#root` + `usage#re-derive` shape). Emitting both defines every
    // shared label twice, which the lane's XML conversion rightly refuses.
    // A node whose text is wholly contained in a same-doc sibling's text is
    // ABSORBED — its bytes already arrive with the ancestor. Equal texts
    // (two addresses of one section) keep the first in topo order.
    let absorbed: Vec<bool> = texts
        .iter()
        .enumerate()
        .map(|(i, (_, addr, _, text))| {
            texts
                .iter()
                .enumerate()
                .any(|(j, (_, other, _, other_text))| {
                    i != j
                        && addr.authority == other.authority
                        && addr.doc_path == other.doc_path
                        && (text.len() < other_text.len() && other_text.contains(text.as_str())
                            || text == other_text && j < i)
                })
        })
        .collect();

    let mut out = String::new();
    let mut renames: Vec<(String, RenameEntry)> = Vec::new();
    for (i, (key, addr, origin, text)) in texts.into_iter().enumerate() {
        if absorbed[i] {
            continue;
        }

        // phase 3 — fold the node's whole `#source` closure RECURSIVELY (§7.3,
        // §8 phase 3): a source that itself declares `#source` folds before it
        // merges into its parent, every node folds once, and a cycle is judged
        // by `source_fold_order`. See [`fold_source_closure`] for the recursion,
        // the legal-cycle forward-declaration rule, and the per-level gate.
        let folded = fold_source_closure(&text, &addr, &closure_source)?;
        // phase 4 — embed over the use/source-resolved body.
        let body = strip_directive_lines(&folded, &[DirectiveKind::Use, DirectiveKind::Source]);
        let expanded = expand_embeds(&body, &closure_source)?;

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
            // Multi-doc closures of ONE package (a contract `@spec`-pulling a
            // sibling doc) would collide on the standard anchors (`root`,
            // `summary`) under a plain origin slug — so a node outside the
            // package's `boot/` contract home carries its doc in the slug
            // (PROP-035 §8 phase 5, the multi-doc rider). Pure per-node:
            // f(origin, doc, label), independent of closure composition, so
            // late additions stay append-only and today's boot-contract
            // units keep their exact names.
            let slug_origin = node_slug_origin(&addr, &origin);
            let (qualified, node_renames) = qualify_contribution(&emitted, &slug_origin);
            renames.extend(node_renames.into_iter().map(|r| (origin.clone(), r)));
            qualified
        } else {
            emitted
        };

        writeln!(out, "{}", crate::markers::open(&key)).unwrap(); // phase 5
        out.push_str(&emitted);
        if !emitted.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "{}", crate::markers::close(&key)).unwrap();
    }

    if qualify {
        // Second pass — resolve the cross-node short references the per-node
        // qualify could not see (B-006 rider).
        out = resolve_cross_node_short_links(&out, &renames)?;
    }
    Ok((out, renames))
}

struct ClosureSectionSource<'a, S> {
    cached_bodies: &'a HashMap<String, String>,
    fallback: &'a S,
}

impl<S: SectionSource> SectionSource for ClosureSectionSource<'_, S> {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.cached_bodies
            .get(&address.without_pin())
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.fallback.section_text(address))
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.fallback.expand_pattern(address)
    }
}

/// The authoring origin of a closure node — `<group>/<name>` for a package, the
/// host token for the host project (PROP-035 §6). Derived from the node's topo
/// key by the same authority half `normal_seed` builds a coordinate from, so a
/// node compiled from another package's `#use` target is qualified under THAT
/// package's origin, not the entry's (B-006 rider).
/// The slug-authority a node's labels qualify under. A doc in a package's
/// contract home — `boot/` (the snippet home) or `contract/` (§4) — keeps
/// the plain origin (today's names, byte-stable); any other doc appends its
/// doc-path so two docs of one package cannot define the same qualified
/// label (`root`, `summary` are near-universal). The doc rides the origin
/// as `/`-joined-with-`.`-segments so [`origin_slug`]'s existing mapping
/// (`/`→`--`, `.`→`-`, lowercase) yields `<origin-slug>--<doc-seg-doc-seg…>`
/// with no qualify-side change.
fn node_slug_origin(addr: &SpecAddress, origin: &str) -> String {
    if addr.doc_path.starts_with("boot/")
        || addr.doc_path.starts_with("contract/")
        || addr.doc_path.is_empty()
    {
        return origin.to_string();
    }
    format!("{origin}/{}", addr.doc_path.replace('/', "."))
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

mod fold;

#[cfg(test)]
mod characterization_tests;
#[cfg(test)]
mod collision_tests;
#[cfg(test)]
mod fold_tests;
#[cfg(test)]
mod inheritance_parity_tests;
#[cfg(test)]
mod tests;
