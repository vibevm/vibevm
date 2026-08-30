//! The mechanism layer's ONE containment and content-witness cell.
//!
//! Every builtin build/package provider asks the same three questions of a
//! path before it trusts it:
//! is it inside the root the engine owns, is it a real regular file rather
//! than a link into somewhere else, and what are its exact bytes? R8-CARGO
//! answered them inside the Cargo cell because it was the only asker.
//! R8-PACKAGE adds two more, so the answers move here: a second copy of a
//! containment check is a second thing to drift, and the refusals it
//! guards are laws (§5.0's ratification 3 — "the verify refusals are laws,
//! not incidents").
//!
//! Deliberately NOT the `vibe-safefs` publication reader, for the reason
//! R8-CARGO recorded and filed as B-120: every safefs read primitive
//! refuses a file with more than one name, because for a file this project
//! *publishes* a second name means a second owner — while Cargo's own
//! release layout hard-links `target/<profile>/<bin>` to
//! `target/<profile>/deps/<bin>-<hash>`, so that primitive refuses every
//! real build artifact. What holds here instead is stated per call: the
//! path is proved to sit inside an engine-owned root, and the final
//! component is refused if it is a link, so nothing outside that root can
//! be reached through it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

/// The fixed window a file is digested through, so a release binary or a
/// large asset never lands in memory whole.
const DIGEST_WINDOW: usize = 64 * 1024;

/// Why one path could not be accepted as a contained regular file.
///
/// It carries no target or output id: this cell knows a path, and the
/// caller — which knows *which declared row* asked — turns the fault into
/// the refusal a human reads. That is why the three shapes are separate
/// variants rather than one string: the containment, link and absence pins
/// are three distinct laws and each caller maps them to its own named
/// refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileFault {
    /// Nothing readable is there.
    Missing(String),
    /// The final component is a symlink, junction or other reparse point.
    Link,
    /// It exists and is not a regular file (a directory, a device, a pipe).
    NotRegular,
    /// It is a regular file whose bytes could not be read.
    Read(String),
}

impl FileFault {
    /// The one-clause reason a refusal quotes.
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::Missing(detail) => detail.clone(),
            Self::Link => "the path is a link, not the file itself".to_owned(),
            Self::NotRegular => "not a regular file".to_owned(),
            Self::Read(detail) => detail.clone(),
        }
    }
}

/// The forward-slashed spelling every recorded path uses.
pub(crate) fn forward_slashed(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// One path's forward-slashed identity relative to a root, or `None` when
/// it is not below it.
///
/// Comparison is over the forward-slashed spellings so a Windows message
/// path and a Windows root agree, and a `..` or empty segment in the tail
/// is refused rather than normalised: a relative identity a later phase
/// joins records by must name exactly one place.
pub(crate) fn relative_to(path: &Path, root: &Path) -> Option<String> {
    let path = forward_slashed(path);
    let root = forward_slashed(root);
    let root = root.trim_end_matches('/');
    let rest = path.strip_prefix(root)?.strip_prefix('/')?;
    if rest.is_empty() || rest.split('/').any(|part| part.is_empty() || part == "..") {
        return None;
    }
    Some(rest.to_owned())
}

/// Why one authored relative path may not be joined to a root.
///
/// This is the *declaration* half of containment — refused before any
/// filesystem call, so a traversal never becomes an `open` this process
/// then has to reason about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathFault {
    /// Empty, or every component was a no-op.
    Empty,
    /// Absolute, or rooted at a drive/prefix.
    Absolute,
    /// Contains a `..` component.
    Traversal,
    /// Contains a `.` component or an empty segment.
    NonCanonical,
}

impl PathFault {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::Empty => "the path names nothing",
            Self::Absolute => "an absolute path is never joined to an engine-owned root",
            Self::Traversal => "a `..` component escapes the root it is joined to",
            Self::NonCanonical => "a `.` or empty component has no single meaning",
        }
    }
}

/// Validate one authored relative path and return its forward-slashed
/// canonical spelling.
///
/// Nothing here touches the filesystem: it judges the *spelling*, which is
/// what makes it usable at `plan` time, where a pure operation may not
/// stat the tree.
pub(crate) fn checked_relative(value: &str) -> Result<String, PathFault> {
    let raw = value.replace('\\', "/");
    if raw.is_empty() {
        return Err(PathFault::Empty);
    }
    let candidate = Path::new(&raw);
    let mut parts: Vec<&str> = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => match part.to_str() {
                Some(text) if !text.is_empty() => parts.push(text),
                _ => return Err(PathFault::NonCanonical),
            },
            Component::ParentDir => return Err(PathFault::Traversal),
            Component::CurDir => return Err(PathFault::NonCanonical),
            Component::RootDir | Component::Prefix(_) => return Err(PathFault::Absolute),
        }
    }
    if raw.contains("//") || raw.ends_with('/') {
        return Err(PathFault::NonCanonical);
    }
    if parts.is_empty() {
        return Err(PathFault::Empty);
    }
    Ok(parts.join("/"))
}

/// Join a validated relative spelling onto a root, component by component.
///
/// Never `Path::join(&str)`: joining a whole spelling would let a platform
/// separator or a drive letter inside the value decide the result.
pub(crate) fn join_relative(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

/// Prove one path is a regular file that is not a link.
///
/// The two refusals are separate on purpose: "it is a link" and "it is not
/// there" are different repairs, and collapsing them would make the link
/// pin invisible the moment a link happened to dangle.
pub(crate) fn prove_regular_file(path: &Path) -> Result<u64, FileFault> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|error| FileFault::Missing(error.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(FileFault::Link);
    }
    if !metadata.file_type().is_file() {
        return Err(FileFault::NotRegular);
    }
    Ok(metadata.len())
}

/// Prove one path is a real directory that is not a link.
///
/// The plugin distributable is a DIRECTORY (§6.2), and its containment law
/// spans "symlinks, junctions and reparse points" — all three of which
/// `symlink_metadata` reports as a symlink file type on the platforms this
/// runs on, which is exactly why the check is on the *link* bit and not on
/// a resolved path comparison.
pub(crate) fn prove_directory(path: &Path) -> Result<(), FileFault> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|error| FileFault::Missing(error.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(FileFault::Link);
    }
    if !metadata.file_type().is_dir() {
        return Err(FileFault::NotRegular);
    }
    Ok(())
}

/// Stream one proven regular file and digest its exact bytes.
///
/// The bytes are streamed, never buffered whole — a release binary is not
/// a value to hold in memory — and the returned length is what was really
/// read, not what a directory entry claimed.
pub(crate) fn digest_file(path: &Path) -> Result<(String, u64), FileFault> {
    prove_regular_file(path)?;
    // An `open` that fails after the proof is the same fact the proof was
    // making — nothing readable is there — so it keeps that variant rather
    // than becoming a read fault of a file that was never opened.
    let mut file =
        std::fs::File::open(path).map_err(|error| FileFault::Missing(error.to_string()))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; DIGEST_WINDOW];
    let mut bytes: u64 = 0;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| FileFault::Read(error.to_string()))?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        hash.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}

/// Read one proven regular file whole, refusing anything larger than the
/// cap.
///
/// A packaging provider inlines textual resources into one document, so it
/// must hold them; the cap is what keeps "textual resource" from becoming
/// "whatever fits in RAM".
pub(crate) fn read_file_bounded(path: &Path, cap: u64) -> Result<Vec<u8>, FileFault> {
    let length = prove_regular_file(path)?;
    if length > cap {
        return Err(FileFault::Read(format!(
            "{length} byte(s) exceeds the {cap}-byte cap for an inlined textual resource"
        )));
    }
    std::fs::read(path).map_err(|error| FileFault::Read(error.to_string()))
}

/// One tree's canonical witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeDigest {
    /// 64 lowercase hex over the canonical manifest.
    pub(crate) digest: String,
    /// How many regular files the digest covers — the census that makes a
    /// silently skipped file visible.
    pub(crate) files: usize,
    /// The total bytes those files hold.
    pub(crate) bytes: u64,
}

/// One entry a tree walk refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeFault {
    /// The tree-relative path, forward-slashed.
    pub(crate) path: String,
    pub(crate) reason: String,
}

/// The canonical directory digest, `sha256-tree/1`, in full.
///
/// 1. Walk the tree ([`walk_tree`]). Every entry is proved NOT to be a
///    link (symlink, junction or reparse point) before it is looked at, so
///    nothing outside the tree can be reached, and every entry is either a
///    directory or a regular file — anything else refuses.
/// 2. For each regular file, take its forward-slashed path relative to the
///    tree root and the SHA-256 of its exact bytes.
/// 3. Those pairs arrive SORTED by relative path, as bytes. That is the
///    step that makes the value independent of the walk: a filesystem is
///    free to hand entries back in any order, and two machines that
///    disagree about it must still agree about the digest.
/// 4. Hash one manifest over the sorted pairs — the literal algorithm
///    name, then `path\0digest\0` for each pair. NUL is the separator
///    because a path cannot contain one, so no two different trees can
///    render to the same manifest bytes.
/// 5. The digest of that manifest is the directory's digest.
///
/// **What it deliberately does not cover.** An empty directory contributes
/// nothing, because it carries no content; two trees that differ only by
/// an empty directory have one digest. That is the same choice every
/// content-addressed tree format makes, and it is stated rather than
/// discovered.
pub(crate) fn tree_digest(root: &Path) -> Result<TreeDigest, TreeFault> {
    let entries = walk_tree(root)?;
    let mut hash = Sha256::new();
    hash.update(TREE_ALGORITHM.as_bytes());
    hash.update(b"\x00");
    let mut bytes: u64 = 0;
    let mut counted = 0_usize;
    for (relative, absolute) in &entries {
        let (digest, length) = digest_file(absolute).map_err(|fault| TreeFault {
            path: relative.clone(),
            reason: fault.reason(),
        })?;
        bytes += length;
        counted += 1;
        hash.update(relative.as_bytes());
        hash.update(b"\x00");
        hash.update(digest.as_bytes());
        hash.update(b"\x00");
    }
    Ok(TreeDigest {
        digest: format!("{:x}", hash.finalize()),
        files: counted,
        bytes,
    })
}

/// The algorithm identity, hashed in so a future revision cannot collide
/// with this one.
const TREE_ALGORITHM: &str = "sha256-tree/1";

/// The canonical file census of one tree: every regular file's
/// forward-slashed tree-relative path and its absolute path, SORTED by the
/// relative path as bytes.
///
/// One walk, three readers: the canonical directory digest above, the
/// engine's deploy input resolution, and the deterministic archive writer —
/// which is exactly why the sort lives here and not in any of them. An
/// archive whose entry order came from `read_dir` would be byte-identical
/// only on one machine, and a digest computed over a different order than
/// the archive was written in would describe a different tree.
pub(crate) fn walk_tree(root: &Path) -> Result<Vec<(String, PathBuf)>, TreeFault> {
    prove_directory(root).map_err(|fault| TreeFault {
        path: ".".to_owned(),
        reason: fault.reason(),
    })?;
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    collect_tree(root, root, &mut entries)?;
    entries.sort_unstable();
    Ok(entries)
}

/// The recursive half: one directory's own entries, then its children.
fn collect_tree(
    root: &Path,
    directory: &Path,
    entries: &mut Vec<(String, PathBuf)>,
) -> Result<(), TreeFault> {
    let relative_of =
        |path: &Path| relative_to(path, root).unwrap_or_else(|| forward_slashed(path));
    let listing = std::fs::read_dir(directory).map_err(|error| TreeFault {
        path: relative_of(directory),
        reason: error.to_string(),
    })?;
    // Read the whole listing first, then walk it in name order: a
    // recursion driven by the filesystem's own order would make the walk —
    // though not the result — vary between machines, and a refusal should
    // name the same entry twice in a row.
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| TreeFault {
            path: relative_of(directory),
            reason: error.to_string(),
        })?;
        children.push(entry.path());
    }
    children.sort();
    for child in children {
        let relative = relative_of(&child);
        let metadata = std::fs::symlink_metadata(&child).map_err(|error| TreeFault {
            path: relative.clone(),
            reason: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(TreeFault {
                path: relative,
                reason: FileFault::Link.reason(),
            });
        }
        if metadata.is_dir() {
            collect_tree(root, &child, entries)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(TreeFault {
                path: relative,
                reason: FileFault::NotRegular.reason(),
            });
        }
        entries.push((relative, child));
    }
    Ok(())
}

#[cfg(test)]
#[path = "contain_tests.rs"]
mod tests;
