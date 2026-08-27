//! Bounded reads: the same capability walk and the same single-link law as
//! [`read_file`](crate::Project::read_file), with an allocation ceiling.
//!
//! `read_file` is for files this project just wrote and just verified. Every
//! other read — state and task documents a later phase inherits — is one a
//! caller cannot size in advance, and an unbounded `read_to_end` there turns a
//! corrupted or hostile length into an unbounded allocation. The bounded entry
//! points keep every refusal the ordinary read enforces and add exactly one
//! law: the answer is at most `cap` bytes, or it is an error — never a
//! truncated prefix presented as the file.

use std::io::Read;

use anyhow::{Context, Result, bail};

use crate::Pinned;
use crate::Project;
use crate::file::cap_options;
use crate::file::verify_regular_single_link;

/// How many bytes one bounded read asks the handle for at a time. A fixed
/// stack window: the loop's exact reservations, not this size, shape the
/// returned buffer.
const READ_CHUNK: usize = 16 * 1024;

impl Project {
    /// Read one file at a project-relative path, or `None` when absent.
    ///
    /// The answer is at most `cap` bytes, and the buffer it lands in is
    /// bounded mechanically, not by trust: bytes are taken through a fixed
    /// window and appended after an exact reservation, so its capacity stays
    /// metadata-derived on a stable read and never passes the cap even when
    /// the file grows mid-read — `read_to_end`'s geometric growth is never
    /// used.
    ///
    /// A file whose metadata length (or growth mid-read) exceeds `cap`
    /// refuses with the real length and the cap, so the caller can raise the
    /// cap or split the read; it never receives a truncated prefix.
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let home = tempfile::tempdir()?;
    /// let project = vibe_safefs::Project::open(home.path())?;
    /// project.write_atomic("docs/state.json", br#"{"ok":true}"#)?;
    /// let bytes = project.read_file_bounded("docs/state.json", 1 << 20)?;
    /// assert_eq!(bytes.as_deref(), Some(br#"{"ok":true}"#.as_slice()));
    /// assert!(project.read_file_bounded("docs/state.json", 4).is_err());
    /// assert_eq!(project.read_file_bounded("docs/missing.json", 64)?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_file_bounded(&self, relative: &str, cap: usize) -> Result<Option<Vec<u8>>> {
        let root = self.root_dir()?;
        self.read_file_bounded_in(&root, relative, cap)
    }

    /// The same bounded read below an already-pinned `directory`, or `None`
    /// when absent. A link, a non-regular file, or a hard link count != 1
    /// refuses exactly as in [`read_file_in`](Self::read_file_in).
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let home = tempfile::tempdir()?;
    /// let project = vibe_safefs::Project::open(home.path())?;
    /// project.write_atomic("docs/state.json", b"12345")?;
    /// let docs = project.dir(&["docs"], false)?;
    /// let bytes = project.read_file_bounded_in(&docs, "state.json", 5)?;
    /// assert_eq!(bytes, Some(b"12345".to_vec()));
    /// assert!(project.read_file_bounded_in(&docs, "state.json", 4).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_file_bounded_in(
        &self,
        directory: &Pinned,
        relative: &str,
        cap: usize,
    ) -> Result<Option<Vec<u8>>> {
        // The fence below leans on `cap + 1` being exactly one past the
        // accepted maximum. `usize::MAX` would wrap it to zero and turn the
        // guard into an empty read, so that cap is a caller error, and it is
        // judged before anything is opened: the refusal cannot depend on the
        // file's state.
        let limit = cap
            .checked_add(1)
            .with_context(|| format!("cap {cap} is usize::MAX: cap + 1 would overflow"))?;
        let Some((holder, name)) = self.holder_of(directory, relative)? else {
            return Ok(None);
        };
        let display = holder.join(&name);
        let mut options = cap_options();
        match holder.dir.open_with(&name, options.read(true)) {
            Ok(file) => {
                let std_file = file.into_std();
                verify_regular_single_link(&std_file, &display)?;
                // One opened final handle is the whole read epoch: the size
                // check and the read see the same open file, so nothing can be
                // swapped between them. The metadata length only *shapes* the
                // allocation — the file can still grow inside the epoch, which
                // is why the read itself stays fenced.
                let metadata_len = std::fs::File::metadata(&std_file)
                    .with_context(|| format!("inspecting `{}`", display.display()))?
                    .len();
                if metadata_len > cap as u64 {
                    bail!(
                        "`{}` is {metadata_len} bytes, over the {cap}-byte cap; raise the cap or \
                         read the file in parts",
                        display.display()
                    );
                }
                // Already cap-bounded by the check above, so the opening
                // reservation is the smaller metadata-derived capacity — never
                // `cap`-sized on the ordinary metadata-stable path, and never
                // `cap + 1`.
                let mut bytes = Vec::with_capacity(metadata_len as usize);
                crate::race_hook::before_bounded_read(&holder, &name);
                // The read is fenced by `take(cap + 1)`, and the buffer below
                // it is bounded mechanically, not by trust: `read_to_end` on a
                // metadata-sized buffer may grow it geometrically past any
                // promise (measured: six reserved bytes growing to twelve
                // under a cap of eight), so bytes are taken through a fixed
                // stack window and appended only after an exact `reserve_exact`
                // top-up. The buffer's capacity therefore never exceeds the
                // larger of the metadata-derived reservation and the appended
                // length — and the appended length never exceeds `cap`.
                let mut fenced = std_file.take(limit as u64);
                let mut chunk = [0u8; READ_CHUNK];
                loop {
                    let used = match fenced.read(&mut chunk) {
                        Ok(used) => used,
                        // The standard `Read` convention: a signal can surface
                        // as one transient, byte-less Interrupted. Continuing
                        // the loop is not a hidden retry of a failed read — it
                        // is the same read's next attempt, visible right here.
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) => {
                            return Err(anyhow::Error::new(error)
                                .context(format!("reading `{}`", display.display())));
                        }
                    };
                    if used == 0 {
                        return Ok(Some(bytes));
                    }
                    // Loop invariant: `bytes.len() <= cap`, so the subtraction
                    // cannot underflow and the comparison cannot overflow. One
                    // chunk past the cap is the growth (or stale-size) case:
                    // refuse, never append a truncated prefix of it.
                    if used > cap - bytes.len() {
                        bail!(
                            "`{}` yielded {used} more bytes at offset {} under the {cap}-byte cap \
                             though its metadata said {metadata_len}; it grew while being read, \
                             and a truncated prefix is not the file's content — raise the cap or \
                             re-read",
                            display.display(),
                            bytes.len()
                        );
                    }
                    // Exact top-up: the reservation fits precisely the bytes
                    // being appended, so no geometric growth can carry the
                    // buffer past the cap.
                    bytes.reserve_exact(used);
                    bytes.extend_from_slice(&chunk[..used]);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(anyhow::Error::new(error).context(format!("opening `{}`", display.display())))
            }
        }
    }
}
