specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#facts");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::facts::{Fact, Frontend, SourceFacts};

/// What one extraction run did — the producer log the incremental
/// acceptance test asserts on.
///
/// ```
/// let log = core_ai_native_conform::ExtractionLog::default();
/// assert_eq!(log.cached, 0);
/// assert!(log.extracted.is_empty());
/// assert!(log.dead_excludes.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct ExtractionLog {
    /// Files actually re-extracted this run (cache misses).
    pub extracted: Vec<String>,
    /// Cache hits.
    pub cached: usize,
    /// `[rust] exclude_substrings` entries that matched no source this
    /// run — dead exclusions, surfaced so the silence of a key that does
    /// nothing is visible (B-059). Advisory; the gate is unaffected.
    pub dead_excludes: Vec<String>,
}

/// Content-addressed fact store under `<repo>/target/conform/facts/`.
///
/// ```no_run
/// use core_ai_native_conform::{Config, ExtractionLog, Store};
/// # use core_ai_native_conform::{Fact, Frontend};
/// # struct NullFrontend;
/// # impl Frontend for NullFrontend {
/// #     fn id(&self) -> &'static str { "null" }
/// #     fn version(&self) -> &'static str { "1" }
/// #     fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> { Vec::new() }
/// # }
///
/// let repo = std::path::Path::new(".");
/// let store = Store::for_rust(repo, &Config::default());
/// let mut log = ExtractionLog::default();
/// let facts = store.extract_workspace(repo, &NullFrontend, &mut log).unwrap();
/// println!("{} file(s) extracted, {} cached", log.extracted.len(), log.cached);
/// # let _ = facts;
/// ```
pub struct Store {
    root: PathBuf,
    roots: Vec<String>,
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
}

impl Store {
    /// The Rust-scan view of the store: scan roots and exclusions come
    /// from the `[rust]` policy table (B-029 moved them out of the flat
    /// root keys); the cache directory is shared (slots are keyed by
    /// frontend id+version, so the languages never collide).
    pub fn for_rust(repo: &Path, config: &Config) -> Store {
        Store {
            root: repo.join("target").join("conform").join("facts"),
            roots: config.rust.roots.clone(),
            skip_dirs: config.rust.skip_dirs.clone(),
            exclude: config.rust.exclude_substrings.clone(),
        }
    }

    /// The TypeScript-scan view of the same store: scan roots and
    /// exclusions come from the `[typescript]` policy table, the cache
    /// directory is shared (slots are keyed by frontend id+version, so
    /// the two languages never collide).
    pub fn for_typescript(repo: &Path, config: &Config) -> Store {
        Store {
            root: repo.join("target").join("conform").join("facts"),
            roots: config.typescript.roots.clone(),
            skip_dirs: config.typescript.skip_dirs.clone(),
            exclude: config.typescript.exclude_substrings.clone(),
        }
    }

    /// The Go-scan view of the same store: scan roots and exclusions
    /// come from the `[go]` policy table; the cache directory is shared
    /// (slots are keyed by frontend id+version).
    pub fn for_go(repo: &Path, config: &Config) -> Store {
        Store {
            root: repo.join("target").join("conform").join("facts"),
            roots: config.go.roots.clone(),
            skip_dirs: config.go.skip_dirs.clone(),
            exclude: config.go.exclude_substrings.clone(),
        }
    }

    fn slot(&self, frontend: &dyn Frontend, content_hash: &str) -> PathBuf {
        // A raw `sha256:<hex>` filename is not portable: on Windows the
        // colon names an NTFS alternate data stream. Thousands of cache
        // entries then accumulate as streams on one zero-byte `sha256` file
        // until the volume returns ERROR_FILE_SYSTEM_LIMITATION (665).
        // The algorithm stays explicit while the ordinary filename uses the
        // same portable separator every host can create.
        let digest = content_hash
            .strip_prefix("sha256:")
            .unwrap_or(content_hash);
        self.root
            .join(format!("{}-{}", frontend.id(), frontend.version()))
            .join(format!("sha256-{digest}.json"))
    }

    /// Extract facts for every workspace source file (Rust layout:
    /// `src/` + `tests/` of each crate dir), reusing cached facts when
    /// `(content-hash, producer)` already has them.
    pub fn extract_workspace(
        &self,
        repo: &Path,
        frontend: &dyn Frontend,
        log: &mut ExtractionLog,
    ) -> Result<Vec<SourceFacts>> {
        let (sources, dead) = workspace_sources(repo, &self.roots, &self.skip_dirs, &self.exclude);
        announce_dead("rust", log, dead);
        self.extract_sources(sources, frontend, log)
    }

    /// Extract facts for every TypeScript source under the configured
    /// roots (flat walk: `.ts`/`.tsx`/`.mts`/`.cts`, `.d.ts` and
    /// `node_modules`-style trees skipped). Same cache, same log.
    pub fn extract_typescript(
        &self,
        repo: &Path,
        frontend: &dyn Frontend,
        log: &mut ExtractionLog,
    ) -> Result<Vec<SourceFacts>> {
        let sources = typescript_sources(repo, &self.roots, &self.skip_dirs, &self.exclude);
        self.extract_sources(sources, frontend, log)
    }

    /// Extract facts for every Go source under the configured roots
    /// (flat walk: `.go`, with `vendor`/`testdata`-style trees skipped).
    /// Same cache, same log.
    pub fn extract_go(
        &self,
        repo: &Path,
        frontend: &dyn Frontend,
        log: &mut ExtractionLog,
    ) -> Result<Vec<SourceFacts>> {
        let sources = go_sources(repo, &self.roots, &self.skip_dirs, &self.exclude);
        self.extract_sources(sources, frontend, log)
    }

    /// The shared cache loop. Two passes: collect every cache miss and
    /// hand the whole set to [`Frontend::warm`] (one batch — the
    /// `ts-tsc` frontend turns this into a single node run), then serve
    /// each file from cache or `extract`.
    fn extract_sources(
        &self,
        sources: Vec<(String, String, String, PathBuf)>,
        frontend: &dyn Frontend,
        log: &mut ExtractionLog,
    ) -> Result<Vec<SourceFacts>> {
        let mut planned: Vec<(String, String, String, PathBuf, String, PathBuf)> = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        for (file, crate_name, module, path) in sources {
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let hash = content_hash(&text);
            let slot = self.slot(frontend, &hash);
            if !slot.exists() {
                pending.push(file.clone());
            }
            planned.push((file, crate_name, module, path, text, slot));
        }
        if !pending.is_empty() {
            frontend.warm(&pending);
        }
        let mut out = Vec::new();
        for (file, crate_name, module, _path, text, slot) in planned {
            let facts: Vec<Fact> = if slot.exists() {
                log.cached += 1;
                let cached = std::fs::read_to_string(&slot)
                    .with_context(|| format!("reading {}", slot.display()))?;
                serde_json::from_str(&cached)
                    .with_context(|| format!("parsing {}", slot.display()))?
            } else {
                let fresh = frontend.extract(&file, &crate_name, &module, &text);
                if let Some(parent) = slot.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&slot, serde_json::to_string(&fresh)?)?;
                log.extracted.push(file.clone());
                fresh
            };
            out.push(SourceFacts {
                file,
                crate_name,
                facts,
            });
        }
        Ok(out)
    }
}

/// `sha256:<hex>` over LF-normalised text — the same convention the
/// rest of the project uses.
///
/// ```
/// let lf = core_ai_native_conform::content_hash("a\nb\n");
/// let crlf = core_ai_native_conform::content_hash("a\r\nb\r\n");
/// assert_eq!(lf, crlf);
/// assert!(lf.starts_with("sha256:"));
/// ```
pub fn content_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// The crate name a scanned directory denotes: its basename, resolved
/// through `std::path::absolute` first so a `.` root names the project
/// directory itself instead of nothing (`Path::new(".").file_name()` is
/// `None` — the bare single-crate layout). The config validator derives
/// literal-root names through this same function, so the scanner and
/// the gated-or-exempt tree invariant can never disagree on what a
/// root is called.
pub(crate) fn crate_dir_name(dir: &Path) -> Option<String> {
    let resolved = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    resolved
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Surface `[<section>] exclude_substrings` entries that matched no
/// source this pass (B-059): a dead exclusion used to be invisible — the
/// gate ran on, the entry filtered nothing, nobody knew. Announced on
/// stderr (the same non-fatal channel [`sarif`](crate::sarif) uses for a
/// skipped report) and recorded in the log for structured consumers. The
/// gate is unaffected.
fn announce_dead(section: &'static str, log: &mut ExtractionLog, dead: Vec<String>) {
    for d in &dead {
        eprintln!(
            "conform: `[{section}] exclude_substrings` entry {d:?} matched no source this \
             run — a dead exclusion was invisible; the gate is unaffected."
        );
    }
    log.dead_excludes.extend(dead);
}

/// One scanned source: `(repo-rel file, crate name, module path, absolute
/// path)`. A type alias keeps the source-gatherers' return shapes readable
/// (and under clippy's complexity threshold).
type SourceEntry = (String, String, String, PathBuf);

/// Enumerate the configured source roots as `(repo-rel file, crate
/// name, module path, absolute path)`. A `<dir>/*` root scans each
/// subdirectory of `<dir>` as one crate; any other root is a literal
/// crate dir. `src/` and `tests/` of each are walked (tests carry the
/// Class-D oracle facts), and files whose path contains an `exclude`
/// substring are skipped. Returns the kept sources and the `exclude`
/// substrings that matched none of them (dead exclusions — B-059).
fn workspace_sources(
    repo: &Path,
    roots: &[String],
    skip_dirs: &[String],
    exclude: &[String],
) -> (Vec<SourceEntry>, Vec<String>) {
    let mut crate_dirs: Vec<PathBuf> = Vec::new();
    for root in roots {
        if let Some(parent) = root.strip_suffix("/*") {
            if let Ok(rd) = std::fs::read_dir(repo.join(parent)) {
                for entry in rd.filter_map(Result::ok) {
                    if entry.path().is_dir()
                        && entry
                            .file_name()
                            .to_str()
                            .is_none_or(|name| !skip_dirs.iter().any(|skip| skip == name))
                    {
                        crate_dirs.push(entry.path());
                    }
                }
            }
        } else {
            let dir = repo.join(root);
            if dir.is_dir() {
                crate_dirs.push(dir);
            }
        }
    }
    crate_dirs.sort();
    crate_dirs.dedup();

    // One hit counter per exclude substring, accumulated over the whole
    // pass — every crate, src/ + tests/. B-059: an entry that hits zero
    // is a dead exclusion; the counter spans the whole pass (not one
    // crate) so a string that filters files in one crate and none in
    // another still counts as live.
    let mut hits = vec![0u32; exclude.len()];
    let mut out = Vec::new();
    for crate_dir in crate_dirs {
        let crate_name = crate_dir_name(&crate_dir).unwrap_or_default();
        let crate_ident = crate_name.replace('-', "_");
        for sub in ["src", "tests"] {
            let dir = crate_dir.join(sub);
            for entry in walkdir::WalkDir::new(&dir)
                .sort_by_file_name()
                .into_iter()
                .filter_entry(|entry| {
                    entry.depth() == 0 || keep_walk_entry(entry, &[], skip_dirs, false)
                })
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if !entry.file_type().is_file()
                    || path.extension().and_then(|e| e.to_str()) != Some("rs")
                {
                    continue;
                }
                let rel_in_crate = path.strip_prefix(&crate_dir).unwrap_or(path);
                let rel_fwd = rel_in_crate.to_string_lossy().replace('\\', "/");
                let file = path
                    .strip_prefix(repo)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");
                // B-059: match BOTH path spaces — the in-crate path
                // (`src/lib.rs`, the old reading) and the repo-relative
                // path (`crates/foo/src/lib.rs`, the space a finding's
                // address lives in). One config key, one meaning; each hit
                // is recorded so a dead entry can be named afterwards.
                let mut excluded = false;
                for (i, s) in exclude.iter().enumerate() {
                    if rel_fwd.contains(s.as_str()) || file.contains(s.as_str()) {
                        hits[i] += 1;
                        excluded = true;
                    }
                }
                if excluded {
                    continue;
                }
                let module = module_path(&crate_ident, &rel_fwd);
                out.push((file, crate_name.clone(), module, path.to_path_buf()));
            }
        }
    }
    let dead = exclude
        .iter()
        .zip(&hits)
        .filter(|(_, h)| **h == 0)
        .map(|(s, _)| s.clone())
        .collect();
    (out, dead)
}

/// TypeScript source extensions the flat walk accepts.
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts"];
/// Ecosystem-wide directory names the TypeScript walk never descends
/// into. Project-specific names come from `[typescript] skip_dirs`.
const TS_SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".git",
    "target",
];

/// Whether one WalkDir entry remains in the traversal. Ecosystem-wide
/// names are built in; exact project-specific names supplement them via
/// the consumer's policy. Hidden-directory handling stays a property of
/// the language walk, not of the configurable list.
pub(crate) fn keep_walk_entry(
    entry: &walkdir::DirEntry,
    built_in: &[&str],
    configured: &[String],
    skip_hidden: bool,
) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    entry
        .file_name()
        .to_str()
        .map(|name| {
            !built_in.contains(&name)
                && !configured.iter().any(|skip| skip == name)
                && (!skip_hidden || !name.starts_with('.'))
        })
        .unwrap_or(true)
}

/// Enumerate TypeScript sources as `(repo-rel file, root name, module,
/// absolute path)`. Unlike the Rust walk there is no crate topology:
/// each configured root (literal dir or `<dir>/*`) is walked whole,
/// the "crate" is the root's directory name, and the module is the
/// repo-relative path itself (TS modules ARE paths). `.d.ts` files are
/// shapes, not code — skipped, matching the extractor.
fn typescript_sources(
    repo: &Path,
    roots: &[String],
    skip_dirs: &[String],
    exclude: &[String],
) -> Vec<(String, String, String, PathBuf)> {
    let mut root_dirs: Vec<PathBuf> = Vec::new();
    for root in roots {
        if let Some(parent) = root.strip_suffix("/*") {
            if let Ok(rd) = std::fs::read_dir(repo.join(parent)) {
                for entry in rd.filter_map(Result::ok) {
                    if entry.path().is_dir() {
                        root_dirs.push(entry.path());
                    }
                }
            }
        } else {
            let dir = repo.join(root);
            if dir.is_dir() {
                root_dirs.push(dir);
            }
        }
    }
    root_dirs.sort();
    root_dirs.dedup();

    let mut out = Vec::new();
    for root_dir in root_dirs {
        let root_name = crate_dir_name(&root_dir).unwrap_or_default();
        for entry in walkdir::WalkDir::new(&root_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|entry| keep_walk_entry(entry, TS_SKIP_DIRS, skip_dirs, true))
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let is_ts = entry.file_type().is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| TS_EXTENSIONS.contains(&e))
                && !path.to_string_lossy().ends_with(".d.ts");
            if !is_ts {
                continue;
            }
            let file = path
                .strip_prefix(repo)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if exclude.iter().any(|s| file.contains(s.as_str())) {
                continue;
            }
            out.push((file.clone(), root_name.clone(), file, path.to_path_buf()));
        }
    }
    out
}

/// Ecosystem-wide directory names the Go walk never descends into —
/// vendored trees, goldens/fixtures, and build output. Project-specific
/// names come from `[go] skip_dirs`. `pub(crate)` so the Go unit
/// enumerator (`config::coverage`) reuses the one list.
pub(crate) const GO_SKIP_DIRS: &[&str] = &["vendor", "testdata", "node_modules", ".git", "target"];

/// Enumerate Go sources as `(repo-rel file, root name, module,
/// absolute path)`. Like the TypeScript walk there is no crate
/// topology: each configured root is walked whole, the "crate" is the
/// root's directory name, and the module is the repo-relative path
/// itself (the extractor reports package facts per file). `_test.go`
/// files ARE walked — the extractor stamps them `in_test` and the
/// census rules scope by it.
fn go_sources(
    repo: &Path,
    roots: &[String],
    skip_dirs: &[String],
    exclude: &[String],
) -> Vec<(String, String, String, PathBuf)> {
    let mut root_dirs: Vec<PathBuf> = Vec::new();
    for root in roots {
        if let Some(parent) = root.strip_suffix("/*") {
            if let Ok(rd) = std::fs::read_dir(repo.join(parent)) {
                for entry in rd.filter_map(Result::ok) {
                    if entry.path().is_dir() {
                        root_dirs.push(entry.path());
                    }
                }
            }
        } else {
            let dir = repo.join(root);
            if dir.is_dir() {
                root_dirs.push(dir);
            }
        }
    }
    root_dirs.sort();
    root_dirs.dedup();

    let mut out = Vec::new();
    for root_dir in root_dirs {
        let root_name = crate_dir_name(&root_dir).unwrap_or_default();
        for entry in walkdir::WalkDir::new(&root_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                // depth 0 is the scan root itself — a literal `.` root
                // must not be eaten by the hidden-dir filter below.
                e.depth() == 0 || keep_walk_entry(e, GO_SKIP_DIRS, skip_dirs, true)
            })
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let is_go = entry.file_type().is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("go");
            if !is_go {
                continue;
            }
            let file = path
                .strip_prefix(repo)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if exclude.iter().any(|s| file.contains(s.as_str())) {
                continue;
            }
            out.push((file.clone(), root_name.clone(), file, path.to_path_buf()));
        }
    }
    out
}

/// `src/lib.rs` → crate root; `src/foo.rs` → `crate::foo` — the rscan
/// scheme, duplicated here because the engine is specmap-independent
/// (the two reconcile when conform grows specmap-aware rules).
fn module_path(crate_ident: &str, rel_fwd: &str) -> String {
    let mut parts = vec![crate_ident.to_string()];
    let trimmed = rel_fwd.strip_prefix("src/").unwrap_or(rel_fwd);
    let comps: Vec<&str> = trimmed.split('/').collect();
    for (i, comp) in comps.iter().enumerate() {
        let is_last = i + 1 == comps.len();
        if is_last {
            let stem = comp.strip_suffix(".rs").unwrap_or(comp);
            if !matches!(stem, "lib" | "main" | "mod") {
                parts.push(stem.to_string());
            }
        } else {
            parts.push((*comp).to_string());
        }
    }
    parts.join("::")
}

/// Order facts deterministically inside one file's record so cache
/// files and SARIF stay byte-stable across runs.
///
/// ```
/// use core_ai_native_conform::{SourceFacts, sort_source_facts};
///
/// let sf = |file: &str| SourceFacts {
///     file: file.into(), crate_name: "x".into(), facts: vec![],
/// };
/// let sorted = sort_source_facts(vec![sf("b.rs"), sf("a.rs")]);
/// assert_eq!(sorted[0].file, "a.rs");
/// ```
pub fn sort_source_facts(mut all: Vec<SourceFacts>) -> Vec<SourceFacts> {
    all.sort_by(|a, b| a.file.cmp(&b.file));
    all
}

#[cfg(test)]
mod tests;
