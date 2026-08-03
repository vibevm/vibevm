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

use crate::address::{SpecAddress, SpecAddressError};
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::embed::{EmbedError, SectionSource, expand_embeds};
use crate::gate::{DuplicateId, first_duplicate};
use crate::merge::fold_source;
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
}

/// Compile the closure reachable from `seed` into a single static document.
pub fn compile_static(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<String, CompileError> {
    let order = topo_order_from(seed, source)?; // phase 2

    let mut out = String::new();
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

        writeln!(out, "{}", crate::markers::open(key)).unwrap(); // phase 5
        out.push_str(&emitted);
        if !emitted.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "{}", crate::markers::close(key)).unwrap();
    }
    Ok(out)
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
}
