//! The documentation page scanner — `rule` citations as `documents`
//! edges (PROP-057 `##PIPE-EDGES-HOST-SIDE`).
//!
//! The traceability engine's dialect has no `<rule>` element and must not
//! grow one: the dialect is closed, shared by three language stacks, and
//! vendored into six copies that `sync-engines` rewrites (R-21). So the
//! reading happens on the HOST side and reaches the engine through the
//! seam built for exactly this — [`CodeScanner`], the trait the
//! TypeScript stack already injects its own scanner through.
//!
//! What this scanner contributes is one [`CodeItem`] per page and one
//! [`Edge`] per `rule`. The item's `crate_name` is the sentinel `<doc>`,
//! after the precedent the JTD scanner set with `<schema>`: a page is not
//! a crate, and a plausible-looking crate name would be a lie the wire
//! schema has no way to catch. No schema changes; no engine edit; no
//! `sync-engines` run.
//!
//! **Every edge is unpinned, and that is the point.** Suspect detection
//! has exactly one entrance — a pin on the edge — so an edge minted here
//! can never go suspect, whatever revisions the cited specification
//! moves through. A citation is live: the page shows the fact's current
//! text at every render, and «the spec moved ahead» is not a state this
//! project keeps (`##OBS-RULE-EDGE-UNPINNED`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EDGES-HOST-SIDE");

use std::path::Path;

use specmap_core::config::Config;
use specmap_core::generated::specmap::{CodeItem, Edge, EdgeProvenance, EdgeVerb, Warning};
use specmap_core::scanner::CodeScanner;

/// A page is not a crate. The sentinel says so plainly, exactly as the
/// JTD scanner's `<schema>` does for a schema file.
pub const DOC_CRATE: &str = "<doc>";

/// The item kind a page takes in the map.
const DOC_ITEM_KIND: &str = "doc-page";

/// Reads a documentation package's pages and mints a `documents` edge for
/// every `rule` citation on them.
///
/// The scanner is constructed with the package's own coordinate because
/// classification needs it: a manual addressing its OWN pages is not
/// citing a specification, and minting a `documents` edge into the
/// manual's own tree would claim a relation that does not exist.
pub struct DocScanner {
    coordinate: String,
}

impl DocScanner {
    /// A scanner for a documentation package published under
    /// `<group>/<name>`.
    pub fn new(coordinate: impl Into<String>) -> DocScanner {
        DocScanner {
            coordinate: coordinate.into(),
        }
    }
}

impl CodeScanner for DocScanner {
    fn id(&self) -> &'static str {
        "vibe-doc-pages"
    }

    fn scan(&self, root: &Path, _cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        // A package that cannot be read yields nothing and says so once.
        // Refusing the whole map because one page is malformed would take
        // the traceability of every OTHER page down with it, and the
        // page's own defect is already reported by `vibe doc check`.
        let found = match vibe_doc::citations::edges(root, &self.coordinate) {
            Ok(found) => found,
            Err(e) => {
                return (
                    Vec::new(),
                    Vec::new(),
                    vec![Warning {
                        code: "doc-pages-unreadable".to_string(),
                        message: format!("documentation pages were not scanned: {e}"),
                        file: vibe_doc::pages::SPEC_ROOT.to_string(),
                        line: 0,
                    }],
                );
            }
        };

        let mut items: Vec<CodeItem> = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();
        for edge in found {
            let symbol = page_symbol(&edge.page);
            if !items.iter().any(|i| i.symbol == symbol) {
                items.push(CodeItem {
                    symbol: symbol.clone(),
                    itemKind: DOC_ITEM_KIND.to_string(),
                    crateName: DOC_CRATE.to_string(),
                    file: edge.file.clone(),
                    line: 1,
                    endLine: None,
                    // A page has no token stream, so it has no
                    // fingerprint; minting a hash of something else
                    // would be inventing a different semantic.
                    fingerprint: None,
                });
            }
            edges.push(Edge {
                fromSymbol: symbol,
                verb: EdgeVerb::Documents,
                uri: edge.uri,
                file: edge.file,
                line: edge.line,
                provenance: EdgeProvenance::Authored,
                // Unpinned, always. This is the single line that keeps a
                // live citation out of suspect detection.
                pinnedR: None,
                // `reason` is mandatory for `deviates` and meaningless
                // here.
                reason: None,
            });
        }
        (items, edges, Vec::new())
    }
}

/// The symbol a page takes in the map: its address inside the package,
/// without the extension. `model/boot-lane.xml` reads `model::boot-lane`,
/// so a map row looks like the module path it stands in for.
fn page_symbol(page_rel: &str) -> String {
    page_rel
        .strip_suffix(".xml")
        .unwrap_or(page_rel)
        .replace('/', "::")
}

#[cfg(test)]
mod tests;
