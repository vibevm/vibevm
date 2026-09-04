//! Concurrent, bounded stdout/stderr evidence.

use std::io::Read;

use sha2::{Digest, Sha256};

use super::model::{HealthError, StreamEvidence, Utf8State};

const DRAIN_CHUNK: usize = 16 * 1024;

pub struct StreamAccumulator {
    cap: usize,
    total: u64,
    digest: Sha256,
    utf8: Utf8Validator,
}

impl StreamAccumulator {
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            total: 0,
            digest: Sha256::new(),
            utf8: Utf8Validator::default(),
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<(), HealthError> {
        self.total = self
            .total
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| HealthError::Execution("stream byte count overflow".to_owned()))?;
        self.digest.update(bytes);
        self.utf8.push(bytes);
        Ok(())
    }

    #[must_use]
    pub fn finish(self) -> StreamEvidence {
        StreamEvidence {
            total_bytes: self.total,
            sha256: format!("sha256:{:x}", self.digest.finalize()),
            truncated: self.total > self.cap as u64,
            utf8: if self.utf8.finish() {
                Utf8State::Valid
            } else {
                Utf8State::Invalid
            },
            // Raw excerpts are never allowed to cross into persistent
            // evidence without an explicit secret classifier. Epoch 1 uses
            // the conservative policy: retain exact digest/count/UTF-8 state
            // and fully redact both excerpts.
            head: Vec::new(),
            tail: Vec::new(),
        }
    }
}

pub fn drain_concurrently<Out, Err>(
    stdout: Out,
    stderr: Err,
    stdout_cap: usize,
    stderr_cap: usize,
) -> Result<(StreamEvidence, StreamEvidence), HealthError>
where
    Out: Read + Send,
    Err: Read + Send,
{
    std::thread::scope(|scope| {
        let out = scope.spawn(move || drain(stdout, stdout_cap, "stdout"));
        let err = scope.spawn(move || drain(stderr, stderr_cap, "stderr"));
        let out = out
            .join()
            .map_err(|_| HealthError::Execution("stdout drain panicked".to_owned()))??;
        let err = err
            .join()
            .map_err(|_| HealthError::Execution("stderr drain panicked".to_owned()))??;
        Ok((out, err))
    })
}

fn drain<R: Read>(mut reader: R, cap: usize, name: &str) -> Result<StreamEvidence, HealthError> {
    let mut accumulator = StreamAccumulator::new(cap);
    let mut chunk = [0_u8; DRAIN_CHUNK];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(accumulator.finish()),
            Ok(used) => accumulator.push(&chunk[..used])?,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(HealthError::Execution(format!(
                    "reading child {name}: {error}"
                )));
            }
        }
    }
}

#[derive(Default)]
struct Utf8Validator {
    valid: bool,
    started: bool,
    remaining: u8,
    next_min: u8,
    next_max: u8,
}

impl Utf8Validator {
    fn push(&mut self, bytes: &[u8]) {
        if !self.started {
            self.started = true;
            self.valid = true;
            self.next_min = 0x80;
            self.next_max = 0xbf;
        }
        if !self.valid {
            return;
        }
        for &byte in bytes {
            if self.remaining != 0 {
                if byte < self.next_min || byte > self.next_max {
                    self.valid = false;
                    return;
                }
                self.remaining -= 1;
                self.next_min = 0x80;
                self.next_max = 0xbf;
                continue;
            }
            match byte {
                0x00..=0x7f => {}
                0xc2..=0xdf => self.sequence(1, 0x80, 0xbf),
                0xe0 => self.sequence(2, 0xa0, 0xbf),
                0xe1..=0xec | 0xee..=0xef => self.sequence(2, 0x80, 0xbf),
                0xed => self.sequence(2, 0x80, 0x9f),
                0xf0 => self.sequence(3, 0x90, 0xbf),
                0xf1..=0xf3 => self.sequence(3, 0x80, 0xbf),
                0xf4 => self.sequence(3, 0x80, 0x8f),
                _ => {
                    self.valid = false;
                    return;
                }
            }
        }
    }

    fn sequence(&mut self, remaining: u8, next_min: u8, next_max: u8) {
        self.remaining = remaining;
        self.next_min = next_min;
        self.next_max = next_max;
    }

    const fn finish(self) -> bool {
        (!self.started || self.valid) && self.remaining == 0
    }
}
