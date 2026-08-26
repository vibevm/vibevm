//! One stateful Markdown fence machine shared by batch and streaming scanners.

/// The exact fence state at a stream boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FenceSnapshot {
    Closed,
    Open { delimiter: char, run: usize },
}

/// Streaming form of [`fence_mask`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FenceTracker {
    open: Option<(char, usize)>,
}

impl FenceTracker {
    /// Resume from an exact boundary state — the lane carries one per node, so
    /// a body wholly inside a fence its predecessor opened is read as fenced.
    pub(crate) fn from_snapshot(snapshot: FenceSnapshot) -> Self {
        Self {
            open: match snapshot {
                FenceSnapshot::Closed => None,
                FenceSnapshot::Open { delimiter, run } => Some((delimiter, run)),
            },
        }
    }

    pub(crate) fn snapshot(&self) -> FenceSnapshot {
        match self.open {
            Some((delimiter, run)) => FenceSnapshot::Open { delimiter, run },
            None => FenceSnapshot::Closed,
        }
    }

    /// Classify one complete logical line and advance the fence state.
    pub(crate) fn classify(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        match self.open {
            Some((delimiter, run)) => {
                if closes_fence(trimmed, delimiter, run) {
                    self.open = None;
                }
                true
            }
            None => {
                if let Some(open) = fence_run(trimmed) {
                    self.open = Some(open);
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// A precomputed mask marking lines inside fenced code blocks, including the
/// fence lines themselves.
pub(crate) fn fence_mask(lines: &[String]) -> Vec<bool> {
    mask_from(lines, FenceSnapshot::Closed)
}

/// [`fence_mask`] for a fragment that resumes at a known boundary.
pub(crate) fn mask_from(lines: &[String], snapshot: FenceSnapshot) -> Vec<bool> {
    let mut tracker = FenceTracker::from_snapshot(snapshot);
    lines.iter().map(|line| tracker.classify(line)).collect()
}

/// The fence run a line opens with — its character and how many of it.
fn fence_run(trimmed: &str) -> Option<(char, usize)> {
    let delimiter = trimmed.chars().next()?;
    if delimiter != '`' && delimiter != '~' {
        return None;
    }
    let run = trimmed
        .chars()
        .take_while(|character| *character == delimiter)
        .count();
    (run >= 3).then_some((delimiter, run))
}

/// A closer is the same delimiter, at least the opener length, and nothing
/// else on the line. An info string can open but never close a fence.
fn closes_fence(trimmed: &str, delimiter: char, open_run: usize) -> bool {
    fence_run(trimmed).is_some_and(|(candidate, run)| candidate == delimiter && run >= open_run)
        && trimmed
            .trim_end()
            .chars()
            .all(|character| character == delimiter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_and_batch_preserve_long_tilde_and_info_string_rules() {
        let lines: Vec<String> = [
            "````markdown",
            "```",
            "# still quoted",
            "````",
            "outside",
            "~~~rust",
            "~~~not-a-close",
            "inside",
            "~~~~",
            "outside again",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let expected = vec![true, true, true, true, false, true, true, true, true, false];
        assert_eq!(fence_mask(&lines), expected);

        let mut tracker = FenceTracker::default();
        let actual: Vec<bool> = lines.iter().map(|line| tracker.classify(line)).collect();
        assert_eq!(actual, expected);
        assert_eq!(tracker.snapshot(), FenceSnapshot::Closed);
    }

    #[test]
    fn snapshot_carries_delimiter_and_run_across_chunks() {
        let mut tracker = FenceTracker::default();
        assert!(tracker.classify("  ~~~~lang"));
        assert_eq!(
            tracker.snapshot(),
            FenceSnapshot::Open {
                delimiter: '~',
                run: 4,
            }
        );
        assert!(tracker.classify("~~~"));
        assert!(matches!(tracker.snapshot(), FenceSnapshot::Open { .. }));
        assert!(tracker.classify("~~~~~"));
        assert_eq!(tracker.snapshot(), FenceSnapshot::Closed);
    }
}
