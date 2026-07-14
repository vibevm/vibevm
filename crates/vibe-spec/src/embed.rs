//! `#embed` expansion (PROP-035 §7.1) — the macro splice.
//!
//! `#embed <spec://…>` is replaced, textually, by the section (or whole
//! document) the address names. Expansion is **recursive to a fixed point**: an
//! embedded section may itself contain `#embed`, and those are expanded too, so
//! no `#embed` survives the output. A cycle guard (PROP-035 §9) keys on the
//! address currently being expanded and rejects a repeat with the offending
//! path (`a → b → a`), the same diagnostic C's include guards give.
//!
//! The section text an address resolves to is supplied by a [`SectionSource`],
//! so the expander is testable without a filesystem. [`FsSectionSource`] is the
//! real one — it composes the whole crate: [`FileResolver`] to find the file,
//! then [`DocTree`] to resolve the anchor to a node and take its text.
//!
//! Spliced text is wrapped in open/close markers (PROP-035 §11) so the result
//! stays reversible.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;

use crate::address::SpecAddress;
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::resolver::FileResolver;

/// Supplies the text a `spec://` address resolves to. Abstract so `#embed`
/// expansion can be driven from a filesystem, an in-memory map, or a test mock.
pub trait SectionSource {
    /// The text of the section (or whole document) `addr` names, or a reason it
    /// could not be produced.
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String>;
}

/// Why `#embed` expansion failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbedError {
    #[error("embed cycle: {}", .0.join(" -> "))]
    Cycle(Vec<String>),
    #[error("cannot resolve embed {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
}

/// Expand every `#embed` in `text` to a fixed point.
pub fn expand_embeds(text: &str, source: &impl SectionSource) -> Result<String, EmbedError> {
    let mut stack = Vec::new();
    expand_rec(text, source, &mut stack)
}

fn expand_rec(
    text: &str,
    source: &impl SectionSource,
    stack: &mut Vec<String>,
) -> Result<String, EmbedError> {
    let directives = Directives::parse(text);
    let embeds: HashMap<usize, &SpecAddress> = directives
        .directives
        .iter()
        .filter(|d| d.kind == DirectiveKind::Embed)
        .map(|d| (d.line, &d.address))
        .collect();

    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let Some(addr) = embeds.get(&i) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };

        let key = addr.without_pin();
        if stack.contains(&key) {
            let mut path = stack.clone();
            path.push(key);
            return Err(EmbedError::Cycle(path));
        }

        let section = source
            .section_text(addr)
            .map_err(|reason| EmbedError::Unresolved {
                addr: addr.to_string(),
                reason,
            })?;

        stack.push(key.clone());
        let expanded = expand_rec(&section, source, stack)?;
        stack.pop();

        writeln!(out, "<!-- embed: {key} -->").unwrap();
        out.push_str(&expanded);
        if !expanded.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "<!-- /embed: {key} -->").unwrap();
    }
    Ok(out)
}

/// The real [`SectionSource`]: resolve the address to a file, parse it, and take
/// the addressed node's text — the crate's layers composed end to end.
pub struct FsSectionSource {
    resolver: FileResolver,
}

impl FsSectionSource {
    pub fn new(resolver: FileResolver) -> Self {
        Self { resolver }
    }
}

impl SectionSource for FsSectionSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let file = self
            .resolver
            .resolve_file(addr)
            .map_err(|e| e.to_string())?;
        let src = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let tree = DocTree::parse(&src);
        let node = tree
            .resolve_path(&addr.anchor)
            .ok_or_else(|| format!("anchor not found in {}", file.display()))?;
        Ok(tree.text(node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn expands_a_simple_embed() {
        let src = MockSource::new(&[("spec://vibevm/a#x", "EMBEDDED BODY")]);
        let out = expand_embeds("before\n#embed spec://vibevm/a#x\nafter\n", &src).unwrap();
        assert!(out.contains("EMBEDDED BODY"));
        assert!(out.contains("before"));
        assert!(out.contains("after"));
        // No directive survives.
        assert!(!out.contains("#embed"));
    }

    #[test]
    fn expands_recursively_to_a_fixed_point() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#x", "level1\n#embed spec://vibevm/b#y"),
            ("spec://vibevm/b#y", "level2"),
        ]);
        let out = expand_embeds("#embed spec://vibevm/a#x\n", &src).unwrap();
        assert!(out.contains("level1"));
        assert!(out.contains("level2"));
        assert!(!out.contains("#embed"));
    }

    #[test]
    fn detects_a_cycle_with_its_path() {
        let src = MockSource::new(&[
            ("spec://vibevm/a#x", "#embed spec://vibevm/b#y"),
            ("spec://vibevm/b#y", "#embed spec://vibevm/a#x"),
        ]);
        let err = expand_embeds("#embed spec://vibevm/a#x\n", &src).unwrap_err();
        match err {
            EmbedError::Cycle(path) => {
                assert_eq!(path.first().unwrap(), "spec://vibevm/a#x");
                assert_eq!(path.last().unwrap(), "spec://vibevm/a#x");
                assert!(path.contains(&"spec://vibevm/b#y".to_string()));
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn reports_an_unresolved_embed() {
        let src = MockSource::new(&[]);
        let err = expand_embeds("#embed spec://vibevm/missing#x\n", &src).unwrap_err();
        assert!(matches!(err, EmbedError::Unresolved { .. }));
    }

    #[test]
    fn markers_wrap_the_splice() {
        let src = MockSource::new(&[("spec://vibevm/a#x", "BODY")]);
        let out = expand_embeds("#embed spec://vibevm/a#x\n", &src).unwrap();
        assert!(out.contains("<!-- embed: spec://vibevm/a#x -->"));
        assert!(out.contains("<!-- /embed: spec://vibevm/a#x -->"));
    }
}
