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
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockSource(HashMap<String, String>);

    impl MockSource {
        fn new(pairs: &[(&str, &str)]) -> Self {
            MockSource(
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            )
        }
    }

    impl SectionSource for MockSource {
        fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
            self.0
                .get(&addr.without_pin())
                .cloned()
                .ok_or_else(|| "not in mock".to_string())
        }
    }

    #[test]
    fn composes_use_ordering_and_embed_expansion() {
        let src = MockSource::new(&[
            (
                "spec://vibevm/a#r",
                "# A {#r}\n#use spec://vibevm/b#r\n#embed spec://vibevm/c#r",
            ),
            ("spec://vibevm/b#r", "# B {#r}\nbee"),
            ("spec://vibevm/c#r", "cee"),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        let out = compile_static(&seed, &src).unwrap();

        // The dependency `b` is emitted before its user `a`.
        let bee = out.find("bee").unwrap();
        let a_heading = out.find("# A").unwrap();
        assert!(bee < a_heading, "dependency must precede its user:\n{out}");
        // The embed is spliced.
        assert!(out.contains("cee"));
        // No directive survives the compile.
        assert!(!out.contains("#use"), "{out}");
        assert!(!out.contains("#embed"), "{out}");
        // Node markers wrap each emission.
        assert!(out.contains("<!-- vibe:begin spec://vibevm/a#r -->"));
        assert!(out.contains("<!-- vibe:end spec://vibevm/b#r -->"));
    }

    #[test]
    fn a_lone_seed_compiles_to_itself() {
        let src = MockSource::new(&[("spec://vibevm/a#r", "# A {#r}\njust me")]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        assert!(out.contains("just me"));
        assert!(out.contains("<!-- vibe:begin spec://vibevm/a#r -->"));
    }

    #[test]
    fn a_cycle_fails_the_compile() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "#use spec://vibevm/a#r"),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        assert!(matches!(
            compile_static(&seed, &src),
            Err(CompileError::UseGraph(_))
        ));
    }

    #[test]
    fn a_clean_fact_override_compiles_to_the_source_version() {
        // Source's `##fact-a` overrides the contract's; the merged view holds one
        // `fact-a`, so the gate passes and the source text wins.
        let src = MockSource::new(&[
            (
                "spec://vibevm/c#root",
                "# API {#root}\n#source spec://vibevm/impl#root\n- ##fact-a contract version\n",
            ),
            (
                "spec://vibevm/impl#root",
                "# Impl {#root}\n- ##fact-a source version\n",
            ),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/c#root").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        assert!(out.contains("source version"), "{out}");
        assert!(!out.contains("contract version"), "{out}");
        assert!(!out.contains("#source"), "{out}");
    }

    #[test]
    fn a_cross_section_fact_collision_fails_the_gate() {
        // The contract's `##dup` (in #a) is not overridden — the matching source
        // section carries no `##dup` — and a source-only section #b re-declares
        // it, so the merged document holds `dup` twice across sections.
        let src = MockSource::new(&[
            (
                "spec://vibevm/c#root",
                "# A {#a}\n#source spec://vibevm/impl#whole\n- ##dup contract's\n",
            ),
            (
                "spec://vibevm/impl#whole",
                "# A {#a}\nplain source a\n# B {#b}\n- ##dup source's\n",
            ),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/c#root").unwrap();
        match compile_static(&seed, &src) {
            Err(CompileError::DuplicateId { dup, .. }) => {
                assert_eq!(dup.id, "dup");
                assert_eq!(dup.first_section, "a");
                assert_eq!(dup.second_section, "b");
            }
            other => panic!("expected a DuplicateId gate error, got {other:?}"),
        }
    }

    #[test]
    fn folds_source_into_a_contract_that_declares_it() {
        let src = MockSource::new(&[
            (
                "spec://org.vibevm.demo/lib/contract/api#root",
                "# API {#root}\n#source spec://org.vibevm.demo/lib/source/impl#root\ncontract-body",
            ),
            (
                "spec://org.vibevm.demo/lib/source/impl#root",
                "# Impl {#root}\nsource-body",
            ),
        ]);
        let seed = SpecAddress::parse("spec://org.vibevm.demo/lib/contract/api#root").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        assert!(out.contains("contract-body"), "{out}");
        assert!(out.contains("source-body"), "{out}");
        // The #source directive is resolved by the fold, not left behind.
        assert!(!out.contains("#source"), "{out}");
    }

    #[test]
    fn at_bang_alias_is_rewritten_to_the_full_address() {
        // B-011 §7.4: in the compiled lane every `@!<Alias>` becomes the full
        // `@spec://<target>` its `#use … as` binding denotes, and the `as` clause
        // leaves with the stripped `#use` line.
        let src = MockSource::new(&[
            (
                "spec://vibevm/a#r",
                "# A {#r}\n#use spec://vibevm/b#r as dep\nSees @!dep here.\n",
            ),
            ("spec://vibevm/b#r", "# B {#r}\nb body\n"),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        // The alias target's full address is spliced in for `@!dep`.
        assert!(out.contains("@spec://vibevm/b#r"), "{out}");
        assert!(!out.contains("@!dep"), "{out}");
        // The declaration line (and its `as dep` clause) is gone with `#use`.
        assert!(!out.contains("#use "), "{out}");
        assert!(!out.contains("as dep"), "{out}");
        // The aliased dependency is still emitted before its user (topo order).
        assert!(out.contains("b body"), "{out}");
    }

    #[test]
    fn at_bang_in_a_fence_is_not_rewritten() {
        // The fence mask governs the rewrite as it governs the scan: an `@!dep`
        // inside a fenced block is prose-as-data, not a use, so it stays put.
        let src = MockSource::new(&[
            (
                "spec://vibevm/a#r",
                "# A {#r}\n#use spec://vibevm/b#r as dep\n```\n@!dep\n```\n",
            ),
            ("spec://vibevm/b#r", "# B {#r}\nb\n"),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        assert!(out.contains("@!dep"), "fenced @!dep must stay: {out}");
    }

    // ---- B-006 rider: per-node qualification (PROP-035 §8 phase 5) -----------
    //
    // `compile_static_qualified` qualifies each node under ITS OWN origin
    // (derived from the topo key), then a second pass resolves the cross-node
    // short links the per-node pass could not see. These six tests pin the
    // contract (Q1–Q6 of the E4-W2-NODE-QUALIFY packet). The fixture addresses
    // use distinct package coordinates (`org.a/a`, `org.b/b`, `org.c/c`) so the
    // nodes carry DISTINCT origins — the case per-node qualify exists for.

    #[test]
    fn q1_two_origins_each_qualify_their_own_label() {
        // Two documents of different origins, linked by `#use`, each defining
        // `##THE-RULE`: the qualified compile emits TWO different qualified
        // names, each under its own origin, and the rename map carries both
        // origins. No ambiguity — coexisting definitions are qualified apart;
        // only an unresolved *cross-node reference* is ambiguous (q4).
        let src = MockSource::new(&[
            (
                "spec://org.a/a/doc#r",
                "# A {#root}\n##THE-RULE a's rule\n#use spec://org.b/b/doc#r\n",
            ),
            ("spec://org.b/b/doc#r", "# B {#root}\n##THE-RULE b's rule\n"),
        ]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        let (out, renames) = compile_static_qualified(&seed, &src).unwrap();

        // Each node's THE-RULE is qualified under its own origin — never the
        // entry's.
        assert!(out.contains("##org-a--a--THE-RULE"), "{out}");
        assert!(out.contains("##org-b--b--THE-RULE"), "{out}");
        // The rename map carries both origins for THE-RULE.
        let rule_origins: Vec<&str> = renames
            .iter()
            .filter(|(_, r)| r.original == "THE-RULE")
            .map(|(o, _)| o.as_str())
            .collect();
        assert!(rule_origins.contains(&"org.a/a"), "{renames:?}");
        assert!(rule_origins.contains(&"org.b/b"), "{renames:?}");
    }

    #[test]
    fn q2_within_node_self_reference_is_qualified_by_its_own_origin() {
        // A node referencing its own label is resolved within the node by the
        // per-node qualify — the same as the old whole-body behaviour — so the
        // second pass never touches it.
        let src = MockSource::new(&[(
            "spec://org.a/a/doc#r",
            "# A {#root}\nSee (#root) and (#OTHER).\n##OTHER a fact\n",
        )]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        let (out, _) = compile_static_qualified(&seed, &src).unwrap();
        assert!(out.contains("{#org-a--a--root}"), "{out}");
        assert!(out.contains("(#org-a--a--root)"), "{out}");
        assert!(out.contains("(#org-a--a--OTHER)"), "{out}");
    }

    #[test]
    fn q3_cross_node_short_link_resolves_to_the_unique_definer() {
        // Node A (origin1) references `(#THE-RULE)`, which ONLY node B (origin2)
        // defines: the second pass rewrites it to B's qualified heir.
        let src = MockSource::new(&[
            (
                "spec://org.a/a/doc#r",
                "# A {#root}\nSee (#THE-RULE) live.\n#use spec://org.b/b/doc#r\n",
            ),
            ("spec://org.b/b/doc#r", "# B {#root}\n##THE-RULE b's\n"),
        ]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        let (out, _) = compile_static_qualified(&seed, &src).unwrap();
        assert!(out.contains("(#org-b--b--THE-RULE)"), "{out}");
        assert!(
            !out.contains("(#THE-RULE)"),
            "the bare cross-node link must be gone: {out}"
        );
    }

    #[test]
    fn q4_ambiguous_cross_node_short_link_fails_with_candidates() {
        // A short link to a label TWO nodes define is a build error citing both
        // candidate heirs (B-011: fail with candidates, never a silent pick).
        let src = MockSource::new(&[
            (
                "spec://org.a/a/doc#r",
                "# A {#root}\nSee (#SHARED).\n#use spec://org.b/b/doc#r\n#use spec://org.c/c/doc#r\n",
            ),
            ("spec://org.b/b/doc#r", "# B {#root}\n##SHARED b's\n"),
            ("spec://org.c/c/doc#r", "# C {#root}\n##SHARED c's\n"),
        ]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        match compile_static_qualified(&seed, &src) {
            Err(CompileError::AmbiguousShortLink { label, candidates }) => {
                assert_eq!(label, "SHARED");
                let joined = candidates.join(" | ");
                assert!(joined.contains("org-b--b--SHARED"), "{joined}");
                assert!(joined.contains("org-c--c--SHARED"), "{joined}");
            }
            other => panic!("expected AmbiguousShortLink, got {other:?}"),
        }
    }

    #[test]
    fn q5_fenced_blocks_are_untouched_by_both_passes() {
        // Fenced code is masked from the per-node qualify AND the second pass:
        // a `##FENCED` inside a fence is never treated as a definition, and a
        // `(#x)` inside a fence is never rewritten — even when `x` is defined
        // unfenced elsewhere. The same `(#x)` outside the fence IS rewritten.
        let src = MockSource::new(&[
            (
                "spec://org.a/a/doc#r",
                "# A {#root}\n#use spec://org.b/b/doc#r\nSee (#ONLY-IN-B) live.\n\
                 ```\n##FENCED and (#ONLY-IN-B) and (#root)\n```\n",
            ),
            ("spec://org.b/b/doc#r", "# B {#root}\n##ONLY-IN-B b's\n"),
        ]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        let (out, renames) = compile_static_qualified(&seed, &src).unwrap();

        // The fenced line is byte-identical — `##FENCED` is not a definition
        // (so it is absent from the rename map) and the fenced short links stay
        // bare.
        assert!(
            out.contains("##FENCED and (#ONLY-IN-B) and (#root)"),
            "{out}"
        );
        assert!(
            !renames.iter().any(|(_, r)| r.original == "FENCED"),
            "fenced ##FENCED must not become a definition: {renames:?}"
        );
        // The same cross-node link OUTSIDE the fence was resolved.
        assert!(out.contains("(#org-b--b--ONLY-IN-B) live"), "{out}");
    }

    #[test]
    fn q6_plain_compile_static_emits_labels_unqualified() {
        // Regression guard for the reference semantics: `compile_static` (the
        // unqualified path) emits labels exactly as authored — no origin prefix,
        // no rename map — over a multi-document closure.
        let src = MockSource::new(&[
            (
                "spec://org.a/a/doc#r",
                "# A {#root}\n##FACT a\n#use spec://org.b/b/doc#r\n",
            ),
            ("spec://org.b/b/doc#r", "# B {#root}\n##FACT b\n"),
        ]);
        let seed = SpecAddress::parse("spec://org.a/a/doc#r").unwrap();
        let out = compile_static(&seed, &src).unwrap();
        // Bare labels survive; no qualified form appears.
        assert!(out.contains("{#root}"), "{out}");
        assert!(out.contains("##FACT a"), "{out}");
        assert!(!out.contains("--root"), "{out}");
        assert!(!out.contains("--FACT"), "{out}");
    }
}
