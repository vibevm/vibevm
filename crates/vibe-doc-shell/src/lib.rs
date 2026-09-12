//! `vibe-doc-shell` — the reader's shell, from wherever it came from
//! (PROP-057 `##SHELL-SERVE-SOURCES`, `##SHELL-XTASK-EMBED`).
//!
//! A documentation page a person reads is two things glued together: the
//! ISLAND, which the Rust pipeline renders out of the package, and the
//! SHELL around it — the head, the styles, the navigation, the behaviour
//! — which is a build of the site package and has nothing to do with any
//! one package's content. The island is `vibe-doc`'s. The shell is this
//! crate's, and the only question this crate answers is «where are the
//! bytes».
//!
//! ## Three sources, one type
//!
//! - **Embedded.** A release build turns on `embedded-shell` and the
//!   shell is compiled into the binary through `include_dir`. Nothing to
//!   fetch, nothing to install, nothing to go missing.
//! - **Store.** A build from source has no shell, so `vibe doc shell
//!   install` downloads the one matching this version — with the
//!   operator's explicit consent, never automatically — into the shared
//!   content-addressed directory `<install root>/vibevm/doc-shell/<sha256>/`
//!   (`##SHELL-INSTALL-COMMAND`).
//! - **Fallback.** Neither: the bare shell in [`fallback`], which is
//!   typography and no scripts. It is a working reader, not an error
//!   page, and it is what «the local reader never touches the network
//!   without consent» costs when nobody consented (`##LOCAL-OFFLINE-SHELL`).
//!
//! Nothing above this crate knows which of the three it got. [`Shell`] is
//! one type with one `asset` call, and [`Shell::provenance`] is the
//! answer to «what is this reader carrying» that `vibe doc serve` can
//! print (`##SHELL-PIN`).
//!
//! ## What it does not do
//!
//! It serves nothing — there is no HTTP here, and the crate does not
//! depend on axum or on `vibe-doc`. It reads no ambient environment: the
//! store root arrives as an argument, resolved at the composition root
//! like every other path in this tree. And it renders nothing: the
//! template arrives finished from the build, and the edits made to it are
//! the few a server must make — the island into its marker, the title and
//! the projection addresses of the page actually being served, and the
//! base a reader mounted at ([`template`]).
//!
//! Every one of those edits is a value REPLACED where it already stands.
//! Nothing is inserted into a served head and nothing is reordered in
//! one: the framework's resumption depends on the head being the head it
//! serialised, and a page that stops resuming stops doing anything at all
//! — silently. [`template`] carries the measurement.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-SERVE-SOURCES");

pub mod digest;
pub mod error;
pub mod fallback;
pub mod index;
pub mod pin;
pub mod template;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use error::{ShellError, ShellResult};
pub use index::Index;
pub use pin::Pin;

/// The base the shell is built with, and the reader's default: the same
/// mount the public site uses, so a link written on a page works in both
/// worlds.
pub const DEFAULT_BASE: &str = "/doc/";

/// The marker the embeddable adapter leaves where the island goes. The
/// shell's own index carries it too; this is the value a fallback shell
/// and a test use when there is no index to ask.
pub const ISLAND_MARKER: &str = "<!--vibe-doc-island-->";

/// The directory a downloaded shell lives under, inside the install root.
/// Beside `versions/` and `build/`, not inside a version instance: the
/// instances are immutable (PROP-019 §2.4), and one download serves every
/// instance that pins the same digest.
pub const STORE_DIR: &str = "vibevm/doc-shell";

/// The digest the build recorded for the shell in this binary, or the
/// empty string when this binary carries none.
pub const DECLARED_DIGEST: &str = env!("VIBE_DOC_SHELL_SHA256");

#[cfg(feature = "embedded-shell")]
mod embedded {
    use include_dir::{Dir, include_dir};
    pub static DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/shell");
}

/// Where a reader's shell came from (`##SHELL-PIN`).
///
/// ```
/// use vibe_doc_shell::{Provenance, Shell};
///
/// // The canonical use: ask a shell what it is, and print the word.
/// let shell = Shell::bare();
/// assert_eq!(shell.provenance(), Provenance::Fallback);
/// assert_eq!(shell.provenance().as_str(), "fallback");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Compiled into this binary.
    Embedded,
    /// Downloaded, with consent, into the machine store.
    Store,
    /// The bare shell this crate carries.
    Fallback,
}

impl Provenance {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Embedded => "embedded",
            Provenance::Store => "store",
            Provenance::Fallback => "fallback",
        }
    }
}

/// The shell a reader is carrying.
///
/// ```
/// use vibe_doc_shell::{Provenance, Shell};
///
/// // The canonical use: take the best shell available, then serve a page
/// // through it without asking which one it turned out to be.
/// let shell = Shell::open(None);
/// assert!(shell.page_template().contains("<!--vibe-doc-island-->"));
///
/// // Nothing embedded and nothing in the store is still a reader — the
/// // bare shell, which is typography and no scripts.
/// let bare = Shell::bare();
/// assert_eq!(bare.provenance(), Provenance::Fallback);
/// assert!(!bare.page_template().contains("<script"));
/// ```
#[derive(Debug)]
pub struct Shell {
    source: Source,
    index: Index,
}

#[derive(Debug)]
enum Source {
    #[cfg(feature = "embedded-shell")]
    Embedded,
    Store(PathBuf),
    Fallback,
}

impl Shell {
    /// Open the best shell available, in the order of the three sources.
    ///
    /// Infallible by design. A shell is the one thing a reader can always
    /// do without: a store directory that does not parse, or is not
    /// there, means the bare shell rather than a refusal to start, and
    /// [`Shell::provenance`] is how a caller sees which it got.
    /// [`Shell::inspect`] is the fallible sibling for the surfaces whose
    /// job is to report on a shell rather than serve one.
    pub fn open(store_root: Option<&Path>) -> Shell {
        Shell::inspect(store_root).unwrap_or_else(|_| Shell {
            source: Source::Fallback,
            index: Index::fallback(),
        })
    }

    /// The bare shell, asked for by name.
    ///
    /// [`Shell::open`] reaches it as a last resort; this is for the two
    /// callers that want it on purpose — a test that means to exercise
    /// the bare reader whatever the build carries, and a reader told to
    /// serve it.
    pub fn bare() -> Shell {
        Shell {
            source: Source::Fallback,
            index: Index::fallback(),
        }
    }

    /// The same, refusing instead of falling back when a store directory
    /// was named and is unreadable.
    pub fn inspect(store_root: Option<&Path>) -> ShellResult<Shell> {
        #[cfg(feature = "embedded-shell")]
        if let Some(file) = embedded::DIR.get_file(index::INDEX_FILE) {
            let text = String::from_utf8_lossy(file.contents());
            let index = Index::parse(&text, Path::new(index::INDEX_FILE))?;
            return Ok(Shell {
                source: Source::Embedded,
                index,
            });
        }
        let Some(root) = store_root else {
            return Ok(Shell {
                source: Source::Fallback,
                index: Index::fallback(),
            });
        };
        let path = root.join(index::INDEX_FILE);
        if !path.is_file() || !root.join(index::PAGE_TEMPLATE).is_file() {
            return Err(ShellError::NoShell {
                path: root.to_path_buf(),
            });
        }
        let text = std::fs::read_to_string(&path).map_err(|e| ShellError::io(&path, &e))?;
        let index = Index::parse(&text, &path)?;
        Ok(Shell {
            source: Source::Store(root.to_path_buf()),
            index,
        })
    }

    /// What this reader is carrying.
    pub fn provenance(&self) -> Provenance {
        match self.source {
            #[cfg(feature = "embedded-shell")]
            Source::Embedded => Provenance::Embedded,
            Source::Store(_) => Provenance::Store,
            Source::Fallback => Provenance::Fallback,
        }
    }

    /// What the shell says about itself.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// One file of the shell, by its `/`-separated relative path.
    ///
    /// A path that is not exactly a sequence of ordinary names answers
    /// `None` before the filesystem is touched — the same refusal the
    /// reader's own routes make, made again here because this is the
    /// other place a request's bytes could become a path
    /// (`##LOCAL-STATIC`).
    pub fn asset(&self, relative: &str) -> Option<Vec<u8>> {
        if !is_plain_path(relative) {
            return None;
        }
        match &self.source {
            #[cfg(feature = "embedded-shell")]
            Source::Embedded => embedded::DIR
                .get_file(relative)
                .map(|file| file.contents().to_vec()),
            Source::Store(root) => {
                let path = under(root, relative)?;
                std::fs::read(path).ok()
            }
            Source::Fallback => match relative {
                fallback::STYLESHEET => Some(fallback::STYLESHEET_CSS.as_bytes().to_vec()),
                _ => None,
            },
        }
    }

    /// The route template the server glues an island into.
    pub fn page_template(&self) -> String {
        match self.asset(index::PAGE_TEMPLATE) {
            Some(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            None => fallback::template(&self.index.base),
        }
    }

    /// Every file of the shell, for a fresh measurement of what is
    /// actually here.
    ///
    /// The index is not among them, and cannot be: it carries the digest
    /// of the shell, so a digest that included it would be a hash of its
    /// own value. It stands to the shell as a lock file stands to what it
    /// locks.
    pub fn files(&self) -> BTreeMap<String, Vec<u8>> {
        let mut out = match &self.source {
            #[cfg(feature = "embedded-shell")]
            Source::Embedded => {
                let mut found = BTreeMap::new();
                collect_embedded(&embedded::DIR, &mut found);
                found
            }
            Source::Store(root) => digest::read_tree(root).unwrap_or_default(),
            Source::Fallback => BTreeMap::new(),
        };
        out.remove(index::INDEX_FILE);
        out
    }

    /// The digest of the bytes this reader is carrying right now, as
    /// opposed to the one the build wrote down.
    pub fn measured_digest(&self) -> String {
        match self.source {
            Source::Fallback => String::new(),
            _ => digest::of(&self.files()),
        }
    }

    /// The shell's whole state, for the command that reports on it.
    pub fn report(&self) -> Report {
        let measured = self.measured_digest();
        let pin = Pin::compiled_in().ok();
        let files = self.files();
        Report {
            provenance: self.provenance(),
            package: self.index.package.clone(),
            version: self.index.version.clone(),
            base: self.index.base.clone(),
            file_count: files.len() as u32,
            byte_count: files.values().map(|bytes| bytes.len() as u64).sum(),
            declared: DECLARED_DIGEST.to_string(),
            recorded: self.index.sha256.clone(),
            measured,
            pin,
        }
    }
}

/// Is `relative` a plain `/`-separated sequence of ordinary names?
///
/// A colon is refused along with the obvious things, and that is not
/// belt-and-braces: on Windows a segment like `C:` carries a DRIVE PREFIX,
/// and [`std::path::PathBuf::push`] of a prefixed component replaces the
/// whole path rather than extending it. Measured, not assumed —
/// `PathBuf::from(r"C:\root\sub")` pushed `C:` then `Windows` comes out
/// as `C:Windows`, which is drive-relative to the process's own directory
/// and nowhere near the shell. No file the bundler writes has a colon in
/// its name, so nothing legitimate is lost.
fn is_plain_path(relative: &str) -> bool {
    !relative.is_empty()
        && relative.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && !segment.contains('\\')
                && !segment.contains('\0')
                && !segment.contains(':')
        })
}

/// Join `relative` under `root`, or `None` when the result is not under
/// it after all.
///
/// The check above says the NAME is ordinary; this says the PATH stayed
/// where it was put. Two checks on one question, because they answer it
/// differently: one reads the request, the other reads what the platform
/// made of it, and the platform is the half that surprises.
fn under(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        path.push(segment);
    }
    path.starts_with(root).then_some(path)
}

#[cfg(feature = "embedded-shell")]
fn collect_embedded(dir: &include_dir::Dir<'_>, out: &mut BTreeMap<String, Vec<u8>>) {
    for file in dir.files() {
        let name = file
            .path()
            .components()
            .filter_map(|part| part.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        out.insert(name, file.contents().to_vec());
    }
    for child in dir.dirs() {
        collect_embedded(child, out);
    }
}

/// What a reader can say about the shell it carries (`##SHELL-PIN`).
///
/// Three digests, and the whole point is that they are three. `declared`
/// is what the build compiled in, `recorded` is what the shell's own
/// index claims, and `measured` is this crate hashing the bytes it is
/// holding. A pin is worth something only when somebody re-measures.
///
/// ```
/// use vibe_doc_shell::Shell;
///
/// // The canonical use: what is this reader carrying, and is it the one
/// // the build pinned? `vibe doc shell status` is these two lines.
/// let report = Shell::open(None).report();
/// assert!(report.matches_pin(), "{}", report.render());
/// assert!(report.render().contains("shell:"));
/// ```
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    #[serde(serialize_with = "provenance_word")]
    pub provenance: Provenance,
    pub package: String,
    pub version: String,
    pub base: String,
    pub file_count: u32,
    pub byte_count: u64,
    pub declared: String,
    pub recorded: String,
    pub measured: String,
    pub pin: Option<Pin>,
}

impl Report {
    /// Do the pin, the build's record and the bytes agree?
    ///
    /// A fallback shell agrees vacuously and says so: there is no shell
    /// to disagree with a pin about, and reporting a mismatch for the
    /// configuration that is working as designed would make the panel red
    /// on every machine without Node.
    ///
    /// `declared` — the digest the build compiled in — is compared only
    /// for an EMBEDDED shell, and that is not a loophole: a binary built
    /// from source compiles nothing in, and the shell it later downloads
    /// is judged against the pin, which is the number that travelled with
    /// the binary. Requiring a compiled-in digest there would mean no
    /// build from source could ever wear a real shell.
    pub fn matches_pin(&self) -> bool {
        if self.provenance == Provenance::Fallback {
            return true;
        }
        let pinned = self.pin.as_ref().map(|pin| pin.sha256.clone());
        if pinned.as_deref() != Some(self.measured.as_str()) || self.recorded != self.measured {
            return false;
        }
        self.provenance != Provenance::Embedded || self.declared == self.measured
    }

    /// The human form.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "shell: {} ({} file(s), {} byte(s))\n",
            self.provenance.as_str(),
            self.file_count,
            self.byte_count
        ));
        if self.provenance == Provenance::Fallback {
            out.push_str(
                "  the bare shell: typography and no scripts, which is what a build \
                 without Node ships\n  `vibe doc shell install` fetches the shell this \
                 version was built with, and asks first\n",
            );
            return out;
        }
        out.push_str(&format!(
            "  built from {}@{} at base {}\n",
            self.package, self.version, self.base
        ));
        out.push_str(&format!("  measured  {}\n", short(&self.measured)));
        out.push_str(&format!("  recorded  {}\n", short(&self.recorded)));
        out.push_str(&format!("  declared  {}\n", short(&self.declared)));
        match &self.pin {
            Some(pin) => out.push_str(&format!("  pinned    {}\n", short(&pin.sha256))),
            None => out.push_str("  pinned    (the pin does not parse)\n"),
        }
        out.push_str(if self.matches_pin() {
            "  the embedded shell matches the pin\n"
        } else {
            "  the embedded shell does NOT match the pin — run `cargo xtask embed-doc-shell`\n"
        });
        out
    }
}

/// A digest as a person reads it: enough to tell two apart.
fn short(digest: &str) -> String {
    if digest.is_empty() {
        return "(none)".to_string();
    }
    digest.chars().take(16).collect()
}

fn provenance_word<S: serde::Serializer>(
    provenance: &Provenance,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(provenance.as_str())
}

#[cfg(test)]
mod tests;
