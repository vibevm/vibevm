use super::cargo_lock::cargo_contains_identity;
use super::text::*;
use super::*;

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
pub(super) fn prepare_cargo(
    before: &[u8],
    package: &str,
    aliases: &[String],
) -> Result<CargoOutput, ScrapeError> {
    prepare_cargo_resolved(before, package, aliases, &BTreeMap::new())
}

pub(super) fn prepare_cargo_resolved(
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
