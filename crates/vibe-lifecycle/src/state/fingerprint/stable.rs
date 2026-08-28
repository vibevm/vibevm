//! Certified stable observation of one selected declared-input file
//! (PROP-054 `##EVIDENCE-MEASUREMENT-CARRIAGE`).
//!
//! The two-read law: an accepted file is one whose no-follow single-link
//! proof and length stayed equal around TWO consecutive capability-relative
//! bounded reads that returned identical bytes. This is detection-bound —
//! it proves the bytes came stably from one object across the observation
//! window, never that an adversarial writer could not have rewritten the
//! same bytes after the final inspect.
//!
//! Refusal here is EVIDENCE-only. The caller still owes the legacy execution
//! fingerprint one ordinary raw read (HEAD's exact behaviour, hardlinks
//! included), so enabling evidence can never change freshness or veto a
//! handler. The refusal stays a typed crate-private fact: A5 will need to
//! distinguish an unavailable baseline from a refused re-observation, and no
//! machine path or body ever enters wire state through it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-MEASUREMENT-CARRIAGE");

use specmark::spec;
use vibe_safefs::Project;

/// Why a declared-input evidence measurement was refused. Bounded, typed and
/// internal — a reason code, never a path, a body or an IO error verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputRefusal {
    /// A selected entry is not a regular file and not a directory — a link,
    /// junction, reparse point or device. The legacy scanner never read it
    /// and must never follow it; its selection alone refuses the manifest.
    NonRegular,
    /// Two selected paths are one physical file under portable
    /// case/normalisation-folded identities.
    Aliased,
    /// The no-follow capability descent or the single-link inspect refused —
    /// a hard link among them (`nlink != 1`).
    Open,
    /// A bounded capability read refused — including growth over the proven
    /// length cap inside the read fence.
    Read,
    /// Proof or length moved across the observation window.
    Unstable,
    /// The two consecutive reads returned different bytes.
    Disagree,
}

/// Observe one selected regular union path and return its CERTIFIED bytes.
///
/// `relative` is the canonical forward-slashed project-relative path. The
/// per-file cap is the PRE-proof length (narrowed checked to `usize`), so the
/// allocation matches the legacy read's and any growth inside the window
/// refuses itself instead of truncating.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-MEASUREMENT-CARRIAGE")]
pub(super) fn observe(project: &Project, relative: &str) -> Result<Vec<u8>, InputRefusal> {
    let mut components = relative.split('/').filter(|segment| !segment.is_empty());
    let name = components.next_back().ok_or(InputRefusal::Open)?;
    let parent: Vec<&str> = components.collect();
    let parent = project
        .dir(&parent, false)
        .map_err(|_| InputRefusal::Open)?;

    let Some((proof_before, length_before)) = project
        .inspect_file_in(&parent, name)
        .map_err(|_| InputRefusal::Open)?
    else {
        return Err(InputRefusal::Unstable);
    };
    let cap = usize::try_from(length_before).map_err(|_| InputRefusal::Read)?;
    let first = project.read_file_bounded_in(&parent, name, cap);
    fire_between_reads(relative);
    let second = project.read_file_bounded_in(&parent, name, cap);
    fire_after_reads(relative);
    let (first, second) = match (first, second) {
        (Ok(Some(first)), Ok(Some(second))) => (first, second),
        (Err(_), _) | (_, Err(_)) => return Err(InputRefusal::Read),
        (Ok(None), _) | (_, Ok(None)) => return Err(InputRefusal::Unstable),
    };
    let Some((proof_after, length_after)) = project
        .inspect_file_in(&parent, name)
        .map_err(|_| InputRefusal::Open)?
    else {
        return Err(InputRefusal::Unstable);
    };
    if proof_before != proof_after
        || length_before != length_after
        || first.len() != second.len()
        || first.len() as u64 != length_before
    {
        return Err(InputRefusal::Unstable);
    }
    if first != second {
        return Err(InputRefusal::Disagree);
    }
    Ok(first)
}

/// Fire the test-only between-reads seam. In a non-test build this is — and
/// must be — a no-op compiled to nothing: the release protocol has no
/// window hook at all.
#[cfg(test)]
fn fire_between_reads(relative: &str) {
    inject::between_reads(relative);
}

#[cfg(not(test))]
fn fire_between_reads(_relative: &str) {}

/// Fire the test-only after-reads seam (both reads done, post-inspect next).
#[cfg(test)]
fn fire_after_reads(relative: &str) {
    inject::after_reads(relative);
}

#[cfg(not(test))]
fn fire_after_reads(_relative: &str) {}

/// Test-only seams in the observation windows a real race occupies: between
/// the two bounded reads, and after both before the post-inspect. They are
/// the deterministic stand-ins for a concurrent writer and are compiled out
/// of every non-test build entirely.
#[cfg(test)]
pub(crate) mod inject {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&str)>;

    thread_local! {
        static BETWEEN_READS: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static AFTER_READS: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Arm (or clear with `None`) the one-shot hook fired between the two
    /// bounded reads of the next observed path.
    pub(crate) fn arm_between_reads(hook: Option<Hook>) {
        BETWEEN_READS.with(|slot| *slot.borrow_mut() = hook);
    }

    /// Arm (or clear with `None`) the one-shot hook fired after both reads,
    /// immediately before the post-inspect.
    pub(crate) fn arm_after_reads(hook: Option<Hook>) {
        AFTER_READS.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(super) fn between_reads(relative: &str) {
        if let Some(hook) = BETWEEN_READS.with(|slot| slot.borrow_mut().take()) {
            hook(relative);
        }
    }

    pub(super) fn after_reads(relative: &str) {
        if let Some(hook) = AFTER_READS.with(|slot| slot.borrow_mut().take()) {
            hook(relative);
        }
    }
}
