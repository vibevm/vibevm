//! `aggregate-reconciliation` — the CLI timing table, recomputed.
//!
//! The rows in `aggregates` are what `vibe install --trace-compile`
//! prints; they are also the one part of the index that restates
//! something the events already say. A carried total is therefore never
//! trusted: this pass walks the events, rebuilds every count and every
//! saturating sum, and compares. It also pins the ROW ORDER to
//! first-appearance in `events`, because a table whose rows permute
//! between two runs of the same compile is a table nobody can diff —
//! the same set in a different order is red here.
//!
//! Arithmetic is checked or saturating throughout: a count that would
//! pass the `uint32` ceiling is refused, never wrapped, and a duration
//! sum sticks at `u32::MAX` with its `saturated` marker set.

use std::collections::BTreeMap;

use crate::generated::compiler_trace_index::e1::index::{Duration, PassEvent, TimingRow};

use super::errors::{DurationSite, ScalarPreview, TimingColumn, TraceIndexError};
use super::scalars::scalar_gate;

/// One pass name's recomputed totals.
pub(super) struct Totals {
    invocations: u32,
    pass: Duration,
    verify: Duration,
    encode: Duration,
}

impl Default for Totals {
    fn default() -> Self {
        Totals {
            invocations: 0,
            pass: zero(),
            verify: zero(),
            encode: zero(),
        }
    }
}

impl Totals {
    fn column(&self, column: TimingColumn) -> &Duration {
        match column {
            TimingColumn::Pass => &self.pass,
            TimingColumn::Verify => &self.verify,
            TimingColumn::Encode => &self.encode,
        }
    }
}

/// The additive identity of the generated `duration` record (which is
/// generator-owned and carries no `Default` of its own).
fn zero() -> Duration {
    Duration {
        micros: 0,
        saturated: false,
    }
}

/// A duration is canonical when `saturated` actually means "the true
/// value was at least the ceiling": the flag is legal only at
/// `u32::MAX`. The converse is NOT a law — an exact measurement may land
/// on `u32::MAX` without having saturated, and that stays legal.
pub(super) fn is_canonical(duration: &Duration) -> bool {
    !duration.saturated || duration.micros == u32::MAX
}

/// Refuse a non-canonical duration at a named site.
pub(super) fn canonical_gate(
    site: DurationSite,
    duration: &Duration,
) -> Result<(), TraceIndexError> {
    if is_canonical(duration) {
        Ok(())
    } else {
        Err(TraceIndexError::NonCanonicalDuration {
            site,
            micros: duration.micros,
        })
    }
}

/// Saturating accumulate: `micros` caps at `u32::MAX` (never wraps) and
/// `saturated` is sticky — an input that was already saturated or a sum
/// that overflowed both set it. Canonical in, canonical out: a total
/// that is marked saturated always sits at the ceiling.
pub(super) fn saturating_add_into(total: &mut Duration, addend: &Duration) {
    let (sum, overflowed) = total.micros.overflowing_add(addend.micros);
    total.saturated = total.saturated || addend.saturated || overflowed;
    total.micros = if overflowed || total.saturated {
        u32::MAX
    } else {
        sum
    };
}

/// Add one event to a pass name's totals, refusing an invocation count
/// the epoch's `uint32` cannot hold.
fn accumulate(totals: &mut Totals, event: &PassEvent, pass: &str) -> Result<(), TraceIndexError> {
    increment_count(&mut totals.invocations, pass)?;
    for (carried, column) in [
        (&event.pass_micros, &mut totals.pass),
        (&event.verify_micros, &mut totals.verify),
        (&event.encode_micros, &mut totals.encode),
    ] {
        if let Some(duration) = carried {
            saturating_add_into(column, duration);
        }
    }
    Ok(())
}

/// Advance one aggregate invocation count. This boundary makes the uint32
/// ceiling testable without constructing four billion event records.
pub(super) fn increment_count(count: &mut u32, pass: &str) -> Result<(), TraceIndexError> {
    *count = count
        .checked_add(1)
        .ok_or_else(|| TraceIndexError::AggregateCountOverflow {
            pass: ScalarPreview::of(pass),
        })?;
    Ok(())
}

/// Rebuild every pass name's totals from the events, in first-appearance
/// order — the ONE implementation of the table, shared by the writer
/// that produces `aggregates` and the validator that refuses a carried
/// table which disagrees with it.
fn recompute(events: &[PassEvent]) -> Result<Vec<(&str, Totals)>, TraceIndexError> {
    // ONE representation, built in first-appearance order as the walk goes:
    // the rows themselves are the result, and the map only remembers where
    // each pass name's row already sits. Keeping the totals in a second
    // container and moving them out afterwards needs an "unreachable" unwrap
    // to reunite them — a panic path in domain logic, in a cell whose whole
    // input is an untrusted file.
    let mut rows: Vec<(&str, Totals)> = Vec::new();
    let mut position_of: BTreeMap<&str, usize> = BTreeMap::new();
    for event in events {
        let pass = event.pass.as_str();
        let position = match position_of.get(pass) {
            Some(position) => *position,
            None => {
                let position = rows.len();
                position_of.insert(pass, position);
                rows.push((pass, Totals::default()));
                position
            }
        };
        let Some((_, totals)) = rows.get_mut(position) else {
            // Construction says this is unreachable, but an untrusted index
            // is never allowed to turn an internal bookkeeping defect into a
            // panic. Reuse the cell's existing missing-row refusal instead.
            return Err(TraceIndexError::AggregateRowMissing {
                pass: ScalarPreview::of(pass),
            });
        };
        accumulate(totals, event, pass)?;
    }
    Ok(rows)
}

/// Build the CLI timing table a writer stores in `aggregates`.
///
/// Same walk, same checked counts and same saturating sums the validator
/// uses, so a produced table reconciles by construction rather than by a
/// second algorithm that happens to agree today.
pub fn build_aggregates(events: &[PassEvent]) -> Result<Vec<TimingRow>, TraceIndexError> {
    Ok(recompute(events)?
        .into_iter()
        .map(|(pass, totals)| TimingRow {
            pass: pass.to_string(),
            invocations: totals.invocations,
            pass_total: totals.pass,
            verify_total: totals.verify,
            encode_total: totals.encode,
        })
        .collect())
}

/// The whole law: rebuild the table from the events, then hold the
/// carried table to it — membership, order, counts and every column.
pub(super) fn aggregate_gate(
    events: &[PassEvent],
    rows: &[TimingRow],
) -> Result<(), TraceIndexError> {
    let rebuilt = recompute(events)?;
    let order: Vec<&str> = rebuilt.iter().map(|(pass, _)| *pass).collect();
    let recomputed: BTreeMap<&str, &Totals> = rebuilt
        .iter()
        .map(|(pass, totals)| (*pass, totals))
        .collect();

    // Every row is a legible identity carrying canonical durations,
    // before any comparison reads its numbers.
    let mut carried: BTreeMap<&str, &TimingRow> = BTreeMap::new();
    for row in rows {
        scalar_gate("aggregates.pass", &row.pass)?;
        for (column, duration) in [
            (TimingColumn::Pass, &row.pass_total),
            (TimingColumn::Verify, &row.verify_total),
            (TimingColumn::Encode, &row.encode_total),
        ] {
            canonical_gate(
                DurationSite::Aggregate {
                    pass: ScalarPreview::of(&row.pass),
                    column,
                },
                duration,
            )?;
        }
        if carried.insert(row.pass.as_str(), row).is_some() {
            return Err(TraceIndexError::AggregateRowDuplicate {
                pass: ScalarPreview::of(&row.pass),
            });
        }
        if !recomputed.contains_key(row.pass.as_str()) {
            return Err(TraceIndexError::AggregateRowUnknown {
                pass: ScalarPreview::of(&row.pass),
            });
        }
    }
    if let Some(missing) = order.iter().find(|pass| !carried.contains_key(**pass)) {
        return Err(TraceIndexError::AggregateRowMissing {
            pass: ScalarPreview::of(missing),
        });
    }

    // Same set, same size — so a positional walk now compares like with
    // like, and any difference is genuinely an ORDER difference.
    for (position, (row, expected)) in rows.iter().zip(order.iter()).enumerate() {
        if row.pass.as_str() != *expected {
            return Err(TraceIndexError::AggregateRowOutOfOrder {
                position,
                carried: ScalarPreview::of(&row.pass),
                expected: ScalarPreview::of(expected),
            });
        }
    }

    for row in rows {
        let totals = &recomputed[row.pass.as_str()];
        if row.invocations != totals.invocations {
            return Err(TraceIndexError::AggregateCountMismatch {
                pass: ScalarPreview::of(&row.pass),
                carried: row.invocations,
                actual: totals.invocations,
            });
        }
        for (column, carried) in [
            (TimingColumn::Pass, &row.pass_total),
            (TimingColumn::Verify, &row.verify_total),
            (TimingColumn::Encode, &row.encode_total),
        ] {
            let recomputed = totals.column(column);
            if carried != recomputed {
                return Err(TraceIndexError::AggregateDurationMismatch {
                    pass: ScalarPreview::of(&row.pass),
                    column,
                    carried: carried.clone(),
                    recomputed: recomputed.clone(),
                });
            }
        }
    }
    Ok(())
}
