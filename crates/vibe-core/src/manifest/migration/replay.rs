//! Exact original-coordinate token replay, without parsing or I/O authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");
use super::*;

/// Reconstruct from original source coordinates, including ordered insertions.
/// Unmentioned bytes are copied verbatim and never passed through a formatter.
pub(super) fn replay_edits(text: &str, edits: &[MigrationTokenEdit]) -> Checked<String> {
    let mut after = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut prior_insertion = None;
    for edit in edits {
        let unprovable = || unprovable(edit.field);
        let start = edit.before_span.start;
        let end = edit.before_span.end;
        if start < cursor
            || end < start
            || (start == end && edit.after_token.is_empty())
            || prior_insertion == Some(start)
        {
            return Err(unprovable());
        }
        let untouched = text.get(cursor..start).ok_or_else(unprovable)?;
        let original = text.get(start..end).ok_or_else(unprovable)?;
        if original != edit.before_token {
            return Err(unprovable());
        }
        after.push_str(untouched);
        after.push_str(&edit.after_token);
        cursor = end;
        prior_insertion = (start == end).then_some(start);
    }
    after.push_str(text.get(cursor..).ok_or_else(|| unprovable("edits"))?);
    Ok(after)
}
