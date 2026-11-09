//! The explicit proxy and CA configuration of a child `git`
//! (`PROP-068#proxy`). Passed per invocation (`-c` and environment);
//! the user's git configuration is never written. SSH remotes are not
//! proxied: their route is the SSH client's own configuration.

use std::path::Path;
use std::sync::OnceLock;

use vibe_core::user_config::RevocationCheck;

use crate::{NetError, NetworkSession};

/// What one child `git` that talks to a remote receives.
///
/// ```
/// use vibe_core::net::ProxyRoute;
/// use vibe_core::user_config::ProxyConfig;
/// use vibe_net::NetworkSession;
///
/// let cfg: ProxyConfig = toml::from_str("https = 'http://proxy.example.corp:8080'\nno_proxy = '.corp.example'").unwrap();
/// let session = NetworkSession::new(ProxyRoute::from_config(&cfg).unwrap());
/// let net = vibe_net::git::for_remote(&session, "https://github.com/org/repo.git").unwrap();
/// assert!(net.config.contains(&"http.proxy=http://proxy.example.corp:8080/".to_owned()));
/// assert!(net.env.iter().any(|(k, v)| k == "NO_PROXY" && v.contains(".corp.example")));
/// // A bypassed host is told to connect directly.
/// let net = vibe_net::git::for_remote(&session, "https://git.corp.example/org/repo.git").unwrap();
/// assert!(net.config.contains(&"http.proxy=".to_owned()));
/// // SSH is out of scope: nothing is added.
/// assert!(vibe_net::git::for_remote(&session, "ssh://git@github.com/org/repo.git").unwrap().is_empty());
/// // No `[proxy]`: the child is left exactly as before.
/// let direct = NetworkSession::new(ProxyRoute::direct());
/// assert!(vibe_net::git::for_remote(&direct, "https://github.com/a/b.git").unwrap().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitNetwork {
    /// `-c` values, in order.
    pub config: Vec<String>,
    /// Environment variables to set on the child.
    pub env: Vec<(String, String)>,
}

impl GitNetwork {
    pub fn is_empty(&self) -> bool {
        self.config.is_empty() && self.env.is_empty()
    }

    /// Apply to a command (before its subcommand arguments).
    pub fn apply(&self, command: &mut std::process::Command) {
        for value in &self.config {
            command.arg("-c").arg(value);
        }
        for (name, value) in &self.env {
            command.env(name, value);
        }
    }
}

/// The `-c` values `[proxy] revocation` adds to a child `git`
/// (`PROP-068#proxy-revocation`).
///
/// `http.schannelCheckRevoke` is git's own key and git applies it only
/// when `http.sslBackend` is `schannel`, so this changes nothing for a
/// git built against OpenSSL and nothing at all off Windows. `check` is
/// the default and adds nothing — the bytes a child receives are exactly
/// what they were before the key existed.
///
/// `best-effort` is Git for Windows' own third value for the key (since
/// 2.26, where it is that build's default); an upstream git parses the
/// key as a boolean and would refuse the word outright, so it is sent
/// only to a git that reports a Git for Windows version, and otherwise
/// falls back to `off` with a note. `git` is the executable about to run,
/// for that one `git --version` (asked at most once per process, and only
/// for `best-effort` on Windows).
///
/// ```
/// use vibe_core::user_config::RevocationCheck;
/// use vibe_net::git::revocation_config;
/// // Nothing is added by the default, on any host.
/// assert!(revocation_config(RevocationCheck::Check, None).is_empty());
/// if cfg!(windows) {
///     assert_eq!(
///         revocation_config(RevocationCheck::Off, None),
///         vec!["http.schannelCheckRevoke=false".to_owned()]
///     );
/// } else {
///     // Schannel is a Windows TLS stack: elsewhere the key is a no-op.
///     assert!(revocation_config(RevocationCheck::Off, None).is_empty());
/// }
/// ```
pub fn revocation_config(revocation: RevocationCheck, git: Option<&Path>) -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let value = match revocation {
        RevocationCheck::Check => return Vec::new(),
        RevocationCheck::Off => "false",
        RevocationCheck::BestEffort if best_effort_supported(git) => "best-effort",
        RevocationCheck::BestEffort => {
            tracing::warn!(
                "`[proxy] revocation = \"best-effort\"` needs a Git for Windows 2.26 or newer; \
                 this git parses `http.schannelCheckRevoke` as a boolean, so revocation \
                 checking is switched off instead"
            );
            "false"
        }
    };
    vec![format!("http.schannelCheckRevoke={value}")]
}

/// Whether the git about to run understands `best-effort`; asked once.
fn best_effort_supported(git: Option<&Path>) -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        let program = git.unwrap_or_else(|| Path::new("git"));
        std::process::Command::new(program)
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .is_some_and(|output| {
                version_takes_best_effort(&String::from_utf8_lossy(&output.stdout))
            })
    })
}

/// Whether a `git --version` line names a Git for Windows 2.26 or newer,
/// the builds whose `http.schannelCheckRevoke` takes `best-effort`.
///
/// ```
/// use vibe_net::git::version_takes_best_effort;
/// assert!(version_takes_best_effort("git version 2.52.0.windows.1"));
/// assert!(version_takes_best_effort("git version 2.26.0.windows.1\n"));
/// assert!(version_takes_best_effort("git version 3.0.0.windows.1"));
/// // Older Git for Windows, and every upstream build, take a boolean only.
/// assert!(!version_takes_best_effort("git version 2.25.1.windows.1"));
/// assert!(!version_takes_best_effort("git version 2.52.0"));
/// assert!(!version_takes_best_effort("git version 2.47.1.msysgit.0"));
/// assert!(!version_takes_best_effort("nonsense"));
/// ```
pub fn version_takes_best_effort(line: &str) -> bool {
    let line = line.trim();
    if !line.contains(".windows.") {
        return false;
    }
    let Some(rest) = line.split_whitespace().nth(2) else {
        return false;
    };
    let mut parts = rest.split('.');
    let (Some(Ok(major)), Some(Ok(minor))) = (
        parts.next().map(str::parse::<u32>),
        parts.next().map(str::parse::<u32>),
    ) else {
        return false;
    };
    (major, minor) >= (2, 26)
}

/// The bypass list a child receives: the configured one plus loopback.
fn no_proxy_env(session: &NetworkSession) -> String {
    let mut list = String::from("localhost,127.0.0.1,::1");
    if let Some(configured) = session.route().no_proxy() {
        list.push(',');
        list.push_str(configured);
    }
    list
}

/// The configuration a child `git` talking to `remote` receives under the
/// current route. A remote that is not HTTP(S) gets nothing.
pub fn for_remote(session: &NetworkSession, remote: &str) -> Result<GitNetwork, NetError> {
    for_remote_with(session, remote, None)
}

/// [`for_remote`], naming the git executable about to run so that
/// `[proxy] revocation = "best-effort"` can ask whether it takes the word.
pub fn for_remote_with(
    session: &NetworkSession,
    remote: &str,
    git: Option<&Path>,
) -> Result<GitNetwork, NetError> {
    let raw = remote.strip_prefix("git+").unwrap_or(remote);
    let Ok(url) = url::Url::parse(raw) else {
        return Ok(GitNetwork::default());
    };
    if !matches!(url.scheme(), "http" | "https") {
        return Ok(GitNetwork::default());
    }
    let mut net = GitNetwork::default();
    if session.route().is_proxied() {
        let proxy = session.proxy_for(&url)?;
        match &proxy {
            // A proxy credential stays out of the argument list (visible
            // to every local user); git reads it from the environment.
            Some(proxy) if !proxy.username().is_empty() || proxy.password().is_some() => {}
            Some(proxy) => net.config.push(format!("http.proxy={proxy}")),
            // An empty value disables proxying for this child entirely.
            None => net.config.push("http.proxy=".to_owned()),
        }
        if let Some(proxy) = &proxy {
            for name in ["HTTPS_PROXY", "HTTP_PROXY"] {
                net.env.push((name.to_owned(), proxy.to_string()));
            }
        }
        net.env.push(("NO_PROXY".to_owned(), no_proxy_env(session)));
    }
    if url.scheme() == "https" {
        // Revocation is a property of the child's own TLS, not of the
        // proxy: it is added for every HTTPS remote, proxied or not.
        net.config
            .extend(revocation_config(session.route().revocation(), git));
    }
    if url.scheme() == "https"
        && let Some(bundle) = session.ca_bundle()?
    {
        net.config
            .push(format!("http.sslCAInfo={}", bundle.display()));
        if cfg!(windows) {
            // Schannel ignores sslCAInfo unless told; the bundle carries
            // the system roots too, so nothing is taken away.
            net.config.push("http.schannelUseSSLCAInfo=true".to_owned());
        }
    }
    Ok(net)
}

/// [`for_remote`] under the current operation's session.
pub fn current_for_remote(remote: &str) -> Result<GitNetwork, NetError> {
    for_remote(&crate::current(), remote)
}

/// The route for a child about to run `git <args>` (in `cwd`): the remote
/// it contacts is a URL among `args`, or the named remote of a `fetch` /
/// `pull` / `push` / `ls-remote`, read from the repository's own config
/// with `binary`. A local-only call gets nothing and never starts px.
pub fn for_args(
    session: &NetworkSession,
    binary: &std::path::Path,
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> Result<GitNetwork, NetError> {
    if !session.route().is_proxied()
        && session.route().ca_file().is_none()
        && session.route().revocation() == RevocationCheck::Check
    {
        return Ok(GitNetwork::default());
    }
    let is_http = |arg: &str| {
        let raw = arg.strip_prefix("git+").unwrap_or(arg);
        raw.starts_with("https://") || raw.starts_with("http://")
    };
    let remote = match args.iter().find(|arg| is_http(arg)) {
        Some(url) => Some((*url).to_owned()),
        None => named_remote_url(binary, args, cwd).filter(|url| is_http(url)),
    };
    match remote {
        Some(remote) => for_remote_with(session, &remote, Some(binary)),
        None => Ok(GitNetwork::default()),
    }
}

/// [`for_args`] applied to `command` under the current session. When the
/// route cannot be served (a px that failed, an unreadable `ca_file`), the
/// child is pointed at an unresolvable proxy instead of going direct, and
/// the cause is logged; the operation reports it once (`PROP-068#proxy-px`).
pub fn apply_for_args(
    command: &mut std::process::Command,
    binary: &std::path::Path,
    args: &[&str],
    cwd: Option<&std::path::Path>,
) {
    match for_args(&crate::current(), binary, args, cwd) {
        Ok(net) => net.apply(command),
        Err(error) => {
            tracing::warn!("{error}");
            for name in ["HTTPS_PROXY", "HTTP_PROXY"] {
                command.env(name, crate::http::UNROUTABLE);
            }
            command.args(["-c", &format!("http.proxy={}", crate::http::UNROUTABLE)]);
        }
    }
}

/// The URL of the remote a network verb names (`origin` in
/// `fetch -- origin <refspec>`), read from the repository's own config.
fn named_remote_url(
    binary: &std::path::Path,
    args: &[&str],
    cwd: Option<&std::path::Path>,
) -> Option<String> {
    let verb = args.first()?;
    if !matches!(*verb, "fetch" | "pull" | "push" | "ls-remote") {
        return None;
    }
    let name = args[1..].iter().find(|arg| !arg.starts_with('-'))?;
    let mut query = std::process::Command::new(binary);
    query.args(["config", "--get", &format!("remote.{name}.url")]);
    if let Some(dir) = cwd {
        query.current_dir(dir);
    }
    let output = query.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
