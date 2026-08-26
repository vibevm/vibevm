//! Recursive TOML decoration transfer for manifest rewrites.
//!
//! The freshly serialised document owns structure and values. This cell only
//! transfers comments and whitespace between matching keys and matching TOML
//! shapes, so a structural edit can never inherit decoration from a removed or
//! differently-typed value.
//!
//! One shape difference is *not* a structural edit: the serialiser
//! canonicalises an authored inline table (`config = { … }`) or inline array
//! of inline tables (`inputs = [{ … }]`) into a header table / array of
//! tables. Same value, same row, different spelling — imposed by the writer,
//! not asked for by the operator. The `key = …` line their comment sat on is
//! gone, so the comment moves to the header line that replaced it. Treating
//! that as a type mismatch would silently delete authored text on the first
//! `vibe install` after someone wrote the manifest the way the docs show it.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use toml_edit::{Array, ArrayOfTables, InlineTable, Item, RawString, Table, Value};

/// Copy decoration below a table whose own root decor is managed by its caller.
pub(super) fn copy_matching_table_items(existing: &Table, new: &mut Table) {
    let keys = new
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    for key in keys {
        let Some(existing_item) = existing.get(&key) else {
            continue;
        };
        let Some(new_item) = new.get(&key) else {
            continue;
        };
        if !item_shapes_match(existing_item, new_item) {
            continue;
        }
        // An authored inline table (`config = { … }`) or inline array of
        // inline tables (`inputs = [{ … }]`) is *canonicalised* by the
        // serializer into a header table / array of tables. The value is the
        // same and the row is the same; only the spelling changed. The
        // operator's `key = …` line is gone, so its comments have to move
        // onto the header that replaced it — dropping them because the shape
        // no longer matches would be a silent loss of authored text.
        let canonicalised = is_canonicalised(existing_item, new_item);
        let key_decor = existing.key(&key).map(|key| key.leaf_decor().clone());

        if !canonicalised
            && let Some(decor) = key_decor.clone()
            && let Some(mut new_key) = new.key_mut(&key)
        {
            *new_key.leaf_decor_mut() = decor;
        }
        if let Some(new_item) = new.get_mut(&key) {
            copy_item_decor(existing_item, new_item);
            if canonicalised
                && let Some(prefix) = key_decor.as_ref().and_then(|decor| decor.prefix()).cloned()
            {
                adopt_header_prefix(new_item, prefix);
            }
        }
    }
}

fn copy_item_decor(existing: &Item, new: &mut Item) {
    match (existing, new) {
        (Item::Table(existing), Item::Table(new)) => copy_table_decor(existing, new),
        (Item::ArrayOfTables(existing), Item::ArrayOfTables(new)) => {
            copy_array_of_tables_decor(existing, new);
        }
        (Item::Value(existing), Item::Value(new)) => copy_value_decor(existing, new),
        (Item::Value(Value::InlineTable(existing)), Item::Table(new)) => {
            adopt_inline_decor(existing, new);
        }
        (Item::Value(Value::Array(existing)), Item::ArrayOfTables(new)) => {
            for index in 0..existing.len().min(new.len()) {
                if let (Some(Value::InlineTable(existing)), Some(new)) =
                    (existing.get(index), new.get_mut(index))
                {
                    adopt_inline_decor(existing, new);
                }
            }
            adopt_array_suffix(existing, new);
        }
        _ => {}
    }
}

/// `inputs = [{ … }] # KEEP` — the note after the closing bracket belongs to
/// the whole array, not to any one row, and the bracket it followed is gone.
/// Its deterministic new owner is the **last** header the array expanded into:
/// that is the line the note still sits after in the rewritten file.
///
/// The last row may already have carried a note of its own
/// (`{ … }, # KEEP-ROW`). Both are real and both are the operator's, so they
/// are merged onto that header in source order — row note first, array note
/// second — rather than one silently displacing the other. An array note the
/// header already ends with is not appended twice, so a second write is a
/// fixpoint.
fn adopt_array_suffix(existing: &Array, new: &mut ArrayOfTables) {
    // Two distinct notes can sit at the tail of an authored array, and
    // toml_edit stores them in two different places:
    //
    //   inputs = [
    //     { path = "Cargo.toml" }, # KEEP-ROW    <- Array::trailing()
    //   ] # KEEP-ARRAY                           <- Array decor suffix
    //
    // Both belong to the operator, so both are carried, in that source order.
    let after_last_row = existing.trailing().as_str().unwrap_or_default();
    let after_bracket = existing
        .decor()
        .suffix()
        .and_then(RawString::as_str)
        .unwrap_or_default();
    let mut notes = String::new();
    for note in [after_last_row, after_bracket] {
        let note = note.trim();
        if note.contains('#') {
            notes.push(' ');
            notes.push_str(note);
        }
    }
    if notes.is_empty() {
        return;
    }
    let last = new.len().saturating_sub(1);
    let Some(table) = new.get_mut(last) else {
        return;
    };
    let carried = table
        .decor()
        .suffix()
        .and_then(RawString::as_str)
        .unwrap_or_default()
        .trim_end()
        .to_owned();
    if carried.ends_with(notes.trim_end()) {
        // Already merged by an earlier write — leave it byte-for-byte.
        return;
    }
    table.decor_mut().set_suffix(format!("{carried}{notes}"));
}

fn item_shapes_match(existing: &Item, new: &Item) -> bool {
    match (existing, new) {
        (Item::Table(_), Item::Table(_)) | (Item::ArrayOfTables(_), Item::ArrayOfTables(_)) => true,
        (Item::Value(existing), Item::Value(new)) => value_shapes_match(existing, new),
        _ => is_canonicalised(existing, new),
    }
}

/// Whether `new` is the header-table canonicalisation of an inline `existing`
/// — the same value, respelled. Only an inline table, or an array whose every
/// element is an inline table, canonicalises this way; anything else is a
/// genuine structural change and inherits no decoration.
fn is_canonicalised(existing: &Item, new: &Item) -> bool {
    match (existing, new) {
        (Item::Value(Value::InlineTable(_)), Item::Table(_)) => true,
        (Item::Value(Value::Array(array)), Item::ArrayOfTables(_)) => {
            !array.is_empty()
                && array
                    .iter()
                    .all(|value| matches!(value, Value::InlineTable(_)))
        }
        _ => false,
    }
}

/// Put the vanished `key = …` line's own prefix on the header that replaced
/// it. For an array of tables that is the first element — the header the
/// comment sat directly above.
///
/// The first element may already have inherited a note of its own (a comment
/// the operator wrote above the first row of a multi-line array). Both notes
/// are real and neither may be clobbered, so they are merged in source order:
/// the key line came first, the row note second.
fn adopt_header_prefix(item: &mut Item, prefix: RawString) {
    let target = match item {
        Item::Table(table) => Some(table),
        Item::ArrayOfTables(tables) => tables.get_mut(0),
        _ => None,
    };
    let Some(table) = target else {
        return;
    };
    let merged = match table.decor().prefix().and_then(RawString::as_str) {
        Some(inherited)
            if inherited.contains('#') && prefix.as_str().is_some_and(|key| key != inherited) =>
        {
            let key = prefix.as_str().unwrap_or_default().trim_end();
            format!("{key}\n{}", inherited.trim_start_matches('\n'))
        }
        _ => {
            table.decor_mut().set_prefix(prefix);
            return;
        }
    };
    table.decor_mut().set_prefix(merged);
}

/// TOML forbids a comment *inside* an inline table, so an inline row can only
/// own the decoration around its braces: a prefix (reachable when the operator
/// wrote a multi-line array) and a trailing `# note`. Both move to the header
/// line that replaced the row. Bare spacing is left behind — copying inline
/// padding into an expanded body would only mangle it.
fn adopt_inline_decor(existing: &InlineTable, new: &mut Table) {
    let decor = existing.decor();
    if let Some(prefix) = decor.prefix()
        && prefix.as_str().is_some_and(|text| text.contains('#'))
    {
        new.decor_mut().set_prefix(prefix.clone());
    }
    if let Some(suffix) = decor.suffix().and_then(RawString::as_str)
        && suffix.contains('#')
    {
        new.decor_mut().set_suffix(suffix.trim_end().to_owned());
    }
}

fn copy_table_decor(existing: &Table, new: &mut Table) {
    *new.decor_mut() = existing.decor().clone();
    copy_matching_table_items(existing, new);
}

fn copy_array_of_tables_decor(existing: &ArrayOfTables, new: &mut ArrayOfTables) {
    // Preserve the established approximation: array elements pair by index
    // up to the shorter side. Changing this to identity matching is a separate
    // semantic decision, not part of recursive decoration support.
    for index in 0..existing.len().min(new.len()) {
        if let (Some(existing), Some(new)) = (existing.get(index), new.get_mut(index)) {
            copy_table_decor(existing, new);
        }
    }
}

fn copy_value_decor(existing: &Value, new: &mut Value) {
    match (existing, new) {
        (Value::String(existing), Value::String(new)) => {
            *new.decor_mut() = existing.decor().clone();
        }
        (Value::Integer(existing), Value::Integer(new)) => {
            *new.decor_mut() = existing.decor().clone();
        }
        (Value::Float(existing), Value::Float(new)) => {
            *new.decor_mut() = existing.decor().clone();
        }
        (Value::Boolean(existing), Value::Boolean(new)) => {
            *new.decor_mut() = existing.decor().clone();
        }
        (Value::Datetime(existing), Value::Datetime(new)) => {
            *new.decor_mut() = existing.decor().clone();
        }
        (Value::Array(existing), Value::Array(new)) => copy_array_decor(existing, new),
        (Value::InlineTable(existing), Value::InlineTable(new)) => {
            copy_inline_table_decor(existing, new);
        }
        _ => {}
    }
}

fn value_shapes_match(existing: &Value, new: &Value) -> bool {
    match (existing, new) {
        (Value::String(_), Value::String(_))
        | (Value::Integer(_), Value::Integer(_))
        | (Value::Float(_), Value::Float(_))
        | (Value::Boolean(_), Value::Boolean(_))
        | (Value::Datetime(_), Value::Datetime(_))
        | (Value::InlineTable(_), Value::InlineTable(_)) => true,
        (Value::Array(existing), Value::Array(new)) => existing.len() == new.len(),
        _ => false,
    }
}

fn copy_inline_table_decor(existing: &InlineTable, new: &mut InlineTable) {
    *new.decor_mut() = existing.decor().clone();
    new.set_preamble(existing.preamble().clone());

    let keys = new
        .iter()
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    for key in keys {
        let Some(existing_value) = existing.get(&key) else {
            continue;
        };
        let Some(new_value) = new.get(&key) else {
            continue;
        };
        if !value_shapes_match(existing_value, new_value) {
            continue;
        }
        let key_decor = existing.key(&key).map(|key| key.leaf_decor().clone());

        if let Some(decor) = key_decor
            && let Some(mut new_key) = new.key_mut(&key)
        {
            *new_key.leaf_decor_mut() = decor;
        }
        if let Some(new_value) = new.get_mut(&key) {
            copy_value_decor(existing_value, new_value);
        }
    }
}

fn copy_array_decor(existing: &Array, new: &mut Array) {
    *new.decor_mut() = existing.decor().clone();

    new.set_trailing(existing.trailing().clone());
    new.set_trailing_comma(existing.trailing_comma());
    for index in 0..existing.len() {
        if let (Some(existing), Some(new)) = (existing.get(index), new.get_mut(index)) {
            copy_value_decor(existing, new);
        }
    }
}
