//! The byte-exact checkout of a working copy **vibe owns and reads as data**.
//!
//! A registry working copy, a package clone and an embedded-source cache are
//! not a user's checkout: vibe reads their files and then *hashes* them — a
//! template snapshot's digest document against the `digest_sha256` its row
//! records (PROP-075 `#MIRROR-SNAPSHOT-LAYOUT`), a package tree against the
//! `content_hash` a lockfile pins, an embedded source against its exact pin.
//! Every one of those comparisons is a comparison of **bytes**, so the only
//! correct rendering of a tracked file into such a directory is the bytes the
//! blob records, on every machine, under every user's Git configuration.
//!
//! Git's working-tree conversion is the opposite default. `core.autocrlf=true`
//! is what the Git for Windows installer writes into the machine's *system*
//! configuration, so on a stock Windows every one of those checkouts rewrote
//! `\n` to `\r\n` and every hash the bytes were supposed to answer failed. The
//! template snapshot's symptom was the visible one — `the snapshot's digest
//! document for <v> does not hash to the \`digest_sha256\` its row records; not
//! primed`, once per version, and a closed network could then render no
//! template-only version at all.
//!
//! Two levers are needed, and the second is the one that is easy to miss:
//!
//! - [`CONFIG`] — the conversion the *configuration* asks for
//!   (`core.autocrlf`, `core.eol`). Passed as `-c` flags, which Git exports to
//!   its own children through `GIT_CONFIG_PARAMETERS`, so a submodule checkout
//!   inherits them.
//! - [`ATTRIBUTES`] — the EOL conversion the *payload itself* asks for. An in-tree
//!   `.gitattributes` saying `* text=auto eol=crlf` converts regardless of
//!   `core.autocrlf=false`; so does a `core.attributesFile`. The one place that
//!   outranks every `.gitattributes` in the tree is `$GIT_DIR/info/attributes`
//!   (gitattributes(5): it has the highest precedence of all), which vibe owns
//!   outright in a directory it created. `* -text` there turns the EOL
//!   machinery off for the whole tree — and only that, so the other
//!   content-changing attributes stay visible to the probes that refuse them.
//!
//! `GIT_ATTR_SOURCE` pointed at the empty tree would do the same job and would
//! reach submodules too, but it is Git 2.40 and newer and its value depends on
//! the repository's object format; `info/attributes` works on every Git and in
//! every repository, so that is the lever used, and a `.gitattributes` *inside
//! a submodule* of a vibe-owned clone stays the one documented gap (B-289).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");

use std::path::Path;

/// `$GIT_DIR/info/attributes` of a working copy vibe reads as data: no
/// text/EOL conversion, whatever the tree's own `.gitattributes` says.
///
/// **Only** that conversion. `ident`, `working-tree-encoding` and `filter` are
/// deliberately left alone: a payload that selects one is *refused* by the
/// admission probes that read exactly these attributes
/// (`native_admission::materialisation`, `shell::prepared`), and a file here
/// that unset them would answer those probes in the payload's place and turn
/// a refusal into a silent neutralisation.
pub(crate) const ATTRIBUTES: &str = concat!(
    "# Written by vibe: this working copy is data vibe reads and hashes, so\n",
    "# every tracked file holds the exact bytes its blob records. This file\n",
    "# outranks every `.gitattributes` in the tree (gitattributes(5)).\n",
    "* -text -eol\n",
);

/// The `-c <key>=<value>` flags that turn off configuration-driven working-tree
/// conversion for one invocation — and, through `GIT_CONFIG_PARAMETERS`, for
/// the submodule checkouts it spawns.
pub(crate) const CONFIG: [&str; 6] = [
    "-c",
    "core.autocrlf=false",
    "-c",
    "core.eol=lf",
    "-c",
    // Nothing is converted, so nothing can round-trip badly; a user's
    // `core.safecrlf=true` must not turn a byte-exact checkout into a refusal.
    "core.safecrlf=false",
];

/// Prepend [`CONFIG`] to one invocation's arguments. The flags must precede the
/// subcommand, exactly as the credential silencing of `shell::env` does.
pub(crate) fn argv<'a>(args: &[&'a str]) -> Vec<&'a str> {
    CONFIG.iter().copied().chain(args.iter().copied()).collect()
}

/// Install [`ATTRIBUTES`] in `git_dir`, reporting whether the file was not
/// already exactly that.
///
/// `Ok(true)` is how a working copy that **predates this rule** announces
/// itself: it was checked out under the user's conversion, its files may hold
/// `\r\n` where their blobs hold `\n`, and the caller owes it a renormalising
/// re-checkout before anything reads it (`normalise_owned_worktree`). That is
/// the upgrade path — a machine that already has a converted clone repairs it
/// on first use rather than carrying the fault until the clone's freshness
/// window expires.
pub(crate) fn install(git_dir: &Path) -> std::io::Result<bool> {
    let path = git_dir.join("info").join("attributes");
    if std::fs::read(&path).is_ok_and(|held| held == ATTRIBUTES.as_bytes()) {
        return Ok(false);
    }
    std::fs::create_dir_all(git_dir.join("info"))?;
    std::fs::write(&path, ATTRIBUTES)?;
    Ok(true)
}

/// Whose rendering of a tracked file a staged checkout holds.
///
/// The two are not interchangeable, and which one a caller needs follows
/// entirely from who reads the result (`git_backend::verbatim`):
///
/// - [`PayloadBytes::Verbatim`] — a tree **vibe** reads: a registry working
///   copy whose template snapshot is hashed against its own record, a package
///   clone whose tree answers a lockfile's `content_hash`, an embedded source
///   checked against its exact pin. Bytes are identity there, so they are the
///   blob's bytes on every machine.
/// - [`PayloadBytes::AsConfigured`] — a working copy published into a **user's**
///   workspace slot, whose identity is its commit and whose files the user then
///   opens in their own tools. That one holds what the user's own `git
///   checkout` would have written, which is what `#NATIVE-MATERIALISATION`
///   captures the configuration and attribute meaning in order to reproduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadBytes {
    /// Exactly the bytes each blob records: no text/EOL conversion, ident
    /// expansion, working-tree encoding or filter, whatever the invocation's
    /// configuration and the payload's own `.gitattributes` ask for.
    Verbatim,
    /// What the invocation's own Git would write, with the captured
    /// configuration and attribute meaning applied.
    AsConfigured,
}
