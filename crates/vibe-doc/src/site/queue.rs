//! What to rebuild — the feed against the state
//! (PROP-057 `##SITE-SOURCE-REGISTRY`, `##SITE-HOST-POLL`).
//!
//! One comparison and no others: for every pair the sources publish now,
//! is there a row saying it was rendered from exactly these bytes? That
//! is the whole of «what changed», and it is deliberately not a diff of
//! two catalogs — a catalog remembered from last time would be a history,
//! and the project keeps none (§14). The state file holds only what is on
//! disk now, so the comparison is always «the world» against «the
//! output», never «the world» against «the world as it was».
//!
//! ## Four verdicts, because four repairs
//!
//! A pair is NEW (nothing stands at that address), MOVED (the address
//! holds a render of other bytes), FAILED (the address holds a «render
//! failed» page) or UNCHANGED. The first three are queued and the fourth
//! is the number a second run reports as zero work. FAILED is the one
//! that must be its own verdict: a package whose source never moves again
//! would otherwise never be retried, because nothing about it would ever
//! differ from what was rendered.
//!
//! ## The debounce is on the source, not on the pair
//!
//! `##SITE-HOST-POLL` leaves a host render alone for a configured
//! interval however often the branch moves. Measured per pair, a branch
//! touching a different package every ten minutes would rebuild
//! continuously and obey the letter of the rule; measured on the source,
//! a burst of commits collapses into one render, which is what the rule
//! is for. The registry has no debounce: its feed only moves when
//! somebody publishes, and a publication is exactly the event a site
//! exists to show.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-HOST-POLL");

use chrono::{DateTime, Duration, Utc};
use vibe_wire::generated::doc_site_state::DocSiteState;

use super::feed::{Origin, Pair};
use super::state;

/// Why a pair is queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing stands at this address yet.
    New,
    /// The address holds a render of other bytes.
    Moved,
    /// The address holds a «render failed» page.
    Failed,
}

impl Verdict {
    /// The word a report prints.
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::New => "new",
            Verdict::Moved => "moved",
            Verdict::Failed => "retry",
        }
    }
}

/// One pair to rebuild, with the reason it is in the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queued {
    pub pair: Pair,
    pub verdict: Verdict,
}

/// A host render the debounce is holding back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// How many host pairs would have been rebuilt.
    pub pairs: usize,
    /// Whole minutes until the debounce lets them through.
    pub minutes_left: i64,
}

/// Everything a run decided before it rendered anything.
#[derive(Debug, Clone, Default)]
pub struct Queue {
    pub rebuild: Vec<Queued>,
    /// How many published pairs already stand rendered from these bytes.
    pub unchanged: usize,
    /// Addresses the output holds that no source publishes any more. The
    /// registry has no history, so a version that left the catalog left
    /// it — and the page under that address is now the only place it
    /// still exists.
    pub gone: Vec<String>,
    /// The host's pairs, held back by the debounce.
    pub held: Option<Held>,
    /// Two sources offering one coordinate and version with different
    /// bytes. Reported and never resolved by picking: a site that chose
    /// silently would serve one registry's package under another's name.
    pub collisions: Vec<String>,
}

impl Queue {
    /// Is there anything to do?
    pub fn is_empty(&self) -> bool {
        self.rebuild.is_empty() && self.gone.is_empty()
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for collision in &self.collisions {
            out.push_str(&format!("  collision {collision}\n"));
        }
        for queued in &self.rebuild {
            out.push_str(&format!(
                "  {:<6} {} ({})\n",
                queued.verdict.as_str(),
                queued.pair.spelled(),
                queued.pair.source,
            ));
        }
        for address in &self.gone {
            out.push_str(&format!("  {:<6} {address}\n", "gone"));
        }
        if let Some(held) = &self.held {
            out.push_str(&format!(
                "  held   {} host pair(s) — the debounce has {} minute(s) to run\n",
                held.pairs, held.minutes_left,
            ));
        }
        out.push_str(&format!(
            "queue: {} to render, {} unchanged, {} gone\n",
            self.rebuild.len(),
            self.unchanged,
            self.gone.len(),
        ));
        out
    }
}

/// Decide what to rebuild.
///
/// `now` and the debounce arrive as arguments for the reason every other
/// instant in this library does: a plan that called the clock itself
/// would answer differently to the same inputs, and a build that cannot
/// be replayed cannot be reviewed.
pub fn plan(
    pairs: &[Pair],
    state: &DocSiteState,
    now: DateTime<Utc>,
    debounce_minutes: u32,
) -> Queue {
    let mut queue = Queue::default();
    let mut seen: Vec<(String, String, String, String)> = Vec::new();

    for pair in pairs {
        let identity = (pair.group.clone(), pair.name.clone(), pair.version.clone());
        if let Some((_, _, _, other)) = seen
            .iter()
            .find(|(g, n, v, _)| (g.clone(), n.clone(), v.clone()) == identity)
        {
            if other != &pair.content_hash {
                queue.collisions.push(format!(
                    "{} is published by two sources with different bytes",
                    pair.spelled()
                ));
            }
            continue;
        }
        seen.push((
            pair.group.clone(),
            pair.name.clone(),
            pair.version.clone(),
            pair.content_hash.clone(),
        ));

        let verdict = match state::rendered(state, &pair.group, &pair.name, &pair.version) {
            None => Some(Verdict::New),
            Some(row) if row.failed => Some(Verdict::Failed),
            Some(row) if row.content_hash != pair.content_hash => Some(Verdict::Moved),
            Some(_) => None,
        };
        match verdict {
            Some(verdict) => queue.rebuild.push(Queued {
                pair: pair.clone(),
                verdict,
            }),
            None => queue.unchanged += 1,
        }
    }

    for row in &state.rendered {
        let still_published = pairs.iter().any(|pair| {
            pair.group == row.group.to_string()
                && pair.name == row.name
                && pair.version == row.version.to_string()
        });
        if !still_published {
            queue
                .gone
                .push(format!("{}/{}@{}", row.group, row.name, row.version));
        }
    }

    hold_the_host(&mut queue, state, now, debounce_minutes);
    queue
}

/// Take the host's pairs back out of the queue when the debounce has not
/// run out.
///
/// Applied after the verdicts and not before, so the report can say how
/// many pairs are waiting: «nothing to do» and «four things to do in
/// twenty minutes» are different answers, and a builder that printed the
/// first when it meant the second would look broken on a branch that
/// moves often.
fn hold_the_host(
    queue: &mut Queue,
    state: &DocSiteState,
    now: DateTime<Utc>,
    debounce_minutes: u32,
) {
    let Some(last) = state.host_rendered_at else {
        return;
    };
    let ready = last + Duration::minutes(i64::from(debounce_minutes));
    if now >= ready {
        return;
    }
    let held: Vec<Queued> = queue
        .rebuild
        .iter()
        .filter(|queued| !matches!(queued.pair.origin, Origin::Registry))
        .cloned()
        .collect();
    if held.is_empty() {
        return;
    }
    queue
        .rebuild
        .retain(|queued| matches!(queued.pair.origin, Origin::Registry));
    queue.held = Some(Held {
        pairs: held.len(),
        // Rounded up, because «0 minutes left» on a render that is still
        // held reads as a defect.
        minutes_left: (ready - now).num_minutes() + 1,
    });
}

#[cfg(test)]
mod tests;
