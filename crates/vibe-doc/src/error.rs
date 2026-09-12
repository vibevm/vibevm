//! The library's one error layer: a single enum per crate, whose messages
//! name the rule they enforce by `spec://` address (the new-crate
//! checklist, PROP-057 `##PIPE-CRATES`).
//!
//! The addresses are not decoration. A documentation check fails in front
//! of an author who is holding a page, not a debugger: the message has to
//! say which law the page (or the fixture, or the product) broke, and
//! where that law is written down.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY");

use std::path::PathBuf;

use specmark::spec;

/// Everything this crate can refuse to do.
///
/// One layer, one enum: the runner, the fixture reader, the normaliser and
/// the `derived` generators all report through it, so a surface above
/// (`vibe doc …`, the MCP tool, the HTTP reader) has exactly one shape to
/// render.
///
/// ```
/// let e = vibe_doc::DocError::Tripwire { message: "`~/.vibe` moved".into() };
/// let msg = e.to_string();
/// assert!(msg.contains("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER"));
/// assert!(msg.contains("fix:"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY")]
#[non_exhaustive]
pub enum DocError {
    /// A file the check needed could not be read or written.
    #[error(
        "{action} `{path}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-LIBRARY; \
         fix: make the path reachable, or point the check at the package that owns it)"
    )]
    Io {
        /// What was being attempted, in the imperative ("reading", "writing").
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A page of the package is not readable through the documentation
    /// vocabulary. The page is at fault, never the reader: the dialect is
    /// closed by law and a foreign construct is a loud refusal
    /// (PROP-045 `##XML-DIALECT-IS-THE-MD-SUBSET`).
    #[error(
        "`{path}` is not readable as a documentation page: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND; \
         fix: write the page in the closed dialect, or record the defect)"
    )]
    Page { path: PathBuf, message: String },

    /// A fixture directory is missing, or its `example.toml` does not
    /// parse. An example names its fixture; a name with nothing behind it
    /// cannot be run and is never quietly skipped.
    #[error(
        "fixture `{fixture}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-045#ROW-DOCVOCAB-EXAMPLE-PURPOSE; \
         fix: add `examples/{fixture}/example.toml` to the documentation package)"
    )]
    Fixture { fixture: String, message: String },

    /// A declared normalisation rule is not a usable line form.
    #[error(
        "fixture `{fixture}`: normalisation pattern `{pattern}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER; \
         fix: spell the rule as a regular expression over one line's form)"
    )]
    Pattern {
        fixture: String,
        pattern: String,
        message: String,
    },

    /// The sandbox could not be prepared, or a fixture build step failed.
    /// A fixture that does not build is a red example, never a skipped one.
    #[error(
        "fixture `{fixture}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER; \
         fix: repair the fixture recipe or the product it exercises)"
    )]
    Sandbox { fixture: String, message: String },

    /// A command line an example or a fixture step spells cannot be run by
    /// the runner. The runner is not a shell: it dispatches a closed set of
    /// programs, so what a page shows is what a reader can type.
    #[error(
        "`{command}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER; \
         fix: document a command the runner can execute, or teach the runner that program)"
    )]
    Command { command: String, message: String },

    /// The real per-user home or the source tree moved during a run. This
    /// is the tripwire firing, and it is fatal by design: a runner that
    /// writes outside its sandbox has already broken the isolation the
    /// examples depend on.
    #[error(
        "the runner changed state outside its sandbox: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-EXAMPLE-RUNNER; \
         fix: stop the run and find what wrote there — never widen the tripwire)"
    )]
    Tripwire { message: String },

    /// `--accept` could not write the captured output back into the page.
    #[error(
        "cannot accept the output of example `{id}` into `{path}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#INV-EXAMPLES-RUN; \
         fix: repair the page, then re-run with --accept)"
    )]
    Accept {
        id: String,
        path: PathBuf,
        message: String,
    },

    /// A `rule` cites an address that no source resolves, or an anchor
    /// that no longer exists. The check knows nothing else about a
    /// citation — no revision, no hash, no «the spec moved ahead»
    /// (PROP-057 `##OBS-RULE-EDGE-UNPINNED`).
    #[error(
        "`{page}`:{line} cites `{uri}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-RULE-EDGE-UNPINNED; \
         fix: correct the address, or leave a tombstone where the rule was renamed)"
    )]
    Citation {
        page: String,
        line: u32,
        uri: String,
        message: String,
    },

    /// A `derived` reference names something the generators cannot build.
    #[error(
        "derived {kind} reference `{reference}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-045#ROW-DOCVOCAB-DERIVED; \
         fix: correct the reference, or generate what it names)"
    )]
    Derived {
        kind: &'static str,
        reference: String,
        message: String,
    },

    /// The manifest of a documentation package does not carry what a card
    /// needs. A `doc` package MUST declare `title` and `abstract`, and the
    /// coordinate it is addressed by must parse — without them there is no
    /// shelf row, no catalogue entry and no site address
    /// (PROP-057 `##CARD-FIELDS`, `##KIND-DOC-MUST-DOCUMENT`).
    #[error(
        "`{path}` {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS; \
         fix: declare the card fields a package of kind `doc` must carry)"
    )]
    Manifest { path: PathBuf, message: String },

    /// A translation cannot be checked against the documentation it
    /// adapts — the coordinate is malformed, or no source holds it. The
    /// check itself asks about STRUCTURE alone, and this is the state
    /// where it has nothing to ask the question of
    /// (PROP-057 `##LOC-MIRROR`).
    ///
    /// The coordinate is spelled `adapts` rather than `source`: a field
    /// called `source` is what `thiserror` reads as the wrapped error,
    /// and a coordinate is not one.
    #[error(
        "the translation adapts `{adapts}`: {message} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR; \
         fix: name a documentation this machine can reach, and mirror it)"
    )]
    Translation { adapts: String, message: String },
}

impl DocError {
    /// An I/O failure with the action that provoked it — the shape every
    /// file touch in this crate reports through, so no call site invents
    /// its own wording.
    pub(crate) fn io(
        action: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> DocError {
        DocError::Io {
            action,
            path: path.into(),
            source,
        }
    }

    /// A card defect, named with the manifest that carries it.
    pub(crate) fn manifest(path: impl Into<PathBuf>, message: impl Into<String>) -> DocError {
        DocError::Manifest {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, DocError>;
