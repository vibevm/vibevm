//! Filtering: keep the records whose `key` equals `value`.

use crate::parse::Rec;

/// Keeps records where `key` equals `value` exactly.
pub fn filter_by_key<'a>(recs: &'a [Rec], key: &str, value: &str) -> Vec<&'a Rec> {
    recs.iter().filter(|r| r.get(key) == Some(value)).collect()
}

/// True when every record carries `key`.
pub fn all_have(recs: &[Rec], key: &str) -> bool {
    recs.iter().all(|r| r.get(key).is_some())
}
