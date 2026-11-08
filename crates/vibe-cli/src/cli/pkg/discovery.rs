//! Argument structs for the read-only package queries — `vibe outdated` and
//! `vibe search`. Split from `pkg.rs` for the file-length budget; `pkg`
//! re-exports both, so `crate::cli::X` paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct OutdatedArgs {
    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Strict authentication gate — same semantics as
    /// `vibe install --auth-required` / `vibe update --auth-required`.
    /// When set, a 401 / 403 from an `auth = "none"` (public)
    /// registry halts the probe instead of walking past. The probe
    /// is read-only, so the trade-off is mostly diagnostic clarity
    /// — you want to see "this private registry is down" rather
    /// than silently miss its packages.
    #[arg(long)]
    pub auth_required: bool,

    /// Also compare GitHub upstream releases declared by locked `describes`
    /// PURLs. Only `pkg:github/<owner>/<repo>@<semver>` is supported; the
    /// package-registry check remains separate.
    #[arg(long)]
    pub upstream: bool,
}

#[derive(Debug, clap::Args)]
#[command(group(
    clap::ArgGroup::new("query_mode")
        .args(["query", "purl"])
        .required(true)
        .multiple(false)
))]
pub struct SearchArgs {
    #[command(flatten)]
    pub source_policy: crate::cli::SourcePolicyArgs,
    /// Free-text query. Tokenised on the server side: lowercase ASCII
    /// alphanumeric runs, common stopwords filtered, single-character
    /// tokens dropped. At least one 2+ character non-stopword must
    /// remain after filtering for the query to match anything.
    /// Mutually exclusive with `--purl`.
    pub query: Vec<String>,

    /// Direct PURL lookup — find every package whose `[package].describes`
    /// or any subskill's `describes` equals this Package URL. Mutually
    /// exclusive with the positional free-text query. Hits carry a
    /// `binding_site` field (`package` vs `subskill`) so consumers see
    /// where the match originated.
    #[arg(long)]
    pub purl: Option<String>,

    /// Restrict results to a single package kind (`flow`, `feat`, `stack`, `tool`, `mcp`, `lang`, `doc`, `app`, `remap`).
    /// Applies only to free-text search; PURL
    /// lookup ignores it.
    #[arg(long)]
    pub kind: Option<String>,

    /// Restrict to one configured `[[registry]]` by name. Default: walk
    /// every registry that has `VIBEVM_INDEX_URL_<R>` set in the
    /// environment.
    #[arg(long)]
    pub registry: Option<String>,

    /// Maximum hits to fetch from each registry's index server. The
    /// server may apply its own cap; the union is then deduplicated
    /// by `(kind, name)` keeping the highest-score hit. Defaults to
    /// 20 — large enough to be useful, small enough that no single
    /// registry dominates. Ignored on `--purl` lookups (PURL match
    /// is exact and saturates well below any reasonable limit).
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// For registries without a configured `VIBEVM_INDEX_URL_<R>`,
    /// fall back to a naive org-walk via the host's REST API. v0
    /// supports GitHub-hosted registries only (`github.com`); other
    /// hosts are reported as unsupported. Slower than an index;
    /// rate-limited; reads `vibe.toml` from each repo's HEAD
    /// via the GitHub Contents API. Ignored on `--purl` lookups.
    #[arg(long = "full-scan")]
    pub full_scan: bool,

    /// Bypass the persistent search cache under
    /// `~/.vibe/search-cache/`. Reads still go to the network even
    /// when a fresh entry exists; freshly fetched results are not
    /// written back. Useful for testing or when an index update
    /// landed and the operator wants to force a refresh without
    /// waiting for the TTL to expire.
    #[arg(long = "no-cache")]
    pub no_cache: bool,

    /// Override the default cache TTL (1 hour) — entries older than
    /// this many seconds are treated as misses. Ignored when
    /// `--no-cache` is set.
    #[arg(long = "cache-ttl")]
    pub cache_ttl: Option<u64>,

    /// Project root with `vibe.toml`. Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}
