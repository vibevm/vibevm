//! Streamed content identity: what a file's bytes are, for a file of any size.
//!
//! The bounded read answers "give me these bytes" and therefore owes an
//! allocation ceiling. This cell answers a different question — "what is this
//! file's content identity" — and a ceiling there is the wrong instrument: a
//! declared prebuilt binary or OS image is legitimately multi-gigabyte, and
//! refusing to witness it for its size would make the honest answer depend on
//! how much RAM the observer happened to have. So nothing here is retained:
//! bytes pass through one fixed stack window into a digest, and the answer is
//! 40 bytes wide whatever the file weighs.
//!
//! ## Why twice
//!
//! One pass proves nothing about content stability. A writer that rewrites
//! bytes in place at the same length, behind a reader that has already passed
//! them, yields a digest over a mixture of old and new content that never
//! existed as a file state — a torn read, invisible to identity and invisible
//! to length. Two passes over the SAME held handle, with a seek between them,
//! turn that into a disagreement and therefore a refusal.
//!
//! The second pass re-reads the pinned object, never a re-opened name: a fresh
//! open would introduce a second object needing its own reconciliation, which
//! is the very ambiguity the held handle exists to remove. The final-name
//! proof is separate and comes last, so it covers both passes.
//!
//! This is detection-bound and says so. It costs 2× the file's bytes per
//! observation, deliberately. What it does not claim: that no mutation
//! occurred, only that none was observed — a rewrite after the last pass, or
//! wholly between them and reverted, is outside what any number of passes can
//! see.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use specmark::spec;

use crate::file::bounded::{READ_CHUNK, ensure_still_final_name};
use crate::file::identity::file_identity;
use crate::file::{cap_options, verify_regular_single_link};
use crate::project::{Pinned, Project};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS");

/// One regular file's proven content identity: how many bytes it holds and the
/// raw SHA-256 of exactly those bytes.
///
/// The digest is raw, not hex. Callers frame it into a domain-separated outer
/// recipe, and a second hex-case vocabulary between here and there is one more
/// thing two implementations can spell differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS")]
pub struct ContentDigest {
    /// Exact byte count, agreed by both passes and by the handle's metadata
    /// before and after them.
    pub len: u64,
    /// Raw SHA-256 over the exact bytes — the same 32 bytes `sha256sum`
    /// prints, so an operator can check one by hand.
    pub sha256: [u8; 32],
}

impl Project {
    /// Prove the content identity of one direct child file of `directory`,
    /// streaming it with no size cap and no retained content.
    ///
    /// `Ok(None)` is absence, and only at the initial open: once a handle has
    /// been read, a vanished name is a refusal, not an answer. Every other
    /// outcome short of a proven digest is an `Err` naming the path and what
    /// disagreed — a directory, a device, a link or reparse point, a name
    /// shared as a hard link, a length or digest the two passes did not agree
    /// on, or a final name that stopped denoting the object that supplied the
    /// bytes. There is deliberately no truncation, no retry as a fresh epoch
    /// and no partial answer: half a content identity is not a smaller claim,
    /// it is a false one.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS"
    )]
    pub fn digest_file_in(&self, directory: &Pinned, name: &str) -> Result<Option<ContentDigest>> {
        crate::component::ensure_safe_component(name)?;
        let display = directory.join(name);
        let mut options = cap_options();
        let opened = match directory.dir.open_with(name, options.read(true)) {
            Ok(opened) => opened,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(anyhow::Error::new(error).context(format!(
                    "opening `{}` without following a link",
                    display.display()
                )));
            }
        };
        let mut held = opened.into_std();
        verify_regular_single_link(&held, &display)?;
        // Taken from the handle, before either pass: this is the object the
        // digest will be about, and the final-name proof below is handed this
        // identity rather than re-deriving one from a handle it did not open.
        let identity = file_identity(&held, &display)?;
        let opening_len = handle_len(&held, &display, "before the first pass")?;

        let first = stream_pass(&mut held, &display, Pass::First)?;
        crate::race_hook::between_stream_passes(directory, name);
        rewind(&mut held, &display)?;
        let second = stream_pass(&mut held, &display, Pass::Second)?;
        let closing_len = handle_len(&held, &display, "after the second pass")?;

        // Four counts, one claim. Metadata alone can be stale and a single
        // pass alone can be torn, so the length is only believed where the
        // filesystem and both readings of the object agree on it.
        if first.len != second.len || first.len != opening_len || first.len != closing_len {
            bail!(
                "`{}` did not hold still while it was measured: metadata said {opening_len} bytes \
                 before and {closing_len} after, the first pass read {} and the second {}; the \
                 file was written while being read — re-measure",
                display.display(),
                first.len,
                second.len
            );
        }
        if first.sha256 != second.sha256 {
            bail!(
                "`{}` changed content while it was measured: two passes over the same {} bytes of \
                 the same object produced different SHA-256 digests, so neither is the file's \
                 content — re-measure",
                display.display(),
                first.len
            );
        }

        ensure_still_final_name(directory, name, identity, &display)?;
        Ok(Some(ContentDigest {
            len: first.len,
            sha256: first.sha256,
        }))
    }
}

/// Which reading of the object a refusal is about, so an I/O error names the
/// pass it interrupted instead of an anonymous "read failed".
#[derive(Clone, Copy)]
enum Pass {
    First,
    Second,
}

impl Pass {
    const fn label(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Second => "second",
        }
    }
}

/// What one pass established. Fixed width — this is the whole reason the file
/// size never becomes an allocation.
struct PassResult {
    len: u64,
    sha256: [u8; 32],
}

/// Stream the whole handle from its current offset into a digest through one
/// fixed stack window, counting bytes in checked `u64`.
///
/// Nothing accumulates: the window is reused, the digest is 32 bytes and the
/// count is 8, so a terabyte and an empty file cost the same memory.
fn stream_pass(held: &mut std::fs::File, display: &Path, pass: Pass) -> Result<PassResult> {
    let mut hash = Sha256::new();
    let mut len = 0_u64;
    let mut window = [0_u8; READ_CHUNK];
    loop {
        let used = match held.read(&mut window) {
            Ok(used) => used,
            // The standard convention: a signal can surface as one transient,
            // byte-less `Interrupted`. Continuing is not a hidden retry of a
            // failed read — it is the same read's next attempt, visible here.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(anyhow::Error::new(error).context(format!(
                    "reading `{}` during the {} content pass",
                    display.display(),
                    pass.label()
                )));
            }
        };
        if used == 0 {
            return Ok(PassResult {
                len,
                sha256: hash.finalize().into(),
            });
        }
        hash.update(&window[..used]);
        // A count that wrapped would understate a file by exactly the amount
        // that made it interesting, so it refuses instead.
        len = len.checked_add(used as u64).ok_or_else(|| {
            anyhow::anyhow!(
                "`{}` yielded more than u64::MAX bytes during the {} content pass",
                display.display(),
                pass.label()
            )
        })?;
    }
}

/// Put the SAME handle back at offset zero for the second pass. A re-open here
/// would answer for whatever the name holds now, which is the substitution the
/// held handle exists to prevent.
fn rewind(held: &mut std::fs::File, display: &Path) -> Result<()> {
    let landed = held
        .seek(SeekFrom::Start(0))
        .with_context(|| format!("rewinding `{}` for its second pass", display.display()))?;
    if landed != 0 {
        bail!(
            "rewinding `{}` for its second pass landed at offset {landed}, not 0",
            display.display()
        );
    }
    Ok(())
}

/// The handle's own length, asked of the object rather than of the name.
fn handle_len(held: &std::fs::File, display: &Path, when: &str) -> Result<u64> {
    Ok(std::fs::File::metadata(held)
        .with_context(|| format!("inspecting `{}` {when}", display.display()))?
        .len())
}
