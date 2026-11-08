//! The sampler of one transfer (`PROP-068#sampling`): when a sample is due,
//! the rate over the last second beside it, the peak of those rates, the
//! samples of the last five minutes for the active file, and the
//! per-second buckets a finished entry keeps.
//!
//! It is pure arithmetic over "how long since the start" and "how many
//! bytes so far", so the cadence, the rate and the bucket cap are tested
//! without a clock or a network.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#sampling");

use std::collections::VecDeque;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::clock::format_millis;

/// A sample is due this long after the previous one at the latest.
pub(crate) const CADENCE: Duration = Duration::from_millis(250);
/// …or as soon as this many bytes arrived since it, whichever is first.
pub(crate) const CADENCE_BYTES: u64 = 1 << 20;
/// The rate of a sample is taken over this window.
const RATE_WINDOW: Duration = Duration::from_secs(1);
/// The active file keeps the samples of this horizon: the graph of the
/// shell's Activity screen.
const ACTIVE_WINDOW: Duration = Duration::from_secs(300);
/// A finished entry keeps at most this many buckets.
pub(crate) const SERIES_CAP: usize = 3600;

/// One sample: the wall clock, the cumulative byte count and the rate over
/// the last second.
///
/// ```
/// use vibe_core::progress::transfer::Sample;
///
/// let sample: Sample =
///     serde_json::from_str(r#"{"at":"2026-11-01T10:00:00.250Z","bytes":1048576,"rate_bps":4194304}"#)
///         .unwrap();
/// assert_eq!(sample.bytes, 1 << 20);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    pub at: String,
    pub bytes: u64,
    #[serde(default)]
    pub rate_bps: u64,
    /// Cumulative successful logical writes; missing means unobserved.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::wire_decimal::optional"
    )]
    pub disk_written_bytes: Option<u64>,
}

/// The bytes that arrived during one bucket of a finished transfer;
/// `second` counts from the transfer's start.
///
/// ```
/// use vibe_core::progress::transfer::Bucket;
///
/// let bucket = Bucket { second: 0, bytes: 512 };
/// assert_eq!(serde_json::to_string(&bucket).unwrap(), r#"{"second":0,"bytes":512}"#);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    pub second: u64,
    pub bytes: u64,
}

pub(crate) struct Sampler {
    started_ms: u64,
    /// `(since start, bytes)` of the samples taken, kept back to the last
    /// one at least a rate window old, which is the rate's base.
    recent: VecDeque<(Duration, u64)>,
    /// The samples of the active window, with their offsets.
    window: VecDeque<(Duration, Sample)>,
    peak: u64,
    /// Bytes counted per second since the start.
    seconds: Vec<u64>,
}

impl Sampler {
    pub(crate) fn new(started_ms: u64) -> Self {
        Self {
            started_ms,
            recent: VecDeque::new(),
            window: VecDeque::new(),
            peak: 0,
            seconds: Vec::new(),
        }
    }

    /// Credit `bytes` that arrived `since` the start to their second.
    pub(crate) fn count(&mut self, since: Duration, bytes: u64) {
        let second = since.as_secs() as usize;
        if self.seconds.len() <= second {
            self.seconds.resize(second + 1, 0);
        }
        self.seconds[second] += bytes;
    }

    /// Whether a sample is due at `since` with `bytes` arrived so far.
    pub(crate) fn due(&self, since: Duration, bytes: u64) -> bool {
        match self.recent.back() {
            None => true,
            Some(&(at, counted)) => {
                since.saturating_sub(at) >= CADENCE
                    || bytes.saturating_sub(counted) >= CADENCE_BYTES
            }
        }
    }

    /// Take a sample. The first one has rate 0; every later one divides the
    /// bytes since the last sample at least a second old (the start, while
    /// the transfer is younger) by the time since it.
    #[cfg(test)]
    pub(crate) fn take(&mut self, since: Duration, bytes: u64) -> Sample {
        self.take_observed(since, bytes, None)
    }

    pub(crate) fn take_observed(
        &mut self,
        since: Duration,
        bytes: u64,
        disk_written_bytes: Option<u64>,
    ) -> Sample {
        let rate_bps = match self.base(since) {
            None => 0,
            Some((at, counted)) => {
                let millis = since.saturating_sub(at).as_millis().max(1) as u64;
                bytes.saturating_sub(counted).saturating_mul(1000) / millis
            }
        };
        self.peak = self.peak.max(rate_bps);
        self.recent.push_back((since, bytes));
        while self.recent.len() > 2 && self.recent[1].0 + RATE_WINDOW <= since {
            self.recent.pop_front();
        }
        let sample = Sample {
            at: format_millis(self.started_ms + since.as_millis() as u64),
            bytes,
            rate_bps,
            disk_written_bytes,
        };
        self.window.push_back((since, sample.clone()));
        while self
            .window
            .front()
            .is_some_and(|(at, _)| *at + ACTIVE_WINDOW < since)
        {
            self.window.pop_front();
        }
        sample
    }

    /// The sample the next rate is measured from.
    fn base(&self, since: Duration) -> Option<(Duration, u64)> {
        let first = *self.recent.front()?;
        Some(
            self.recent
                .iter()
                .rev()
                .find(|(at, _)| *at + RATE_WINDOW <= since)
                .copied()
                .unwrap_or(first),
        )
    }

    pub(crate) fn disk_written_bytes(&self) -> Option<u64> {
        self.window
            .back()
            .and_then(|(_, sample)| sample.disk_written_bytes)
    }

    pub(crate) fn peak(&self) -> u64 {
        self.peak
    }

    /// The samples of the last five minutes, oldest first.
    pub(crate) fn samples(&self) -> Vec<Sample> {
        self.window
            .iter()
            .map(|(_, sample)| sample.clone())
            .collect()
    }

    /// The per-second series of a transfer that lasted `since`: one bucket
    /// per second from the start to the end, merged `ceil(n / 3600)` at a
    /// time when there are more than [`SERIES_CAP`]; the bucket width is
    /// the second half of the answer. The buckets sum to every byte counted.
    pub(crate) fn series(&self, since: Duration) -> (Vec<Bucket>, u64) {
        let mut seconds = self.seconds.clone();
        let length = (since.as_secs() as usize + 1).max(seconds.len());
        seconds.resize(length, 0);
        let width = length.div_ceil(SERIES_CAP).max(1);
        let buckets = seconds
            .chunks(width)
            .enumerate()
            .map(|(index, chunk)| Bucket {
                second: (index * width) as u64,
                bytes: chunk.iter().sum(),
            })
            .collect();
        (buckets, width as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn a_sample_is_due_every_quarter_second_or_every_mebibyte() {
        let mut sampler = Sampler::new(0);
        assert!(sampler.due(ms(0), 0), "the start is a sample");
        sampler.take(ms(0), 0);
        assert!(!sampler.due(ms(100), 1000));
        assert!(sampler.due(ms(250), 1000), "a quarter second passed");
        assert!(sampler.due(ms(10), CADENCE_BYTES), "a mebibyte arrived");
        sampler.take(ms(10), CADENCE_BYTES);
        assert!(!sampler.due(ms(20), CADENCE_BYTES + 10));
    }

    #[test]
    fn the_rate_is_over_the_last_second_and_the_peak_is_kept() {
        let mut sampler = Sampler::new(1_000);
        assert_eq!(sampler.take(ms(0), 0).rate_bps, 0, "the first sample");
        // Younger than a second: measured from the start.
        assert_eq!(sampler.take(ms(250), 250_000).rate_bps, 1_000_000);
        assert_eq!(sampler.take(ms(500), 500_000).rate_bps, 1_000_000);
        assert_eq!(sampler.take(ms(1000), 3_000_000).rate_bps, 3_000_000);
        // From now on the base is the last sample at least a second old.
        let late = sampler.take(ms(2000), 4_000_000);
        assert_eq!(late.rate_bps, 1_000_000, "bytes of 1000..2000 over 1 s");
        assert_eq!(sampler.peak(), 3_000_000);
        assert_eq!(late.at, "1970-01-01T00:00:03.000Z");
    }

    #[test]
    fn the_active_window_keeps_five_minutes() {
        let mut sampler = Sampler::new(0);
        for second in 0..=400 {
            sampler.take(Duration::from_secs(second), second * 10);
        }
        let samples = sampler.samples();
        assert_eq!(samples.first().unwrap().bytes, 1000, "second 100");
        assert_eq!(samples.len(), 301);
    }

    #[test]
    fn buckets_sum_to_the_bytes_and_are_capped() {
        let mut sampler = Sampler::new(0);
        sampler.count(ms(100), 10);
        sampler.count(ms(900), 5);
        sampler.count(ms(2500), 7);
        let (series, width) = sampler.series(ms(3100));
        assert_eq!(width, 1);
        assert_eq!(
            series.iter().map(|b| b.bytes).collect::<Vec<_>>(),
            [15, 0, 7, 0]
        );

        let mut long = Sampler::new(0);
        for second in 0..7300u64 {
            long.count(Duration::from_secs(second), 1);
        }
        let (series, width) = long.series(Duration::from_secs(7299));
        assert_eq!(width, 3, "ceil(7300 / 3600)");
        assert!(series.len() <= SERIES_CAP);
        assert_eq!(series[1].second, 3);
        assert_eq!(series.iter().map(|b| b.bytes).sum::<u64>(), 7300);
    }
}
