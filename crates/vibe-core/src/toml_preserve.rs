//! Comment-preserving rewrites of the user's TOML configuration files
//! (`~/.vibe/config.toml`, `~/.vibe/registry.toml`,
//! `<project>/.vibe/registry.local.toml`).
//!
//! A command that edits one of these files changes a value in memory and
//! saves the whole typed struct. Re-serialising that struct would drop
//! every comment the person wrote and reformat every table they did not
//! touch. Instead the existing file is opened as a [`toml_edit::DocumentMut`]
//! and only the keys whose *value* changed are rewritten: an unchanged
//! table, array element or key keeps its bytes, comments and position; a
//! removed one is removed; a changed one is replaced (recursing into a
//! table so its unchanged keys keep their comments); a new one is appended.
//!
//! The result is checked before it is returned: it must parse back into
//! exactly the value the caller asked to save. When it does not — or when
//! the existing file cannot be read as both TOML and the typed struct —
//! the plain serialisation is written instead, so preservation can only
//! ever cost formatting, never a value.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers");

use serde::Serialize;
use serde::de::DeserializeOwned;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, TableLike};

/// Render `value` for writing over a file whose current text is
/// `existing`, keeping that text's comments and the formatting of every
/// part whose value did not change.
///
/// ```
/// use std::collections::BTreeMap;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct Config {
///     name: String,
///     #[serde(default)]
///     extra: BTreeMap<String, String>,
/// }
///
/// let existing = "# my config\nname = 'old' # keep me?\n\n[extra]\n# untouched\nk = 'v'\n";
/// let mut extra = BTreeMap::new();
/// extra.insert("k".to_owned(), "v".to_owned());
/// let value = Config { name: "new".into(), extra };
/// let rendered = vibe_core::toml_preserve::render_preserving(Some(existing), &value).unwrap();
/// assert!(rendered.starts_with("# my config\n"));
/// assert!(rendered.contains("[extra]\n# untouched\nk = 'v'\n"));
/// assert!(rendered.contains("name = \"new\" # keep me?"));
/// ```
pub fn render_preserving<T>(existing: Option<&str>, value: &T) -> Result<String, toml::ser::Error>
where
    T: Serialize + DeserializeOwned,
{
    let rendered = toml::to_string_pretty(value)?;
    let Some(existing) = existing else {
        return Ok(rendered);
    };
    let wanted = as_table(value)?;
    let (Ok(mut document), Ok(old), Ok(fresh)) = (
        existing.parse::<DocumentMut>(),
        toml::from_str::<T>(existing),
        rendered.parse::<DocumentMut>(),
    ) else {
        return Ok(rendered);
    };
    let old = as_table(&old)?;
    patch_table(document.as_table_mut(), &old, &wanted, fresh.as_table());
    let patched = document.to_string();
    // The patched text must say exactly what the caller asked to save.
    let faithful = toml::from_str::<T>(&patched)
        .ok()
        .and_then(|reparsed| as_table(&reparsed).ok())
        .is_some_and(|reparsed| reparsed == wanted);
    Ok(if faithful { patched } else { rendered })
}

fn as_table<T: Serialize>(value: &T) -> Result<toml::Table, toml::ser::Error> {
    match toml::Value::try_from(value)? {
        toml::Value::Table(table) => Ok(table),
        // A config struct always serialises as a table; anything else has
        // no keys to preserve, and the caller falls back to the rendering.
        _ => Ok(toml::Table::new()),
    }
}

/// Bring `document` from `old` to `new`, taking the spelling of every
/// changed part from `fresh` (the plain rendering of `new`).
fn patch_table(
    document: &mut dyn TableLike,
    old: &toml::Table,
    new: &toml::Table,
    fresh: &dyn TableLike,
) {
    let keys: Vec<&String> = old
        .keys()
        .chain(new.keys().filter(|key| !old.contains_key(*key)))
        .collect();
    for key in keys {
        match (old.get(key), new.get(key)) {
            (Some(before), Some(after)) if before == after => {}
            (_, None) => {
                document.remove(key);
            }
            (Some(toml::Value::Table(before)), Some(toml::Value::Table(after))) => {
                let Some(fresh_item) = fresh.get(key) else {
                    continue;
                };
                match (
                    document.get_mut(key).and_then(Item::as_table_like_mut),
                    fresh_item.as_table_like(),
                ) {
                    (Some(table), Some(fresh_table)) => {
                        patch_table(table, before, after, fresh_table);
                    }
                    _ => replace(document, key, fresh_item),
                }
            }
            (Some(toml::Value::Array(before)), Some(toml::Value::Array(after))) => {
                let Some(fresh_item) = fresh.get(key) else {
                    continue;
                };
                match (
                    document.get_mut(key).and_then(Item::as_array_of_tables_mut),
                    fresh_item.as_array_of_tables(),
                ) {
                    (Some(tables), Some(fresh_tables)) => {
                        patch_array_of_tables(tables, before, after, fresh_tables);
                    }
                    _ => replace(document, key, fresh_item),
                }
            }
            (_, Some(_)) => {
                if let Some(fresh_item) = fresh.get(key) {
                    replace(document, key, fresh_item);
                }
            }
        }
    }
}

/// `[[x]]` elements: an element equal to one already on disk keeps that
/// element's bytes; with the length unchanged, a changed element is patched
/// in place; anything else is a fresh table. Removed elements disappear.
fn patch_array_of_tables(
    document: &mut ArrayOfTables,
    old: &[toml::Value],
    new: &[toml::Value],
    fresh: &ArrayOfTables,
) {
    let mut used = vec![false; old.len()];
    let mut matched: Vec<Option<usize>> = new
        .iter()
        .map(|wanted| {
            let found = (0..old.len()).find(|&index| !used[index] && old[index] == *wanted);
            if let Some(index) = found {
                used[index] = true;
            }
            found
        })
        .collect();
    let in_place = old.len() == new.len();
    let mut rebuilt = ArrayOfTables::new();
    for (index, wanted) in new.iter().enumerate() {
        let existing = document.get(index);
        let table = match matched[index].take() {
            Some(from) => document.get(from).cloned(),
            None => match (in_place && !used[index], old.get(index), wanted, existing) {
                (
                    true,
                    Some(toml::Value::Table(before)),
                    toml::Value::Table(after),
                    Some(existing),
                ) => fresh.get(index).map(|fresh_table| {
                    used[index] = true;
                    let mut table = existing.clone();
                    patch_table(&mut table, before, after, fresh_table);
                    table
                }),
                _ => None,
            },
        }
        .or_else(|| fresh.get(index).map(detached_table));
        if let Some(table) = table {
            rebuilt.push(table);
        }
    }
    *document = rebuilt;
}

/// Replace one key's item, keeping a value's surrounding comment, an
/// inline value inline and a table at the place in the file it already had.
fn replace(document: &mut dyn TableLike, key: &str, fresh: &Item) {
    let mut item = detached(fresh);
    match (document.get(key), &mut item) {
        (Some(Item::Value(existing)), Item::Value(value)) => {
            *value.decor_mut() = existing.decor().clone();
        }
        (Some(Item::Value(_)), Item::Table(_)) => {
            if let Ok(value) = item.clone().into_value() {
                item = Item::Value(value);
            }
        }
        (Some(Item::Table(existing)), Item::Table(table)) => {
            if let Some(position) = existing.position() {
                table.set_position(position);
            }
            *table.decor_mut() = existing.decor().clone();
        }
        _ => {}
    }
    // Assign in place: inserting would mint a fresh key and drop the
    // comments that sit on the key line (and above it).
    match document.get_mut(key) {
        Some(slot) => *slot = item,
        None => {
            document.insert(key, item);
        }
    }
}

/// A copy of `item` with no document positions, so it is laid out where
/// it is inserted rather than where it stood in the rendering it came from.
fn detached(item: &Item) -> Item {
    match item {
        Item::Table(table) => Item::Table(detached_table(table)),
        Item::ArrayOfTables(tables) => {
            let mut copy = ArrayOfTables::new();
            for table in tables.iter() {
                copy.push(detached_table(table));
            }
            Item::ArrayOfTables(copy)
        }
        other => other.clone(),
    }
}

fn detached_table(table: &Table) -> Table {
    let mut copy = Table::new();
    copy.set_implicit(table.is_implicit());
    for (key, item) in table.iter() {
        copy.insert(key, detached(item));
    }
    copy
}

#[cfg(test)]
#[path = "toml_preserve/tests.rs"]
mod tests;
