//! The machine's download rate limit (`PROP-068#limit`): one token bucket
//! shared by every HTTP transfer of the process, so `[transfer] limit`
//! bounds their sum rather than each one.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#limit");

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::user_config::TransferConfig;

/// A rate limit in bytes per second, shared by every clone.
///
/// ```
/// use vibe_core::progress::transfer::TransferLimit;
///
/// let limit = TransferLimit::parse("5 MB/s").unwrap().unwrap();
/// assert_eq!(limit.bytes_per_second(), 5_000_000);
/// assert_eq!(TransferLimit::parse("2MiB").unwrap().unwrap().bytes_per_second(), 2 << 20);
/// assert!(TransferLimit::parse("0").unwrap().is_none(), "zero means none");
/// assert!(TransferLimit::parse("fast").is_err());
/// ```
#[derive(Clone, Debug)]
pub struct TransferLimit {
    bytes_per_second: u64,
    bucket: Arc<Mutex<Bucket>>,
}

#[derive(Debug)]
struct Bucket {
    /// May go negative: a reader that overdrew sleeps the debt off, and
    /// the next one waits behind it.
    tokens: f64,
    at: Instant,
}

impl TransferLimit {
    /// A limit of `bytes_per_second`; zero is no limit.
    pub fn new(bytes_per_second: u64) -> Option<Self> {
        (bytes_per_second > 0).then(|| Self {
            bytes_per_second,
            // The bucket starts empty: a full one would let the first
            // quarter second through unthrottled, which a short download
            // would report as the limit being ignored.
            bucket: Arc::new(Mutex::new(Bucket {
                tokens: 0.0,
                at: Instant::now(),
            })),
        })
    }

    /// Read `"5 MB/s"`, `"500 KB"`, `"2MiB/s"` or a bare byte count;
    /// decimal units are powers of 1000, `KiB`/`MiB`/`GiB` of 1024, and
    /// `"0"` is no limit.
    pub fn parse(text: &str) -> Result<Option<Self>, String> {
        let trimmed = text.trim();
        let figure = trimmed.strip_suffix("/s").unwrap_or(trimmed).trim();
        let split = figure
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(figure.len());
        let (number, unit) = figure.split_at(split);
        let value: f64 = number
            .parse()
            .map_err(|_| format!("`{text}` is not a rate such as \"5 MB/s\""))?;
        let scale: f64 = match unit.trim().to_ascii_lowercase().as_str() {
            "" | "b" => 1.0,
            "k" | "kb" => 1e3,
            "m" | "mb" => 1e6,
            "g" | "gb" => 1e9,
            "kib" => 1024.0,
            "mib" => 1024.0 * 1024.0,
            "gib" => 1024.0 * 1024.0 * 1024.0,
            _ => {
                return Err(format!(
                    "`{text}` names an unknown unit; use B, KB, MB, GB or KiB, MiB, GiB"
                ));
            }
        };
        Ok(Self::new((value * scale).round() as u64))
    }

    /// The `[transfer]` table of the user-level config; no key is no limit.
    pub fn from_config(config: &TransferConfig) -> Result<Option<Self>, String> {
        match config.limit.as_deref() {
            None => Ok(None),
            Some(text) => Self::parse(text),
        }
    }

    pub fn bytes_per_second(&self) -> u64 {
        self.bytes_per_second
    }

    /// Spend `bytes`, sleeping as long as the bucket is in debt for them.
    /// The sleep happens outside the lock, so concurrent transfers queue on
    /// the shared debt instead of on each other's sleep.
    pub(crate) fn take(&self, bytes: u64) {
        let rate = self.bytes_per_second as f64;
        let wait = {
            let Ok(mut bucket) = self.bucket.lock() else {
                return;
            };
            let now = Instant::now();
            let refill = now.duration_since(bucket.at).as_secs_f64() * rate;
            // A quarter second of burst at most, so a pause does not bank
            // a long unthrottled stretch.
            bucket.tokens = (bucket.tokens + refill).min(rate / 4.0);
            bucket.at = now;
            bucket.tokens -= bytes as f64;
            (bucket.tokens < 0.0).then(|| Duration::from_secs_f64(-bucket.tokens / rate))
        };
        if let Some(wait) = wait {
            std::thread::sleep(wait);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bucket_holds_the_average_rate() {
        let limit = TransferLimit::new(4 << 20).unwrap();
        let started = Instant::now();
        for _ in 0..32 {
            limit.take(64 << 10);
        }
        // 2 MiB at 4 MiB/s: half a second, give or take scheduling.
        let elapsed = started.elapsed().as_secs_f64();
        assert!((0.4..1.0).contains(&elapsed), "{elapsed}");
    }
}
