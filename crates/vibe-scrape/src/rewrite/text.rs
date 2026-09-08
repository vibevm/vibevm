use super::*;

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn fail(message: impl Into<String>) -> ScrapeError {
    ScrapeError::Rewrite(message.into())
}

pub(super) fn apply_edits(before: &[u8], mut edits: Vec<Edit>) -> Result<Vec<u8>, ScrapeError> {
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

pub(super) fn check_set_cardinality(
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

pub(super) fn check_per_file_cardinality(
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

pub(super) fn inventory_files(inventory: &[InventoryEntry]) -> BTreeSet<String> {
    inventory
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
        .map(|entry| entry.path.clone())
        .collect()
}

pub(super) fn read_candidate(
    project: &Project,
    entry: &InventoryEntry,
) -> Result<Vec<u8>, ScrapeError> {
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

pub(super) fn selected_paths(
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

pub(super) fn any_pattern(patterns: &[String], path: &str) -> Result<bool, ScrapeError> {
    for pattern in patterns {
        if Glob::parse(pattern)?.matches(path) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn line_spans(bytes: &[u8]) -> Vec<(usize, usize, usize)> {
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

pub(super) fn trim_ascii(bytes: &[u8]) -> &[u8] {
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

pub(super) fn valid_registered_reference(bytes: &[u8]) -> bool {
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

pub(super) fn validate_rust_arguments(
    form: &str,
    arguments: &str,
    is_macro: bool,
) -> Result<(), ScrapeError> {
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

pub(super) fn managed_markers(marker: &str) -> (Vec<u8>, Vec<u8>) {
    (
        format!("<{marker}>").into_bytes(),
        format!("</{marker}>").into_bytes(),
    )
}

pub(super) fn prepare_managed(before: &[u8], marker: &str) -> Result<RewriteOutput, ScrapeError> {
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

pub(super) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
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

pub(super) fn prepare_exact_text(
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

pub(super) fn prepare_toml_array(
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

pub(super) fn unique_subslice_span(
    haystack: &[u8],
    needle: &[u8],
) -> Option<std::ops::Range<usize>> {
    let start = find_subslice(haystack, needle)?;
    if find_subslice(&haystack[start + needle.len()..], needle).is_some() {
        return None;
    }
    Some(start..start + needle.len())
}

pub(super) fn dependency_identity<'a>(key: &'a str, item: &'a toml_edit::Item) -> Option<&'a str> {
    item.get("package")
        .and_then(toml_edit::Item::as_str)
        .or(Some(key))
}

pub(super) fn is_workspace_inherited(item: &toml_edit::Item) -> bool {
    item.get("workspace")
        .and_then(toml_edit::Item::as_bool)
        .unwrap_or(false)
}
