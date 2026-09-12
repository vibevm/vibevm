//! `vibe-doc-server` — the local documentation reader (PROP-057 §11).
//!
//! `vibe doc serve` starts this. It binds **the loopback and nothing
//! else**, serves the pages of the documentation package it was pointed
//! at, and reaches the network never: no request to vibevm.org, no CDN,
//! no font service, no analytics tag (`##LOCAL-SERVE`,
//! `##LOCAL-NO-TELEMETRY`). That is not a hardening pass over a general
//! server — it is what the mode is for. This is how the documentation of
//! a proprietary package is read, and content that leaves the machine
//! has already failed.
//!
//! ## What it serves
//!
//! **Pages, in the shell.** A request for a page renders the island out
//! of the package on THAT request and glues it into the route template
//! the shell carries — the same prerendered route the public site serves,
//! built from the same source by the same adapter, with a marked hole
//! where the island goes (`##SHELL-SERVE-SOURCES`). The bytes around the
//! island are provably the site's, which is what makes the parity test in
//! this crate worth running. A binary carrying no shell serves the bare
//! one: typography, no scripts, still a page a person reads.
//!
//! Beside them: the shell's own statics, the two projections a page has
//! as files, the page manifest, the four `llms` tiers — the endpoints an
//! agent reads (§7.3 of the vision, `##SEO-LLMS-FILES`) — and the
//! `spec://` resolver, which is a route here and a redirect table on a
//! static host (the plan's fork F-15).
//!
//! ## The embedding contract is the shell's, and this server's job is
//! not to break it
//!
//! `##LOCAL-EMBEDDING-CONTRACT` is a conversation between a host
//! application and the PAGE: the host opens an address with
//! `{ "open": "spec://…" }`, the reader answers a local link with
//! `{ "openFile": "<path>" }`, the theme arrives as `{ "theme": … }` and
//! reading settings travel both ways as `{ "settings": {…} }`. Every one
//! of those is implemented in the shell's own behaviour and none of them
//! is a request, so there is nothing here to implement — only two things
//! not to get wrong. The page must be FRAMEABLE by the host that launched
//! it, which is what `--frame-ancestor <origin>` is for and why it is a
//! launch parameter rather than a setting (a webview's origin changes
//! from window to window). And the shell's scripts must actually run,
//! which is why the policy names each one by the hash of its bytes
//! instead of forbidding them all.
//!
//! One half of the contract does have a server answer here: a host that
//! opens a `spec://` address can ask `<base>resolve?uri=…` and be told
//! which page it is, rather than parsing a coordinate itself.
//!
//! ## Why it repeats `vibe-index`'s form instead of importing it
//!
//! `##PIPE-CRATES` says so, and the reason is visible in the manifests:
//! `vibe-index`'s router is inseparable from an `AppState` holding a
//! package index, and taking it would bring `reqwest`, `flate2`,
//! `walkdir` and the index itself along for about a hundred lines of
//! shape. So the SHAPE is repeated — axum 0.8, one router builder, RFC
//! 7807 errors, `oneshot` tests with no listener bound — and no code is
//! shared. The tree has taken that trade deliberately before, between
//! `vibe-index` and `vibe-registry`'s content hasher.
//!
//! ## The one form this server must not repeat
//!
//! `data_dir.join(<capture>)` over a percent-decoded path capture
//! (`##LOCAL-STATIC`). A router matches the RAW path and an extractor
//! decodes the capture afterwards, so `%2F` becomes a separator after
//! the match and walks out of the root. Every address here is decoded
//! and then split by this crate, and a segment that is not exactly one
//! ordinary name is refused before anything touches the filesystem.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-SERVE");

pub mod error;
pub mod routes;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use chrono::{DateTime, Utc};
use vibe_doc::citations::SpecSources;

pub use error::{ServerError, ServerResult};

/// The address the reader binds, and the only one it may.
///
/// `##LOCAL-SERVE` says «on 127.0.0.1 only», and «considered and
/// rejected: binding to `0.0.0.0`» is recorded beside it. There is no
/// flag for this and there is not going to be one: a reader that can be
/// reached from another machine is serving somebody else's proprietary
/// documentation to them.
pub const BIND_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// The port `vibe doc serve` takes when the operator names none.
pub const DEFAULT_PORT: u16 = 8413;

/// The content policy, verbatim from `##LOCAL-CSP` — one string,
/// because a policy assembled from parts is a policy nobody can read in
/// review. `frame-ancestors` is the only part that varies, and it is a
/// launch parameter because a webview's origin changes from window to
/// window.
pub const CSP_WITHOUT_FRAME_ANCESTORS: &str = "default-src 'self'; script-src 'self'; \
     style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; \
     media-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'";

/// The policy for a reader wearing `shell`, with its scripts named by
/// their bytes.
///
/// Two directives move away from the constant above, and both move
/// because the SHELL is there:
///
/// **`script-src` gains a hash per inline script.** The shell's head
/// carries scripts that cannot become files — the theme, which must run
/// before the first stylesheet or a reader watches their own setting
/// fail; the router's scroll restoration, which must be undone before the
/// router's bootstrap; the framework's own loader. Each is named by the
/// sha256 of its bytes, computed HERE, at start-up, from the shell this
/// binary is carrying — never written into a constant, which is exactly
/// what the deferral X-035 asked for. No external source is named at any
/// point: the whole page comes from this process.
///
/// **`style-src` gains `'unsafe-inline'`, and only with a real shell.**
/// The framework inlines each component's stylesheet into the document,
/// and the reading settings write the measure and the font size onto an
/// element as a `style` attribute — which no hash can cover, because a
/// hash names an element's CONTENT and an attribute has none. The public
/// site's own build reached the same place for the same reason. The bare
/// shell needs none of it and does not get it: its stylesheet is a file.
///
/// ```
/// use vibe_doc_server::content_policy;
/// use vibe_doc_shell::{Shell, template};
///
/// // The canonical use: the shell a reader resolved, the template it
/// // carries, and the origin allowed to frame it.
/// let shell = Shell::open(None);
/// let page = shell.page_template();
/// let policy = content_policy(&shell, &page, Some("vscode-webview://abc"));
///
/// assert!(policy.starts_with("default-src 'self'"));
/// assert!(policy.ends_with("frame-ancestors vscode-webview://abc"));
/// // Every inline script the shell carries is named by its own bytes.
/// for body in template::inline_scripts(&page) {
///     assert!(policy.contains(&template::hash_of(body)));
/// }
/// ```
pub fn content_policy(
    shell: &vibe_doc_shell::Shell,
    template: &str,
    frame_ancestor: Option<&str>,
) -> String {
    let hashes = vibe_doc_shell::template::hashes(template);
    let script = if hashes.is_empty() {
        "'self'".to_string()
    } else {
        format!("'self' {}", hashes.join(" "))
    };
    let style = if shell.provenance() == vibe_doc_shell::Provenance::Fallback {
        "'self'"
    } else {
        "'self' 'unsafe-inline'"
    };
    format!(
        "default-src 'self'; script-src {script}; style-src {style}; img-src 'self' data:; \
         font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; \
         base-uri 'none'; form-action 'none'; frame-ancestors {}",
        frame_ancestor.unwrap_or("'none'")
    )
}

/// What the reader was started with.
///
/// ```
/// use vibe_doc_server::{Config, DEFAULT_PORT};
///
/// // A reader for one package, on the site's own mount.
/// let config = Config::new("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0");
/// assert_eq!(config.base, "/doc/");
/// assert_eq!(config.port, DEFAULT_PORT);
/// // There is no host field: the reader binds the loopback and nothing
/// // else, which is a rule rather than a default.
/// assert!(config.frame_ancestor.is_none());
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// The documentation package to serve.
    pub package_dir: PathBuf,
    /// The path the documentation is mounted at, with both slashes:
    /// `/doc/` on the site, and the same by default here so a link
    /// written on a page works in both worlds.
    pub base: String,
    pub port: u16,
    /// The origin allowed to frame the reader, for an embedded webview.
    /// Absent means `'none'`, which is the right answer for a reader
    /// started from a terminal.
    pub frame_ancestor: Option<String>,
    /// The language the caller expects the package to be in; a package
    /// in another one is refused at startup rather than served under a
    /// wrong label.
    pub lang: Option<String>,
}

impl Config {
    /// A reader for `package_dir` on the site's own base and the default
    /// port.
    pub fn new(package_dir: impl Into<PathBuf>) -> Config {
        Config {
            package_dir: package_dir.into(),
            base: vibe_doc::content::SITE_BASE.to_string(),
            port: DEFAULT_PORT,
            frame_ancestor: None,
            lang: None,
        }
    }
}

/// The reader's state: where the package is, what it is called, and the
/// world its citations resolve against.
///
/// Everything here is resolved ONCE, at startup, and everything else is
/// read per request. That split is the contract: `##LOCAL-SERVE` says
/// the island is rendered on every request, so an author who edits a
/// page sees the edit on reload — while the coordinate, the base and
/// the generated `derived` text are properties of the run and re-reading
/// them per request would only make the reader slower and less
/// predictable.
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc_server::{Config, Reader};
///
/// let tmp = tempfile::tempdir().unwrap();
/// std::fs::write(
///     tmp.path().join("vibe.toml"),
///     "[package]\nname = \"a-docs\"\ngroup = \"com.example\"\nversion = \"0.1.0\"\nkind = \"doc\"\n\
///      title = \"A\"\nabstract = \"What it covers.\"\n\
///      [[documents]]\npackage = \"com.example/a\"\nversion = \"^1.0\"\n",
/// )
/// .unwrap();
///
/// let when = Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap();
/// let reader = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when).unwrap();
/// // The reader answers at the package's own address — the one the
/// // public site mounts it at, so a link works in both worlds.
/// assert_eq!(reader.mount(), "/doc/com.example/a-docs/0.1.0/");
/// assert!(reader.csp.ends_with("frame-ancestors 'none'"));
/// ```
#[derive(Debug)]
pub struct Reader {
    pub package_dir: PathBuf,
    pub base: String,
    /// `<group>/<name>/<version>/` — the one address prefix this reader
    /// answers to.
    pub prefix: String,
    pub csp: String,
    pub sources: SpecSources,
    pub derived: std::collections::BTreeMap<String, String>,
    pub rendered_at: DateTime<Utc>,
    /// The shell this reader wears, and the route template it carries
    /// with the reader's own base already in it. Both are resolved ONCE:
    /// the shell cannot change while a process runs, and re-reading a
    /// megabyte of it per request would buy nothing.
    pub shell: vibe_doc_shell::Shell,
    pub template: String,
}

impl Reader {
    /// Open `config`'s package: read its coordinate, fix its address
    /// prefix and its content policy.
    ///
    /// `sources` is the world a citation resolves against; the composition
    /// root names it, as everywhere else in this pipeline.
    pub fn open(
        config: &Config,
        sources: SpecSources,
        rendered_at: DateTime<Utc>,
    ) -> ServerResult<Reader> {
        Reader::wearing(config, sources, rendered_at, vibe_doc_shell::Shell::bare())
    }

    /// The same, wearing a shell the caller resolved.
    ///
    /// The shell arrives rather than being looked up, for the reason
    /// everything else in this pipeline does: where a machine keeps its
    /// files is the composition root's knowledge, and a server that went
    /// looking would be a server with an opinion about a home directory.
    pub fn wearing(
        config: &Config,
        sources: SpecSources,
        rendered_at: DateTime<Utc>,
        shell: vibe_doc_shell::Shell,
    ) -> ServerResult<Reader> {
        if let Some(lang) = &config.lang {
            vibe_doc::build::expect_language(&config.package_dir, lang)?;
        }
        let (coordinate, version) = coordinate_of(&config.package_dir)?;
        // The template is repointed at this reader's mount before
        // anything else touches it: the shell was built for one base, and
        // every address inside it is the build's own.
        let template = vibe_doc_shell::template::rebase(
            &shell.page_template(),
            &shell.index().base,
            &config.base,
        );
        Ok(Reader {
            package_dir: config.package_dir.clone(),
            base: config.base.clone(),
            prefix: format!("{coordinate}/{version}/"),
            csp: content_policy(&shell, &template, config.frame_ancestor.as_deref()),
            sources,
            derived: std::collections::BTreeMap::new(),
            rendered_at,
            shell,
            template,
        })
    }

    /// Carry generated `derived` text, built once by the caller.
    ///
    /// A block's text is a function of the product's binary, and the
    /// binary does not change while a server runs — so generating it per
    /// request would spawn a process per block per page view for an
    /// answer that cannot have moved.
    #[must_use]
    pub fn with_derived(mut self, derived: std::collections::BTreeMap<String, String>) -> Reader {
        self.derived = derived;
        self
    }

    /// The address this reader serves its pages under, for a start-up
    /// line an operator can click.
    pub fn mount(&self) -> String {
        format!("{}{}", self.base, self.prefix)
    }
}

/// `<group>/<name>` and the version of the package at `dir`.
///
/// Through the typed manifest model, not a second reading of TOML: the
/// address a reader answers to is the package's IDENTITY, and identity
/// has one home (`vibe_core::manifest`). A documentation package that
/// does not parse there is one `vibe check` already refuses.
fn coordinate_of(dir: &Path) -> ServerResult<(String, String)> {
    let path = dir.join(vibe_core::manifest::Manifest::FILENAME);
    let manifest =
        vibe_core::manifest::Manifest::read(&path).map_err(|e| ServerError::Package {
            path: path.clone(),
            message: format!("is not a readable manifest: {e}"),
        })?;
    let Some(package) = manifest.package else {
        return Err(ServerError::Package {
            path,
            message: "carries no `[package]` table — a reader serves a package at its \
                      coordinate, and a project has none to serve under"
                .to_string(),
        });
    };
    Ok((
        format!("{}/{}", package.group.as_str(), package.name),
        package.version.to_string(),
    ))
}

/// Build the reader's router.
///
/// Every address is answered by one handler that parses the path itself
/// ([`routes::dispatch`]) rather than by captures a router decodes, for
/// the reason in this module's header. `/healthz` is the single
/// exception, and it reads no path at all.
///
/// ```
/// use vibe_doc_server::{Config, Reader, build_app};
/// use vibe_doc::citations::SpecSources;
/// use chrono::{TimeZone, Utc};
///
/// let tmp = tempfile::tempdir().unwrap();
/// std::fs::write(
///     tmp.path().join("vibe.toml"),
///     "[package]\nname = \"a-docs\"\ngroup = \"com.example\"\nversion = \"0.1.0\"\nkind = \"doc\"\n\
///      title = \"A\"\nabstract = \"What it covers.\"\n\
///      [[documents]]\npackage = \"com.example/a\"\nversion = \"^1.0\"\n",
/// )
/// .unwrap();
/// let when = Utc.with_ymd_and_hms(2026, 9, 12, 0, 0, 0).unwrap();
/// let reader = Reader::open(&Config::new(tmp.path()), SpecSources::new(), when).unwrap();
/// assert_eq!(reader.mount(), "/doc/com.example/a-docs/0.1.0/");
/// let _app = build_app(std::sync::Arc::new(reader));
/// ```
pub fn build_app(reader: Arc<Reader>) -> Router {
    routes::router(reader)
}

/// Run the reader until the operator interrupts it.
///
/// Synchronous: it builds its own runtime, exactly as `vibe-index serve`
/// does, so a CLI verb stays a CLI verb and no `async` reaches the
/// command layer.
///
/// ```no_run
/// use chrono::Utc;
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc_server::{Config, DEFAULT_PORT, Reader, serve};
///
/// // The canonical use: open a package, then read it until Ctrl-C.
/// let config = Config::new("vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0");
/// let reader = Reader::open(&config, SpecSources::new(), Utc::now())?;
/// serve(reader, DEFAULT_PORT)?;
/// # Ok::<(), vibe_doc_server::ServerError>(())
/// ```
pub fn serve(reader: Reader, port: u16) -> ServerResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| ServerError::Bind {
            message: format!("no runtime: {e}"),
        })?;
    runtime.block_on(async move {
        let mount = reader.mount();
        let app = build_app(Arc::new(reader));
        let address = SocketAddr::from((BIND_HOST, port));
        let listener =
            tokio::net::TcpListener::bind(address)
                .await
                .map_err(|e| ServerError::Bind {
                    message: format!("cannot listen on {address}: {e}"),
                })?;
        let bound = listener.local_addr().unwrap_or(address);
        println!("reading documentation at http://{bound}{mount}");
        println!("  (loopback only; stop with Ctrl-C)");
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| ServerError::Bind {
                message: format!("the reader stopped: {e}"),
            })
    })
}

#[cfg(test)]
mod tests;
