//! Recursive TOML decoration transfer for manifest rewrites.
//!
//! The freshly serialised document owns structure and values. This cell only
//! transfers comments and whitespace between matching keys and matching TOML
//! shapes, so a structural edit can never inherit decoration from a removed or
//! differently-typed value.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use toml_edit::{Array, ArrayOfTables, InlineTable, Item, Table, Value};

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
        let key_decor = existing.key(&key).map(|key| key.leaf_decor().clone());

        if let Some(decor) = key_decor
            && let Some(mut new_key) = new.key_mut(&key)
        {
            *new_key.leaf_decor_mut() = decor;
        }
        if let Some(new_item) = new.get_mut(&key) {
            copy_item_decor(existing_item, new_item);
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
        _ => {}
    }
}

fn item_shapes_match(existing: &Item, new: &Item) -> bool {
    match (existing, new) {
        (Item::Table(_), Item::Table(_)) | (Item::ArrayOfTables(_), Item::ArrayOfTables(_)) => true,
        (Item::Value(existing), Item::Value(new)) => value_shapes_match(existing, new),
        _ => false,
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
