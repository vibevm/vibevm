//! The inline compilation pipeline (PROP-035 §8) — the primitives, composed.
//!
//! `compile_inline` runs the phases in the fixed order the spec pins:
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
//! otherwise be a dangling directive in the compiled `INLINE.md`. `@spec`
//! in-place references are left in prose (their target is likewise already
//! above). No `#embed` survives (§7.1).
//!
//! This is the algorithmic, LLM-free inline compiler (§2) — the reference
//! semantics the structural loader is later checked against.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::address::{SpecAddress, SpecAddressError};
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::embed::{EmbedError, SectionSource, expand_embeds};
use crate::merge::fold_source;
use crate::use_graph::{UseGraphError, topo_order_from};

/// Why inline compilation failed.
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
}

/// Compile the closure reachable from `seed` into a single inline document.
pub fn compile_inline(
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

        // phase 3 — fold source into a contract that declares #source.
        let folded = match first_source_directive(&text) {
            Some(source_addr) => {
                let contract_tree = DocTree::parse(&text);
                let src_text = source.section_text(&source_addr).map_err(|reason| {
                    CompileError::Unresolved {
                        addr: source_addr.to_string(),
                        reason,
                    }
                })?;
                fold_source(&contract_tree, &DocTree::parse(&src_text))
            }
            None => text,
        };
        // phase 4 — embed over the use/source-resolved body.
        let body = strip_directive_lines(&folded, &[DirectiveKind::Use, DirectiveKind::Source]);
        let expanded = expand_embeds(&body, source)?;

        writeln!(out, "<!-- begin: {key} -->").unwrap(); // phase 5
        out.push_str(&expanded);
        if !expanded.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "<!-- end: {key} -->").unwrap();
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
        let out = compile_inline(&seed, &src).unwrap();

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
        assert!(out.contains("<!-- begin: spec://vibevm/a#r -->"));
        assert!(out.contains("<!-- end: spec://vibevm/b#r -->"));
    }

    #[test]
    fn a_lone_seed_compiles_to_itself() {
        let src = MockSource::new(&[("spec://vibevm/a#r", "# A {#r}\njust me")]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        let out = compile_inline(&seed, &src).unwrap();
        assert!(out.contains("just me"));
        assert!(out.contains("<!-- begin: spec://vibevm/a#r -->"));
    }

    #[test]
    fn a_cycle_fails_the_compile() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#r", "#use spec://vibevm/b#r"),
            ("spec://vibevm/b#r", "#use spec://vibevm/a#r"),
        ]);
        let seed = SpecAddress::parse("spec://vibevm/a#r").unwrap();
        assert!(matches!(
            compile_inline(&seed, &src),
            Err(CompileError::UseGraph(_))
        ));
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
        let out = compile_inline(&seed, &src).unwrap();
        assert!(out.contains("contract-body"), "{out}");
        assert!(out.contains("source-body"), "{out}");
        // The #source directive is resolved by the fold, not left behind.
        assert!(!out.contains("#source"), "{out}");
    }
}
