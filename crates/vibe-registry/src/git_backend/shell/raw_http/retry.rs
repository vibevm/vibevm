//! When a refused raw read is worth asking again, and how long it waits
//! before it does.
//!
//! **Why this exists.** The fast path above treats every answer it cannot
//! interpret as a reason to ask git, and that is right for an answer
//! *about the file*. A `429` is not one: the host is saying "not now",
//! about us rather than about the repository, and the ladder it drops
//! into is expensive. Measured on the redbook graph, 2026-09-14: exactly
//! one read of the 243-read resolve walk came back `429`, and the
//! archive → clone → fetch → checkout → submodule ladder that followed
//! spent about twenty seconds of the ninety the whole phase took. A
//! pause and a second ask are cheaper than that by an order of
//! magnitude.
//!
//! **What it must never become** is a second timeout. Every decision here
//! is bounded twice over — by [`MAX_ATTEMPTS`] and by [`PAUSE_BUDGET`] —
//! so the worst case a single file can cost in waiting is a few seconds,
//! after which the read falls into exactly the ladder it would have taken
//! without any of this, carrying exactly the diagnosis it carried before.
//!
//! Everything in this module is a pure decision about numbers and text:
//! no request is made here, and no sleeping is done here either (the
//! caller owns the pause, so a test can hold the clock).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf");

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How many times one file may be asked for before the answer becomes
/// git's problem: the first attempt and two more.
///
/// Three rather than more because the pause is the cheap part and the
/// request is not: a host that has refused three times inside a couple of
/// seconds is rate-limiting in earnest, and the ladder below — which can
/// actually clone — is the better answer to that than a fourth ask.
pub(super) const MAX_ATTEMPTS: u32 = 3;

/// The pause before the second attempt. It doubles for each further one
/// — 0.5 s, 1 s, 2 s — so a host that is briefly busy is given room
/// without the read losing its character as a fast path.
const FIRST_PAUSE_MS: u64 = 500;

/// The longest one file's read may spend waiting, counting every pause
/// together.
///
/// This is also the ceiling on what a host's own `Retry-After` can buy:
/// a host that asks for a minute is asking for something this path
/// cannot give — the resolve walk reads hundreds of files — so its wish
/// is honoured as far as the budget goes and the read then takes the
/// ladder. Five seconds is roughly a quarter of what the ladder cost in
/// the measurement above, which is the trade this whole module is.
const PAUSE_BUDGET: Duration = Duration::from_secs(5);

/// Did this status refuse *the file*, or refuse *just now*?
///
/// Only two kinds of answer are the latter: `429`, the rate limit this
/// module exists for, and the `5xx` range, where the host is telling us
/// about itself rather than about the repository. Everything else —
/// `404`, `400`, `401`, `403`, a redirect this client did not follow —
/// is an answer that will not change in half a second, and asking again
/// would only delay the fall-back that already handles it.
pub(super) fn refused_for_now(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

/// Is this transport failure worth a second attempt?
///
/// Two exclusions, and both are about not paying twice for the same
/// verdict:
///
/// - **A name that did not resolve will not resolve.** DNS is not the
///   thing a backoff rides out, and `reqwest` does not type name
///   resolution separately — so this reads the error's own chain, the
///   same substring classification `classify_failure` applies to git's
///   stderr. Getting it wrong costs one pause, never a wrong answer.
/// - **A timeout has already spent the budget.** `READ_TIMEOUT_SECS` in
///   the parent module states the rule this keeps: ten seconds without
///   an answer means this is no longer a fast path. Asking twice more
///   would make one file cost half a minute — the very thing the read
///   was written to avoid.
pub(super) fn worth_another_try(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_builder() || error.is_redirect() {
        return false;
    }
    !names_an_unknown_host(&chain_text(error))
}

/// Every message in an error's `source()` chain, joined and lowercased.
fn chain_text(error: &reqwest::Error) -> String {
    let mut text = String::new();
    let mut link: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(e) = link {
        text.push_str(&e.to_string().to_lowercase());
        text.push_str("; ");
        link = e.source();
    }
    text
}

/// The shapes a name-resolution failure takes on the platforms this is
/// built for — `hyper`'s own wrapper, plus what the three C libraries
/// underneath it say. Matched against the lowercased chain.
const UNKNOWN_HOST_MARKERS: [&str; 5] = [
    "dns error",
    "failed to lookup address",
    "no such host",
    "name or service not known",
    "nodename nor servname",
];

/// Does this error text say the host does not exist?
pub(super) fn names_an_unknown_host(lowercased_chain: &str) -> bool {
    UNKNOWN_HOST_MARKERS
        .iter()
        .any(|marker| lowercased_chain.contains(marker))
}

/// How long to wait before the attempt after `attempt`, or `None` when
/// there is not to be one.
///
/// `asked` is what the host's own `Retry-After` requested, and it wins
/// over the doubling schedule when present — a host that names a number
/// knows more about its own limit than a constant does. What it cannot
/// do is outlast [`PAUSE_BUDGET`]: the pause is trimmed to whatever the
/// budget has left, and an exhausted budget ends the retrying as surely
/// as a spent attempt does.
///
/// `attempt` is 1-based and the first branch bounds it, so the shift
/// below is over 0 or 1.
pub(super) fn pause_before_next(
    attempt: u32,
    asked: Option<Duration>,
    already_waited: Duration,
) -> Option<Duration> {
    if attempt >= MAX_ATTEMPTS {
        return None;
    }
    let left = PAUSE_BUDGET.checked_sub(already_waited)?;
    if left.is_zero() {
        return None;
    }
    let doubling = Duration::from_millis(FIRST_PAUSE_MS << (attempt - 1));
    Some(asked.unwrap_or(doubling).min(left))
}

/// What a `Retry-After` header asks for, as a duration from now.
///
/// RFC 9110 allows two spellings and this reads both: a plain count of
/// seconds, and an HTTP date to wait until. The date form is read in its
/// preferred `IMF-fixdate` spelling only — the two obsolete formats the
/// RFC still tolerates are not worth the parser, and an unreadable value
/// is simply no value: the doubling schedule then sets the pause, which
/// is what happens for a header that is absent altogether.
///
/// A date already past yields zero, not a negative wait: the host is
/// saying the limit has lifted.
pub(super) fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let until = imf_fixdate_to_unix(value)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(Duration::from_secs(until.saturating_sub(now)))
}

/// The three-letter month names an `IMF-fixdate` uses, in order.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `Sun, 06 Nov 1994 08:49:37 GMT` → seconds since the unix epoch.
///
/// Hand-rolled for the reason `decode_base64` next door is: one call
/// site does not earn a dependency, and this one is a fixed-width format
/// with no locale and no zone to negotiate — the RFC pins the spelling
/// down to the comma. The weekday is not checked against the date: it is
/// redundant, and a host that disagrees with itself about it is still
/// telling us plainly enough when to come back.
///
/// `None` for anything that is not exactly that shape, and for a date
/// before the epoch — both mean "no usable value", which the caller
/// already knows how to handle.
///
/// Reachable from the module's tests next door, where the RFC's own
/// example is asserted against the instant it names; a wait computed
/// from `SystemTime::now()` could not pin that down.
pub(super) fn imf_fixdate_to_unix(value: &str) -> Option<u64> {
    let after_weekday = value.split_once(", ")?.1;
    let mut fields = after_weekday.split(' ');
    let day: u64 = fields.next()?.parse().ok()?;
    let month_name = fields.next()?;
    let month = MONTHS.iter().position(|m| *m == month_name)? as u64 + 1;
    let year: u64 = fields.next()?.parse().ok()?;
    let mut clock = fields.next()?.split(':');
    let hour: u64 = clock.next()?.parse().ok()?;
    let minute: u64 = clock.next()?.parse().ok()?;
    let second: u64 = clock.next()?.parse().ok()?;
    if clock.next().is_some() || fields.next()? != "GMT" || fields.next().is_some() {
        return None;
    }
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days_from_epoch(year, month, day)? * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days from 1970-01-01 to a proleptic-Gregorian `(year, month, day)`.
///
/// Howard Hinnant's `days_from_civil`, which is the standard way to do
/// this without a calendar library: shift the year to start in March so
/// the leap day lands last, count whole 400-year eras, then the day
/// within the era. `None` before the epoch — the only dates this reads
/// are `Retry-After` values, and one of those pointing at 1969 is not a
/// value worth rescuing.
fn days_from_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if year < 1970 || !(1..=12).contains(&month) {
        return None;
    }
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted / 400;
    let year_of_era = shifted - era * 400;
    let march_month = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * march_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}
