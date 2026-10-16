//! Explicit trusted preparation and exact process approval. The host injects
//! its ProcessRunner (production: PodBay); no ambient/default runner exists.
use super::{
    ClientExecutable,
    executable_observation::{ExecutableObservation, observe},
};
use crate::process::{ProcessOutput, ProcessRunner, ProcessSpec, StreamMode, client_environment};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use vibe_agent_projection::client_commands::{Client, Operation, render, supported_version};
use vibe_wire::{
    behaviour::client_command,
    generated::client_command::e1::{
        plan::Plan,
        request::{Request, RequestClient, RequestOperation},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub executable: ClientExecutable,
    pub cwd: PathBuf,
    pub user_home: PathBuf,
    pub roots: Vec<PathBuf>,
    pub environment_ref: String,
    pub principal: String,
    pub policy: String,
    /// Trusted current version-evidence generation, not a request assertion.
    pub evidence: String,
    pub revoked: bool,
}
/// Trusted host policy/currentness input, never supplied by a package descriptor.
/// Implementations must return the complete current binding or refuse. This is
/// not a resolver and must not probe, refresh, install or invoke native code.
pub trait Authority: Send + Sync {
    fn current(&self, request: &Request) -> Result<Context, &'static str>;
}
pub struct Host {
    authority: Arc<dyn Authority>,
    runner: Arc<dyn ProcessRunner>,
    id: u64,
}
static IDS: AtomicU64 = AtomicU64::new(1);
/// Private held evidence; even a serialized Plan cannot construct this value.
pub struct Prepared {
    request: Request,
    context: Context,
    executable: ExecutableObservation,
    roots: Vec<(vibe_safefs::Project, String)>,
    spec: ProcessSpec,
    marketplace: Option<PathBuf>,
    plan: Plan,
    host: u64,
    id: u64,
}
/// Immutable, non-deserializable authority; no generic ExactGrant conversion.
pub struct ProcessGrant {
    host: u64,
    prepared: u64,
}
impl Prepared {
    /// Pure query over held observations. No runner, reprepare or world access.
    pub fn inspect(&self) -> &Plan {
        &self.plan
    }
}
fn recipe(q: &Request) -> Result<(Client, Operation), &'static str> {
    client_command::request(q)?;
    let client = match q.client {
        RequestClient::Claude => Client::Claude,
        RequestClient::Codex => Client::Codex,
        RequestClient::Opencode => Client::OpenCode,
    };
    if !supported_version(client, &q.version) {
        return Err("unsupported version");
    }
    let op = match q.operation {
        RequestOperation::Version => Operation::Version,
        RequestOperation::List => Operation::List,
        RequestOperation::MarketplaceAdd => {
            Operation::MarketplaceAdd(q.path.as_ref().ok_or("path")?.into())
        }
        RequestOperation::Install => Operation::Install(q.coordinate.clone().ok_or("coordinate")?),
        RequestOperation::Remove => Operation::Remove(q.coordinate.clone().ok_or("coordinate")?),
    };
    render(client, &op)?;
    Ok((client, op))
}
fn environment(
    context: &Context,
    client: Client,
) -> std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    let private = match client {
        Client::Claude => Some(("CLAUDE_CONFIG_DIR", context.user_home.join(".claude"))),
        Client::Codex => Some(("CODEX_HOME", context.user_home.join(".codex"))),
        Client::OpenCode => None,
    };
    client_environment(
        &context.user_home,
        private.as_ref().map(|(k, p)| (*k, p.as_path())),
    )
}
fn utf8(p: &std::ffi::OsStr) -> Result<String, &'static str> {
    p.to_str()
        .map(str::to_owned)
        .ok_or("unrepresentable native value")
}
impl Host {
    pub fn new(authority: Arc<dyn Authority>, runner: Arc<dyn ProcessRunner>) -> Self {
        Self {
            authority,
            runner,
            id: IDS.fetch_add(1, Ordering::Relaxed),
        }
    }
    /// Explicit effectful preparation performs only a bounded version probe.
    /// All operation argv remains inert until separate approval and execution.
    pub fn prepare(&self, request: Request) -> Result<Prepared, &'static str> {
        let (client, op) = recipe(&request)?;
        let context = self.authority.current(&request)?;
        if context.revoked
            || [
                &context.environment_ref,
                &context.principal,
                &context.policy,
                &context.evidence,
            ]
            .iter()
            .any(|v| v.is_empty())
        {
            return Err("missing or revoked host authority");
        }
        let launcher = context
            .executable
            .resolved_path()
            .ok_or("unbound executable")?;
        let executable = observe(launcher).map_err(|_| "executable unavailable")?;
        let mut roots = Vec::new();
        for p in context
            .roots
            .iter()
            .chain([&context.cwd, &context.user_home])
        {
            if !p.is_absolute()
                || p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err("unbound root");
            }
            let root = vibe_safefs::Project::open(p).map_err(|_| "root unavailable")?;
            let token = root.root_correlation_token().map_err(|_| "root identity")?;
            roots.push((root, token));
        }
        if context.roots.is_empty()
            || !context.roots.iter().any(|r| context.cwd.starts_with(r))
            || !context
                .roots
                .iter()
                .any(|r| context.user_home.starts_with(r))
        {
            return Err("cwd/home outside approved roots");
        }
        let marketplace = if let Operation::MarketplaceAdd(p) = &op {
            let canonical = std::fs::canonicalize(p).map_err(|_| "marketplace unavailable")?;
            let permitted = context
                .roots
                .iter()
                .any(|r| std::fs::canonicalize(r).is_ok_and(|root| canonical.starts_with(root)));
            if !permitted {
                return Err("marketplace outside approved roots");
            }
            Some(canonical)
        } else {
            None
        };
        let spec = ProcessSpec {
            program: launcher.into(),
            args: render(client, &op)?,
            cwd: context.cwd.clone(),
            env: environment(&context, client),
            stdin: None,
            stdout: StreamMode::Capture,
            stderr: StreamMode::Capture,
            scratch: context.cwd.clone(),
        };
        // Refuse nonrepresentable wire values before any process invocation.
        let argv: Vec<String> = spec
            .args
            .iter()
            .map(|s| utf8(s))
            .collect::<Result<_, _>>()?;
        let env: std::collections::BTreeMap<String, String> = spec
            .env
            .iter()
            .map(|(k, v)| Ok((utf8(k)?, utf8(v)?)))
            .collect::<Result<_, &'static str>>()?;
        let root_names: Vec<String> = context
            .roots
            .iter()
            .map(|p| utf8(p.as_os_str()))
            .collect::<Result<_, _>>()?;
        let launcher_text = utf8(spec.program.as_os_str())?;
        let resolved_text = utf8(executable.resolved.as_os_str())?;
        let cwd_text = utf8(spec.cwd.as_os_str())?;
        let mut probe = spec.clone();
        probe.args = render(client, &Operation::Version)?;
        let output = self
            .runner
            .run(&probe)
            .map_err(|_| "version probe failed")?;
        if output.code != Some(0) || output.stdout_truncated || output.stderr_truncated {
            return Err("incomplete version evidence");
        }
        let reply = std::str::from_utf8(&output.stdout).map_err(|_| "non UTF8 version")?;
        let observed_version = reply.split_whitespace().find(|s| {
            let parts: Vec<_> = s.split('.').collect();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        });
        if observed_version != Some(request.version.as_str()) {
            return Err("version evidence mismatch");
        }
        let plan = client_command::read_plan(&serde_json::to_vec(&serde_json::json!({
            "schema":1,"request":request,"executable":launcher_text,"resolved":resolved_text,
            "executable_digest":executable.state.sha256,"executable_identity":executable.identity,
            "version_reply":reply.trim(),"argv":argv,"cwd":cwd_text,"environment_ref":context.environment_ref,"environment":env,
            "roots":root_names,"principal":context.principal,"policy":context.policy,"evidence":context.evidence,
            "effects":if matches!(op,Operation::Version|Operation::List) { vec!["process execution", "client state observation"] } else { vec!["process execution", "client user state mutation", "client may execute installed active content"] }
        })).map_err(|_|"plan encoding")?)?;
        let prepared = Prepared {
            request,
            context,
            executable,
            roots,
            spec,
            marketplace,
            plan,
            host: self.id,
            id: IDS.fetch_add(1, Ordering::Relaxed),
        };
        self.check(&prepared)?;
        Ok(prepared)
    }
    pub(crate) fn check(&self, p: &Prepared) -> Result<(), &'static str> {
        if p.host != self.id {
            return Err("foreign prepared command");
        }
        let now = self.authority.current(&p.request)?;
        if now.revoked || now != p.context {
            return Err("stale context/version/environment/cwd/roots/principal/policy");
        }
        let (client, operation) = recipe(&p.request)?;
        if let Operation::MarketplaceAdd(path) = operation {
            if Some(std::fs::canonicalize(path).map_err(|_| "marketplace stale")?) != p.marketplace
            {
                return Err("marketplace mapping drift");
            }
        }
        if environment(&now, client) != p.spec.env {
            return Err("environment drift");
        }
        if observe(&p.executable.launcher).map_err(|_| "executable stale")? != p.executable {
            return Err("executable drift");
        }
        for (root, token) in &p.roots {
            let named = vibe_safefs::Project::open(root.root_path()).map_err(|_| "root stale")?;
            if named
                .root_correlation_token()
                .map_err(|_| "root identity")?
                != *token
                || root
                    .root_correlation_token()
                    .map_err(|_| "held root identity")?
                    != *token
            {
                return Err("root replaced");
            }
        }
        Ok(())
    }
    pub fn approve(&self, p: &Prepared) -> Result<ProcessGrant, &'static str> {
        self.check(p)?;
        Ok(ProcessGrant {
            host: self.id,
            prepared: p.id,
        })
    }
    /// Read-only exact opaque authority validation for composed preflight.
    pub fn validate_process_grant(
        &self,
        p: &Prepared,
        grant: Option<&ProcessGrant>,
    ) -> Result<(), &'static str> {
        let grant = grant.ok_or("missing process grant")?;
        if grant.host != self.id || grant.prepared != p.id {
            return Err("wrong or broadened process grant");
        }
        self.check(p)
    }
    pub fn execute(
        &self,
        p: &Prepared,
        grant: Option<&ProcessGrant>,
    ) -> Result<ProcessOutput, &'static str> {
        self.validate_process_grant(p, grant)?;
        self.runner
            .run(&p.spec)
            .map_err(|_| "approved process failed; effects unknown")
    }
}
#[cfg(test)]
mod m_08_d_1_cases;
