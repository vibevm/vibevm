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
use specmap_core::scanner::{CodeScanner, CompositeScanner, DefaultScanner};

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

/// The host's documentation pages — every in-tree package of kind `doc`,
/// scanned where it lies.
///
/// A host's own `spec_roots` do not reach a documentation package: the
/// pages sit inside a package slot under the project's package root, and
/// the engine's dialect could not read them there anyway
/// (`##PIPE-EDGES-HOST-SIDE`). So the policy for WHERE pages live is a
/// host file — this one — and it is a walk rather than a list: a list of
/// documentation packages is a list that goes stale the day somebody
/// opens a translation, and the tree already says which packages are
/// documentation.
///
/// What it buys is that `vibe explain`, `vibe query` and `vibe select`
/// answer «who documents this rule» — over a map built fresh, which is
/// the posture those three already take.
pub struct HostDocScanner {
    /// `(package directory relative to the root, its coordinate)`.
    packages: Vec<(String, String)>,
}

impl HostDocScanner {
    /// Find the in-tree documentation packages of the project at `root`.
    pub fn of(root: &Path) -> HostDocScanner {
        HostDocScanner {
            packages: doc_packages(root),
        }
    }

    /// Whether this project holds any documentation at all. A tree with
    /// none needs no composition, and saying so lets a caller keep the
    /// plain build.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

impl CodeScanner for HostDocScanner {
    fn id(&self) -> &'static str {
        "vibe-doc-pages-host"
    }

    fn scan(&self, root: &Path, cfg: &Config) -> (Vec<CodeItem>, Vec<Edge>, Vec<Warning>) {
        let mut items = Vec::new();
        let mut edges = Vec::new();
        let mut warnings = Vec::new();
        for (rel, coordinate) in &self.packages {
            let (mut found, mut minted, mut said) =
                DocScanner::new(coordinate.clone()).scan(&root.join(rel), cfg);
            // The package scanned itself at its own root, so every path it
            // produced is package-relative; the host map addresses files
            // from the project root.
            for item in &mut found {
                item.file = format!("{rel}/{}", item.file);
            }
            for edge in &mut minted {
                edge.file = format!("{rel}/{}", edge.file);
            }
            for warning in &mut said {
                warning.file = format!("{rel}/{}", warning.file);
            }
            items.append(&mut found);
            edges.append(&mut minted);
            warnings.append(&mut said);
        }
        (items, edges, warnings)
    }
}

/// Build the project's traceability map in memory, with its documentation
/// in it.
///
/// This is what the three read-only queries build from. A project with no
/// documentation gets exactly the map it got before — the composition is
/// skipped rather than made empty, so nothing about the plain case moves.
pub fn host_map(root: &Path, cfg: &Config) -> specmap_core::generated::specmap::Specmap {
    let pages = HostDocScanner::of(root);
    if pages.is_empty() {
        return specmap_core::index::build(root, cfg);
    }
    let default = DefaultScanner::new();
    let composite = CompositeScanner::new(vec![&default as &dyn CodeScanner, &pages]);
    specmap_core::index::build_with_scanner(root, cfg, &composite)
}

/// Walk the project's package root for slots declaring `kind = "doc"`.
///
/// Read as TOML data and not through the typed manifest: the question is
/// «does this slot call itself documentation», and a strict parse would
/// drop a package over a field this walk never looks at — leaving its
/// pages silently out of the map, which is the one failure mode a
/// coverage answer must not have.
fn doc_packages(root: &Path) -> Vec<(String, String)> {
    let packages_root = root.join(vibe_core::layout::current_packages_root());
    let mut found = Vec::new();
    let Ok(groups) = std::fs::read_dir(&packages_root) else {
        return found;
    };
    for group in groups.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        let Ok(names) = std::fs::read_dir(&group) else {
            continue;
        };
        for name in names.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let Ok(versions) = std::fs::read_dir(&name) else {
                continue;
            };
            for slot in versions.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                if let Some(coordinate) = doc_coordinate(&slot)
                    && let Ok(rel) = slot.strip_prefix(root)
                {
                    found.push((forward_slashed(rel), coordinate));
                }
            }
        }
    }
    found.sort();
    found
}

/// `<group>/<name>` when the slot's manifest declares kind `doc`.
fn doc_coordinate(slot: &Path) -> Option<String> {
    let text = std::fs::read_to_string(slot.join("vibe.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let package = value.get("package")?;
    if package.get("kind")?.as_str()? != "doc" {
        return None;
    }
    let group = package.get("group")?.as_str()?;
    let name = package.get("name")?.as_str()?;
    Some(format!("{group}/{name}"))
}

fn forward_slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
