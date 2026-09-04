//! Read-only preparation of the schema-1 scrape rewrite algebra.
//!
//! This module never writes the inspected project.  Each adapter computes exact
//! after-bytes and the caller later applies them under the transaction's
//! before-digest precondition.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-B");

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use crate::contract::{
    Assertion, Contract, DependencyManager, Language, NodeManager, PerFileMatches, RewriteRule,
    RustForm, SetMatches,
};
use crate::glob::Glob;
use crate::model::{
    Blocker, ByteSpan, EntryKind, Inventory, InventoryEntry, NativeLockChange, PreparedRewrite,
    ScrapeError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewritePreparation {
    pub rewrites: Vec<PreparedRewrite>,
    pub blockers: Vec<Blocker>,
}

#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    end: usize,
    replacement: Vec<u8>,
}

#[derive(Debug)]
struct Candidate {
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    spans: Vec<ByteSpan>,
    native_lock_evidence: Option<NativeLockEvidence>,
}

#[derive(Debug)]
struct NativeLockEvidence {
    manager: &'static str,
    before_graph: Vec<String>,
    after_graph: Vec<String>,
    removed: Vec<String>,
}

type RewriteOutput = (Vec<u8>, usize, Vec<String>, Vec<ByteSpan>);
type CargoOutput = (Vec<u8>, usize, Vec<String>, BTreeSet<String>, Vec<ByteSpan>);
type RustImportOutput = (Vec<Edit>, BTreeSet<String>, BTreeSet<String>);
type JsonEditOutput = (Vec<Edit>, usize, Vec<String>, Vec<ByteSpan>);

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn fail(message: impl Into<String>) -> ScrapeError {
    ScrapeError::Rewrite(message.into())
}

fn apply_edits(before: &[u8], mut edits: Vec<Edit>) -> Result<Vec<u8>, ScrapeError> {
    edits.sort_by_key(|edit| (edit.start, edit.end));
    for pair in edits.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(fail("rewrite produced overlapping syntax spans"));
        }
    }
    let mut after = Vec::with_capacity(before.len());
    let mut cursor = 0;
    for edit in edits {
        if edit.start > edit.end || edit.end > before.len() || edit.start < cursor {
            return Err(fail("rewrite produced an invalid byte span"));
        }
        after.extend_from_slice(&before[cursor..edit.start]);
        after.extend_from_slice(&edit.replacement);
        cursor = edit.end;
    }
    after.extend_from_slice(&before[cursor..]);
    Ok(after)
}

fn check_set_cardinality(
    id: &str,
    cardinality: SetMatches,
    count: usize,
) -> Result<(), ScrapeError> {
    let valid = match cardinality {
        SetMatches::ZeroOrMore => true,
        SetMatches::OneOrMore => count >= 1,
        SetMatches::ExactlyOne => count == 1,
    };
    if valid {
        Ok(())
    } else {
        Err(fail(format!(
            "rewrite `{id}` cardinality mismatch: observed {count} matches"
        )))
    }
}

fn check_per_file_cardinality(
    id: &str,
    cardinality: PerFileMatches,
    path: &str,
    count: usize,
) -> Result<(), ScrapeError> {
    let valid = match cardinality {
        PerFileMatches::ZeroOrOnePerFile => count <= 1,
        PerFileMatches::ExactlyOnePerFile => count == 1,
    };
    if valid {
        Ok(())
    } else {
        Err(fail(format!(
            "rewrite `{id}` cardinality mismatch for `{path}`: observed {count} matches"
        )))
    }
}

fn inventory_files(inventory: &[InventoryEntry]) -> BTreeSet<String> {
    inventory
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .map(|entry| entry.path.clone())
        .collect()
}

fn read_candidate(project: &Project, entry: &InventoryEntry) -> Result<Vec<u8>, ScrapeError> {
    let expected_size = entry.bytes.ok_or_else(|| {
        fail(format!(
            "rewrite target `{}` has no inventoried size",
            entry.path
        ))
    })?;
    let cap = usize::try_from(expected_size).map_err(|_| {
        fail(format!(
            "rewrite target `{}` is {expected_size} bytes, which exceeds this platform's bounded-read address space",
            entry.path
        ))
    })?;
    let snapshot = project
        .read_file_snapshot_bounded(&entry.path, cap)
        .map_err(|error| {
            fail(format!(
                "cannot snapshot rewrite target `{}`: {error:#}",
                entry.path
            ))
        })?
        .ok_or_else(|| {
            fail(format!(
                "rewrite target `{}` disappeared after inventory",
                entry.path
            ))
        })?;
    let expected_digest = entry
        .sha256
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .ok_or_else(|| {
            fail(format!(
                "rewrite target `{}` has no inventoried digest",
                entry.path
            ))
        })?;
    let expected_identity = entry.identity.ok_or_else(|| {
        fail(format!(
            "rewrite target `{}` has no inventoried filesystem identity",
            entry.path
        ))
    })?;
    if snapshot.sha256 != expected_digest
        || snapshot.size != expected_size
        || snapshot.unix_mode != entry.unix_mode
        || snapshot.identity != expected_identity
        || snapshot.bytes.len() != cap
    {
        return Err(fail(format!(
            "rewrite preparation observed identity/digest/size/mode drift at `{}`",
            entry.path
        )));
    }
    Ok(snapshot.bytes)
}

fn selected_paths(
    files: &BTreeSet<String>,
    patterns: &[String],
    exclude: &[String],
) -> Result<Vec<String>, ScrapeError> {
    let patterns = patterns
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let exclude = exclude
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let includes_git = patterns.iter().any(Glob::can_match_git);
    let excludes_all_git = exclude
        .iter()
        .any(|pattern| pattern.as_str() == ".git/**" || pattern.as_str() == "**");
    if includes_git && !excludes_all_git {
        return Err(fail(
            "rewrite selector can address protected `.git` metadata; add an effective exclusion",
        ));
    }
    Ok(files
        .iter()
        .filter(|path| {
            patterns.iter().any(|pattern| pattern.matches(path))
                && !exclude.iter().any(|pattern| pattern.matches(path))
        })
        .cloned()
        .collect())
}

fn any_pattern(patterns: &[String], path: &str) -> Result<bool, ScrapeError> {
    for pattern in patterns {
        if Glob::parse(pattern)?.matches(path) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn line_spans(bytes: &[u8]) -> Vec<(usize, usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let content_end = bytes[start..]
            .iter()
            .position(|byte| *byte == b'\n' || *byte == b'\r')
            .map_or(bytes.len(), |at| start + at);
        let mut end = content_end;
        if end < bytes.len() && bytes[end] == b'\r' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\n' {
            end += 1;
        }
        spans.push((start, content_end, end));
        start = end;
    }
    if bytes.is_empty() {
        spans.push((0, 0, 0));
    }
    spans
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |at| at + 1);
    &bytes[start..end]
}

fn valid_registered_reference(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.iter().any(u8::is_ascii_whitespace) {
        return false;
    }
    let Ok(value) = std::str::from_utf8(bytes) else {
        return false;
    };
    let uri = value.split_once("://").is_some_and(|(scheme, rest)| {
        !scheme.is_empty()
            && !rest.is_empty()
            && scheme
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || matches!(byte, b'+' | b'-' | b'.'))
    });
    let anchored_id = value.contains('#')
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b'#' | b':')
        });
    uri || anchored_id
}

fn validate_rust_arguments(form: &str, arguments: &str, is_macro: bool) -> Result<(), ScrapeError> {
    use syn::parse::Parser as _;
    if is_macro {
        if form != "scope" {
            return Err(fail(format!(
                "invalid Specmark `{form}!` grammar: only `scope!` is registered as a macro"
            )));
        }
        let values = syn::punctuated::Punctuated::<syn::LitStr, syn::Token![,]>::parse_terminated
            .parse_str(arguments)
            .map_err(|_| fail("invalid Specmark `scope!` grammar: expected string literals"))?;
        if values.is_empty()
            || values
                .iter()
                .any(|value| !valid_registered_reference(value.value().as_bytes()))
        {
            return Err(fail(
                "invalid Specmark `scope!` grammar: expected registered references",
            ));
        }
        return Ok(());
    }
    if form == "scope" {
        return Err(fail(
            "invalid Specmark `scope` attribute grammar: scope is registered only as a macro",
        ));
    }
    let values = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
        .parse_str(arguments)
        .map_err(|_| fail(format!("invalid Specmark `{form}` attribute grammar")))?;
    if values.is_empty() {
        return Err(fail(format!(
            "invalid Specmark `{form}` attribute grammar: empty metadata"
        )));
    }
    for value in values {
        let literal = match value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) => value,
            syn::Expr::Assign(assign) => match *assign.right {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(value),
                    ..
                }) if matches!(*assign.left, syn::Expr::Path(_)) => value,
                _ => {
                    return Err(fail(format!(
                        "invalid Specmark `{form}` attribute grammar: values must be string references"
                    )));
                }
            },
            _ => {
                return Err(fail(format!(
                    "invalid Specmark `{form}` attribute grammar: expected string or name = string"
                )));
            }
        };
        if !valid_registered_reference(literal.value().as_bytes()) {
            return Err(fail(format!(
                "invalid Specmark `{form}` attribute grammar: unregistered reference"
            )));
        }
    }
    Ok(())
}

fn managed_markers(marker: &str) -> (Vec<u8>, Vec<u8>) {
    (
        format!("<{marker}>").into_bytes(),
        format!("</{marker}>").into_bytes(),
    )
}

fn prepare_managed(before: &[u8], marker: &str) -> Result<RewriteOutput, ScrapeError> {
    if marker != "vibevm" {
        return Err(fail(format!(
            "managed marker `{marker}` is not a registered schema-1 provider identity"
        )));
    }
    let (begin, end) = managed_markers(marker);
    let spans = line_spans(before);
    let mut open: Option<usize> = None;
    let mut pair: Option<(usize, usize)> = None;
    for (line_start, content_end, line_end) in spans {
        let line = &before[line_start..content_end];
        let trimmed = trim_ascii(line);
        let embedded = find_subslice(line, &begin).is_some() || find_subslice(line, &end).is_some();
        if trimmed == begin {
            if open.is_some() || pair.is_some() {
                return Err(fail("managed block has duplicate or nested begin markers"));
            }
            open = Some(line_start);
        } else if trimmed == end {
            let Some(start) = open.take() else {
                return Err(fail("managed block has an orphaned or reversed end marker"));
            };
            if pair.is_some() {
                return Err(fail("managed block marker occurs more than once"));
            }
            pair = Some((start, line_end));
        } else if embedded {
            return Err(fail(
                "managed marker is embedded instead of occupying a whole line",
            ));
        }
    }
    if open.is_some() {
        return Err(fail("managed block has no matching end marker"));
    }
    let Some((mut start, mut end)) = pair else {
        return Ok((before.to_vec(), 0, Vec::new(), Vec::new()));
    };

    // If the block is surrounded by blank physical lines, consume exactly one
    // side so deleting it cannot create a third blank line.  No other byte is
    // normalized.
    if start > 0 && end < before.len() {
        let previous_start = before[..start]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at + 1);
        let next_end = before[end..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(before.len(), |at| end + at + 1);
        if trim_ascii(&before[previous_start..start]).is_empty()
            && trim_ascii(&before[end..next_end]).is_empty()
        {
            end = next_end;
        }
    } else if start == 0 && end == before.len() {
        start = 0;
    }
    let span = ByteSpan {
        start: u64::try_from(start).map_err(|_| fail("managed block span exceeds u64"))?,
        end: u64::try_from(end).map_err(|_| fail("managed block span exceeds u64"))?,
        node: format!("managed-block:{marker}@{start}..{end}"),
    };
    Ok((
        apply_edits(
            before,
            vec![Edit {
                start,
                end,
                replacement: Vec::new(),
            }],
        )?,
        1,
        vec![format!("managed-block:{marker}@{start}..{end}")],
        vec![span],
    ))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn count_subslice(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut cursor = 0;
    while cursor + needle.len() <= haystack.len() {
        if &haystack[cursor..cursor + needle.len()] == needle {
            count += 1;
            cursor += needle.len();
        } else {
            cursor += 1;
        }
    }
    count
}

fn prepare_exact_text(
    before: &[u8],
    expected_sha256: &str,
    needle: &str,
    replacement: &str,
    occurrences: usize,
) -> Result<RewriteOutput, ScrapeError> {
    if digest(before) != expected_sha256 {
        return Err(fail(
            "exact-text rewrite complete-file SHA-256 does not match",
        ));
    }
    let observed = count_subslice(before, needle.as_bytes());
    if observed != occurrences {
        return Err(fail(format!(
            "exact-text rewrite expected {occurrences} occurrences, observed {observed}"
        )));
    }
    let mut edits = Vec::with_capacity(observed);
    let mut spans = Vec::with_capacity(observed);
    let mut cursor = 0;
    while let Some(relative) = find_subslice(&before[cursor..], needle.as_bytes()) {
        let start = cursor + relative;
        edits.push(Edit {
            start,
            end: start + needle.len(),
            replacement: replacement.as_bytes().to_vec(),
        });
        spans.push(ByteSpan {
            start: u64::try_from(start).map_err(|_| fail("exact-text span exceeds u64"))?,
            end: u64::try_from(start + needle.len())
                .map_err(|_| fail("exact-text span exceeds u64"))?,
            node: format!("exact-text-occurrence@{start}"),
        });
        cursor = start + needle.len();
    }
    Ok((
        apply_edits(before, edits)?,
        observed,
        (0..observed)
            .map(|index| format!("exact-text-occurrence:{index}"))
            .collect(),
        spans,
    ))
}

fn prepare_toml_array(
    before: &[u8],
    table_path: &[String],
    key: &str,
    values: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    let source =
        std::str::from_utf8(before).map_err(|_| fail("TOML rewrite target is not UTF-8"))?;
    let mut document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| fail(format!("cannot parse TOML rewrite target: {error}")))?;
    let mut current = document.as_table_mut();
    for component in table_path {
        current = current
            .get_mut(component)
            .and_then(toml_edit::Item::as_table_mut)
            .ok_or_else(|| {
                fail(format!(
                    "TOML table path `{}` is absent",
                    table_path.join(".")
                ))
            })?;
    }
    let array = current
        .get_mut(key)
        .and_then(toml_edit::Item::as_array_mut)
        .ok_or_else(|| fail(format!("TOML key `{key}` is not an array")))?;
    let wanted = values.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut removed = Vec::new();
    let mut spans = Vec::new();
    for index in (0..array.len()).rev() {
        if let Some(value) = array.get(index).and_then(toml_edit::Value::as_str)
            && wanted.contains(value)
        {
            let source_span = array.get(index).expect("reverse index is in bounds").span();
            let span = match source_span {
                Some(span) => span,
                None => {
                    let rendered = toml_edit::Value::from(value.to_owned()).to_string();
                    unique_subslice_span(before, rendered.as_bytes()).ok_or_else(|| {
                        fail(format!(
                            "TOML value `{value}` has no unambiguous original source span"
                        ))
                    })?
                }
            };
            spans.push(ByteSpan {
                start: u64::try_from(span.start).map_err(|_| fail("TOML span exceeds u64"))?,
                end: u64::try_from(span.end).map_err(|_| fail("TOML span exceeds u64"))?,
                node: format!("toml:{}.{key}[{index}]={value}", table_path.join(".")),
            });
            removed.push((index, value.to_owned()));
            array.remove(index);
        }
    }
    removed.reverse();
    let nodes = removed
        .iter()
        .map(|(index, value)| format!("toml:{}.{key}[{index}]={value}", table_path.join(".")))
        .collect::<Vec<_>>();
    spans.reverse();
    Ok((
        document.to_string().into_bytes(),
        removed.len(),
        nodes,
        spans,
    ))
}

fn unique_subslice_span(haystack: &[u8], needle: &[u8]) -> Option<std::ops::Range<usize>> {
    let start = find_subslice(haystack, needle)?;
    if find_subslice(&haystack[start + needle.len()..], needle).is_some() {
        return None;
    }
    Some(start..start + needle.len())
}

fn dependency_identity<'a>(key: &'a str, item: &'a toml_edit::Item) -> Option<&'a str> {
    item.get("package")
        .and_then(toml_edit::Item::as_str)
        .or(Some(key))
}

fn is_workspace_inherited(item: &toml_edit::Item) -> bool {
    item.get("workspace")
        .and_then(toml_edit::Item::as_bool)
        .unwrap_or(false)
}

struct CargoRemovalContext<'a> {
    source: &'a [u8],
    package: &'a str,
    allow_aliases: &'a BTreeSet<&'a str>,
    workspace_aliases: &'a BTreeMap<String, String>,
    known_aliases: &'a mut BTreeSet<String>,
    nodes: &'a mut Vec<String>,
    spans: &'a mut Vec<ByteSpan>,
}

fn remove_dependency_entries(
    table: &mut toml_edit::Table,
    table_name: &str,
    context: &mut CargoRemovalContext<'_>,
) -> Result<usize, ScrapeError> {
    let keys = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    let mut removed = 0;
    for key in keys {
        let item = &table[&key];
        let identity = dependency_identity(&key, item).unwrap_or(&key);
        let inherited = is_workspace_inherited(item);
        let targets_package = identity == context.package
            || (inherited
                && context
                    .workspace_aliases
                    .get(&key)
                    .is_some_and(|identity| identity == context.package));
        if !targets_package {
            continue;
        }
        if !context.allow_aliases.is_empty()
            && key != context.package
            && !context.allow_aliases.contains(key.as_str())
        {
            return Err(fail(format!(
                "Cargo package `{}` is present as unallowed alias `{key}` in [{table_name}]",
                context.package
            )));
        }
        let span = item_source_span(
            context.source,
            &key,
            item,
            &format!("Cargo dependency `{table_name}.{key}`"),
        )?;
        context.spans.push(span);
        context.known_aliases.insert(key.clone());
        table.remove(&key);
        context.nodes.push(format!("cargo:{table_name}.{key}"));
        removed += 1;
    }
    Ok(removed)
}

fn remove_feature_edges(
    document: &mut toml_edit::DocumentMut,
    source: &[u8],
    aliases: &BTreeSet<String>,
    nodes: &mut Vec<String>,
    spans: &mut Vec<ByteSpan>,
) -> Result<usize, ScrapeError> {
    let Some(features) = document
        .get_mut("features")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return Ok(0);
    };
    let mut removed = 0;
    for (feature, item) in features.iter_mut() {
        let Some(array) = item.as_array_mut() else {
            continue;
        };
        for index in (0..array.len()).rev() {
            let Some(edge) = array.get(index).and_then(toml_edit::Value::as_str) else {
                continue;
            };
            let target = edge
                .strip_prefix("dep:")
                .unwrap_or(edge)
                .split_once('/')
                .map_or(edge.strip_prefix("dep:").unwrap_or(edge), |(name, _)| name)
                .trim_end_matches('?');
            if aliases.contains(target) {
                let value = array.get(index).expect("reverse index is in bounds");
                spans.push(value_source_span(
                    source,
                    value,
                    &format!("Cargo feature edge `{feature}[{index}]`"),
                )?);
                nodes.push(format!("cargo:features.{feature}[{index}]={edge}"));
                array.remove(index);
                removed += 1;
            }
        }
    }
    Ok(removed)
}

fn item_source_span(
    source: &[u8],
    key: &str,
    item: &toml_edit::Item,
    label: &str,
) -> Result<ByteSpan, ScrapeError> {
    let span = item
        .span()
        .or_else(|| keyed_item_span(source, key, item))
        .ok_or_else(|| fail(format!("{label} has no unambiguous original source span")))?;
    Ok(ByteSpan {
        start: u64::try_from(span.start).map_err(|_| fail("Cargo span exceeds u64"))?,
        end: u64::try_from(span.end).map_err(|_| fail("Cargo span exceeds u64"))?,
        node: label.to_owned(),
    })
}

fn keyed_item_span(
    source: &[u8],
    key: &str,
    item: &toml_edit::Item,
) -> Option<std::ops::Range<usize>> {
    let rendered = item.to_string();
    let rendered = rendered.trim().as_bytes();
    let quoted_key = toml_edit::Key::new(key).to_string();
    let mut found = None;
    for (line_start, content_end, _) in line_spans(source) {
        let line = &source[line_start..content_end];
        let leading = line.len() - line.trim_ascii_start().len();
        let trimmed = &line[leading..];
        let dotted = format!("{key}.");
        if trimmed.starts_with(dotted.as_bytes()) && trimmed.contains(&b'=') {
            let start = line_start + leading;
            let end = line_start + leading + trimmed.trim_ascii_end().len();
            if found.replace(start..end).is_some() {
                return None;
            }
            continue;
        }
        let key_len = if trimmed.starts_with(key.as_bytes())
            && trimmed
                .get(key.len())
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'=')
        {
            key.len()
        } else if trimmed.starts_with(quoted_key.as_bytes()) {
            quoted_key.len()
        } else {
            continue;
        };
        let after_key = &trimmed[key_len..];
        let equal = after_key.iter().position(|byte| *byte == b'=')?;
        let value_region = &after_key[equal + 1..];
        let value_leading = value_region.len() - value_region.trim_ascii_start().len();
        if !value_region[value_leading..].starts_with(rendered) {
            continue;
        }
        let start = line_start + leading;
        let end = line_start + leading + key_len + equal + 1 + value_leading + rendered.len();
        if found.replace(start..end).is_some() {
            return None;
        }
    }
    found
}

fn value_source_span(
    source: &[u8],
    value: &toml_edit::Value,
    label: &str,
) -> Result<ByteSpan, ScrapeError> {
    let span = value
        .span()
        .or_else(|| unique_subslice_span(source, value.to_string().as_bytes()))
        .ok_or_else(|| fail(format!("{label} has no unambiguous original source span")))?;
    Ok(ByteSpan {
        start: u64::try_from(span.start).map_err(|_| fail("Cargo span exceeds u64"))?,
        end: u64::try_from(span.end).map_err(|_| fail("Cargo span exceeds u64"))?,
        node: label.to_owned(),
    })
}

#[cfg(test)]
fn prepare_cargo(
    before: &[u8],
    package: &str,
    aliases: &[String],
) -> Result<CargoOutput, ScrapeError> {
    prepare_cargo_resolved(before, package, aliases, &BTreeMap::new())
}

fn prepare_cargo_resolved(
    before: &[u8],
    package: &str,
    aliases: &[String],
    workspace_aliases: &BTreeMap<String, String>,
) -> Result<CargoOutput, ScrapeError> {
    let source = std::str::from_utf8(before).map_err(|_| fail("Cargo manifest is not UTF-8"))?;
    let mut document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| fail(format!("cannot parse Cargo manifest: {error}")))?;
    let allow_aliases = aliases.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut known_aliases = BTreeSet::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    let mut count = 0;
    let mut removal = CargoRemovalContext {
        source: before,
        package,
        allow_aliases: &allow_aliases,
        workspace_aliases,
        known_aliases: &mut known_aliases,
        nodes: &mut nodes,
        spans: &mut spans,
    };

    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = document
            .get_mut(table_name)
            .and_then(toml_edit::Item::as_table_mut)
        {
            count += remove_dependency_entries(table, table_name, &mut removal)?;
        }
    }
    if let Some(workspace) = document
        .get_mut("workspace")
        .and_then(toml_edit::Item::as_table_mut)
        && let Some(table) = workspace
            .get_mut("dependencies")
            .and_then(toml_edit::Item::as_table_mut)
    {
        count += remove_dependency_entries(table, "workspace.dependencies", &mut removal)?;
    }
    if let Some(target) = document
        .get_mut("target")
        .and_then(toml_edit::Item::as_table_mut)
    {
        for (selector, selector_item) in target.iter_mut() {
            let Some(selector_table) = selector_item.as_table_mut() else {
                continue;
            };
            for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(table) = selector_table
                    .get_mut(table_name)
                    .and_then(toml_edit::Item::as_table_mut)
                {
                    count += remove_dependency_entries(
                        table,
                        &format!("target.{selector}.{table_name}"),
                        &mut removal,
                    )?;
                }
            }
        }
    }
    if let Some(patch) = document
        .get_mut("patch")
        .and_then(toml_edit::Item::as_table_mut)
    {
        for (registry, registry_item) in patch.iter_mut() {
            let Some(table) = registry_item.as_table_mut() else {
                continue;
            };
            count += remove_dependency_entries(table, &format!("patch.{registry}"), &mut removal)?;
        }
    }
    if let Some(replace) = document
        .get_mut("replace")
        .and_then(toml_edit::Item::as_table_mut)
    {
        let keys = replace
            .iter()
            .map(|(key, _)| key.to_owned())
            .collect::<Vec<_>>();
        for key in keys {
            let identity = key.split_once(':').map_or(key.as_str(), |(name, _)| name);
            if identity == package {
                let item = &replace[&key];
                removal.spans.push(item_source_span(
                    removal.source,
                    &key,
                    item,
                    &format!("Cargo replace `{key}`"),
                )?);
                replace.remove(&key);
                removal.nodes.push(format!("cargo:replace.{key}"));
                count += 1;
            }
        }
    }
    count += remove_feature_edges(
        &mut document,
        removal.source,
        removal.known_aliases,
        removal.nodes,
        removal.spans,
    )?;
    let after = document.to_string().into_bytes();
    let parsed_after = std::str::from_utf8(&after)
        .expect("toml_edit emits UTF-8")
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| fail(format!("rewritten Cargo manifest is invalid: {error}")))?;
    if cargo_contains_identity(parsed_after.as_table(), package, &known_aliases) {
        return Err(fail(format!(
            "Cargo residual resolver still finds package `{package}` or an authorized alias"
        )));
    }
    spans.sort_by_key(|span| (span.start, span.end));
    for pair in spans.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(fail(
                "Cargo evidence spans overlap in the original preimage",
            ));
        }
    }
    Ok((after, count, nodes, known_aliases, spans))
}

fn cargo_workspace_alias_map<'a>(
    manifests: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Result<BTreeMap<String, String>, ScrapeError> {
    let mut aliases = BTreeMap::new();
    for (path, bytes) in manifests {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| fail(format!("Cargo manifest `{path}` is not UTF-8")))?;
        let document = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|error| fail(format!("cannot parse Cargo manifest `{path}`: {error}")))?;
        let Some(dependencies) = document
            .get("workspace")
            .and_then(toml_edit::Item::as_table)
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml_edit::Item::as_table)
        else {
            continue;
        };
        for (alias, item) in dependencies {
            if is_workspace_inherited(item) {
                return Err(fail(format!(
                    "workspace dependency `{alias}` in `{path}` recursively inherits workspace identity"
                )));
            }
            let identity = dependency_identity(alias, item)
                .ok_or_else(|| fail(format!("workspace dependency `{alias}` has no identity")))?;
            if let Some(prior) = aliases.insert(alias.to_owned(), identity.to_owned())
                && prior != identity
            {
                return Err(fail(format!(
                    "Cargo workspace alias `{alias}` resolves to both `{prior}` and `{identity}`; workspace ownership is ambiguous"
                )));
            }
        }
    }
    Ok(aliases)
}

#[derive(Debug)]
struct CargoManifestNode {
    path: String,
    dir: String,
    bytes: Vec<u8>,
    has_package: bool,
    has_workspace: bool,
    workspace_members: Vec<String>,
    workspace_exclude: Vec<String>,
}

#[derive(Debug)]
struct CargoTopology {
    manifests: BTreeMap<String, CargoManifestNode>,
    locks: BTreeSet<String>,
}

impl CargoTopology {
    fn build<'a>(
        manifests: impl IntoIterator<Item = (&'a str, &'a [u8])>,
        locks: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, ScrapeError> {
        let mut nodes = BTreeMap::new();
        for (path, bytes) in manifests {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| fail(format!("Cargo manifest `{path}` is not UTF-8")))?;
            let document = text
                .parse::<toml_edit::DocumentMut>()
                .map_err(|error| fail(format!("cannot parse Cargo manifest `{path}`: {error}")))?;
            let dir = cargo_parent(path, "Cargo.toml")?;
            let has_package = document
                .get("package")
                .and_then(toml_edit::Item::as_table)
                .is_some();
            let workspace = document
                .get("workspace")
                .and_then(toml_edit::Item::as_table);
            let has_workspace = workspace.is_some();
            let workspace_members = cargo_workspace_paths(
                workspace.and_then(|table| table.get("members")),
                path,
                &dir,
                "members",
            )?;
            let workspace_exclude = cargo_workspace_paths(
                workspace.and_then(|table| table.get("exclude")),
                path,
                &dir,
                "exclude",
            )?;
            let node = CargoManifestNode {
                path: path.to_owned(),
                dir,
                bytes: bytes.to_vec(),
                has_package,
                has_workspace,
                workspace_members,
                workspace_exclude,
            };
            if nodes.insert(path.to_owned(), node).is_some() {
                return Err(fail(format!("duplicate Cargo manifest `{path}`")));
            }
        }
        Ok(Self {
            manifests: nodes,
            locks: locks.into_iter().map(str::to_owned).collect(),
        })
    }

    fn workspace_root_for(
        &self,
        manifest: &str,
    ) -> Result<Option<&CargoManifestNode>, ScrapeError> {
        let node = self.manifests.get(manifest).ok_or_else(|| {
            fail(format!(
                "Cargo manifest `{manifest}` is absent from topology"
            ))
        })?;
        if node.has_workspace {
            return Ok(Some(node));
        }
        let roots = self
            .manifests
            .values()
            .filter(|candidate| candidate.has_workspace)
            .filter_map(
                |candidate| match cargo_workspace_contains(candidate, manifest) {
                    Ok(true) => Some(Ok(candidate)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect::<Result<Vec<_>, ScrapeError>>()?;
        match roots.as_slice() {
            [] => Ok(None),
            [root] => Ok(Some(*root)),
            _ => Err(ScrapeError::blocked(format!(
                "Cargo manifest `{manifest}` is selected by multiple workspace roots"
            ))),
        }
    }

    fn workspace_aliases_for(
        &self,
        manifest: &str,
    ) -> Result<BTreeMap<String, String>, ScrapeError> {
        let Some(root) = self.workspace_root_for(manifest)? else {
            return Ok(BTreeMap::new());
        };
        cargo_workspace_alias_map(std::iter::once((root.path.as_str(), root.bytes.as_slice())))
    }

    fn owned_lock_for(&self, manifest: &str) -> Result<Option<String>, ScrapeError> {
        let node = self.manifests.get(manifest).ok_or_else(|| {
            fail(format!(
                "Cargo manifest `{manifest}` is absent from topology"
            ))
        })?;
        let owner = self.workspace_root_for(manifest)?.unwrap_or(node);
        let lock = cargo_child(&owner.dir, "Cargo.lock");
        Ok(self.locks.contains(&lock).then_some(lock))
    }

    fn owned_locks<'a>(
        &self,
        manifests: impl IntoIterator<Item = &'a str>,
    ) -> Result<BTreeSet<String>, ScrapeError> {
        let mut result = BTreeSet::new();
        for manifest in manifests {
            if let Some(lock) = self.owned_lock_for(manifest)? {
                result.insert(lock);
            }
        }
        Ok(result)
    }

    fn source_manifest(&self, source: &str) -> Result<&CargoManifestNode, ScrapeError> {
        let owners = self
            .manifests
            .values()
            .filter(|manifest| manifest.has_package && cargo_dir_contains(&manifest.dir, source))
            .collect::<Vec<_>>();
        let Some(longest) = owners.iter().map(|owner| owner.dir.len()).max() else {
            return Err(ScrapeError::blocked(format!(
                "Rust source `{source}` has no owning package Cargo.toml"
            )));
        };
        let nearest = owners
            .into_iter()
            .filter(|owner| owner.dir.len() == longest)
            .collect::<Vec<_>>();
        match nearest.as_slice() {
            [owner] => Ok(*owner),
            _ => Err(ScrapeError::blocked(format!(
                "Rust source `{source}` has ambiguous owning Cargo manifests"
            ))),
        }
    }
}

fn cargo_parent(path: &str, leaf: &str) -> Result<String, ScrapeError> {
    if path == leaf {
        return Ok(String::new());
    }
    path.strip_suffix(&format!("/{leaf}"))
        .map(str::to_owned)
        .ok_or_else(|| fail(format!("Cargo path `{path}` is not a `{leaf}` path")))
}

fn cargo_child(dir: &str, leaf: &str) -> String {
    if dir.is_empty() {
        leaf.to_owned()
    } else {
        format!("{dir}/{leaf}")
    }
}

fn cargo_dir_contains(dir: &str, path: &str) -> bool {
    dir.is_empty() || path.starts_with(&(dir.to_owned() + "/"))
}

fn cargo_workspace_paths(
    item: Option<&toml_edit::Item>,
    manifest: &str,
    root: &str,
    field: &str,
) -> Result<Vec<String>, ScrapeError> {
    let Some(item) = item else {
        return Ok(Vec::new());
    };
    let values = item.as_array().ok_or_else(|| {
        ScrapeError::blocked(format!(
            "Cargo workspace `{manifest}` has non-array `{field}`"
        ))
    })?;
    let mut result = Vec::new();
    for value in values {
        let relative = value.as_str().ok_or_else(|| {
            ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has non-string `{field}` member"
            ))
        })?;
        if relative.is_empty()
            || relative.starts_with('/')
            || relative.contains('\\')
            || relative.split('/').any(|part| part == "." || part == "..")
        {
            return Err(ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has unsupported `{field}` path `{relative}`"
            )));
        }
        let joined = cargo_child(root, relative.trim_end_matches('/'));
        let pattern = cargo_child(&joined, "Cargo.toml");
        Glob::parse(&pattern).map_err(|error| {
            ScrapeError::blocked(format!(
                "Cargo workspace `{manifest}` has unsupported `{field}` pattern `{relative}`: {error}"
            ))
        })?;
        result.push(pattern);
    }
    result.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    result.dedup();
    Ok(result)
}

fn cargo_workspace_contains(
    workspace: &CargoManifestNode,
    manifest: &str,
) -> Result<bool, ScrapeError> {
    let included = workspace
        .workspace_members
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|pattern| pattern.matches(manifest));
    let excluded = workspace
        .workspace_exclude
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|pattern| pattern.matches(manifest));
    Ok(included && !excluded)
}

fn cargo_rule_selects_manifest(manifests: &[String], manifest: &str) -> Result<bool, ScrapeError> {
    manifests
        .iter()
        .map(|pattern| Glob::parse(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map(|patterns| patterns.iter().any(|pattern| pattern.matches(manifest)))
}

fn cargo_topology_from_current(
    current: &BTreeMap<String, Vec<u8>>,
    files: &BTreeSet<String>,
) -> Result<CargoTopology, ScrapeError> {
    CargoTopology::build(
        current.iter().filter_map(|(path, bytes)| {
            path.ends_with("Cargo.toml")
                .then_some((path.as_str(), bytes.as_slice()))
        }),
        files
            .iter()
            .filter(|path| path.ends_with("Cargo.lock"))
            .map(String::as_str),
    )
}

fn observed_specmark_aliases_for_source(
    contract: &Contract,
    topology: &CargoTopology,
    source: &str,
) -> Result<BTreeSet<String>, ScrapeError> {
    let owner = topology.source_manifest(source)?;
    let matching = contract
        .rewrite
        .iter()
        .filter_map(|rule| {
            let RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                aliases,
                ..
            } = rule
            else {
                return None;
            };
            (package == "core-ai-native-specmark").then_some((id, manifests, package, aliases))
        })
        .filter_map(
            |row| match cargo_rule_selects_manifest(row.1, &owner.path) {
                Ok(true) => Some(Ok(row)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let (rule_id, _, package, aliases) = match matching.as_slice() {
        [only] => *only,
        [] => return Ok(BTreeSet::new()),
        _ => {
            return Err(ScrapeError::blocked(format!(
                "Rust source `{source}` is owned by `{}` but multiple Specmark Cargo removal rules claim it",
                owner.path
            )));
        }
    };
    let workspace_aliases = topology.workspace_aliases_for(&owner.path)?;
    let (_, _, _, observed, _) =
        prepare_cargo_resolved(&owner.bytes, package, aliases, &workspace_aliases).map_err(
            |error| {
                ScrapeError::blocked(format!(
                    "Cargo rule `{rule_id}` cannot prove Specmark ownership for Rust source `{source}` through `{}`: {error}",
                    owner.path
                ))
            },
        )?;
    Ok(observed
        .into_iter()
        .map(|alias| alias.replace('-', "_"))
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CargoLockPackageId {
    name: String,
    version: String,
    source: Option<String>,
}

#[derive(Debug)]
struct CargoLockDependency {
    array_index: usize,
    target: usize,
}

#[derive(Debug)]
struct CargoLockNode {
    id: CargoLockPackageId,
    dependencies: Vec<CargoLockDependency>,
}

#[derive(Debug)]
struct CargoLockGraph {
    nodes: Vec<CargoLockNode>,
    roots: Vec<usize>,
}

type CargoLockOutput = (RewriteOutput, Option<NativeLockEvidence>);

fn cargo_lock_blocked(message: impl Into<String>) -> ScrapeError {
    ScrapeError::blocked(message)
}

fn cargo_lock_package_id(
    table: &toml_edit::Table,
    index: usize,
) -> Result<CargoLockPackageId, ScrapeError> {
    let field = |name: &str| {
        table
            .get(name)
            .and_then(toml_edit::Item::as_str)
            .ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has no string `{name}`"
                ))
            })
    };
    let source = match table.get("source") {
        None => None,
        Some(item) => {
            let value = item.as_str().ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has a non-string `source`"
                ))
            })?;
            if value.is_empty() {
                return Err(cargo_lock_blocked(format!(
                    "Cargo.lock [[package]] #{index} has an empty `source`"
                )));
            }
            Some(value.to_owned())
        }
    };
    Ok(CargoLockPackageId {
        name: field("name")?.to_owned(),
        version: field("version")?.to_owned(),
        source,
    })
}

fn cargo_lock_dependency_selector(
    value: &str,
) -> Result<(&str, Option<&str>, Option<&str>), ScrapeError> {
    let mut fields = value.split_ascii_whitespace();
    let name = fields
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| cargo_lock_blocked("Cargo.lock contains an empty dependency selector"))?;
    let Some(version) = fields.next() else {
        return Ok((name, None, None));
    };
    let rest = fields.collect::<Vec<_>>();
    if rest.is_empty() {
        return Ok((name, Some(version), None));
    }
    if rest.len() != 1 {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` has an unsupported shape"
        )));
    }
    let source = rest[0]
        .strip_prefix('(')
        .and_then(|source| source.strip_suffix(')'))
        .filter(|source| !source.is_empty())
        .ok_or_else(|| {
            cargo_lock_blocked(format!(
                "Cargo.lock dependency selector `{value}` has an invalid source"
            ))
        })?;
    Ok((name, Some(version), Some(source)))
}

fn resolve_cargo_lock_dependency(
    value: &str,
    identities: &[CargoLockPackageId],
) -> Result<usize, ScrapeError> {
    let (name, version, source) = cargo_lock_dependency_selector(value)?;
    let candidates = identities
        .iter()
        .enumerate()
        .filter(|(_, identity)| {
            identity.name == name
                && version.is_none_or(|version| identity.version == version)
                && source.is_none_or(|source| identity.source.as_deref() == Some(source))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [only] => Ok(*only),
        [] => Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` resolves to no [[package]]"
        ))),
        _ => Err(cargo_lock_blocked(format!(
            "Cargo.lock dependency selector `{value}` is ambiguous"
        ))),
    }
}

fn parse_cargo_lock_graph(
    document: &toml_edit::DocumentMut,
) -> Result<CargoLockGraph, ScrapeError> {
    match document
        .get("version")
        .and_then(toml_edit::Item::as_integer)
    {
        Some(3 | 4) => {}
        Some(version) => {
            return Err(cargo_lock_blocked(format!(
                "Cargo.lock format version {version} is unsupported by schema-1 reconciliation"
            )));
        }
        None => {
            return Err(cargo_lock_blocked(
                "Cargo.lock has no supported integer format version",
            ));
        }
    }
    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| cargo_lock_blocked("Cargo.lock has no [[package]] graph"))?;
    if packages.is_empty() {
        return Err(cargo_lock_blocked(
            "Cargo.lock has an empty [[package]] graph",
        ));
    }
    let identities = packages
        .iter()
        .enumerate()
        .map(|(index, table)| cargo_lock_package_id(table, index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = BTreeSet::new();
    if let Some(duplicate) = identities
        .iter()
        .find(|identity| !unique.insert((*identity).clone()))
    {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock repeats package identity `{}`",
            cargo_lock_identity(duplicate)
        )));
    }
    let mut indegree = vec![0_usize; identities.len()];
    let mut nodes = Vec::with_capacity(identities.len());
    for (index, table) in packages.iter().enumerate() {
        let mut dependencies = Vec::new();
        let mut dependency_targets = BTreeSet::new();
        if let Some(item) = table.get("dependencies") {
            let array = item.as_array().ok_or_else(|| {
                cargo_lock_blocked(format!(
                    "Cargo.lock package `{}` has non-array `dependencies`",
                    cargo_lock_identity(&identities[index])
                ))
            })?;
            for (array_index, value) in array.iter().enumerate() {
                let selector = value.as_str().ok_or_else(|| {
                    cargo_lock_blocked(format!(
                        "Cargo.lock package `{}` has a non-string dependency",
                        cargo_lock_identity(&identities[index])
                    ))
                })?;
                let target = resolve_cargo_lock_dependency(selector, &identities)?;
                if !dependency_targets.insert(target) {
                    return Err(cargo_lock_blocked(format!(
                        "Cargo.lock package `{}` repeats dependency target `{selector}`",
                        cargo_lock_identity(&identities[index])
                    )));
                }
                indegree[target] = indegree[target].checked_add(1).ok_or_else(|| {
                    cargo_lock_blocked("Cargo.lock dependency indegree overflows usize")
                })?;
                dependencies.push(CargoLockDependency {
                    array_index,
                    target,
                });
            }
        }
        nodes.push(CargoLockNode {
            id: identities[index].clone(),
            dependencies,
        });
    }
    let roots = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect();
    Ok(CargoLockGraph { nodes, roots })
}

fn cargo_lock_identity(identity: &CargoLockPackageId) -> String {
    let source = identity.source.as_deref().unwrap_or("");
    format!(
        "n{}:{}|v{}:{}|s{}:{}",
        identity.name.len(),
        identity.name,
        identity.version.len(),
        identity.version,
        source.len(),
        source
    )
}

fn cargo_lock_graph_evidence(graph: &CargoLockGraph) -> Vec<String> {
    let mut evidence = graph
        .nodes
        .iter()
        .map(|node| format!("node|{}", cargo_lock_identity(&node.id)))
        .collect::<Vec<_>>();
    evidence.extend(graph.nodes.iter().flat_map(|node| {
        node.dependencies.iter().map(|dependency| {
            format!(
                "edge|{}|{}",
                cargo_lock_identity(&node.id),
                cargo_lock_identity(&graph.nodes[dependency.target].id)
            )
        })
    }));
    evidence.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    evidence
}

fn cargo_lock_package_spans(
    source: &[u8],
    expected_packages: usize,
) -> Result<Vec<std::ops::Range<usize>>, ScrapeError> {
    let starts = line_spans(source)
        .into_iter()
        .filter_map(|(line_start, content_end, _)| {
            (source[line_start..content_end].trim_ascii() == b"[[package]]").then_some(line_start)
        })
        .collect::<Vec<_>>();
    if starts.len() != expected_packages {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock has {expected_packages} parsed packages but {} canonical [[package]] headers; annotated or noncanonical headers are unsupported",
            starts.len()
        )));
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(index, start)| *start..starts.get(index + 1).copied().unwrap_or(source.len()))
        .collect())
}

fn cargo_lock_value_span(
    source: &[u8],
    package_span: &std::ops::Range<usize>,
    value: &toml_edit::Value,
    label: &str,
) -> Result<ByteSpan, ScrapeError> {
    let span = value.span().or_else(|| {
        let rendered = value.to_string();
        unique_subslice_span(&source[package_span.clone()], rendered.as_bytes())
            .map(|span| package_span.start + span.start..package_span.start + span.end)
    });
    let span = span.filter(|span| {
        span.start >= package_span.start && span.end <= package_span.end && span.start <= span.end
    });
    let span = span.ok_or_else(|| {
        cargo_lock_blocked(format!(
            "{label} has no unambiguous source span inside its package table"
        ))
    })?;
    Ok(ByteSpan {
        start: u64::try_from(span.start)
            .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
        end: u64::try_from(span.end)
            .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
        node: label.to_owned(),
    })
}

fn cargo_lock_reachable(
    graph: &CargoLockGraph,
    root: usize,
    cut_root_edges: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut reachable = BTreeSet::from([root]);
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        for dependency in &graph.nodes[node].dependencies {
            if node == root && cut_root_edges.contains(&dependency.array_index) {
                continue;
            }
            if reachable.insert(dependency.target) {
                pending.push(dependency.target);
            }
        }
    }
    reachable
}

fn prepare_cargo_lock(before: &[u8], package: &str) -> Result<CargoLockOutput, ScrapeError> {
    let source =
        std::str::from_utf8(before).map_err(|_| cargo_lock_blocked("Cargo.lock is not UTF-8"))?;
    let mut document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| cargo_lock_blocked(format!("cannot parse Cargo.lock: {error}")))?;
    let before_graph = parse_cargo_lock_graph(&document)?;
    let targets = before_graph
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node.id.name == package).then_some(index))
        .collect::<BTreeSet<_>>();
    if targets.is_empty() {
        return Ok(((before.to_vec(), 0, Vec::new(), Vec::new()), None));
    }
    let root = match before_graph.roots.as_slice() {
        [root] => *root,
        [] => {
            return Err(cargo_lock_blocked(
                "Cargo.lock graph has no unique root package",
            ));
        }
        roots => {
            return Err(cargo_lock_blocked(format!(
                "Cargo.lock graph has {} root packages; schema-1 reconciliation supports exactly one",
                roots.len()
            )));
        }
    };
    if before_graph.nodes[root].id.source.is_some() {
        return Err(cargo_lock_blocked(
            "Cargo.lock unique graph root is registry/source-backed rather than a local project package",
        ));
    }
    if targets.contains(&root) {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock root package is the requested removal identity `{package}`"
        )));
    }
    let cut_root_edges = before_graph.nodes[root]
        .dependencies
        .iter()
        .filter_map(|dependency| {
            targets
                .contains(&dependency.target)
                .then_some(dependency.array_index)
        })
        .collect::<BTreeSet<_>>();
    if cut_root_edges.is_empty() {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock package `{package}` is not a direct dependency of the unique root; manifest-to-lock authorization is ambiguous"
        )));
    }
    let reachable = cargo_lock_reachable(&before_graph, root, &cut_root_edges);
    if targets.iter().any(|target| reachable.contains(target)) {
        return Err(cargo_lock_blocked(format!(
            "Cargo.lock package `{package}` remains reachable through a retained dependency"
        )));
    }
    let removed_indices = (0..before_graph.nodes.len())
        .filter(|index| !reachable.contains(index))
        .collect::<BTreeSet<_>>();
    let mut removed = removed_indices
        .iter()
        .map(|index| cargo_lock_identity(&before_graph.nodes[*index].id))
        .collect::<Vec<_>>();
    removed.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));

    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .expect("graph parser established [[package]]");
    let package_spans = cargo_lock_package_spans(before, packages.len())?;
    let root_dependencies = packages
        .get(root)
        .expect("root index came from the parsed graph")
        .get("dependencies")
        .and_then(toml_edit::Item::as_array)
        .expect("cut edges establish a root dependency array");
    let mut spans = cut_root_edges
        .iter()
        .map(|index| {
            cargo_lock_value_span(
                before,
                &package_spans[root],
                root_dependencies
                    .get(*index)
                    .expect("cut dependency index came from the parsed graph"),
                "Cargo.lock root dependency removal",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    for index in &removed_indices {
        let span = package_spans
            .get(*index)
            .expect("removed index came from the parsed graph");
        spans.push(ByteSpan {
            start: u64::try_from(span.start)
                .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
            end: u64::try_from(span.end)
                .map_err(|_| cargo_lock_blocked("Cargo.lock span exceeds u64"))?,
            node: format!(
                "Cargo.lock package `{}`",
                cargo_lock_identity(&before_graph.nodes[*index].id)
            ),
        });
    }
    spans.sort_by_key(|span| (span.start, span.end));
    for pair in spans.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(cargo_lock_blocked(
                "Cargo.lock graph evidence spans overlap",
            ));
        }
    }

    let packages = document
        .get_mut("package")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
        .expect("graph parser established mutable [[package]]");
    let root_table = packages
        .get_mut(root)
        .expect("root index came from the parsed graph");
    let dependencies_empty = {
        let dependencies = root_table
            .get_mut("dependencies")
            .and_then(toml_edit::Item::as_array_mut)
            .expect("cut edges establish a mutable root dependency array");
        for index in cut_root_edges.iter().rev() {
            dependencies.remove(*index);
        }
        dependencies.is_empty()
    };
    if dependencies_empty {
        root_table.remove("dependencies");
    }
    for index in removed_indices.iter().rev() {
        packages.remove(*index);
    }
    let after = document.to_string().into_bytes();
    let after_document = std::str::from_utf8(&after)
        .expect("toml_edit emits UTF-8")
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| cargo_lock_blocked(format!("rewritten Cargo.lock is invalid: {error}")))?;
    let after_graph = parse_cargo_lock_graph(&after_document)?;
    if after_graph.nodes.iter().any(|node| node.id.name == package) {
        return Err(cargo_lock_blocked(format!(
            "rewritten Cargo.lock still contains package `{package}`"
        )));
    }
    let nodes = removed
        .iter()
        .map(|identity| format!("cargo-lock:removed:{identity}"))
        .collect::<Vec<_>>();
    let matches = cut_root_edges.len() + removed_indices.len();
    Ok((
        (after, matches, nodes, spans),
        Some(NativeLockEvidence {
            manager: "cargo",
            before_graph: cargo_lock_graph_evidence(&before_graph),
            after_graph: cargo_lock_graph_evidence(&after_graph),
            removed,
        }),
    ))
}

fn cargo_contains_identity(
    table: &toml_edit::Table,
    package: &str,
    aliases: &BTreeSet<String>,
) -> bool {
    fn dependencies_contain(
        table: Option<&toml_edit::Table>,
        package: &str,
        aliases: &BTreeSet<String>,
    ) -> bool {
        table.is_some_and(|table| {
            table.iter().any(|(key, item)| {
                key == package
                    || aliases.contains(key)
                    || item.get("package").and_then(toml_edit::Item::as_str) == Some(package)
            })
        })
    }

    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if dependencies_contain(
            table.get(name).and_then(toml_edit::Item::as_table),
            package,
            aliases,
        ) {
            return true;
        }
    }
    if dependencies_contain(
        table
            .get("workspace")
            .and_then(toml_edit::Item::as_table)
            .and_then(|workspace| workspace.get("dependencies"))
            .and_then(toml_edit::Item::as_table),
        package,
        aliases,
    ) {
        return true;
    }
    if let Some(targets) = table.get("target").and_then(toml_edit::Item::as_table) {
        for target in targets.iter().filter_map(|(_, item)| item.as_table()) {
            for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if dependencies_contain(
                    target.get(name).and_then(toml_edit::Item::as_table),
                    package,
                    aliases,
                ) {
                    return true;
                }
            }
        }
    }
    if table
        .get("patch")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|patches| {
            patches
                .iter()
                .filter_map(|(_, item)| item.as_table())
                .any(|registry| dependencies_contain(Some(registry), package, aliases))
        })
    {
        return true;
    }
    if table
        .get("replace")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|replace| {
            replace
                .iter()
                .any(|(key, _)| key.split_once(':').map_or(key, |(name, _)| name) == package)
        })
    {
        return true;
    }
    table
        .get("features")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|features| {
            features.iter().any(|(_, item)| {
                item.as_array().is_some_and(|array| {
                    array
                        .iter()
                        .filter_map(toml_edit::Value::as_str)
                        .any(|edge| {
                            aliases.iter().any(|alias| {
                                edge == alias
                                    || edge == format!("dep:{alias}")
                                    || edge.starts_with(&format!("{alias}/"))
                                    || edge.starts_with(&format!("{alias}?/"))
                            })
                        })
                })
            })
        })
}

#[derive(Debug, Clone)]
enum JsonKind {
    Object(Vec<JsonMember>),
    Array,
    Scalar,
}

#[derive(Debug, Clone)]
struct JsonNode {
    end: usize,
    kind: JsonKind,
}

#[derive(Debug, Clone)]
struct JsonMember {
    key: String,
    key_start: usize,
    value: JsonNode,
    comma_before: Option<usize>,
    comma_after: Option<usize>,
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    fn parse(bytes: &'a [u8]) -> Result<JsonNode, ScrapeError> {
        serde_json::from_slice::<serde_json::Value>(bytes)
            .map_err(|error| fail(format!("invalid JSON rewrite target: {error}")))?;
        let mut parser = Self { bytes, cursor: 0 };
        parser.ws();
        let node = parser.value()?;
        parser.ws();
        if parser.cursor != bytes.len() {
            return Err(fail("JSON contains trailing non-whitespace bytes"));
        }
        Ok(node)
    }

    fn ws(&mut self) {
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.cursor += 1;
        }
    }

    fn value(&mut self) -> Result<JsonNode, ScrapeError> {
        self.ws();
        let kind = match self.bytes.get(self.cursor).copied() {
            Some(b'{') => self.object()?,
            Some(b'[') => self.array()?,
            Some(b'"') => {
                self.string()?;
                JsonKind::Scalar
            }
            Some(b't') => {
                self.literal(b"true")?;
                JsonKind::Scalar
            }
            Some(b'f') => {
                self.literal(b"false")?;
                JsonKind::Scalar
            }
            Some(b'n') => {
                self.literal(b"null")?;
                JsonKind::Scalar
            }
            Some(b'-' | b'0'..=b'9') => {
                self.number()?;
                JsonKind::Scalar
            }
            _ => return Err(fail("invalid JSON value")),
        };
        Ok(JsonNode {
            end: self.cursor,
            kind,
        })
    }

    fn object(&mut self) -> Result<JsonKind, ScrapeError> {
        self.cursor += 1;
        self.ws();
        let mut members = Vec::new();
        let mut prior_comma = None;
        if self.bytes.get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(JsonKind::Object(members));
        }
        loop {
            self.ws();
            let key_start = self.cursor;
            let key_span = self.string()?;
            let key = serde_json::from_slice::<String>(&self.bytes[key_span.0..key_span.1])
                .map_err(|error| fail(format!("invalid JSON object key: {error}")))?;
            self.ws();
            if self.bytes.get(self.cursor) != Some(&b':') {
                return Err(fail("JSON object key has no colon"));
            }
            self.cursor += 1;
            let value = self.value()?;
            self.ws();
            let comma_after = if self.bytes.get(self.cursor) == Some(&b',') {
                let comma = self.cursor;
                self.cursor += 1;
                Some(comma)
            } else {
                None
            };
            members.push(JsonMember {
                key,
                key_start,
                value,
                comma_before: prior_comma,
                comma_after,
            });
            if comma_after.is_none() {
                if self.bytes.get(self.cursor) != Some(&b'}') {
                    return Err(fail(
                        "JSON object member is not followed by comma or close brace",
                    ));
                }
                self.cursor += 1;
                break;
            }
            prior_comma = comma_after;
        }
        Ok(JsonKind::Object(members))
    }

    fn array(&mut self) -> Result<JsonKind, ScrapeError> {
        self.cursor += 1;
        self.ws();
        let mut values = Vec::new();
        if self.bytes.get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(JsonKind::Array);
        }
        loop {
            values.push(self.value()?);
            self.ws();
            match self.bytes.get(self.cursor) {
                Some(b',') => {
                    self.cursor += 1;
                    self.ws();
                }
                Some(b']') => {
                    self.cursor += 1;
                    break;
                }
                _ => {
                    return Err(fail(
                        "JSON array value is not followed by comma or close bracket",
                    ));
                }
            }
        }
        let _ = values;
        Ok(JsonKind::Array)
    }

    fn string(&mut self) -> Result<(usize, usize), ScrapeError> {
        let start = self.cursor;
        if self.bytes.get(self.cursor) != Some(&b'"') {
            return Err(fail("JSON string expected"));
        }
        self.cursor += 1;
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return Ok((start, self.cursor));
                }
                b'\\' => {
                    self.cursor += 2;
                    if self.cursor > self.bytes.len() {
                        return Err(fail("truncated JSON escape"));
                    }
                }
                0..=0x1f => return Err(fail("control byte in JSON string")),
                _ => self.cursor += 1,
            }
        }
        Err(fail("unterminated JSON string"))
    }

    fn literal(&mut self, expected: &[u8]) -> Result<(), ScrapeError> {
        if self.bytes.get(self.cursor..self.cursor + expected.len()) != Some(expected) {
            return Err(fail("invalid JSON literal"));
        }
        self.cursor += expected.len();
        Ok(())
    }

    fn number(&mut self) -> Result<(), ScrapeError> {
        let start = self.cursor;
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E')
        }) {
            self.cursor += 1;
        }
        serde_json::from_slice::<serde_json::Number>(&self.bytes[start..self.cursor])
            .map(|_| ())
            .map_err(|error| fail(format!("invalid JSON number: {error}")))
    }
}

fn json_object_at<'a>(
    node: &'a JsonNode,
    path: &[String],
) -> Result<&'a [JsonMember], ScrapeError> {
    if path.is_empty() {
        return match &node.kind {
            JsonKind::Object(members) => Ok(members),
            _ => Err(fail("selected JSON path is not an object")),
        };
    }
    let JsonKind::Object(members) = &node.kind else {
        return Err(fail(format!(
            "JSON path component `{}` descends through a non-object",
            path[0]
        )));
    };
    let matches = members
        .iter()
        .filter(|member| member.key == path[0])
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(fail(format!(
            "JSON object path component `{}` occurs {} times",
            path[0],
            matches.len()
        )));
    }
    json_object_at(&matches[0].value, &path[1..])
}

fn prepare_json_members(
    before: &[u8],
    object: &[String],
    members: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    let root = JsonParser::parse(before)?;
    let (edits, count, nodes, spans) = json_member_edits(&root, object, members)?;
    let after = apply_edits(before, edits)?;
    let parsed_after = JsonParser::parse(&after)?;
    let residual = json_object_at(&parsed_after, object)?;
    if residual.iter().any(|entry| members.contains(&entry.key)) {
        return Err(fail("JSON registered member remains after preparation"));
    }
    Ok((after, count, nodes, spans))
}

fn json_member_edits(
    root: &JsonNode,
    object: &[String],
    members: &[String],
) -> Result<JsonEditOutput, ScrapeError> {
    let object_members = json_object_at(root, object)?;
    let wanted = members.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    for (index, member) in object_members.iter().enumerate() {
        if wanted.contains(member.key.as_str()) {
            if object_members[..index]
                .iter()
                .any(|earlier| earlier.key == member.key)
            {
                return Err(fail(format!(
                    "duplicate JSON member `{}` is ambiguous",
                    member.key
                )));
            }
            selected.push(index);
        }
    }
    let count = selected.len();
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < selected.len() {
        let first = selected[cursor];
        let mut last = first;
        cursor += 1;
        while cursor < selected.len() && selected[cursor] == last + 1 {
            last = selected[cursor];
            cursor += 1;
        }
        let (start, end) = if last + 1 < object_members.len() {
            (
                object_members[first].key_start,
                object_members[last]
                    .comma_after
                    .ok_or_else(|| fail("JSON removal run has no following comma"))?
                    + 1,
            )
        } else if first > 0 {
            (
                object_members[first]
                    .comma_before
                    .ok_or_else(|| fail("JSON removal run has no preceding comma"))?,
                object_members[last].value.end,
            )
        } else {
            (
                object_members[first].key_start,
                object_members[last].value.end,
            )
        };
        let identities = object_members[first..=last]
            .iter()
            .map(|member| member.key.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let node = format!("json:{}.[{identities}]", object.join("."));
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
        spans.push(ByteSpan {
            start: u64::try_from(start).map_err(|_| fail("JSON span exceeds u64"))?,
            end: u64::try_from(end).map_err(|_| fail("JSON span exceeds u64"))?,
            node: node.clone(),
        });
        nodes.push(node);
    }
    Ok((edits, count, nodes, spans))
}

fn registered_ts_tag(line: &[u8]) -> Option<&'static str> {
    let mut line = trim_ascii(line);
    if let Some(rest) = line.strip_prefix(b"*") {
        line = trim_ascii(rest);
    }
    let rest = line.strip_prefix(b"@")?;
    for tag in ["spec", "verifies", "cell", "scope"] {
        let tag_bytes = tag.as_bytes();
        if rest.starts_with(tag_bytes)
            && rest
                .get(tag_bytes.len())
                .is_some_and(u8::is_ascii_whitespace)
            && valid_registered_reference(trim_ascii(&rest[tag_bytes.len()..]))
        {
            return Some(tag);
        }
    }
    None
}

fn parse_tree(
    bytes: &[u8],
    language: tree_sitter::Language,
    label: &str,
) -> Result<tree_sitter::Tree, ScrapeError> {
    std::str::from_utf8(bytes).map_err(|_| fail(format!("{label} source is not UTF-8")))?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .map_err(|error| fail(format!("cannot load {label} grammar: {error}")))?;
    let tree = parser
        .parse(bytes, None)
        .ok_or_else(|| fail(format!("{label} parser returned no tree")))?;
    if tree.root_node().has_error() || tree.root_node().is_missing() {
        return Err(fail(format!("{label} source contains a parse error")));
    }
    Ok(tree)
}

fn collect_nodes(node: tree_sitter::Node<'_>, kind: &str, out: &mut Vec<(usize, usize)>) {
    if node.kind() == kind {
        out.push((node.start_byte(), node.end_byte()));
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_nodes(cursor.node(), kind, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn syntax_fingerprint(node: tree_sitter::Node<'_>, bytes: &[u8], out: &mut Vec<u8>) {
    if node.kind() == "comment" {
        return;
    }
    out.extend_from_slice(node.kind().as_bytes());
    out.push(b'(');
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            syntax_fingerprint(cursor.node(), bytes, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    } else {
        out.extend_from_slice(&bytes[node.byte_range()]);
    }
    out.push(b')');
}

fn is_attached_jsdoc(root: tree_sitter::Node<'_>, bytes: &[u8], comment_end: usize) -> bool {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    let mut next: Option<tree_sitter::Node<'_>> = None;
    while let Some(node) = stack.pop() {
        if node.is_named()
            && node.kind() != "comment"
            && node.start_byte() >= comment_end
            && next.is_none_or(|known| node.start_byte() < known.start_byte())
        {
            next = Some(node);
        }
        cursor.reset(node);
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    let Some(next) = next else { return false };
    if !bytes[comment_end..next.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        return false;
    }
    let kind = next.kind();
    kind.ends_with("declaration")
        || kind.ends_with("signature")
        || matches!(
            kind,
            "class"
                | "interface_declaration"
                | "enum_declaration"
                | "method_definition"
                | "public_field_definition"
                | "lexical_declaration"
                | "variable_declaration"
                | "export_statement"
        )
}

fn prepare_typescript(before: &[u8], tsx: bool) -> Result<RewriteOutput, ScrapeError> {
    let language: tree_sitter::Language = if tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    };
    let before_tree = parse_tree(before, language.clone(), "TypeScript")?;
    let mut comments = Vec::new();
    collect_nodes(before_tree.root_node(), "comment", &mut comments);
    let mut edits = Vec::new();
    let mut count = 0;
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for (comment_start, comment_end) in comments {
        let comment = &before[comment_start..comment_end];
        if !comment.starts_with(b"/**")
            || !is_attached_jsdoc(before_tree.root_node(), before, comment_end)
        {
            continue;
        }
        let mut line_edits = Vec::new();
        for (local_start, local_content_end, local_end) in line_spans(comment) {
            if let Some(tag) = registered_ts_tag(&comment[local_start..local_content_end]) {
                let start = comment_start + local_start;
                let end = comment_start + local_end;
                line_edits.push((start, end));
                spans.push(ByteSpan {
                    start: u64::try_from(start)
                        .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                    end: u64::try_from(end).map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                    node: format!("typescript-jsdoc:@{tag}@{start}"),
                });
                nodes.push(format!("typescript-jsdoc:@{tag}@{start}"));
                count += 1;
            }
        }
        if line_edits.is_empty() && !comment.contains(&b'\n') && comment.len() >= 5 {
            let body_start = 3;
            let body_end = comment.len() - 2;
            let body = &comment[body_start..body_end];
            for at in 0..body.len() {
                if body[at] != b'@' {
                    continue;
                }
                if let Some(tag) = registered_ts_tag(trim_ascii(&body[at..])) {
                    let start = comment_start + body_start + at;
                    let end = comment_start + body_end;
                    line_edits.push((start, end));
                    spans.push(ByteSpan {
                        start: u64::try_from(start)
                            .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                        end: u64::try_from(end)
                            .map_err(|_| fail("TypeScript tag span exceeds u64"))?,
                        node: format!("typescript-jsdoc:@{tag}@{start}"),
                    });
                    nodes.push(format!("typescript-jsdoc:@{tag}@{start}"));
                    count += 1;
                    break;
                }
            }
        }
        if line_edits.is_empty() {
            continue;
        }
        let local = line_edits
            .iter()
            .map(|(start, end)| Edit {
                start: start - comment_start,
                end: end - comment_start,
                replacement: Vec::new(),
            })
            .collect();
        let rewritten = apply_edits(comment, local)?;
        let body = if rewritten.len() >= 5 {
            trim_ascii(&rewritten[3..rewritten.len() - 2])
        } else {
            &rewritten[..]
        };
        let (start, end, replacement) = if body.is_empty() || body == b"*" {
            // The comment contained metadata only.  Remove its complete line
            // when possible, otherwise remove only the comment span.
            let line_start = before[..comment_start]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |at| at + 1);
            let line_end = before[comment_end..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(before.len(), |at| comment_end + at + 1);
            if trim_ascii(&before[line_start..comment_start]).is_empty()
                && trim_ascii(&before[comment_end..line_end]).is_empty()
            {
                (line_start, line_end, Vec::new())
            } else {
                (comment_start, comment_end, Vec::new())
            }
        } else {
            (comment_start, comment_end, rewritten)
        };
        edits.push(Edit {
            start,
            end,
            replacement,
        });
    }
    let after = apply_edits(before, edits)?;
    let after_tree = parse_tree(&after, language, "rewritten TypeScript")?;
    let mut erased_before = Vec::new();
    let mut parsed_after = Vec::new();
    syntax_fingerprint(before_tree.root_node(), before, &mut erased_before);
    syntax_fingerprint(after_tree.root_node(), &after, &mut parsed_after);
    if erased_before != parsed_after {
        return Err(fail(
            "TypeScript registered-metadata erasure changed the parsed product tree",
        ));
    }
    Ok((after, count, nodes, spans))
}

fn valid_go_directive(line: &[u8]) -> bool {
    let Some(payload) = trim_ascii(line).strip_prefix(b"//spec:") else {
        return false;
    };
    let payload = trim_ascii(payload);
    valid_registered_reference(payload)
}

fn prepare_go_directives(before: &[u8]) -> Result<RewriteOutput, ScrapeError> {
    let language: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();
    let before_tree = parse_tree(before, language.clone(), "Go")?;
    let mut comments = Vec::new();
    collect_nodes(before_tree.root_node(), "comment", &mut comments);
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for (comment_start, comment_end) in comments {
        let (start, end, whole_line) = containing_line(before, comment_start, comment_end);
        if whole_line && valid_go_directive(&before[comment_start..comment_end]) {
            edits.push(Edit {
                start,
                end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(start).map_err(|_| fail("Go directive span exceeds u64"))?,
                end: u64::try_from(end).map_err(|_| fail("Go directive span exceeds u64"))?,
                node: format!("go:comment-group-line@{comment_start}..{comment_end}"),
            });
            nodes.push(format!(
                "go:comment-group-line@{comment_start}..{comment_end}"
            ));
        }
    }
    let count = edits.len();
    let after = apply_edits(before, edits)?;
    let after_tree = parse_tree(&after, language, "rewritten Go")?;
    let mut erased_before = Vec::new();
    let mut parsed_after = Vec::new();
    syntax_fingerprint(before_tree.root_node(), before, &mut erased_before);
    syntax_fingerprint(after_tree.root_node(), &after, &mut parsed_after);
    if erased_before != parsed_after {
        return Err(fail(
            "Go registered-metadata erasure changed the parsed product tree",
        ));
    }
    Ok((after, count, nodes, spans))
}

fn go_module_on_line(line: &str, module: &str, block: Option<&str>) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return false;
    }
    let words = trimmed.split_ascii_whitespace().collect::<Vec<_>>();
    match block {
        Some("require" | "exclude" | "tool") => words.first().copied() == Some(module),
        Some("replace") => words
            .split(|word| *word == "=>")
            .any(|side| side.first().copied() == Some(module)),
        _ => match words.as_slice() {
            [directive, name, ..] if matches!(*directive, "require" | "exclude" | "tool") => {
                *name == module
            }
            ["replace", rest @ ..] => rest
                .split(|word| *word == "=>")
                .any(|side| side.first().copied() == Some(module)),
            _ => false,
        },
    }
}

fn prepare_go_mod(before: &[u8], modules: &[String]) -> Result<RewriteOutput, ScrapeError> {
    std::str::from_utf8(before).map_err(|_| fail("go.mod is not UTF-8"))?;
    let mut block: Option<&str> = None;
    let mut edits = Vec::new();
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    let lines = line_spans(before);
    let mut block_start = None;
    let mut block_rows = 0usize;
    let mut block_removed = 0usize;
    let mut pending_empty_blocks = Vec::new();
    for (start, content_end, end) in &lines {
        let line = std::str::from_utf8(&before[*start..*content_end]).expect("validated UTF-8");
        let trimmed = line.trim();
        if block.is_none() {
            for directive in ["require", "exclude", "replace", "tool"] {
                if trimmed == format!("{directive} (") {
                    block = Some(directive);
                    block_start = Some(*start);
                    block_rows = 0;
                    block_removed = 0;
                    break;
                }
            }
            if block.is_none() {
                for module in modules {
                    if go_module_on_line(line, module, None) {
                        edits.push(Edit {
                            start: *start,
                            end: *end,
                            replacement: Vec::new(),
                        });
                        spans.push(ByteSpan {
                            start: u64::try_from(*start)
                                .map_err(|_| fail("go.mod span exceeds u64"))?,
                            end: u64::try_from(*end)
                                .map_err(|_| fail("go.mod span exceeds u64"))?,
                            node: format!("go.mod:{module}@{start}"),
                        });
                        nodes.push(format!("go.mod:{module}@{start}"));
                    }
                }
            }
            continue;
        }
        if trimmed == ")" {
            if block_rows == block_removed {
                pending_empty_blocks.push((block_start.expect("open block"), *end));
            }
            block = None;
            block_start = None;
            continue;
        }
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            block_rows += 1;
            for module in modules {
                if go_module_on_line(line, module, block) {
                    edits.push(Edit {
                        start: *start,
                        end: *end,
                        replacement: Vec::new(),
                    });
                    spans.push(ByteSpan {
                        start: u64::try_from(*start)
                            .map_err(|_| fail("go.mod span exceeds u64"))?,
                        end: u64::try_from(*end).map_err(|_| fail("go.mod span exceeds u64"))?,
                        node: format!("go.mod:{module}@{start}"),
                    });
                    nodes.push(format!("go.mod:{module}@{start}"));
                    block_removed += 1;
                }
            }
        }
    }
    if block.is_some() {
        return Err(fail("unterminated go.mod directive block"));
    }
    for (start, end) in pending_empty_blocks {
        edits.retain(|edit| edit.end <= start || edit.start >= end);
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
    }
    // A line cannot validly name two requested module identities.  Catching it
    // above as two edits would otherwise look like an internal overlap.
    edits.sort_by_key(|edit| (edit.start, edit.end));
    edits.dedup_by_key(|edit| (edit.start, edit.end));
    let count = nodes.len();
    let after = apply_edits(before, edits)?;
    let text = std::str::from_utf8(&after).expect("validated UTF-8");
    if modules.iter().any(|module| {
        text.lines()
            .any(|line| go_module_on_line(line, module, None))
    }) {
        return Err(fail("requested Go module identity remains in go.mod"));
    }
    Ok((after, count, nodes, spans))
}

fn prepare_go_sum(before: &[u8], modules: &[String]) -> Result<RewriteOutput, ScrapeError> {
    std::str::from_utf8(before).map_err(|_| fail("go.sum is not UTF-8"))?;
    for (start, content_end, _) in line_spans(before) {
        let line = std::str::from_utf8(&before[start..content_end]).expect("validated UTF-8");
        let Some(name) = line.split_ascii_whitespace().next() else {
            continue;
        };
        if modules
            .iter()
            .any(|module| name == module || name == format!("{module}/go.mod"))
        {
            return Err(ScrapeError::blocked(format!(
                "go.sum graph reconciliation for `{name}` requires the sealed manager-native Go resolver; syntax-only checksum deletion is forbidden"
            )));
        }
    }
    Ok((before.to_vec(), 0, Vec::new(), Vec::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustTokenKind {
    Ident(String),
    Punct(u8),
}

#[derive(Debug, Clone)]
struct RustToken {
    kind: RustTokenKind,
    start: usize,
    end: usize,
}

fn rust_tokens(bytes: &[u8]) -> Result<Vec<RustToken>, ScrapeError> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |at| index + at + 1);
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            let mut depth = 1usize;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index..].starts_with(b"/*") {
                    depth += 1;
                    index += 2;
                } else if bytes[index..].starts_with(b"*/") {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            if depth != 0 {
                return Err(fail("unterminated Rust block comment"));
            }
            continue;
        }
        let raw_prefix = if bytes[index] == b'r' {
            Some(1)
        } else if matches!(bytes[index], b'b' | b'c') && bytes.get(index + 1) == Some(&b'r') {
            Some(2)
        } else {
            None
        };
        if let Some(prefix_len) = raw_prefix {
            let mut cursor = index + prefix_len;
            while bytes.get(cursor) == Some(&b'#') {
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'"') {
                let hashes = cursor - index - prefix_len;
                cursor += 1;
                loop {
                    let Some(relative) = bytes[cursor..].iter().position(|byte| *byte == b'"')
                    else {
                        return Err(fail("unterminated Rust raw string"));
                    };
                    cursor += relative + 1;
                    if bytes
                        .get(cursor..cursor + hashes)
                        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
                    {
                        index = cursor + hashes;
                        break;
                    }
                }
                continue;
            }
        }
        if bytes[index] == b'\''
            && bytes
                .get(index + 1)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphabetic())
        {
            let mut after_name = index + 2;
            while bytes
                .get(after_name)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
            {
                after_name += 1;
            }
            if bytes.get(after_name) != Some(&b'\'') {
                tokens.push(RustToken {
                    kind: RustTokenKind::Punct(b'\''),
                    start: index,
                    end: index + 1,
                });
                index += 1;
                continue;
            }
        }
        if matches!(bytes[index], b'"' | b'\'') {
            let delimiter = bytes[index];
            let mut cursor = index + 1;
            let mut escaped = false;
            while cursor < bytes.len() {
                if escaped {
                    escaped = false;
                } else if bytes[cursor] == b'\\' {
                    escaped = true;
                } else if bytes[cursor] == delimiter {
                    cursor += 1;
                    break;
                }
                cursor += 1;
            }
            if cursor > bytes.len() || bytes.get(cursor.saturating_sub(1)) != Some(&delimiter) {
                // Apostrophes that start lifetimes are punctuation, not chars.
                if delimiter == b'\'' {
                    tokens.push(RustToken {
                        kind: RustTokenKind::Punct(delimiter),
                        start: index,
                        end: index + 1,
                    });
                    index += 1;
                    continue;
                }
                return Err(fail("unterminated Rust string literal"));
            }
            index = cursor;
            continue;
        }
        if bytes[index] == b'_' || bytes[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while bytes
                .get(index)
                .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
            {
                index += 1;
            }
            tokens.push(RustToken {
                kind: RustTokenKind::Ident(
                    std::str::from_utf8(&bytes[start..index])
                        .expect("ASCII identifier")
                        .to_owned(),
                ),
                start,
                end: index,
            });
            continue;
        }
        tokens.push(RustToken {
            kind: RustTokenKind::Punct(bytes[index]),
            start: index,
            end: index + 1,
        });
        index += 1;
    }
    Ok(tokens)
}

fn token_ident(token: Option<&RustToken>) -> Option<&str> {
    match token.map(|token| &token.kind) {
        Some(RustTokenKind::Ident(value)) => Some(value),
        _ => None,
    }
}

fn token_punct(token: Option<&RustToken>, punct: u8) -> bool {
    matches!(token.map(|token| &token.kind), Some(RustTokenKind::Punct(value)) if *value == punct)
}

fn matching_delimiter(tokens: &[RustToken], open_at: usize) -> Option<usize> {
    let open = match tokens.get(open_at)?.kind {
        RustTokenKind::Punct(value @ (b'(' | b'[' | b'{')) => value,
        _ => return None,
    };
    let close = match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => unreachable!(),
    };
    let mut stack = vec![close];
    for (index, token) in tokens.iter().enumerate().skip(open_at + 1) {
        let RustTokenKind::Punct(value) = token.kind else {
            continue;
        };
        match value {
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b'{' => stack.push(b'}'),
            value if stack.last() == Some(&value) => {
                stack.pop();
                if stack.is_empty() {
                    return Some(index);
                }
            }
            b')' | b']' | b'}' => return None,
            _ => {}
        }
    }
    None
}

fn containing_line(bytes: &[u8], start: usize, end: usize) -> (usize, usize, bool) {
    let line_start = bytes[..start]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |at| at + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |at| end + at + 1);
    let whole = trim_ascii(&bytes[line_start..start]).is_empty()
        && trim_ascii(&bytes[end..line_end]).is_empty();
    (line_start, line_end, whole)
}

fn rust_import_edits_syn(
    file: &syn::File,
    text: &str,
    bytes: &[u8],
    crate_aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RustImportOutput, ScrapeError> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;

    #[derive(Default)]
    struct Uses(Vec<syn::ItemUse>);
    impl<'ast> syn::visit::Visit<'ast> for Uses {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            self.0.push(node.clone());
        }
    }

    let mut all_uses = Uses::default();
    all_uses.visit_file(file);
    let uses = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item) => Some(item.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let top_level_spans = uses
        .iter()
        .filter_map(|item| rust_span_offsets(text, item.span()))
        .collect::<BTreeSet<_>>();
    for item in &all_uses.0 {
        let relevant = match &item.tree {
            syn::UseTree::Path(path) => crate_aliases.contains(&path.ident.to_string()),
            syn::UseTree::Name(name) => crate_aliases.contains(&name.ident.to_string()),
            syn::UseTree::Rename(rename) => crate_aliases.contains(&rename.ident.to_string()),
            _ => false,
        };
        if relevant {
            let span = rust_span_offsets(text, item.span())
                .ok_or_else(|| fail("syn ItemUse has no stable source span"))?;
            if !top_level_spans.contains(&span) {
                return Err(fail(
                    "non-top-level Specmark import has lexical scope the schema-1 adapter does not resolve",
                ));
            }
        }
    }
    let mut edits = Vec::new();
    let mut imported_forms = BTreeSet::new();
    let mut qualified_aliases = crate_aliases.clone();
    for item in uses {
        let inherited = matches!(item.vis, syn::Visibility::Inherited);
        let (root, tail) = match &item.tree {
            syn::UseTree::Path(path) if crate_aliases.contains(&path.ident.to_string()) => {
                (path.ident.to_string(), Some(path.tree.as_ref()))
            }
            syn::UseTree::Name(name) if crate_aliases.contains(&name.ident.to_string()) => {
                (name.ident.to_string(), None)
            }
            syn::UseTree::Rename(rename) if crate_aliases.contains(&rename.ident.to_string()) => {
                if inherited {
                    qualified_aliases.insert(rename.rename.to_string());
                }
                (rename.ident.to_string(), None)
            }
            _ => continue,
        };
        if !inherited {
            return Err(fail(format!(
                "Rust macro re-export from Specmark alias `{root}` is not erasable"
            )));
        }
        let (item_start, item_end) = rust_span_offsets(text, item.span())
            .ok_or_else(|| fail("syn ItemUse has no stable source span"))?;
        let whole_edit = || {
            let (line_start, line_end, whole) = containing_line(bytes, item_start, item_end);
            Edit {
                start: if whole { line_start } else { item_start },
                end: if whole { line_end } else { item_end },
                replacement: Vec::new(),
            }
        };
        let Some(tail) = tail else {
            qualified_aliases.insert(root);
            edits.push(whole_edit());
            continue;
        };
        match tail {
            syn::UseTree::Name(name) if forms.contains(&name.ident.to_string()) => {
                imported_forms.insert(name.ident.to_string());
                edits.push(whole_edit());
            }
            syn::UseTree::Rename(rename) if forms.contains(&rename.ident.to_string()) => {
                imported_forms.insert(rename.rename.to_string());
                edits.push(whole_edit());
            }
            syn::UseTree::Glob(_) => {
                return Err(fail(format!(
                    "glob import from Specmark alias `{root}` is ambiguous"
                )));
            }
            syn::UseTree::Group(group) => {
                let mut leaves = Vec::new();
                let mut selected = Vec::new();
                for (index, leaf) in group.items.iter().enumerate() {
                    let (start, end) = rust_span_offsets(text, leaf.span())
                        .ok_or_else(|| fail("syn grouped import leaf has no source span"))?;
                    leaves.push((start, end));
                    match leaf {
                        syn::UseTree::Name(name) if name.ident == "self" => {
                            qualified_aliases.insert(root.clone());
                            selected.push(index);
                        }
                        syn::UseTree::Rename(rename) if rename.ident == "self" => {
                            qualified_aliases.insert(rename.rename.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Name(name) if forms.contains(&name.ident.to_string()) => {
                            imported_forms.insert(name.ident.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Rename(rename)
                            if forms.contains(&rename.ident.to_string()) =>
                        {
                            imported_forms.insert(rename.rename.to_string());
                            selected.push(index);
                        }
                        syn::UseTree::Glob(_) => {
                            return Err(fail(format!(
                                "glob import from Specmark alias `{root}` is ambiguous"
                            )));
                        }
                        syn::UseTree::Path(_) | syn::UseTree::Group(_) => {
                            return Err(fail(format!(
                                "nested grouped import from Specmark alias `{root}` is ambiguous"
                            )));
                        }
                        _ => {}
                    }
                }
                if selected.is_empty() {
                    continue;
                }
                if selected.len() == leaves.len() {
                    edits.push(whole_edit());
                    continue;
                }
                let selected = selected.into_iter().collect::<BTreeSet<_>>();
                let mut index = 0;
                while index < leaves.len() {
                    if !selected.contains(&index) {
                        index += 1;
                        continue;
                    }
                    let first = index;
                    let mut last = index;
                    while last + 1 < leaves.len() && selected.contains(&(last + 1)) {
                        last += 1;
                    }
                    let (start, end) = if last + 1 < leaves.len() {
                        let gap = &bytes[leaves[last].1..leaves[last + 1].0];
                        let comma = gap.iter().position(|byte| *byte == b',').ok_or_else(|| {
                            fail("grouped Specmark import leaf has no following comma")
                        })?;
                        if gap[..comma].iter().any(|byte| !byte.is_ascii_whitespace()) {
                            return Err(fail(
                                "grouped Specmark import trivia before comma is not safely owned",
                            ));
                        }
                        (leaves[first].0, leaves[last].1 + comma + 1)
                    } else {
                        let gap = &bytes[leaves[first - 1].1..leaves[first].0];
                        let comma =
                            gap.iter().rposition(|byte| *byte == b',').ok_or_else(|| {
                                fail("grouped Specmark import leaf has no preceding comma")
                            })?;
                        if gap[comma + 1..]
                            .iter()
                            .any(|byte| !byte.is_ascii_whitespace())
                        {
                            return Err(fail(
                                "grouped Specmark import trivia after comma is not safely owned",
                            ));
                        }
                        (leaves[first - 1].1 + comma, leaves[last].1)
                    };
                    if bytes[start..end]
                        .windows(2)
                        .any(|window| window == b"//" || window == b"/*")
                    {
                        return Err(fail(
                            "grouped Specmark import carries comment trivia; exact ownership is ambiguous",
                        ));
                    }
                    edits.push(Edit {
                        start,
                        end,
                        replacement: Vec::new(),
                    });
                    index = last + 1;
                }
            }
            syn::UseTree::Path(_) => {
                return Err(fail(format!(
                    "nested Specmark import from alias `{root}` is ambiguous"
                )));
            }
            _ => {}
        }
    }
    Ok((edits, imported_forms, qualified_aliases))
}

#[cfg(any())]
fn rust_import_edits(
    bytes: &[u8],
    tokens: &[RustToken],
    crate_aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RustImportOutput, ScrapeError> {
    let mut edits = Vec::new();
    let mut imported_forms = BTreeSet::new();
    let mut qualified_aliases = crate_aliases.clone();
    let mut index = 0;
    while index < tokens.len() {
        let is_pub = token_ident(tokens.get(index)) == Some("pub")
            && token_ident(tokens.get(index + 1)) == Some("use");
        let use_at = if is_pub { index + 1 } else { index };
        if token_ident(tokens.get(use_at)) != Some("use") {
            index += 1;
            continue;
        }
        let Some(root) = token_ident(tokens.get(use_at + 1)) else {
            index += 1;
            continue;
        };
        if !crate_aliases.contains(root) {
            index += 1;
            continue;
        }
        if is_pub {
            return Err(fail(format!(
                "Rust macro re-export from Specmark alias `{root}` is not erasable"
            )));
        }
        let semi = (use_at + 2..tokens.len())
            .find(|at| token_punct(tokens.get(*at), b';'))
            .ok_or_else(|| fail("unterminated Rust use item"))?;
        if (use_at + 2..semi).any(|at| token_punct(tokens.get(at), b'*')) {
            return Err(fail(format!(
                "glob import from Specmark alias `{root}` is ambiguous"
            )));
        }
        let start = tokens[index].start;
        let end = tokens[semi].end;
        let (line_start, line_end, whole_line) = containing_line(bytes, start, end);
        let removal = |replacement: Vec<u8>| {
            if replacement.is_empty() && whole_line {
                Edit {
                    start: line_start,
                    end: line_end,
                    replacement,
                }
            } else {
                Edit {
                    start,
                    end,
                    replacement,
                }
            }
        };

        // `use specmark as sm;`
        if token_ident(tokens.get(use_at + 2)) == Some("as") {
            let local = token_ident(tokens.get(use_at + 3))
                .ok_or_else(|| fail("malformed renamed Specmark import"))?;
            qualified_aliases.insert(local.to_owned());
            edits.push(removal(Vec::new()));
            index = semi + 1;
            continue;
        }
        if !token_punct(tokens.get(use_at + 2), b':') || !token_punct(tokens.get(use_at + 3), b':')
        {
            return Err(fail("unsupported Specmark import form"));
        }
        if let Some(form) = token_ident(tokens.get(use_at + 4)) {
            if forms.contains(form) {
                let local = if token_ident(tokens.get(use_at + 5)) == Some("as") {
                    token_ident(tokens.get(use_at + 6)).unwrap_or(form)
                } else {
                    form
                };
                imported_forms.insert(local.to_owned());
                edits.push(removal(Vec::new()));
            }
            index = semi + 1;
            continue;
        }
        if !token_punct(tokens.get(use_at + 4), b'{') {
            index = semi + 1;
            continue;
        }
        let close = matching_delimiter(tokens, use_at + 4)
            .ok_or_else(|| fail("malformed grouped Specmark import"))?;
        let mut remove_ranges = Vec::new();
        let mut cursor = use_at + 5;
        while cursor < close {
            while cursor < close && token_punct(tokens.get(cursor), b',') {
                cursor += 1;
            }
            if cursor >= close {
                break;
            }
            let leaf_start = cursor;
            while cursor < close && !token_punct(tokens.get(cursor), b',') {
                cursor += 1;
            }
            let leaf_end = cursor;
            let Some(form) = token_ident(tokens.get(leaf_start)) else {
                return Err(fail(
                    "nested or malformed grouped Specmark import is ambiguous",
                ));
            };
            if form == "self" {
                let local = if token_ident(tokens.get(leaf_start + 1)) == Some("as") {
                    token_ident(tokens.get(leaf_start + 2)).unwrap_or(root)
                } else {
                    root
                };
                qualified_aliases.insert(local.to_owned());
                remove_ranges.push((leaf_start, leaf_end));
            } else if forms.contains(form) {
                let local = if token_ident(tokens.get(leaf_start + 1)) == Some("as") {
                    token_ident(tokens.get(leaf_start + 2)).unwrap_or(form)
                } else {
                    form
                };
                imported_forms.insert(local.to_owned());
                remove_ranges.push((leaf_start, leaf_end));
            }
        }
        if !remove_ranges.is_empty() {
            let retained = (use_at + 5..close)
                .filter(|at| {
                    !remove_ranges
                        .iter()
                        .any(|(start, end)| at >= start && at < end)
                        && !token_punct(tokens.get(*at), b',')
                })
                .count();
            if retained == 0 {
                edits.push(removal(Vec::new()));
            } else {
                let mut pieces = Vec::new();
                let mut cursor = use_at + 5;
                while cursor < close {
                    while cursor < close && token_punct(tokens.get(cursor), b',') {
                        cursor += 1;
                    }
                    if cursor >= close {
                        break;
                    }
                    let leaf_start = cursor;
                    while cursor < close && !token_punct(tokens.get(cursor), b',') {
                        cursor += 1;
                    }
                    let leaf_end = cursor;
                    if !remove_ranges.contains(&(leaf_start, leaf_end)) {
                        pieces.push(
                            trim_ascii(&bytes[tokens[leaf_start].start..tokens[leaf_end - 1].end])
                                .to_vec(),
                        );
                    }
                }
                let mut replacement = Vec::new();
                replacement.extend_from_slice(&bytes[start..tokens[use_at + 4].end]);
                for (piece_index, piece) in pieces.iter().enumerate() {
                    if piece_index > 0 {
                        replacement.extend_from_slice(b", ");
                    }
                    replacement.extend_from_slice(piece);
                }
                replacement.extend_from_slice(&bytes[tokens[close].start..end]);
                edits.push(removal(replacement));
            }
        }
        index = semi + 1;
    }
    Ok((edits, imported_forms, qualified_aliases))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustAstNode {
    form: String,
    start: usize,
    end: usize,
    attribute: bool,
    top_level: bool,
}

fn rust_span_offsets(text: &str, span: proc_macro2::Span) -> Option<(usize, usize)> {
    fn at(text: &str, location: proc_macro2::LineColumn) -> Option<usize> {
        if location.line == 0 {
            return None;
        }
        let mut offset = 0usize;
        for line in text.split_inclusive('\n').take(location.line - 1) {
            offset = offset.checked_add(line.len())?;
        }
        offset
            .checked_add(location.column)
            .filter(|offset| *offset <= text.len())
    }
    Some((at(text, span.start())?, at(text, span.end())?))
}

fn registered_rust_path(
    path: &syn::Path,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Option<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [form] if imported_forms.contains(form) && forms.contains(form) => Some(form.clone()),
        [root, form] if qualified_aliases.contains(root) && forms.contains(form) => {
            Some(form.clone())
        }
        _ => None,
    }
}

fn rust_ast_nodes(
    file: &syn::File,
    text: &str,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Vec<RustAstNode> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;
    struct Collector<'a> {
        text: &'a str,
        qualified_aliases: &'a BTreeSet<String>,
        imported_forms: &'a BTreeSet<String>,
        forms: &'a BTreeSet<String>,
        nodes: Vec<RustAstNode>,
        depth: usize,
    }
    impl<'ast> syn::visit::Visit<'ast> for Collector<'_> {
        fn visit_item(&mut self, node: &'ast syn::Item) {
            if let syn::Item::Mod(module) = node
                && let Some((_, items)) = &module.content
            {
                for attribute in &module.attrs {
                    self.visit_attribute(attribute);
                }
                self.depth += 1;
                for item in items {
                    self.visit_item(item);
                }
                self.depth -= 1;
                return;
            }
            syn::visit::visit_item(self, node);
        }

        fn visit_block(&mut self, node: &'ast syn::Block) {
            self.depth += 1;
            syn::visit::visit_block(self, node);
            self.depth -= 1;
        }

        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if let Some(form) = registered_rust_path(
                &node.path,
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            ) && let Some((start, end)) = rust_span_offsets(self.text, node.span())
            {
                self.nodes.push(RustAstNode {
                    form,
                    start,
                    end,
                    attribute: false,
                    top_level: self.depth == 0,
                });
            }
            syn::visit::visit_macro(self, node);
        }

        fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
            if let Some(form) = registered_rust_path(
                node.path(),
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            ) && let Some((start, end)) = rust_span_offsets(self.text, node.span())
            {
                self.nodes.push(RustAstNode {
                    form,
                    start,
                    end,
                    attribute: true,
                    top_level: self.depth == 0,
                });
            }
            syn::visit::visit_attribute(self, node);
        }
    }
    let mut collector = Collector {
        text,
        qualified_aliases,
        imported_forms,
        forms,
        nodes: Vec::new(),
        depth: 0,
    };
    collector.visit_file(file);
    collector.nodes
}

fn opaque_rust_macro_ranges(
    file: &syn::File,
    text: &str,
    qualified_aliases: &BTreeSet<String>,
    imported_forms: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Vec<(usize, usize)> {
    use syn::spanned::Spanned as _;
    use syn::visit::Visit as _;
    struct Collector<'a> {
        text: &'a str,
        qualified_aliases: &'a BTreeSet<String>,
        imported_forms: &'a BTreeSet<String>,
        forms: &'a BTreeSet<String>,
        ranges: Vec<(usize, usize)>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Collector<'_> {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if registered_rust_path(
                &node.path,
                self.qualified_aliases,
                self.imported_forms,
                self.forms,
            )
            .is_none()
                && let Some(range) = rust_span_offsets(self.text, node.tokens.span())
            {
                self.ranges.push(range);
            }
            // Macro token trees are opaque syntax. Deliberately do not recurse.
        }
    }
    let mut collector = Collector {
        text,
        qualified_aliases,
        imported_forms,
        forms,
        ranges: Vec::new(),
    };
    collector.visit_file(file);
    collector.ranges
}

fn rust_erasure_oracle(
    file: &syn::File,
    text: &str,
    bytes: &[u8],
    aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<syn::File, ScrapeError> {
    let (mut edits, imported_forms, qualified_aliases) =
        rust_import_edits_syn(file, text, bytes, aliases, forms)?;
    let nodes = rust_ast_nodes(file, text, &qualified_aliases, &imported_forms, forms);
    for node in nodes {
        if !node.top_level {
            return Err(fail(format!(
                "non-top-level Specmark `{}` metadata is outside the erasure oracle's lexical scope",
                node.form
            )));
        }
        let (start, end) = if node.attribute {
            (node.start, node.end)
        } else {
            let mut syntactic_end = node.end;
            while bytes
                .get(syntactic_end)
                .is_some_and(|byte| matches!(*byte, b' ' | b'\t'))
            {
                syntactic_end += 1;
            }
            if bytes.get(syntactic_end) == Some(&b';') {
                syntactic_end += 1;
            }
            let (line_start, line_end, whole) = containing_line(bytes, node.start, syntactic_end);
            if !whole {
                return Err(fail(format!(
                    "Specmark `{}` macro is not a complete metadata-only statement in the erasure oracle",
                    node.form
                )));
            }
            (line_start, line_end)
        };
        edits.push(Edit {
            start,
            end,
            replacement: Vec::new(),
        });
    }
    let erased = apply_edits(bytes, edits)?;
    let erased = std::str::from_utf8(&erased).expect("Rust source was validated UTF-8");
    syn::parse_file(erased).map_err(|error| {
        fail(format!(
            "independent Rust registered-metadata erasure does not parse: {error}"
        ))
    })
}

fn prepare_rust(
    before: &[u8],
    aliases: &BTreeSet<String>,
    forms: &BTreeSet<String>,
) -> Result<RewriteOutput, ScrapeError> {
    let text = std::str::from_utf8(before).map_err(|_| fail("Rust source is not UTF-8"))?;
    let parsed_before = syn::parse_file(text).map_err(|error| {
        fail(format!(
            "Rust source does not parse before erasure: {error}"
        ))
    })?;
    let tokens = rust_tokens(before)?;
    let (mut edits, imported_forms, qualified_aliases) =
        rust_import_edits_syn(&parsed_before, text, before, aliases, forms)?;
    let ast_nodes = rust_ast_nodes(
        &parsed_before,
        text,
        &qualified_aliases,
        &imported_forms,
        forms,
    );
    if let Some(node) = ast_nodes.iter().find(|node| !node.top_level) {
        return Err(fail(format!(
            "non-top-level Specmark `{}` metadata has lexical scope the schema-1 adapter does not resolve",
            node.form
        )));
    }
    let opaque_macros = opaque_rust_macro_ranges(
        &parsed_before,
        text,
        &qualified_aliases,
        &imported_forms,
        forms,
    );
    let import_spans = edits
        .iter()
        .map(|edit| (edit.start, edit.end))
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let Some(name) = token_ident(Some(token)) else {
            continue;
        };
        if qualified_aliases.contains(name)
            && !import_spans
                .iter()
                .any(|(start, end)| token.start >= *start && token.end <= *end)
            && !(token_punct(tokens.get(index + 1), b':')
                && token_punct(tokens.get(index + 2), b':'))
        {
            return Err(fail(format!(
                "Rust Specmark alias `{name}` is shadowed or used as a value"
            )));
        }
    }
    let mut nodes = edits
        .iter()
        .map(|edit| format!("rust:syn-item-use@{}..{}", edit.start, edit.end))
        .collect::<Vec<_>>();
    let mut spans = edits
        .iter()
        .map(|edit| {
            Ok(ByteSpan {
                start: u64::try_from(edit.start)
                    .map_err(|_| fail("Rust import span exceeds u64"))?,
                end: u64::try_from(edit.end).map_err(|_| fail("Rust import span exceeds u64"))?,
                node: format!("rust:syn-item-use@{}..{}", edit.start, edit.end),
            })
        })
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let mut count = 0;
    let mut index = 0;
    while index < tokens.len() {
        if opaque_macros
            .iter()
            .any(|(start, end)| tokens[index].start >= *start && tokens[index].end <= *end)
        {
            index += 1;
            continue;
        }
        if import_spans
            .iter()
            .any(|(start, end)| tokens[index].start >= *start && tokens[index].end <= *end)
        {
            index += 1;
            continue;
        }
        let mut form = None;
        let mut bang_or_attr = None;
        if let Some(root) = token_ident(tokens.get(index)) {
            if qualified_aliases.contains(root)
                && token_punct(tokens.get(index + 1), b':')
                && token_punct(tokens.get(index + 2), b':')
            {
                if let Some(candidate) = token_ident(tokens.get(index + 3))
                    && forms.contains(candidate)
                {
                    form = Some(candidate.to_owned());
                    bang_or_attr = Some(index + 4);
                }
            } else if imported_forms.contains(root) {
                form = Some(root.to_owned());
                bang_or_attr = Some(index + 1);
            }
        }
        let Some(form) = form else {
            index += 1;
            continue;
        };
        let marker_at = bang_or_attr.expect("form has suffix");
        if token_punct(tokens.get(marker_at), b'!') {
            let open_at = marker_at + 1;
            let close_at = matching_delimiter(&tokens, open_at)
                .ok_or_else(|| fail(format!("invalid Specmark `{form}!` macro grammar")))?;
            let arguments = &text[tokens[open_at].end..tokens[close_at].start];
            validate_rust_arguments(&form, arguments, true)?;
            if !ast_nodes.iter().any(|node| {
                !node.attribute
                    && node.form == form
                    && node.start == tokens[index].start
                    && node.end >= tokens[close_at].end
            }) {
                return Err(fail(format!(
                    "Specmark `{form}!` candidate is not a matching syn macro node"
                )));
            }
            let mut end = tokens[close_at].end;
            if token_punct(tokens.get(close_at + 1), b';') {
                end = tokens[close_at + 1].end;
            }
            let (line_start, line_end, whole_line) =
                containing_line(before, tokens[index].start, end);
            if !whole_line {
                return Err(fail(format!(
                    "Specmark `{form}!` is not a complete metadata-only statement"
                )));
            }
            edits.push(Edit {
                start: line_start,
                end: line_end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(line_start).map_err(|_| fail("Rust span exceeds u64"))?,
                end: u64::try_from(line_end).map_err(|_| fail("Rust span exceeds u64"))?,
                node: format!("rust-macro:{form}@{}", tokens[index].start),
            });
            nodes.push(format!("rust-macro:{form}@{}", tokens[index].start));
            count += 1;
            index = close_at + 1;
            continue;
        }
        // Attributes begin with `#[`; the token currently found is their path.
        if index >= 2
            && token_punct(tokens.get(index - 2), b'#')
            && token_punct(tokens.get(index - 1), b'[')
        {
            let close_at = matching_delimiter(&tokens, index - 1)
                .ok_or_else(|| fail(format!("invalid Specmark `{form}` attribute grammar")))?;
            let start = tokens[index - 2].start;
            let end = tokens[close_at].end;
            if !ast_nodes.iter().any(|node| {
                node.attribute && node.form == form && node.start == start && node.end == end
            }) {
                return Err(fail(format!(
                    "Specmark `{form}` candidate is not a matching syn attribute node"
                )));
            }
            let arguments = if token_punct(tokens.get(marker_at), b'(') {
                let argument_close = matching_delimiter(&tokens, marker_at)
                    .ok_or_else(|| fail(format!("invalid Specmark `{form}` attribute grammar")))?;
                &text[tokens[marker_at].end..tokens[argument_close].start]
            } else {
                ""
            };
            validate_rust_arguments(&form, arguments, false)?;
            edits.push(Edit {
                start,
                end,
                replacement: Vec::new(),
            });
            spans.push(ByteSpan {
                start: u64::try_from(start).map_err(|_| fail("Rust span exceeds u64"))?,
                end: u64::try_from(end).map_err(|_| fail("Rust span exceeds u64"))?,
                node: format!("rust-attribute:{form}@{start}"),
            });
            nodes.push(format!("rust-attribute:{form}@{start}"));
            count += 1;
            index = close_at + 1;
            continue;
        }
        index += 1;
    }
    let after = apply_edits(before, edits)?;
    let after_text = std::str::from_utf8(&after).expect("validated UTF-8");
    let parsed_after = syn::parse_file(after_text).map_err(|error| {
        fail(format!(
            "Rust source does not parse after metadata erasure: {error}"
        ))
    })?;
    let erased_before_ast = rust_erasure_oracle(&parsed_before, text, before, aliases, forms)?;
    use quote::ToTokens as _;
    if erased_before_ast.into_token_stream().to_string()
        != parsed_after.into_token_stream().to_string()
    {
        return Err(fail(
            "Rust erase_registered_metadata(parse(before)) differs from parse(after)",
        ));
    }
    let after_tokens = rust_tokens(&after)?;
    for (index, token) in after_tokens.iter().enumerate() {
        let Some(name) = token_ident(Some(token)) else {
            continue;
        };
        if qualified_aliases.contains(name)
            && token_punct(after_tokens.get(index + 1), b':')
            && token_punct(after_tokens.get(index + 2), b':')
        {
            return Err(fail(format!(
                "unresolved Specmark alias reference `{name}::...` remains after erasure"
            )));
        }
        if imported_forms.contains(name)
            && (token_punct(after_tokens.get(index + 1), b'!')
                || (index >= 2
                    && token_punct(after_tokens.get(index - 2), b'#')
                    && token_punct(after_tokens.get(index - 1), b'[')))
        {
            return Err(fail(format!(
                "unresolved imported Specmark form `{name}` remains after erasure"
            )));
        }
    }
    Ok((after, count, nodes, spans))
}

fn json_mentions_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object.contains_key(key) || object.values().any(|value| json_mentions_key(value, key))
        }
        serde_json::Value::Array(array) => array.iter().any(|value| json_mentions_key(value, key)),
        _ => false,
    }
}

fn prepare_node_lock(
    before: &[u8],
    manager: NodeManager,
    packages: &[String],
) -> Result<RewriteOutput, ScrapeError> {
    if manager != NodeManager::Npm {
        return Err(ScrapeError::blocked(format!(
            "selected {:?} lock reconciliation requires a sealed deterministic manager-native plan before execution",
            manager
        )));
    }
    let value: serde_json::Value = serde_json::from_slice(before)
        .map_err(|error| fail(format!("cannot parse npm lockfile: {error}")))?;
    for package in packages {
        if json_mentions_key(&value, package)
            || json_mentions_key(&value, &format!("node_modules/{package}"))
        {
            return Err(ScrapeError::blocked(format!(
                "npm lock graph reconciliation for `{package}` requires the sealed manager-native npm resolver; syntax-only lock deletion is forbidden"
            )));
        }
    }
    Ok((before.to_vec(), 0, Vec::new(), Vec::new()))
}

fn prepare_node_manifest(
    before: &[u8],
    packages: &[String],
    script_paths: &[Vec<String>],
    config_paths: &[Vec<String>],
) -> Result<RewriteOutput, ScrapeError> {
    let root = JsonParser::parse(before)?;
    let mut targets = BTreeMap::<Vec<String>, BTreeSet<String>>::new();
    let mut count = 0;
    let mut nodes = Vec::new();
    let mut spans = Vec::new();
    for table in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        for package in packages {
            targets
                .entry(vec![table.to_owned()])
                .or_default()
                .insert(package.clone());
        }
    }
    for path in script_paths.iter().chain(config_paths) {
        let (member, object) = path
            .split_last()
            .ok_or_else(|| fail("Node script/config member path is empty"))?;
        targets
            .entry(object.to_vec())
            .or_default()
            .insert(member.clone());
    }
    let mut edits = Vec::new();
    for (object, members) in targets {
        let members = members.into_iter().collect::<Vec<_>>();
        match json_object_at(&root, &object) {
            Ok(_) => {
                let (mut local_edits, local_count, mut local_nodes, mut local_spans) =
                    json_member_edits(&root, &object, &members)?;
                count += local_count;
                edits.append(&mut local_edits);
                nodes.append(&mut local_nodes);
                spans.append(&mut local_spans);
            }
            Err(_) if object.len() == 1 => {}
            Err(error) => return Err(error),
        }
    }
    let after = apply_edits(before, edits)?;
    JsonParser::parse(&after)?;
    Ok((after, count, nodes, spans))
}

fn virtual_bytes(
    project: &Project,
    path: &str,
    inventory: &BTreeMap<String, &InventoryEntry>,
    current: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, ScrapeError> {
    if let Some(bytes) = current.get(path) {
        return Ok(bytes.clone());
    }
    let entry = inventory
        .get(path)
        .ok_or_else(|| fail(format!("rewrite target `{path}` is absent from inventory")))?;
    read_candidate(project, entry)
}

fn candidate(
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    _nodes: Vec<String>,
    spans: Vec<ByteSpan>,
) -> Candidate {
    Candidate {
        path,
        before,
        after,
        matches,
        spans,
        native_lock_evidence: None,
    }
}

fn cargo_lock_candidate(
    path: String,
    before: Vec<u8>,
    after: Vec<u8>,
    matches: usize,
    spans: Vec<ByteSpan>,
    evidence: NativeLockEvidence,
) -> Candidate {
    Candidate {
        path,
        before,
        after,
        matches,
        spans,
        native_lock_evidence: Some(evidence),
    }
}

fn prepare_record(id: &str, kind: &str, candidate: Candidate) -> PreparedRewrite {
    let before_sha256 = digest(&candidate.before);
    let after_sha256 = digest(&candidate.after);
    let native_lock_change = candidate
        .native_lock_evidence
        .map(|evidence| NativeLockChange {
            manager: evidence.manager.to_owned(),
            path: candidate.path.clone(),
            before_sha256: before_sha256.clone(),
            after_sha256: after_sha256.clone(),
            before_graph: evidence.before_graph,
            after_graph: evidence.after_graph,
            removed: evidence.removed,
            authorizing_rewrite_id: id.to_owned(),
        });
    PreparedRewrite {
        id: id.to_owned(),
        kind: kind.to_owned(),
        path: candidate.path,
        adapter_epoch: 1,
        spans: candidate.spans,
        before_sha256,
        before_bytes: candidate.before.len() as u64,
        after_bytes: candidate.after,
        after_sha256,
        matches: candidate.matches as u64,
        reason: format!("schema-1 `{kind}` registered metadata/dependency removal"),
        native_lock_change,
    }
}

fn validate_relocations(
    contract: &Contract,
    inventory: &[InventoryEntry],
) -> Result<(), ScrapeError> {
    let paths = inventory
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    for relocation in &contract.relocate {
        let source_exists = paths.contains(relocation.from.as_str())
            || paths
                .iter()
                .any(|path| path.starts_with(&(relocation.from.clone() + "/")));
        if relocation.required && !source_exists {
            return Err(fail(format!(
                "required relocation `{}` source `{}` is absent",
                relocation.id, relocation.from
            )));
        }
        if !source_exists {
            continue;
        }
        if paths.contains(relocation.to.as_str())
            || paths
                .iter()
                .any(|path| path.starts_with(&(relocation.to.clone() + "/")))
        {
            return Err(fail(format!(
                "relocation `{}` destination `{}` already exists",
                relocation.id, relocation.to
            )));
        }
        let mut ancestor = relocation.to.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if inventory
                .iter()
                .any(|entry| entry.path == parent && entry.kind == EntryKind::File)
            {
                return Err(fail(format!(
                    "relocation `{}` destination `{}` has file `{parent}` as an ancestor",
                    relocation.id, relocation.to
                )));
            }
            ancestor = parent;
        }
        if relocation.from == ".git"
            || relocation.from.starts_with(".git/")
            || relocation.to == ".git"
            || relocation.to.starts_with(".git/")
        {
            return Err(fail(format!(
                "relocation `{}` addresses protected .git metadata",
                relocation.id
            )));
        }
        for source_path in paths.iter().filter(|path| {
            *path == &relocation.from.as_str() || path.starts_with(&(relocation.from.clone() + "/"))
        }) {
            let suffix = source_path
                .strip_prefix(&relocation.from)
                .expect("selected relocation descendant has source prefix");
            let mapped = format!("{}{}", relocation.to, suffix);
            if paths.contains(mapped.as_str()) {
                return Err(fail(format!(
                    "relocation `{}` mapped descendant `{mapped}` already exists",
                    relocation.id
                )));
            }
            let mut mapped_ancestor = mapped.as_str();
            while let Some((parent, _)) = mapped_ancestor.rsplit_once('/') {
                if inventory
                    .iter()
                    .any(|entry| entry.path == parent && entry.kind == EntryKind::File)
                {
                    return Err(fail(format!(
                        "relocation `{}` mapped descendant `{mapped}` has file `{parent}` as an ancestor",
                        relocation.id
                    )));
                }
                mapped_ancestor = parent;
            }
            let mut explicitly_kept = false;
            let mut selected_for_deletion = false;
            for rule in &contract.classify {
                if !any_pattern(rule.patterns(), source_path)? {
                    continue;
                }
                explicitly_kept |= matches!(rule, crate::contract::ClassifyRule::Keep { .. });
                selected_for_deletion |= matches!(
                    rule,
                    crate::contract::ClassifyRule::Delete { .. }
                        | crate::contract::ClassifyRule::Generated { .. }
                );
            }
            if selected_for_deletion && !explicitly_kept {
                return Err(fail(format!(
                    "relocation `{}` source member `{source_path}` is not effectively kept",
                    relocation.id
                )));
            }
        }
        for rule in &contract.classify {
            let deleting = matches!(
                rule,
                crate::contract::ClassifyRule::Delete { .. }
                    | crate::contract::ClassifyRule::Generated { .. }
            );
            if deleting
                && (any_pattern(rule.patterns(), &relocation.to)?
                    || any_pattern(rule.patterns(), &(relocation.to.clone() + "/probe"))?)
            {
                return Err(fail(format!(
                    "relocation `{}` destination `{}` remains inside a deletion selector",
                    relocation.id, relocation.to
                )));
            }
        }
    }
    Ok(())
}

fn cargo_path_prefix_present(table: &toml_edit::Table, prefixes: &[String]) -> bool {
    for (key, item) in table.iter() {
        if toml_item_path_prefix_present(Some(key), item, prefixes) {
            return true;
        }
    }
    false
}

fn toml_item_path_prefix_present(
    key: Option<&str>,
    item: &toml_edit::Item,
    prefixes: &[String],
) -> bool {
    if key == Some("path")
        && item
            .as_str()
            .is_some_and(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
    {
        return true;
    }
    if let Some(table) = item.as_table() {
        return cargo_path_prefix_present(table, prefixes);
    }
    if let Some(array) = item.as_array_of_tables() {
        return array
            .iter()
            .any(|table| cargo_path_prefix_present(table, prefixes));
    }
    item.as_value()
        .is_some_and(|value| toml_value_path_prefix_present(key, value, prefixes))
}

fn toml_value_path_prefix_present(
    key: Option<&str>,
    value: &toml_edit::Value,
    prefixes: &[String],
) -> bool {
    if key == Some("path")
        && value
            .as_str()
            .is_some_and(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
    {
        return true;
    }
    if let Some(table) = value.as_inline_table() {
        return table.iter().any(|(child_key, child)| {
            toml_value_path_prefix_present(Some(child_key), child, prefixes)
        });
    }
    value.as_array().is_some_and(|array| {
        array
            .iter()
            .any(|child| toml_value_path_prefix_present(None, child, prefixes))
    })
}

fn assert_language_absent(
    bytes: &[u8],
    language: Language,
    rust_aliases: &BTreeSet<String>,
    tsx: bool,
) -> Result<bool, ScrapeError> {
    match language {
        Language::TypeScript => Ok(prepare_typescript(bytes, tsx)?.1 == 0),
        Language::Go => Ok(prepare_go_directives(bytes)?.1 == 0),
        Language::Rust => {
            if rust_aliases.is_empty() {
                return Ok(true);
            }
            let forms = ["scope", "spec", "verifies", "cell"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            Ok(prepare_rust(bytes, rust_aliases, &forms)?.1 == 0)
        }
    }
}

/// One exact entry in the actual projected final tree. Core constructs this
/// only after dispositions, rewrites and relocation destinations are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedEntry {
    pub path: String,
    pub kind: EntryKind,
    pub bytes: Option<Vec<u8>>,
    pub unix_mode: Option<u32>,
}

/// Validate the actual projected final inventory. No classify rule is used as
/// a proxy for a disposition: callers pass the real kept/rewritten/relocated
/// paths, including directories and mapped relocation destinations.
pub fn validate_projected_final(
    contract: &Contract,
    projected_entries: &[ProjectedEntry],
) -> Result<(), ScrapeError> {
    contract.validate()?;
    let mut by_path = BTreeMap::new();
    for entry in projected_entries {
        crate::glob::PortablePath::parse(&entry.path)?;
        if by_path.insert(entry.path.as_str(), entry).is_some() {
            return Err(fail(format!(
                "projected final inventory contains duplicate path `{}`",
                entry.path
            )));
        }
        if (entry.kind == EntryKind::File) != entry.bytes.is_some() {
            return Err(fail(format!(
                "projected final entry `{}` has bytes inconsistent with its kind",
                entry.path
            )));
        }
    }
    for entry in projected_entries {
        let mut descendant = entry.path.as_str();
        while let Some((parent, _)) = descendant.rsplit_once('/') {
            if by_path
                .get(parent)
                .is_some_and(|ancestor| ancestor.kind == EntryKind::File)
            {
                return Err(fail(format!(
                    "projected final path `{}` has file `{parent}` as an ancestor",
                    entry.path
                )));
            }
            descendant = parent;
        }
    }
    let files = projected_entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();
    let cargo_topology = CargoTopology::build(
        projected_entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File && entry.path.ends_with("Cargo.toml"))
            .map(|entry| {
                (
                    entry.path.as_str(),
                    entry
                        .bytes
                        .as_deref()
                        .expect("file-kind projected entry has bytes"),
                )
            }),
        files
            .iter()
            .filter(|path| path.ends_with("Cargo.lock"))
            .map(String::as_str),
    )?;
    for assertion in &contract.assertions {
        match assertion {
            Assertion::PathsAbsentV1 { id, patterns } => {
                if let Some(path) = selected_paths(&files, patterns, &[])?.first() {
                    return Err(fail(format!(
                        "assertion `{id}` leaves selected path `{path}` in the scraped tree"
                    )));
                }
            }
            Assertion::TextLiteralAbsentV1 {
                id,
                patterns,
                needles,
            } => {
                for path in selected_paths(&files, patterns, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    if let Some(needle) = needles
                        .iter()
                        .find(|needle| find_subslice(bytes, needle.as_bytes()).is_some())
                    {
                        return Err(fail(format!(
                            "assertion `{id}` finds literal `{needle}` in `{path}`"
                        )));
                    }
                }
            }
            Assertion::CargoPathPrefixAbsentV1 {
                id,
                manifests,
                prefixes,
            } => {
                for path in selected_paths(&files, manifests, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let text = std::str::from_utf8(bytes).map_err(|_| {
                        fail(format!(
                            "assertion `{id}` Cargo target `{path}` is not UTF-8"
                        ))
                    })?;
                    let document = text.parse::<toml_edit::DocumentMut>().map_err(|error| {
                        fail(format!("assertion `{id}` cannot parse `{path}`: {error}"))
                    })?;
                    if cargo_path_prefix_present(document.as_table(), prefixes) {
                        return Err(fail(format!(
                            "assertion `{id}` finds a forbidden Cargo path prefix in `{path}`"
                        )));
                    }
                }
            }
            Assertion::LanguageMetadataAbsentV1 {
                id,
                language,
                patterns,
            } => {
                for path in selected_paths(&files, patterns, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let empty_aliases = BTreeSet::new();
                    let rust_aliases = if *language == Language::Rust {
                        contracted_specmark_aliases_for_source(contract, &cargo_topology, &path)?
                    } else {
                        BTreeSet::new()
                    };
                    if !assert_language_absent(
                        bytes,
                        *language,
                        if *language == Language::Rust {
                            &rust_aliases
                        } else {
                            &empty_aliases
                        },
                        path.ends_with(".tsx"),
                    )? {
                        return Err(fail(format!(
                            "assertion `{id}` finds registered language metadata in `{path}`"
                        )));
                    }
                }
            }
            Assertion::DependencyIdentitiesAbsentV1 {
                id,
                manager,
                manifests,
                identities,
            } => {
                for path in selected_paths(&files, manifests, &[])? {
                    let bytes = projected_bytes(&by_path, &path)?;
                    let present = match manager {
                        DependencyManager::Cargo => {
                            let mut found = false;
                            for identity in identities {
                                found |= cargo_document_contains_identity(bytes, identity)?;
                            }
                            found
                        }
                        DependencyManager::Npm
                        | DependencyManager::Pnpm
                        | DependencyManager::Yarn => {
                            let value: serde_json::Value =
                                serde_json::from_slice(bytes).map_err(|error| {
                                    fail(format!("assertion `{id}` cannot parse `{path}`: {error}"))
                                })?;
                            [
                                "dependencies",
                                "devDependencies",
                                "optionalDependencies",
                                "peerDependencies",
                            ]
                            .iter()
                            .filter_map(|table| {
                                value.get(*table).and_then(serde_json::Value::as_object)
                            })
                            .any(|table| {
                                identities
                                    .iter()
                                    .any(|identity| table.contains_key(identity))
                            })
                        }
                        DependencyManager::Go => {
                            let text = std::str::from_utf8(bytes).map_err(|_| {
                                fail(format!("assertion `{id}` target `{path}` is not UTF-8"))
                            })?;
                            identities.iter().any(|identity| {
                                text.lines()
                                    .any(|line| go_module_on_line(line, identity, None))
                            })
                        }
                    };
                    if present {
                        return Err(fail(format!(
                            "assertion `{id}` finds a forbidden dependency identity in `{path}`"
                        )));
                    }
                }
            }
        }
    }
    validate_registered_rewrite_residue(contract, &by_path, &files, &cargo_topology)
}

fn projected_bytes<'a>(
    entries: &BTreeMap<&str, &'a ProjectedEntry>,
    path: &str,
) -> Result<&'a [u8], ScrapeError> {
    entries
        .get(path)
        .and_then(|entry| entry.bytes.as_deref())
        .ok_or_else(|| {
            fail(format!(
                "projected final file `{path}` has no complete bytes"
            ))
        })
}

fn cargo_document_contains_identity(bytes: &[u8], package: &str) -> Result<bool, ScrapeError> {
    let text = std::str::from_utf8(bytes).map_err(|_| fail("Cargo manifest is not UTF-8"))?;
    let document = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| fail(format!("cannot parse Cargo manifest: {error}")))?;
    Ok(cargo_contains_identity(
        document.as_table(),
        package,
        &BTreeSet::new(),
    ))
}

fn contracted_specmark_aliases_for_source(
    contract: &Contract,
    topology: &CargoTopology,
    source: &str,
) -> Result<BTreeSet<String>, ScrapeError> {
    let owner = topology.source_manifest(source)?;
    let matching = contract
        .rewrite
        .iter()
        .filter_map(|rule| {
            let RewriteRule::CargoPackageRemoveV1 {
                manifests,
                package,
                aliases,
                ..
            } = rule
            else {
                return None;
            };
            (package == "core-ai-native-specmark").then_some((manifests, package, aliases))
        })
        .filter_map(
            |row| match cargo_rule_selects_manifest(row.0, &owner.path) {
                Ok(true) => Some(Ok(row)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, ScrapeError>>()?;
    let (_, package, aliases) = match matching.as_slice() {
        [only] => *only,
        [] => return Ok(BTreeSet::new()),
        _ => {
            return Err(fail(format!(
                "Rust source `{source}` has multiple owning Specmark Cargo removal rules"
            )));
        }
    };
    Ok(aliases
        .iter()
        .map(|alias| alias.replace('-', "_"))
        .chain(std::iter::once(package.replace('-', "_")))
        .collect())
}

fn validate_registered_rewrite_residue(
    contract: &Contract,
    entries: &BTreeMap<&str, &ProjectedEntry>,
    files: &BTreeSet<String>,
    cargo_topology: &CargoTopology,
) -> Result<(), ScrapeError> {
    for rule in &contract.rewrite {
        match rule {
            RewriteRule::ManagedBlockRemoveV1 {
                id, paths, marker, ..
            } => {
                let (begin, end) = managed_markers(marker);
                for path in paths {
                    let Some(entry) = entries.get(path.as_str()) else {
                        continue;
                    };
                    let bytes = entry.bytes.as_deref().ok_or_else(|| {
                        fail(format!("projected managed target `{path}` has no bytes"))
                    })?;
                    if find_subslice(bytes, &begin).is_some()
                        || find_subslice(bytes, &end).is_some()
                    {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered managed marker `{marker}` in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::RustSpecmarkStripV1 {
                id,
                patterns,
                exclude,
                forms,
                ..
            } => {
                let forms = forms
                    .iter()
                    .map(|form| {
                        match form {
                            RustForm::Scope => "scope",
                            RustForm::Spec => "spec",
                            RustForm::Verifies => "verifies",
                            RustForm::Cell => "cell",
                        }
                        .to_owned()
                    })
                    .collect::<BTreeSet<_>>();
                for path in selected_paths(files, patterns, exclude)? {
                    if !path.ends_with(".rs") {
                        continue;
                    }
                    let bytes = projected_bytes(entries, &path)?;
                    let rust_aliases =
                        contracted_specmark_aliases_for_source(contract, cargo_topology, &path)?;
                    if prepare_rust(bytes, &rust_aliases, &forms)?.1 != 0 {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered Rust metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                ..
            } => {
                let selected_manifests = selected_paths(files, manifests, &[])?;
                for path in &selected_manifests {
                    if cargo_document_contains_identity(projected_bytes(entries, path)?, package)? {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves Cargo package `{package}` in `{path}`"
                        )));
                    }
                }
                for lockfile in
                    cargo_topology.owned_locks(selected_manifests.iter().map(String::as_str))?
                {
                    match prepare_cargo_lock(projected_bytes(entries, &lockfile)?, package) {
                        Ok(_) => {}
                        Err(ScrapeError::Blocked(message)) => {
                            return Err(fail(format!(
                                "rewrite `{id}` leaves unresolved Cargo.lock identity in `{lockfile}`: {message}"
                            )));
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::NodePackageRemoveV1 {
                id,
                package_json,
                lockfile,
                manager,
                packages,
                script_paths,
                config_paths,
                ..
            } => {
                let Some(_) = entries.get(package_json.as_str()) else {
                    continue;
                };
                let bytes = projected_bytes(entries, package_json)?;
                let (_, count, _, _) =
                    prepare_node_manifest(bytes, packages, script_paths, config_paths)?;
                if count != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered Node manifest identities in `{package_json}`"
                    )));
                }
                let lock_bytes = projected_bytes(entries, lockfile)?;
                match prepare_node_lock(lock_bytes, *manager, packages) {
                    Ok(_) => {}
                    Err(ScrapeError::Blocked(message)) => {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves unresolved Node lock identity in `{lockfile}`: {message}"
                        )));
                    }
                    Err(error) => return Err(error),
                }
            }
            RewriteRule::GoModuleRemoveV1 {
                id,
                go_mod,
                go_sum,
                modules,
                ..
            } => {
                let Some(_) = entries.get(go_mod.as_str()) else {
                    continue;
                };
                if prepare_go_mod(projected_bytes(entries, go_mod)?, modules)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered Go module identities in `{go_mod}`"
                    )));
                }
                if let Some(go_sum) = go_sum
                    && entries.contains_key(go_sum.as_str())
                {
                    match prepare_go_sum(projected_bytes(entries, go_sum)?, modules) {
                        Ok(_) => {}
                        Err(ScrapeError::Blocked(message)) => {
                            return Err(fail(format!(
                                "rewrite `{id}` leaves unresolved Go checksum identity in `{go_sum}`: {message}"
                            )));
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::TomlArrayValuesRemoveV1 {
                id,
                path,
                table,
                key,
                values,
                ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if prepare_toml_array(projected_bytes(entries, path)?, table, key, values)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered TOML values in `{path}`"
                    )));
                }
            }
            RewriteRule::TypeScriptSpecCommentsStripV1 {
                id,
                patterns,
                exclude,
                ..
            } => {
                for path in selected_paths(files, patterns, exclude)? {
                    if prepare_typescript(projected_bytes(entries, &path)?, path.ends_with(".tsx"))?
                        .1
                        != 0
                    {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered TypeScript metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::GoSpecDirectivesStripV1 {
                id,
                patterns,
                exclude,
                ..
            } => {
                for path in selected_paths(files, patterns, exclude)? {
                    if prepare_go_directives(projected_bytes(entries, &path)?)?.1 != 0 {
                        return Err(fail(format!(
                            "rewrite `{id}` leaves registered Go metadata in `{path}`"
                        )));
                    }
                }
            }
            RewriteRule::JsonMemberRemoveV1 {
                id,
                path,
                object,
                members,
                ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if prepare_json_members(projected_bytes(entries, path)?, object, members)?.1 != 0 {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves registered JSON members in `{path}`"
                    )));
                }
            }
            RewriteRule::TextExactReplaceV1 {
                id, path, before, ..
            } => {
                let Some(_) = entries.get(path.as_str()) else {
                    continue;
                };
                if find_subslice(projected_bytes(entries, path)?, before.as_bytes()).is_some() {
                    return Err(fail(format!(
                        "rewrite `{id}` leaves its registered preimage in `{path}`"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Prepare every schema-1 rewrite without mutating `root`.
///
/// Rules are evaluated and returned in fixed adapter-kind, rule-id, byte-sorted
/// path order. A later rule that targets the same file sees the already prepared
/// bytes and therefore has an exact transactional preimage.
pub trait InventoryView {
    fn entries(&self) -> &[InventoryEntry];
}

impl InventoryView for Inventory {
    fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }
}

impl InventoryView for [InventoryEntry] {
    fn entries(&self) -> &[InventoryEntry] {
        self
    }
}

impl InventoryView for Vec<InventoryEntry> {
    fn entries(&self) -> &[InventoryEntry] {
        self
    }
}

pub fn prepare_rewrites<I: InventoryView + ?Sized>(
    project: &Project,
    contract: &Contract,
    inventory: &I,
) -> Result<RewritePreparation, ScrapeError> {
    contract.validate()?;
    let inventory = inventory.entries();
    let inventory_by_path = inventory
        .iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    validate_relocations(contract, inventory)?;
    let files = inventory_files(inventory);
    let mut current = BTreeMap::<String, Vec<u8>>::new();
    let mut records = Vec::new();
    let mut blockers: Vec<Blocker> = Vec::new();
    for entry in inventory
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
    {
        let bytes = read_candidate(project, entry)?;
        let observed_digest = digest(&bytes);
        if entry.sha256.as_deref() != Some(observed_digest.as_str()) {
            return Err(fail(format!(
                "rewrite preparation observed concurrent change at `{}`",
                entry.path
            )));
        }
        current.insert(entry.path.clone(), bytes);
    }
    let mut rules = contract.rewrite.iter().collect::<Vec<_>>();
    rules.sort_by_key(|rule| {
        let priority = match rule {
            RewriteRule::ManagedBlockRemoveV1 { .. } => 0,
            RewriteRule::RustSpecmarkStripV1 { .. }
            | RewriteRule::TypeScriptSpecCommentsStripV1 { .. }
            | RewriteRule::GoSpecDirectivesStripV1 { .. } => 1,
            RewriteRule::CargoPackageRemoveV1 { .. }
            | RewriteRule::TomlArrayValuesRemoveV1 { .. }
            | RewriteRule::JsonMemberRemoveV1 { .. } => 2,
            RewriteRule::NodePackageRemoveV1 { .. } | RewriteRule::GoModuleRemoveV1 { .. } => 3,
            RewriteRule::TextExactReplaceV1 { .. } => 4,
        };
        (priority, rule.id())
    });

    for rule in rules {
        let mut candidates = Vec::new();
        match rule {
            RewriteRule::ManagedBlockRemoveV1 {
                id,
                paths,
                marker,
                matches,
            } => {
                for path in paths {
                    if !files.contains(path) {
                        if *matches == PerFileMatches::ExactlyOnePerFile {
                            return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                        }
                        continue;
                    }
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    let expected = contract
                        .baseline
                        .iter()
                        .find(|baseline| baseline.path == *path)
                        .ok_or_else(|| {
                            ScrapeError::blocked(format!(
                                "managed block `{id}` at `{path}` requires an exact whole-target baseline digest proving provider ownership"
                            ))
                        })?;
                    if expected.sha256 != digest(&before) {
                        return Err(ScrapeError::blocked(format!(
                            "managed block `{id}` at `{path}` differs from its ownership baseline"
                        )));
                    }
                    let (after, count, nodes, spans) = prepare_managed(&before, marker)?;
                    check_per_file_cardinality(id, *matches, path, count)?;
                    candidates.push(candidate(
                        path.clone(),
                        before.clone(),
                        after,
                        count,
                        nodes,
                        spans,
                    ));
                }
            }
            RewriteRule::RustSpecmarkStripV1 {
                id,
                patterns,
                exclude,
                forms,
                matches,
            } => {
                let topology = cargo_topology_from_current(&current, &files)?;
                let forms = forms
                    .iter()
                    .map(|form| {
                        match form {
                            RustForm::Scope => "scope",
                            RustForm::Spec => "spec",
                            RustForm::Verifies => "verifies",
                            RustForm::Cell => "cell",
                        }
                        .to_owned()
                    })
                    .collect::<BTreeSet<_>>();
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                let mut authority_blocked = false;
                for path in paths {
                    let specmark_aliases =
                        match observed_specmark_aliases_for_source(contract, &topology, &path) {
                            Ok(aliases) => aliases,
                            Err(ScrapeError::Blocked(message)) => {
                                blockers.push(
                                    Blocker::new("rust-cargo-ownership-unresolved", message)
                                        .at(&path)
                                        .rule(id),
                                );
                                authority_blocked = true;
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) =
                        prepare_rust(&before, &specmark_aliases, &forms)?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                if !authority_blocked {
                    check_set_cardinality(id, *matches, total)?;
                }
            }
            RewriteRule::CargoPackageRemoveV1 {
                id,
                manifests,
                package,
                aliases,
                matches,
            } => {
                let topology = cargo_topology_from_current(&current, &files)?;
                let paths = selected_paths(&files, manifests, &[])?;
                let mut owned_locks = BTreeSet::new();
                let mut total = 0;
                let mut authority_blocked = false;
                for path in &paths {
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    let workspace_aliases = match topology.workspace_aliases_for(path) {
                        Ok(aliases) => aliases,
                        Err(ScrapeError::Blocked(message)) => {
                            blockers.push(
                                Blocker::new("cargo-ownership-ambiguous", message)
                                    .at(path)
                                    .rule(id),
                            );
                            authority_blocked = true;
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                    if let Some(lock) = topology.owned_lock_for(path)? {
                        owned_locks.insert(lock);
                    }
                    let (after, count, nodes, _, spans) =
                        prepare_cargo_resolved(&before, package, aliases, &workspace_aliases)?;
                    total += count;
                    candidates.push(candidate(
                        path.clone(),
                        before.clone(),
                        after,
                        count,
                        nodes,
                        spans,
                    ));
                }
                if !authority_blocked {
                    check_set_cardinality(id, *matches, total)?;
                }
                for lockfile in owned_locks {
                    let before = virtual_bytes(project, &lockfile, &inventory_by_path, &current)?;
                    match prepare_cargo_lock(&before, package) {
                        Ok(((after, lock_count, _nodes, spans), Some(evidence))) => {
                            candidates.push(cargo_lock_candidate(
                                lockfile.clone(),
                                before,
                                after,
                                lock_count,
                                spans,
                                evidence,
                            ));
                        }
                        Ok(((after, lock_count, nodes, spans), None)) => candidates.push(
                            candidate(lockfile.clone(), before, after, lock_count, nodes, spans),
                        ),
                        Err(ScrapeError::Blocked(message)) => blockers.push(
                            Blocker::new("native-lock-reconciliation-required", message)
                                .at(&lockfile)
                                .rule(id),
                        ),
                        Err(error) => return Err(error),
                    }
                }
            }
            RewriteRule::TomlArrayValuesRemoveV1 {
                id,
                path,
                table,
                key,
                values,
                matches,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_toml_array(&before, table, key, values)?;
                check_set_cardinality(id, *matches, count)?;
                candidates.push(candidate(
                    path.clone(),
                    before.clone(),
                    after,
                    count,
                    nodes,
                    spans,
                ));
            }
            RewriteRule::TypeScriptSpecCommentsStripV1 {
                id,
                patterns,
                exclude,
                matches,
            } => {
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                for path in paths {
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) =
                        prepare_typescript(&before, path.ends_with(".tsx"))?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::GoSpecDirectivesStripV1 {
                id,
                patterns,
                exclude,
                matches,
            } => {
                let paths = selected_paths(&files, patterns, exclude)?;
                let mut total = 0;
                for path in paths {
                    let before = virtual_bytes(project, &path, &inventory_by_path, &current)?;
                    let (after, count, nodes, spans) = prepare_go_directives(&before)?;
                    total += count;
                    candidates.push(candidate(path, before, after, count, nodes, spans));
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::JsonMemberRemoveV1 {
                id,
                path,
                object,
                members,
                matches,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!("rewrite `{id}` target `{path}` is absent")));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_json_members(&before, object, members)?;
                check_set_cardinality(id, *matches, count)?;
                candidates.push(candidate(path.clone(), before, after, count, nodes, spans));
            }
            RewriteRule::NodePackageRemoveV1 {
                id,
                package_json,
                lockfile,
                manager,
                packages,
                script_paths,
                config_paths,
                matches,
            } => {
                if !files.contains(package_json) || !files.contains(lockfile) {
                    return Err(fail(format!(
                        "rewrite `{id}` requires both package manifest and selected lockfile"
                    )));
                }
                let package_before =
                    virtual_bytes(project, package_json, &inventory_by_path, &current)?;
                let (package_after, package_count, package_nodes, package_spans) =
                    prepare_node_manifest(&package_before, packages, script_paths, config_paths)?;
                let lock_before = virtual_bytes(project, lockfile, &inventory_by_path, &current)?;
                let (lock_after, lock_count, lock_nodes, lock_spans) =
                    match prepare_node_lock(&lock_before, *manager, packages) {
                        Ok(output) => output,
                        Err(ScrapeError::Blocked(message)) => {
                            blockers.push(
                                Blocker::new("native-lock-reconciliation-required", message)
                                    .at(lockfile)
                                    .rule(id),
                            );
                            (lock_before.clone(), 0, Vec::new(), Vec::new())
                        }
                        Err(error) => return Err(error),
                    };
                let total = package_count + lock_count;
                check_set_cardinality(id, *matches, total)?;
                candidates.push(candidate(
                    package_json.clone(),
                    package_before,
                    package_after,
                    package_count,
                    package_nodes,
                    package_spans,
                ));
                candidates.push(candidate(
                    lockfile.clone(),
                    lock_before,
                    lock_after,
                    lock_count,
                    lock_nodes,
                    lock_spans,
                ));
            }
            RewriteRule::GoModuleRemoveV1 {
                id,
                go_mod,
                go_sum,
                modules,
                matches,
            } => {
                if !files.contains(go_mod) {
                    return Err(fail(format!("rewrite `{id}` target `{go_mod}` is absent")));
                }
                let before = virtual_bytes(project, go_mod, &inventory_by_path, &current)?;
                let (after, count, nodes, spans) = prepare_go_mod(&before, modules)?;
                let mut total = count;
                candidates.push(candidate(
                    go_mod.clone(),
                    before,
                    after,
                    count,
                    nodes,
                    spans,
                ));
                if let Some(path) = go_sum
                    && files.contains(path)
                {
                    let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                    match prepare_go_sum(&before, modules) {
                        Ok((after, count, nodes, spans)) => {
                            total += count;
                            candidates.push(candidate(
                                path.clone(),
                                before,
                                after,
                                count,
                                nodes,
                                spans,
                            ));
                        }
                        Err(ScrapeError::Blocked(message)) => blockers.push(
                            Blocker::new("native-lock-reconciliation-required", message)
                                .at(path)
                                .rule(id),
                        ),
                        Err(error) => return Err(error),
                    }
                }
                check_set_cardinality(id, *matches, total)?;
            }
            RewriteRule::TextExactReplaceV1 {
                id,
                path,
                sha256,
                before: needle,
                after: replacement,
                occurrences,
            } => {
                if !files.contains(path) {
                    return Err(fail(format!(
                        "exact-text rewrite target `{path}` is absent"
                    )));
                }
                let before = virtual_bytes(project, path, &inventory_by_path, &current)?;
                let occurrences = usize::try_from(*occurrences).map_err(|_| {
                    fail(format!(
                        "exact-text rewrite `{id}` occurrence count exceeds this platform's address space"
                    ))
                })?;
                let (after, count, nodes, spans) =
                    prepare_exact_text(&before, sha256, needle, replacement, occurrences)?;
                candidates.push(candidate(
                    path.clone(),
                    before.clone(),
                    after,
                    count,
                    nodes,
                    spans,
                ));
            }
        }
        for candidate in candidates {
            if candidate.before == candidate.after {
                continue;
            }
            current.insert(candidate.path.clone(), candidate.after.clone());
            records.push(prepare_record(rule.id(), rule.kind_name(), candidate));
        }
    }

    blockers.sort_by(|left, right| {
        (&left.code, &left.path, &left.rule_id, &left.message).cmp(&(
            &right.code,
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    blockers.dedup();
    Ok(RewritePreparation {
        rewrites: records,
        blockers,
    })
}

#[cfg(test)]
mod tests;
