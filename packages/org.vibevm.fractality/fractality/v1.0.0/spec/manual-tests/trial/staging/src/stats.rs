//! Counting: key frequencies across a batch of records.

use std::collections::BTreeMap;

use crate::parse::Rec;

/// Counts how many records carry each key (a key counted once per
/// record, however many times it appeared before last-wins).
pub fn count_keys(recs: &[Rec]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for rec in recs {
        for (k, _) in &rec.pairs {
            *out.entry(format!("{}", k)).or_insert(0) += 1;
        }
    }
    out
}
