//! The payload sidecar — the erasable half of the cache, kept outside the
//! repository (PROP-043 §7.5, DRIFT-016).
//!
//! Two different things used to live in `cache.json`, and two laws pulled
//! on it in opposite directions: the campaign verdicts cannot be
//! regenerated, and the parse payload can be erased at any moment with no
//! knowledge lost. This module is the second half moving out. `cache.json`
//! keeps the identity a verdict was formed against — path, content hash,
//! rollup, campaign map — and stays in git; the `ParsedDoc` that identity
//! stands for lives here, in a per-repository, per-branch directory under
//! the tool's own per-user home, where its size costs the repository
//! nothing.
//!
//! Everything in this module is best-effort by construction. An absent
//! directory, an absent file, unreadable bytes, a foreign schema, a
//! payload that disagrees with the record in git: each is a cache miss
//! that parses — never a warning, never an error. That is the erasure law
//! with teeth, and it is the whole safety argument for storing anything
//! outside the tree: a machine that has never seen this store runs the
//! campaign identically, only slower.
//!
//! Two things this module deliberately does not do. It never asks what a
//! branch is — the adapter asks git and passes the answer in as data,
//! exactly as DRIFT-009 passes the crate→commit map (PROP-043 §2, the
//! separability law). And it never reads the environment: the per-user
//! home arrives already resolved, through the one settings chokepoint, so
//! `VIBE_SETTINGS` relocates this store along with everything else and
//! there is no second variable to forget (F-055).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#erasure");

use crate::cache::write_atomic;
use crate::doc::ParsedDoc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Schema of the sidecar file. A foreign value reads as an empty store:
/// the payload is regenerable, so there is nothing here worth migrating.
pub const PAYLOAD_SCHEMA: u32 = 1;

/// The one file a payload bucket holds.
pub const PAYLOAD_FILE: &str = "payloads.json";

/// The store's home under the per-user settings directory, alongside
/// `registries/` and `search-cache/` — the same class of big derived data.
const STORE_ROOT: &str = "progress-cache";

/// The bucket a checkout with no nameable branch falls back to.
const DETACHED: &str = "detached";

/// Hex characters of sha256 kept in a bucket-name disambiguator.
const HASH_LEN: usize = 6;

/// Reduce `s` to characters every filesystem accepts in a directory name —
/// ASCII alphanumerics, `.`, `_`, `-` — folding everything else to `-`.
///
/// Lossy on purpose, and never load-bearing on its own: every name this
/// builds carries a hash of the original beside it.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// The first [`HASH_LEN`] hex digits of `s`'s sha256 — enough to separate
/// names that the slug collapses, short enough to read in a path.
fn short_hash(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let hex = format!("{:x}", h.finalize());
    hex[..HASH_LEN].to_string()
}

/// The per-repository bucket name: the checkout's own directory name plus
/// a short hash of its path (`vibevm-3f9a1c`).
///
/// The name is for whoever reads `~/.vibe/progress-cache/` by eye; the
/// hash is what makes it correct. Two clones of one repository carry the
/// same directory name and hold different work, so they must never share
/// a bucket — and unlike a repo's parent directory, a hash is always
/// available and always distinct.
pub fn repo_id(root: &Path) -> String {
    let name = root
        .file_name()
        .map(|n| slugify(&n.to_string_lossy()))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "repo".to_string());
    format!("{name}-{}", short_hash(&root.to_string_lossy()))
}

/// The per-branch bucket name: the branch with everything path-hostile
/// folded to `-`, plus a short hash of the original.
///
/// `feature/foo` is a legal branch name and must not become a nested
/// path; the hash is what keeps it out of the bucket belonging to
/// `feature-foo`, which is equally legal and a different corpus. A
/// checkout with no branch to name — a detached HEAD, or a tree git
/// cannot answer about — gets the `detached` bucket rather than an error:
/// the payload is optional by construction.
pub fn branch_slug(branch: Option<&str>) -> String {
    match branch.map(str::trim).filter(|b| !b.is_empty()) {
        Some(b) => format!("{}-{}", slugify(b), short_hash(b)),
        None => DETACHED.to_string(),
    }
}

/// Where one campaign's payloads live, first hit wins (DRIFT-016 §4.2):
/// an explicit `[progress] cache_dir` from `progress.toml` — absolute, or
/// relative to the project root — otherwise
/// `<settings-home>/progress-cache/<repo-id>/<branch-slug>/`. The campaign
/// id keys the leaf either way, so two campaigns in one checkout share the
/// branch bucket without sharing a store.
///
/// `None` is "no sidecar this run": nothing configured and no per-user
/// home to resolve. That is a cold run, not a failure.
///
/// ```
/// use progress_core::sidecar::resolve_dir;
/// use std::path::Path;
///
/// let home = Path::new("/home/u/.vibe");
/// let root = Path::new("/w/vibevm");
///
/// // The default: per-repository, then per-branch, then per-campaign.
/// let main = resolve_dir(root, None, Some(home), Some("main"), "c1")
///     .expect("a home resolves a directory");
/// assert!(main.starts_with(home.join("progress-cache")));
/// assert!(main.ends_with("c1"));
///
/// // A slash in a branch keys a *sibling* bucket, never a nested path.
/// let slashed = resolve_dir(root, None, Some(home), Some("feature/x"), "c1")
///     .expect("dir");
/// assert_eq!(
///     slashed.parent().and_then(Path::parent),
///     main.parent().and_then(Path::parent),
/// );
/// assert_ne!(slashed, main);
///
/// // Nothing configured and no home is simply no sidecar.
/// assert_eq!(resolve_dir(root, None, None, Some("main"), "c1"), None);
/// ```
pub fn resolve_dir(
    root: &Path,
    cfg_dir: Option<&str>,
    home: Option<&Path>,
    branch: Option<&str>,
    campaign_id: &str,
) -> Option<PathBuf> {
    let bucket = match cfg_dir.map(str::trim).filter(|d| !d.is_empty()) {
        Some(d) if Path::new(d).is_absolute() => PathBuf::from(d),
        Some(d) => root.join(d),
        None => home?
            .join(STORE_ROOT)
            .join(repo_id(root))
            .join(branch_slug(branch)),
    };
    Some(bucket.join(slugify(campaign_id)))
}

/// The on-disk shape: a schema and one `ParsedDoc` per observed path.
///
/// Deliberately *only* that. A verdict never leaves `cache.json`
/// (DRIFT-016 §5), and this type is why that is checkable rather than
/// merely intended — there is nowhere in it for one to go.
#[derive(Debug, Deserialize)]
struct StoreFile {
    schema: u32,
    docs: BTreeMap<String, ParsedDoc>,
}

/// The borrowing counterpart used to write. The store is the whole
/// observed corpus every run, and cloning it just to serialise it would
/// pay DRIFT-010's measured clone cost for nothing.
#[derive(Debug, Serialize)]
struct StoreRef<'a> {
    schema: u32,
    docs: BTreeMap<&'a str, &'a ParsedDoc>,
}

/// One campaign's payload store: read once at the head of a run, written
/// once at the end — the same one-read-one-write shape the cache has.
///
/// A store with no directory is a store that always misses, which is what
/// a run without a campaign zone (or without a resolvable per-user home)
/// gets. Nothing distinguishes it from a store whose files were deleted
/// five seconds ago, and nothing should.
#[derive(Debug, Default)]
pub struct Payloads {
    dir: Option<PathBuf>,
    docs: BTreeMap<String, ParsedDoc>,
}

impl Payloads {
    /// Load the store under `dir`.
    ///
    /// Every failure is an empty store, silently: no directory configured,
    /// an absent directory, an absent file, bytes that do not parse, a
    /// schema this build does not know. None of them is reportable,
    /// because none of them changes what the run answers — only how long
    /// it takes (PROP-043 §7.5, DRIFT-016 §4.3).
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#erasure")]
    pub fn load(dir: Option<PathBuf>) -> Payloads {
        let docs = dir.as_deref().and_then(read_store).unwrap_or_default();
        Payloads { dir, docs }
    }

    /// The directory this store reads and writes, when it has one.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// The payload for `path`, when the payload is genuinely *for* `path`
    /// at `hash`.
    ///
    /// The identity check is the whole point of keeping a hash on both
    /// sides of the split: a store that outlived the file it describes
    /// answers `None`, never an answer for the wrong bytes.
    pub fn get(&self, path: &str, hash: &str) -> Option<&ParsedDoc> {
        let doc = self.docs.get(path)?;
        (doc.path == path && doc.content_hash == hash).then_some(doc)
    }

    /// Write exactly `docs` back, replacing whatever was there.
    ///
    /// Best-effort and silent: a store that cannot be written — a
    /// read-only home, a full disk, no directory at all — is a store that
    /// will miss next run, which is a slower run and nothing else. The
    /// hard-error path in this system is `cache.json`, and only that.
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#erasure")]
    pub fn store<'a>(&self, docs: impl IntoIterator<Item = &'a ParsedDoc>) {
        let Some(dir) = &self.dir else {
            return;
        };
        let file = StoreRef {
            schema: PAYLOAD_SCHEMA,
            docs: docs
                .into_iter()
                .map(|d| (d.path.as_str(), d))
                .collect::<BTreeMap<_, _>>(),
        };
        // Compact, not pretty: nothing diffs this file — it is outside the
        // repository precisely so that no one has to.
        if let Ok(body) = serde_json::to_string(&file) {
            let _ = write_atomic(&dir.join(PAYLOAD_FILE), body.as_bytes());
        }
    }
}

/// Read a bucket's payloads, or `None` for every way of not having them.
fn read_store(dir: &Path) -> Option<BTreeMap<String, ParsedDoc>> {
    let text = std::fs::read_to_string(dir.join(PAYLOAD_FILE)).ok()?;
    let file: StoreFile = serde_json::from_str(&text).ok()?;
    (file.schema == PAYLOAD_SCHEMA).then_some(file.docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The branch is part of the bucket's identity: a different branch is
    /// a different corpus, so one store across branches would hand a
    /// reader the right hash with the wrong parse.
    #[test]
    fn branch_keys_the_bucket() {
        let home = Path::new("/h/.vibe");
        let root = Path::new("/w/vibevm");
        let at = |b: Option<&str>| resolve_dir(root, None, Some(home), b, "c").expect("dir");
        let repo_bucket = |p: &Path| p.parent().and_then(Path::parent).map(Path::to_path_buf);

        let main = at(Some("main"));
        let next = at(Some("next"));
        assert_ne!(main, next, "two branches, two buckets");
        assert_eq!(
            repo_bucket(&main),
            repo_bucket(&next),
            "…both under one repository bucket"
        );

        // A slash is folded, not obeyed: `feature/x` names one directory.
        let slashed = at(Some("feature/x"));
        assert_eq!(repo_bucket(&slashed), repo_bucket(&main), "still one level");
        // …and folding it does not merge it with the branch that spells
        // the fold out literally. Both are legal; they are not the same.
        assert_ne!(slashed, at(Some("feature-x")), "the hash separates them");

        // No branch to name is a bucket, not a failure.
        assert!(at(None).parent().expect("branch dir").ends_with(DETACHED));
        assert_eq!(at(Some("   ")), at(None), "an empty answer is no answer");
    }

    /// Two clones of one repository carry the same directory name and
    /// different work — the path hash is what keeps them apart.
    #[test]
    fn two_clones_of_one_repo_never_share_a_bucket() {
        let home = Path::new("/h/.vibe");
        let a = resolve_dir(
            Path::new("/w/a/vibevm"),
            None,
            Some(home),
            Some("main"),
            "c",
        );
        let b = resolve_dir(
            Path::new("/w/b/vibevm"),
            None,
            Some(home),
            Some("main"),
            "c",
        );
        assert_ne!(a, b);
        assert!(repo_id(Path::new("/w/a/vibevm")).starts_with("vibevm-"));
    }

    /// The explicit escape hatch wins outright, and the campaign id keys
    /// the leaf under whichever bucket was chosen.
    #[test]
    fn cfg_dir_and_campaign_id_shape_the_leaf() {
        let home = Path::new("/h/.vibe");
        let root = Path::new("/w/vibevm");
        let at = |c| resolve_dir(root, Some("payloads"), Some(home), Some("main"), c).expect("dir");

        // Relative to the project root, and nowhere near the home.
        assert_eq!(at("c1"), root.join("payloads").join("c1"));
        // Two campaigns, one bucket, two stores.
        assert_ne!(at("c1"), at("c2"));
        assert_eq!(at("c1").parent(), at("c2").parent());
        // An absolute value is taken as given.
        let abs = resolve_dir(root, Some("/srv/store"), Some(home), Some("main"), "c1");
        assert_eq!(abs, Some(Path::new("/srv/store").join("c1")));
    }

    /// Every way of not having a store is the same way: an empty one. The
    /// erasure law is only real if losing the file is indistinguishable
    /// from never having had it.
    #[test]
    fn absent_or_corrupt_store_is_an_empty_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = |p: Option<PathBuf>| Payloads::load(p).get("a.md", "h").is_none();

        assert!(missing(None), "no directory at all");
        assert!(
            missing(Some(dir.path().join("nowhere"))),
            "absent directory"
        );

        let bucket = dir.path().join("bucket");
        std::fs::create_dir_all(&bucket).expect("mkdir");
        std::fs::write(bucket.join(PAYLOAD_FILE), b"{ not json").expect("write");
        assert!(missing(Some(bucket.clone())), "unreadable bytes");

        std::fs::write(bucket.join(PAYLOAD_FILE), br#"{"schema":99,"docs":{}}"#).expect("write");
        assert!(missing(Some(bucket)), "a schema this build does not know");
    }

    /// The store round-trips what a run puts in it, and answers only for
    /// the bytes it actually holds.
    #[test]
    fn store_round_trips_and_answers_only_for_its_own_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bucket = dir.path().join("bucket");
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        // The directory does not exist yet: writing creates it, the way
        // every other atomic write in this crate does.
        Payloads::load(Some(bucket.clone())).store([&doc]);

        let back = Payloads::load(Some(bucket));
        let got = back.get("a.md", &doc.content_hash).expect("hit");
        assert_eq!(got.path, doc.path);
        assert_eq!(got.markers.len(), doc.markers.len());
        assert!(back.get("a.md", "deadbeef").is_none(), "stale hash");
        assert!(back.get("b.md", &doc.content_hash).is_none(), "wrong path");
    }
}
