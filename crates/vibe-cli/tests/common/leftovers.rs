//! The leftover check of a cancelled or rolled-back operation: every root
//! the command can write — the project with its `.vibe/`, its own HOME,
//! settings, registry and search caches, and its own TMPDIR — is
//! snapshotted before and after, and the difference must be empty except
//! for entries an explicit, justified allow-list names.
//!
//! ```ignore
//! let isolation = Isolation::new(outer.path());
//! let watched = isolation.watched(&user).root("project", project.path());
//! let before = watched.snapshot();
//! let mut command = user.vibe();
//! isolation.apply(&mut command);
//! /* run the cancelled operation */
//! assert_no_leftovers(&before, &watched.snapshot(), &[STOP_FILE]);
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::UserScratch;

/// One entry of a watched tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    File(Vec<u8>),
    Dir,
    Link(PathBuf),
}

/// Every entry under every watched root, keyed by root name and relative path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot(BTreeMap<(String, PathBuf), Entry>);

/// An entry the operation may leave changed, and why.
pub struct Allowed {
    /// The watched root's name, as given to [`Watched::root`].
    pub root: &'static str,
    /// Whether the relative path, with what is there after the operation
    /// (`None` when it was removed), is this kind of entry.
    pub matches: fn(&Path, Option<&Entry>) -> bool,
    /// Where the rule is documented; printed when a check fails.
    pub why: &'static str,
}

/// The roots one command may write.
#[derive(Debug, Clone, Default)]
pub struct Watched(Vec<(&'static str, PathBuf)>);

impl Watched {
    pub fn root(mut self, name: &'static str, path: &Path) -> Self {
        self.0.push((name, path.to_path_buf()));
        self
    }

    /// The paths of the roots with these names.
    pub fn roots_named(&self, names: &[&str]) -> Vec<PathBuf> {
        self.0
            .iter()
            .filter(|(name, _)| names.contains(name))
            .map(|(_, path)| path.clone())
            .collect()
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut out = BTreeMap::new();
        for (name, root) in &self.0 {
            if root.exists() {
                visit(name, root, root, &mut out);
            }
        }
        Snapshot(out)
    }
}

fn visit(name: &str, root: &Path, at: &Path, out: &mut BTreeMap<(String, PathBuf), Entry>) {
    for entry in fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        let rel = path.strip_prefix(root).unwrap().to_path_buf();
        let meta = fs::symlink_metadata(&path).unwrap();
        let key = (name.to_owned(), rel);
        if meta.file_type().is_symlink() {
            out.insert(key, Entry::Link(fs::read_link(&path).unwrap()));
        } else if meta.is_dir() {
            out.insert(key, Entry::Dir);
            visit(name, root, &path, out);
        } else {
            out.insert(key, Entry::File(fs::read(&path).unwrap()));
        }
    }
}

/// The difference is empty but for `allow`; a failure lists every
/// unexplained entry with what happened to it.
pub fn assert_no_leftovers(before: &Snapshot, after: &Snapshot, allow: &[Allowed]) {
    let mut unexplained = Vec::new();
    for key in before.0.keys().chain(after.0.keys()) {
        let (old, new) = (before.0.get(key), after.0.get(key));
        if old == new {
            continue;
        }
        let (root, rel) = key;
        if allow
            .iter()
            .any(|rule| rule.root == root && (rule.matches)(rel, new))
        {
            continue;
        }
        let what = match (old, new) {
            (None, Some(_)) => "left behind",
            (Some(_), None) => "removed",
            _ => "changed",
        };
        let line = format!("{root}/{} ({what})", rel.display());
        if !unexplained.contains(&line) {
            unexplained.push(line);
        }
    }
    assert!(
        unexplained.is_empty(),
        "the operation left garbage or changed state outside its allow-list:\n  {}\nallowed:\n  {}",
        unexplained.join("\n  "),
        allow
            .iter()
            .map(|rule| format!("{}: {}", rule.root, rule.why))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// A fresh HOME and TMPDIR for one command, under the fixture, so whatever
/// the command leaves there is its own.
pub struct Isolation {
    pub home: PathBuf,
    pub tmp: PathBuf,
}

impl Isolation {
    pub fn new(under: &Path) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let home = under.join(format!("isolated-{n}-home"));
        let tmp = under.join(format!("isolated-{n}-tmp"));
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&tmp).unwrap();
        Self { home, tmp }
    }

    /// The variables that point every per-user and temporary location here.
    pub fn envs(&self) -> Vec<(&'static str, PathBuf)> {
        vec![
            ("HOME", self.home.clone()),
            ("USERPROFILE", self.home.clone()),
            ("XDG_CONFIG_HOME", self.home.join(".config")),
            ("XDG_CACHE_HOME", self.home.join(".cache")),
            ("XDG_DATA_HOME", self.home.join(".local/share")),
            ("XDG_STATE_HOME", self.home.join(".local/state")),
            ("CODEX_HOME", self.home.join(".codex")),
            ("CLAUDE_CONFIG_DIR", self.home.join(".claude")),
            ("TMPDIR", self.tmp.clone()),
            ("TMP", self.tmp.clone()),
            ("TEMP", self.tmp.clone()),
        ]
    }

    /// Point every per-user and temporary location of `command` here.
    pub fn apply(&self, command: &mut assert_cmd::Command) {
        command.envs(self.envs());
    }

    /// The roots of this isolation and of `user`'s scratch, named `home`,
    /// `tmp`, `settings`, `registry-cache` and `search-cache`.
    pub fn watched(&self, user: &UserScratch) -> Watched {
        Watched::default()
            .root("home", &self.home)
            .root("tmp", &self.tmp)
            .root("settings", &user.settings)
            .root("registry-cache", &user.cache)
            .root("search-cache", &user.search_cache)
    }
}

/// Every object of a content store under `root` is complete: each version
/// directory `v<version>` has its `v<version>.sha256` sidecar and each
/// sidecar its directory, and no staging or temporary name remains. A cache
/// may keep what an operation fetched only in this shape.
pub fn assert_complete_store(root: &Path) {
    fn visit(at: &Path, problems: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(at) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.ends_with(".tmp") || name.starts_with(".tmp") || name.contains("partial") {
                problems.push(format!("{} is a staging leftover", path.display()));
            }
            if path.is_dir() {
                if name.starts_with('v') && name[1..].starts_with(|c: char| c.is_ascii_digit()) {
                    if !at.join(format!("{name}.sha256")).is_file() {
                        problems.push(format!("{} has no sidecar", path.display()));
                    }
                } else {
                    visit(&path, problems);
                }
            } else if let Some(object) = name.strip_suffix(".sha256")
                && !at.join(object).is_dir()
            {
                problems.push(format!("{} names no object", path.display()));
            }
        }
    }
    let mut problems = Vec::new();
    visit(root, &mut problems);
    assert!(
        problems.is_empty(),
        "incomplete store objects:\n  {}",
        problems.join("\n  ")
    );
}
