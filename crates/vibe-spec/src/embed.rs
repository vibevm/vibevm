use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::address::SpecAddress;
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::resolver::FileResolver;

pub(crate) mod occurrences;
use occurrences::{EmbedOccurrence, EmbedRegion, EmbedRegionKind, EmbedTape, ExpandedBody};

pub trait SectionSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String>;

    /// Initial roots may carry separately witnessed internal-artifact
    /// provenance. References never gain that role by sharing their address.
    fn root_source_metadata(
        &self,
        addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        self.source_metadata(addr, active_formats)
    }
    fn read_resolved_root_source(
        &self,
        addr: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.read_resolved_source(addr, metadata)
    }
    /// Checked on every reached reference, including before cached root reuse.
    fn admit_reference(&self, _addr: &SpecAddress) -> Result<(), String> {
        Ok(())
    }

    fn source_metadata(
        &self,
        addr: &SpecAddress,
        _active_formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        ResolvedSourceMetadata::new(
            PathBuf::from(&addr.doc_path),
            "markdown".to_owned(),
            logical_stem(addr),
        )
    }

    fn read_resolved_source(
        &self,
        addr: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        if metadata.format() != "markdown" {
            return Err("custom source metadata requires an explicit admitted reader".to_owned());
        }
        Ok(ResolvedSource::markdown(
            self.section_text(addr)?,
            metadata.physical_stem,
        ))
    }

    fn resolved_source(
        &self,
        addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<ResolvedSource, String> {
        let metadata = self.source_metadata(addr, active_formats)?;
        self.read_resolved_source(addr, metadata)
    }

    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        Ok(vec![addr.clone()])
    }
}

/// Text-free physical source identity resolved before content admission.
/// This is a locator, not a cross-call root witness: a later read revalidates
/// the locator and admits the caller's current root alias for that read epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourceMetadata {
    origin: PathBuf,
    format: crate::compiler::ir::SourceFormatId,
    physical_stem: String,
    // Locator context only: revalidation must use the same active suffix set.
    active_formats: Vec<String>,
}

impl ResolvedSourceMetadata {
    pub fn new(
        origin: impl Into<PathBuf>,
        format: String,
        physical_stem: String,
    ) -> Result<Self, String> {
        if format != "markdown"
            && (physical_stem.is_empty()
                || physical_stem.len() > 128
                || physical_stem.chars().any(char::is_control)
                || physical_stem.contains(['/', '\\']))
        {
            return Err("physical source stem is empty, unsafe, or exceeds 128 bytes".to_owned());
        }
        Ok(Self {
            origin: origin.into(),
            format: crate::compiler::ir::SourceFormatId::new(format)
                .map_err(|error| error.to_string())?,
            physical_stem,
            active_formats: Vec::new(),
        })
    }

    pub fn origin(&self) -> &Path {
        &self.origin
    }

    pub fn format(&self) -> &str {
        self.format.as_str()
    }

    pub fn physical_stem(&self) -> &str {
        &self.physical_stem
    }
}

#[derive(Debug)]
pub struct ResolvedSource {
    text: String,
    format: crate::compiler::ir::SourceFormatId,
    physical_stem: String,
}

impl ResolvedSource {
    pub(crate) fn markdown(text: String, physical_stem: String) -> Self {
        Self {
            text,
            format: crate::compiler::ir::SourceFormatId::canonical_markdown(),
            physical_stem,
        }
    }

    pub(crate) fn custom(
        text: String,
        format: String,
        physical_stem: String,
    ) -> Result<Self, String> {
        Ok(Self {
            text,
            format: crate::compiler::ir::SourceFormatId::new(format)
                .map_err(|error| error.to_string())?,
            physical_stem,
        })
    }

    pub fn format(&self) -> &str {
        self.format.as_str()
    }

    pub fn physical_stem(&self) -> &str {
        &self.physical_stem
    }

    pub(crate) fn into_parts(self) -> (String, crate::compiler::ir::SourceFormatId, String) {
        (self.text, self.format, self.physical_stem)
    }
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
    let mut resolve = |address: &SpecAddress| source.section_text(address);
    let mut edge = |_from: &str, _ordinal: usize, _to: &SpecAddress| {};
    expand_with(text, "", &mut resolve, &mut edge)
}

pub(crate) fn expand_with(
    text: &str,
    root_context: &str,
    resolve: &mut impl FnMut(&SpecAddress) -> Result<String, String>,
    edge: &mut impl FnMut(&str, usize, &SpecAddress),
) -> Result<String, EmbedError> {
    expand_annotated(text, root_context, resolve, edge).map(|body| body.legacy_text())
}

pub(crate) fn expand_annotated(
    text: &str,
    root_context: &str,
    resolve: &mut impl FnMut(&SpecAddress) -> Result<String, String>,
    edge: &mut impl FnMut(&str, usize, &SpecAddress),
) -> Result<ExpandedBody, EmbedError> {
    let mut stack = Vec::new();
    expand_rec(text, root_context, resolve, edge, &mut stack, &[])
}

fn expand_rec(
    text: &str,
    context: &str,
    resolve: &mut impl FnMut(&SpecAddress) -> Result<String, String>,
    edge: &mut impl FnMut(&str, usize, &SpecAddress),
    stack: &mut Vec<String>,
    path: &[usize],
) -> Result<ExpandedBody, EmbedError> {
    let directives = Directives::parse(text);
    let embeds: HashMap<usize, (usize, &SpecAddress)> = directives
        .directives
        .iter()
        .filter(|d| d.kind == DirectiveKind::Embed)
        .enumerate()
        .map(|(ordinal, d)| (d.line, (ordinal, &d.address)))
        .collect();

    let mut out = String::new();
    let mut occurrences = Vec::new();
    let source_tree = DocTree::parse(text);
    let mut line_map = vec![0];
    let mut own_rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let Some((ordinal, addr)) = embeds.get(&i) else {
            out.push_str(line);
            out.push('\n');
            line_map.push(out.lines().count());
            continue;
        };

        let key = addr.without_pin();
        if stack.contains(&key) {
            let mut path = stack.clone();
            path.push(key);
            return Err(EmbedError::Cycle(path));
        }

        let section = resolve(addr).map_err(|reason| EmbedError::Unresolved {
            addr: addr.to_string(),
            reason,
        })?;

        edge(context, *ordinal, addr);
        stack.push(key.clone());
        let mut child_path = path.to_vec();
        child_path.push(*ordinal);
        let expanded = expand_rec(&section, &key, resolve, edge, stack, &child_path)?;
        stack.pop();
        let start = out.lines().count();
        let end = start + expanded.text.lines().count();
        let mut region = EmbedRegion {
            kind: EmbedRegionKind::Preamble,
            start: 0,
            end: text.lines().count(),
        };
        let mut owner_nodes = source_tree.children(source_tree.root()).to_vec();
        while let Some(id) = owner_nodes.pop() {
            owner_nodes.extend_from_slice(source_tree.children(id));
            let node = source_tree.node(id);
            if node.span.contains(&i) && node.span.len() <= region.end - region.start {
                region = EmbedRegion {
                    kind: node
                        .id
                        .clone()
                        .map_or(EmbedRegionKind::Unanchored, EmbedRegionKind::Unit),
                    start: node.span.start,
                    end: node.span.end,
                };
            } else if matches!(region.kind, EmbedRegionKind::Preamble) && node.span.start > i {
                region.end = region.end.min(node.span.start);
            }
        }
        own_rows.push(occurrences.len());
        occurrences.push(EmbedOccurrence {
            path: child_path,
            target: (*addr).clone(),
            source: if context.is_empty() {
                "<compatibility-input>".into()
            } else {
                context.into()
            },
            directive_line: i,
            start,
            end,
            intrinsic_empty: expanded.text.is_empty(),
            suppressed: false,
            owner: region,
        });
        occurrences.extend(expanded.tape.occurrences.into_iter().map(|mut occurrence| {
            occurrence.start += start;
            occurrence.end += start;
            occurrence.owner.start += start;
            occurrence.owner.end += start;
            occurrence
        }));
        out.push_str(&expanded.text);
        line_map.push(out.lines().count());
    }
    for index in own_rows {
        let row = &mut occurrences[index];
        row.owner.start = line_map[row.owner.start];
        row.owner.end = line_map[row.owner.end];
    }
    Ok(ExpandedBody {
        tape: EmbedTape::new(&out, occurrences),
        text: out,
    })
}

pub struct FsSectionSource {
    resolver: FileResolver,
    overlay: BTreeMap<PathBuf, Arc<[u8]>>,
}

impl FsSectionSource {
    pub fn new(resolver: FileResolver) -> Self {
        Self {
            resolver,
            overlay: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_overlay(resolver: FileResolver, overlay: BTreeMap<PathBuf, Arc<[u8]>>) -> Self {
        Self { resolver, overlay }
    }
}

impl SectionSource for FsSectionSource {
    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.resolver
            .expand_pattern(addr)
            .map_err(|e| e.to_string())
    }

    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let location = self
            .resolver
            .locate_source(addr, &[])
            .map_err(|error| error.to_string())?;
        self.read_location(addr, location).map(|source| source.text)
    }

    fn source_metadata(
        &self,
        addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        let resolved = self
            .resolver
            .resolve_source_file(addr, active_formats)
            .map_err(|error| error.to_string())?;
        let mut metadata =
            ResolvedSourceMetadata::new(resolved.path, resolved.format, resolved.physical_stem)?;
        metadata.active_formats = active_formats.to_vec();
        Ok(metadata)
    }

    fn read_resolved_source(
        &self,
        addr: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.read_resolved(addr, metadata)
    }
}

fn logical_stem(addr: &SpecAddress) -> String {
    addr.doc_path
        .rsplit('/')
        .next()
        .unwrap_or(&addr.doc_path)
        .to_owned()
}

fn project_bytes(file: &Path, bytes: &[u8], origin: &str) -> Result<String, String> {
    let raw = std::str::from_utf8(bytes).map_err(|error| {
        format!(
            "{origin} spec source `{}` is not UTF-8: {error}",
            file.display()
        )
    })?;
    vibe_specdoc::project_spec_text(file, raw)
        .map(|(src, _kind)| src)
        .map_err(|error| error.to_string())
}

fn resolve_section(src: &str, file: &Path, addr: &SpecAddress) -> Result<String, String> {
    let tree = DocTree::parse(src);
    let selected = tree.resolve_unique_path(&addr.anchor).map_err(|error| {
        format!(
            "ambiguous requested anchor path `{}` in {}: {error}",
            addr.anchor.join("."),
            file.display()
        )
    })?;
    let node = match selected {
        Some(node) => node,
        None => {
            // B-011 §6.1 layer 3: a missed short anchor answers with its
            // qualified heirs, never emptiness. The flat segment being
            // resolved is the lookup's short name; the tree's
            // `<origin-slug>--<short>` tails are the rename's heirs.
            let candidates = addr
                .anchor
                .first()
                .map(|short| tree.qualified_candidates(short.as_str()))
                .filter(|c| !c.is_empty())
                .map(|c| {
                    format!(
                        " (qualified candidates for `{}`: {})",
                        addr.anchor.first().map(String::as_str).unwrap_or(""),
                        c.join(", ")
                    )
                })
                .unwrap_or_default();
            return Err(format!(
                "anchor not found in {}{candidates}",
                file.display()
            ));
        }
    };
    Ok(tree.text(node))
}

#[cfg(test)]
#[path = "embed/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "embed/overlay_tests.rs"]
mod overlay_tests;

#[cfg(test)]
mod source_reads;

mod read;
pub use read::resolve_captured_declared_source;
