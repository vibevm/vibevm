//! Independent content witnesses for declared artifacts (PROP-054
//! `##ARTIFACT-WITNESS-ALGORITHMS`).
//!
//! A reply names an artifact; this cell asks the filesystem what is actually
//! there. The two are different claims, and only the second is evidence: a
//! handler that says it produced `dist/app.exe` has told us its intent, and a
//! witness over the bytes at that path is the only thing a later run can
//! compare against.
//!
//! **The OS object selects the algorithm, never the declared `kind`.** A row
//! may call itself `wheel`, `image` or anything a future provider invents; the
//! witness says how it was hashed, so an unknown semantic kind can never gate
//! physical hashing. A regular file is `sha256:file-v1`, a directory is
//! `sha256:tree-v1`, and everything else is refused.
//!
//! **Refusal is evidence-only.** It never vetoes a handler, never changes
//! freshness, and never leaks a machine path or a file body into state. A
//! refused artifact keeps its `{id, kind, path}` row and writes no witness at
//! all — the honest shape for "this run could not establish a comparable
//! object here". The typed cause stays crate-private: A5 owns the mapping from
//! a live re-observation to the wire's closed reason vocabulary.
//!
//! **Refusal is per artifact**, with exactly one widening: inside a directory
//! artifact, a refusal anywhere in the subtree refuses that whole tree, because
//! a tree witness is ONE digest over a set and a partial digest would be a
//! false claim about the scope. It never touches a sibling artifact.
//!
//! This is detection-bound and claims no atomic snapshot. What it establishes:
//! every file's bytes came from one object that did not move across its own
//! two-pass read, and no directory changed identity or child set while its
//! subtree was measured.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS");

use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_safefs::{ContentDigest, EntryProof, Pinned, Project};
use vibe_wire::generated::lifecycle_state::{StateArtifact, StateDigestWitness};

/// The regular-file form: a domain-separated frame over the size and the raw
/// inner digest of the exact bytes.
const FILE_ALGORITHM: &str = "sha256:file-v1";
const FILE_SEED: &[u8] = b"sha256:file-v1\0epoch=1\0";

/// The directory form: the same frame over a deterministic preorder walk.
const TREE_ALGORITHM: &str = "sha256:tree-v1";
const TREE_SEED: &[u8] = b"sha256:tree-v1\0epoch=1\0";

/// The bounded-walk fences. There is deliberately no TOTAL entry or byte cap:
/// a declared OS image may be enormous and is still witnessed. What is bounded
/// is what one step must hold in memory — one directory's names — and how far
/// the recursion may go.
const MAX_DIRECTORY_WIDTH: usize = 1_000_000;
pub(crate) const MAX_DEPTH: usize = 256;

/// Why this run could not establish a witness. Bounded, typed and internal —
/// a cause, never a path, a body or an I/O error verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY")]
pub(crate) enum WitnessRefusal {
    /// Nothing at that path.
    Absent,
    /// The final name is not a regular single-link file and not a directory —
    /// a device, or an object the no-follow open would not accept.
    NotRegular,
    /// The no-follow descent refused: an ancestor is a symlink, junction,
    /// reparse point or not a directory at all.
    Linked,
    /// Two direct children of one directory are one physical file under
    /// portable case/normalisation folding.
    Aliased,
    /// A well-formed absolute path that is genuinely somewhere else. Reaching
    /// outside the project through the project capability is the exact thing
    /// the capability exists to prevent.
    Outside,
    /// The row's path is not one this law can locate at all.
    Malformed,
    /// An object moved while it was being measured: a directory's identity or
    /// child set changed across its subtree, or a final name stopped denoting
    /// what it denoted.
    Moved,
    /// A file's two content passes disagreed, or its length did.
    Torn,
    /// The bounded walk refused: a directory wider than
    /// [`MAX_DIRECTORY_WIDTH`], deeper than [`MAX_DEPTH`], or holding a name
    /// this crate cannot spell.
    Unbounded,
    /// The capability itself could not be opened or read.
    Io,
}

/// What one observation established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WitnessOutcome {
    Measured(StateDigestWitness),
    Refused(WitnessRefusal),
}

impl WitnessOutcome {
    /// The witness, or `None` for every refusal — the exact pair a
    /// [`StateArtifact`](vibe_wire::generated::lifecycle_state::StateArtifact)
    /// carries alongside its `measured_run_id`.
    pub(crate) fn measured(self) -> Option<StateDigestWitness> {
        match self {
            Self::Measured(witness) => Some(witness),
            Self::Refused(_) => None,
        }
    }
}

/// One durable artifact row for an execution that PRODUCED or ACCEPTED this
/// artifact, from an observation already taken.
///
/// It consumes an outcome rather than taking one, and that is the whole point:
/// the same physical observation is cloned into the invocation's transient map
/// and — only at a production boundary — turned into the durable pair. Nothing
/// is ever observed twice merely to fill two carriers.
///
/// The witness and the run that took it are ONE fact, so they are written
/// together or not at all: an id beside no witness attributes a measurement
/// that does not exist, and a witness beside no id is one nobody can be held
/// to. A refusal lands on `{None, None}`; the identity triple always survives,
/// so the reopen path is never lost.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-ARTIFACT-WITNESS")]
pub(crate) fn state_row(
    run_id: &str,
    id: String,
    kind: String,
    path: String,
    outcome: &WitnessOutcome,
) -> StateArtifact {
    let witness = outcome.clone().measured();
    StateArtifact {
        measured_run_id: witness.is_some().then(|| run_id.to_string()),
        witness,
        id,
        kind,
        path,
    }
}

/// One batch's view of the project: the capability is opened ONCE and every
/// artifact in the batch is witnessed through it.
///
/// The root open is the only ambient-authority step, and a failure to take it
/// is not fatal — every artifact in the batch is then honestly refused, which
/// is what an evidence-only observer owes. It never becomes a handler error.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-ARTIFACT-WITNESS")]
pub(crate) struct ArtifactObserver {
    project: Option<Project>,
    project_root: String,
}

impl ArtifactObserver {
    /// Pin the project for this checkpoint batch.
    pub(crate) fn new(project_root: &str) -> Self {
        Self {
            project: Project::open(std::path::Path::new(project_root)).ok(),
            project_root: project_root.to_string(),
        }
    }

    /// Witness one declared artifact row.
    ///
    /// Lexical normalisation and containment belong to
    /// [`eligible_relative`](super::eligible_relative) — the same law the
    /// pre-spend contract check uses — so this cell never re-implements what
    /// "below the project" means.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS"
    )]
    pub(crate) fn observe(&self, id: &str, path: &str) -> WitnessOutcome {
        match self.witness(id, path) {
            Ok(witness) => WitnessOutcome::Measured(witness),
            Err(cause) => WitnessOutcome::Refused(cause),
        }
    }

    fn witness(&self, id: &str, path: &str) -> Result<StateDigestWitness, WitnessRefusal> {
        let relative = match super::eligible_relative(id, path, &self.project_root) {
            Ok(Some(relative)) => relative,
            Ok(None) => return Err(WitnessRefusal::Outside),
            Err(_) => return Err(WitnessRefusal::Malformed),
        };
        let project = self.project.as_ref().ok_or(WitnessRefusal::Io)?;
        let (parents, name) = split_relative(relative)?;
        let holder = match project.dir_if_present(&parents) {
            Ok(Some(holder)) => holder,
            Ok(None) => return Err(WitnessRefusal::Absent),
            // A no-follow descent that refused: a link, a junction, a reparse
            // point or a non-directory ancestor. It is never followed.
            Err(_) => return Err(WitnessRefusal::Linked),
        };

        // The OS decides the algorithm. `open_child_checked` answers the
        // directory question unambiguously — `Ok(Some)` is a link-free
        // directory, `Ok(None)` is absence — and everything else falls through
        // to the regular-file question.
        match holder.open_child_checked(name) {
            Ok(Some(root)) => {
                let proof = root.proof().map_err(|_| WitnessRefusal::Moved)?;
                let digest = tree_digest(project, &root, "", 0)?;
                // The artifact root is rebound-checked through ITS parent too,
                // so a swap of the whole tree after the walk refuses.
                rebound(&holder, name, proof)?;
                Ok(digest)
            }
            Ok(None) => Err(WitnessRefusal::Absent),
            Err(_) => file_witness(project, &holder, name).map(|(witness, _)| witness),
        }
    }
}

/// Split a canonical forward-slashed relative path into its parent chain and
/// final name. The path already passed `eligible_relative`, so an empty final
/// component here is a defect, not a caller mistake.
fn split_relative(relative: &str) -> Result<(Vec<&str>, &str), WitnessRefusal> {
    let mut components = relative.split('/').filter(|part| !part.is_empty());
    let name = components.next_back().ok_or(WitnessRefusal::Malformed)?;
    Ok((components.collect(), name))
}

/// `sha256:file-v1` over one regular file, plus the length the walk needs.
///
/// The bytes never land in memory: `digest_file_in` streams them twice through
/// a fixed window and proves the object did not move across its own read, so
/// this frame carries the inner digest rather than the content.
fn file_witness(
    project: &Project,
    holder: &Pinned,
    name: &str,
) -> Result<(StateDigestWitness, ContentDigest), WitnessRefusal> {
    let content = digest_file(project, holder, name)?;
    let mut hash = Sha256::new();
    hash.update(FILE_SEED);
    frame(&mut hash, "size", content.len.to_string());
    frame(&mut hash, "content_sha256", content.sha256);
    Ok((
        StateDigestWitness {
            algorithm: FILE_ALGORITHM.to_string(),
            digest: format!("sha256:{:x}", hash.finalize()),
            // The count pair belongs to the input-manifest form alone; on an
            // artifact form it is `CountDefect::Unexpected` on the wire. A
            // tree's counts are digest material, never members.
            files: None,
            bytes: None,
        },
        content,
    ))
}

/// One stable file digest, with the primitive's refusals classified.
///
/// `inspect_file_in` first, so the difference between "not a regular
/// single-link file" and "a regular file that would not hold still" is
/// structural rather than read off an error message.
fn digest_file(
    project: &Project,
    holder: &Pinned,
    name: &str,
) -> Result<ContentDigest, WitnessRefusal> {
    match project.inspect_file_in(holder, name) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(WitnessRefusal::Absent),
        // A link, a reparse point, a device, or a name shared as a hard link.
        Err(_) => return Err(WitnessRefusal::NotRegular),
    }
    match project.digest_file_in(holder, name) {
        Ok(Some(content)) => Ok(content),
        // It was a regular single-link file a moment ago and is gone now.
        Ok(None) => Err(WitnessRefusal::Moved),
        // It proved regular, so a refusal here is the content or the length
        // failing to hold still across the two passes.
        Err(_) => Err(WitnessRefusal::Torn),
    }
}

/// Refuse unless `name` still denotes, through `parent`, the object `proof`
/// describes — the directory twin of the file primitive's final-name law.
fn rebound(parent: &Pinned, name: &str, proof: EntryProof) -> Result<(), WitnessRefusal> {
    match parent.open_child_checked(name) {
        Ok(Some(current)) => {
            let current = current.proof().map_err(|_| WitnessRefusal::Moved)?;
            (current == proof)
                .then_some(())
                .ok_or(WitnessRefusal::Moved)
        }
        Ok(None) | Err(_) => Err(WitnessRefusal::Moved),
    }
}

/// `sha256:tree-v1` over a directory artifact.
///
/// Depth-first **preorder**: at each directory the direct child names are
/// sorted by raw UTF-8 bytes, a directory entry is framed before its subtree is
/// visited, and the three counts are framed last. Counts-last is what makes the
/// walk streamable — a counts-first recipe would have to retain the whole tree
/// before it could frame anything.
///
/// The root itself is not an entry: identical content at two paths is the same
/// content, and the row carries the location. Every descendant directory IS an
/// entry, including an empty one, so `{}` cannot match `{empty/}`.
///
/// No `shippable_entry` exclusion applies. A `target/` or an empty hook
/// directory inside a *declared output* is part of the bytes the producer chose
/// to claim, and silently skipping it would hide a real change.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-WITNESS-ALGORITHMS")]
fn tree_digest(
    project: &Project,
    root: &Pinned,
    prefix: &str,
    depth: usize,
) -> Result<StateDigestWitness, WitnessRefusal> {
    let mut hash = Sha256::new();
    hash.update(TREE_SEED);
    let mut counts = TreeCounts::default();
    walk(project, root, prefix, depth, &mut hash, &mut counts)?;
    frame(&mut hash, "directory_count", counts.directories.to_string());
    frame(&mut hash, "file_count", counts.files.to_string());
    frame(&mut hash, "total_bytes", counts.bytes.to_string());
    Ok(StateDigestWitness {
        algorithm: TREE_ALGORITHM.to_string(),
        digest: format!("sha256:{:x}", hash.finalize()),
        files: None,
        bytes: None,
    })
}

/// What the walk accumulates. Fixed width, so memory stays a function of
/// directory width and depth rather than of the tree's size.
#[derive(Default)]
struct TreeCounts {
    directories: u64,
    files: u64,
    bytes: u128,
}

impl TreeCounts {
    fn add_directory(&mut self) -> Result<(), WitnessRefusal> {
        self.directories = self
            .directories
            .checked_add(1)
            .ok_or(WitnessRefusal::Unbounded)?;
        Ok(())
    }

    /// A count that saturated would understate a tree by exactly the amount
    /// that made it interesting, so both of these refuse instead.
    fn add_file(&mut self, len: u64) -> Result<(), WitnessRefusal> {
        self.files = self.files.checked_add(1).ok_or(WitnessRefusal::Unbounded)?;
        self.bytes = self
            .bytes
            .checked_add(u128::from(len))
            .ok_or(WitnessRefusal::Unbounded)?;
        Ok(())
    }
}

/// One directory's contribution, with its detection boundary around it:
/// identity and sorted child set proved before the subtree and again after,
/// and every child directory rebound-checked through this parent once its own
/// subtree is done.
fn walk(
    project: &Project,
    directory: &Pinned,
    prefix: &str,
    depth: usize,
    hash: &mut Sha256,
    counts: &mut TreeCounts,
) -> Result<(), WitnessRefusal> {
    if depth > MAX_DEPTH {
        return Err(WitnessRefusal::Unbounded);
    }
    let proof_before = directory.proof().map_err(|_| WitnessRefusal::Moved)?;
    let before = sorted_children(project, directory)?;
    // Portable identity on the DIRECT names is enough for the whole tree: two
    // full paths that fold together must already fold at their first differing
    // ancestor, which is a direct-name collision in that ancestor.
    vibe_safefs::judge_selection(before.iter().map(String::as_str))
        .map_err(|_| WitnessRefusal::Aliased)?;

    for name in &before {
        let path = join(prefix, name);
        match directory.open_child_checked(name) {
            Ok(Some(child)) => {
                let child_proof = child.proof().map_err(|_| WitnessRefusal::Moved)?;
                frame(hash, "entry_kind", "directory");
                frame(hash, "path", path.as_bytes());
                counts.add_directory()?;
                walk(project, &child, &path, depth + 1, hash, counts)?;
                drop(child);
                rebound(directory, name, child_proof)?;
            }
            Ok(None) => return Err(WitnessRefusal::Moved),
            Err(_) => {
                let (_, content) = file_witness(project, directory, name)?;
                frame(hash, "entry_kind", "file");
                frame(hash, "path", path.as_bytes());
                frame(hash, "size", content.len.to_string());
                frame(hash, "content_sha256", content.sha256);
                counts.add_file(content.len)?;
            }
        }
    }

    fire_between_listings(prefix);
    let after = sorted_children(project, directory)?;
    let proof_after = directory.proof().map_err(|_| WitnessRefusal::Moved)?;
    if before != after || proof_before != proof_after {
        return Err(WitnessRefusal::Moved);
    }
    Ok(())
}

/// The bounded enumeration, in the canonical order the digest depends on.
///
/// Sorting is the caller's law by design — the primitive returns the
/// filesystem's order — so it happens exactly here, byte-wise over raw UTF-8.
fn sorted_children(project: &Project, directory: &Pinned) -> Result<Vec<String>, WitnessRefusal> {
    let mut names = project
        .child_names_bounded(directory, MAX_DIRECTORY_WIDTH)
        .map_err(|_| WitnessRefusal::Unbounded)?;
    names.sort_unstable();
    Ok(names)
}

/// Artifact-root-relative, forward-slashed. The root contributes no prefix.
fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

/// The house field frame `be64(label_len)||label||be64(value_len)||value`,
/// applied to this cell's own hasher.
fn frame(hash: &mut Sha256, label: &str, value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    hash.update((label.len() as u64).to_be_bytes());
    hash.update(label.as_bytes());
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

/// Fire the test-only seam in a directory's observation window: after its
/// children were walked and BEFORE the post-listing that must agree with the
/// pre-listing. Compiled to nothing in every non-test build — the release
/// protocol has no window hook at all.
#[cfg(test)]
fn fire_between_listings(prefix: &str) {
    inject::between_listings(prefix);
}

#[cfg(not(test))]
fn fire_between_listings(_prefix: &str) {}

/// The deterministic stand-in for a concurrent writer inside one directory's
/// pre/post window. Compiled out of every non-test build.
#[cfg(test)]
pub(crate) mod inject {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&str)>;

    thread_local! {
        static BETWEEN_LISTINGS: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Arm (or clear with `None`) the one-shot hook fired between the next
    /// directory's pre- and post-listing. It fires once and disarms itself, so
    /// one armed mutation stays one mutation.
    pub(crate) fn arm_between_listings(hook: Option<Hook>) {
        BETWEEN_LISTINGS.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(super) fn between_listings(prefix: &str) {
        if let Some(hook) = BETWEEN_LISTINGS.with(|slot| slot.borrow_mut().take()) {
            hook(prefix);
        }
    }
}
