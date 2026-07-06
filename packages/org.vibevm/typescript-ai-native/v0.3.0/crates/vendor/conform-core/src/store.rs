specmark::scope!("spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#facts");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::facts::{Fact, Frontend, SourceFacts};

/// What one extraction run did — the producer log the incremental
/// acceptance test asserts on.
///
/// ```
/// let log = conform_core::ExtractionLog::default();
/// assert_eq!(log.cached, 0);
/// assert!(log.extracted.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct ExtractionLog {
    /// Files actually re-extracted this run (cache misses).
    pub extracted: Vec<String>,
    /// Cache hits.
    pub cached: usize,
}

/// Content-addressed fact store under `<repo>/target/conform/facts/`.
///
/// ```no_run
/// use conform_core::{Config, ExtractionLog, Store};
/// # use conform_core::{Fact, Frontend};
/// # struct NullFrontend;
/// # impl Frontend for NullFrontend {
/// #     fn id(&self) -> &'static str { "null" }
/// #     fn version(&self) -> &'static str { "1" }
/// #     fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> { Vec::new() }
/// # }
///
/// let repo = std::path::Path::new(".");
/// let store = Store::at_repo(repo, &Config::default());
/// let mut log = ExtractionLog::default();
/// let facts = store.extract_workspace(repo, &NullFrontend, &mut log).unwrap();
/// println!("{} file(s) extracted, {} cached", log.extracted.len(), log.cached);
/// # let _ = facts;
/// ```
pub struct Store {
    root: PathBuf,
    roots: Vec<String>,
    exclude: Vec<String>,
}

impl Store {
    pub fn at_repo(repo: &Path, config: &Config) -> Store {
        Store {
            root: repo.join("target").join("conform").join("facts"),
            roots: config.roots.clone(),
            exclude: config.exclude_substrings.clone(),
        }
    }

    fn slot(&self, frontend: &dyn Frontend, content_hash: &str) -> PathBuf {
        self.root
            .join(format!("{}-{}", frontend.id(), frontend.version()))
            .join(format!("{content_hash}.json"))
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
        let sources = workspace_sources(repo, &self.roots, &self.exclude);
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
        let sources = typescript_sources(repo, &self.roots, &self.exclude);
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
/// let lf = conform_core::content_hash("a\nb\n");
/// let crlf = conform_core::content_hash("a\r\nb\r\n");
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

/// Enumerate the configured source roots as `(repo-rel file, crate
/// name, module path, absolute path)`. A `<dir>/*` root scans each
/// subdirectory of `<dir>` as one crate; any other root is a literal
/// crate dir. `src/` and `tests/` of each are walked (tests carry the
/// Class-D oracle facts), and files whose path contains an `exclude`
/// substring are skipped.
fn workspace_sources(
    repo: &Path,
    roots: &[String],
    exclude: &[String],
) -> Vec<(String, String, String, PathBuf)> {
    let mut crate_dirs: Vec<PathBuf> = Vec::new();
    for root in roots {
        if let Some(parent) = root.strip_suffix("/*") {
            if let Ok(rd) = std::fs::read_dir(repo.join(parent)) {
                for entry in rd.filter_map(Result::ok) {
                    if entry.path().is_dir() {
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

    let mut out = Vec::new();
    for crate_dir in crate_dirs {
        let crate_name = crate_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let crate_ident = crate_name.replace('-', "_");
        for sub in ["src", "tests"] {
            let dir = crate_dir.join(sub);
            for entry in walkdir::WalkDir::new(&dir)
                .sort_by_file_name()
                .into_iter()
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
                if exclude.iter().any(|s| rel_fwd.contains(s.as_str())) {
                    continue;
                }
                let module = module_path(&crate_ident, &rel_fwd);
                let file = path
                    .strip_prefix(repo)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((file, crate_name.clone(), module, path.to_path_buf()));
            }
        }
    }
    out
}

/// TypeScript source extensions the flat walk accepts.
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts"];
/// Directory names the TypeScript walk never descends into — resolved
/// installs and build output, mirroring the extractor's own skip list.
const TS_SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".git",
    "vibedeps",
    "target",
];

/// Enumerate TypeScript sources as `(repo-rel file, root name, module,
/// absolute path)`. Unlike the Rust walk there is no crate topology:
/// each configured root (literal dir or `<dir>/*`) is walked whole,
/// the "crate" is the root's directory name, and the module is the
/// repo-relative path itself (TS modules ARE paths). `.d.ts` files are
/// shapes, not code — skipped, matching the extractor.
fn typescript_sources(
    repo: &Path,
    roots: &[String],
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
        let root_name = root_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        for entry in walkdir::WalkDir::new(&root_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !e.file_type().is_dir()
                    || e.file_name()
                        .to_str()
                        .map(|n| !TS_SKIP_DIRS.contains(&n) && !n.starts_with('.'))
                        .unwrap_or(true)
            })
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
/// use conform_core::{SourceFacts, sort_source_facts};
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
