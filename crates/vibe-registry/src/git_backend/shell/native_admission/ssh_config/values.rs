//! Lower selected OpenSSH values, preserving ordered identities and exact routing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::super::platform;
use super::*;
pub(super) struct Endpoint<'a> {
    pub host: &'a str,
    pub original: &'a str,
    pub user: &'a str,
    pub port: u16,
}
/// The refusal for a path-valued OpenSSH setting this capture cannot use.
/// It names the client that produced the value, the keyword it was reported
/// under, the exact value and the way out, because the setting may have come
/// from any file of the Include closure or from the client's own defaults.
pub(super) fn refused_path(
    input: &SshConfigSelection,
    key: &str,
    value: &str,
    reason: &'static str,
    fix: &'static str,
) -> GitError {
    GitError::SshConfigPath {
        executable: input.executable.display().to_string(),
        key: key.to_owned(),
        value: value.to_owned(),
        reason,
        fix,
    }
}
/// The caller's captured `PROGRAMDATA`, under whichever spelling the
/// invocation used — Windows compares variable names without case. Only the
/// environment the composition owner explicitly captured is read; nothing
/// ambient is consulted here.
fn program_data(input: &SshConfigSelection) -> Option<&str> {
    input
        .environment
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("PROGRAMDATA"))
        .map(|(_, value)| value.as_str())
}
pub(super) fn path(
    value: &str,
    input: &SshConfigSelection,
    endpoint: Option<&Endpoint<'_>>,
    windows: bool,
    key: &str,
) -> Result<PathBuf, GitError> {
    expand_path(value, input, endpoint, false, windows)
        .map(PathBuf::from)
        .map_err(|reason| refused_path(input, key, value, reason, EXPANSION_FIX))
}
pub(super) fn deferred_path(
    value: &str,
    input: &SshConfigSelection,
    windows: bool,
    key: &str,
) -> Result<String, GitError> {
    expand_path(value, input, None, true, windows)
        .map_err(|reason| refused_path(input, key, value, reason, EXPANSION_FIX))
}
const EXPANSION_FIX: &str = "write the path out in full in your SSH configuration, or remove the setting to use the client default";
fn expand_path(
    value: &str,
    input: &SshConfigSelection,
    endpoint: Option<&Endpoint<'_>>,
    deferred: bool,
    windows: bool,
) -> Result<String, &'static str> {
    let mut expanded = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.next() != Some('{') {
                return Err("unsupported SSH path environment syntax");
            }
            let mut name = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(c) => name.push(c),
                    None => return Err("unterminated SSH path variable"),
                }
            }
            if name == "HOME" {
                expanded.push_str(input.home.to_str().ok_or("SSH home is not UTF-8")?);
            } else {
                expanded.push_str(
                    input
                        .environment
                        .get(&name)
                        .ok_or("SSH path expansion variable is unavailable")?,
                );
            }
        } else if c == '%' {
            match chars.next() {
                Some('%') => {
                    expanded.push('%');
                    if deferred {
                        expanded.push('%');
                    }
                }
                Some('d') => expanded.push_str(input.home.to_str().ok_or("SSH home is not UTF-8")?),
                Some(token) => {
                    if deferred && matches!(token, 'h' | 'n' | 'r' | 'p') {
                        expanded.push('%');
                        expanded.push(token);
                        continue;
                    }
                    let endpoint =
                        endpoint.ok_or("context-dependent SSH Include token is not captured")?;
                    match token {
                        'h' => expanded.push_str(endpoint.host),
                        'n' => expanded.push_str(endpoint.original),
                        'r' => expanded.push_str(endpoint.user),
                        'p' => expanded.push_str(&endpoint.port.to_string()),
                        _ => return Err("unsupported SSH path token"),
                    }
                }
                None => return Err("incomplete SSH path token"),
            }
        } else {
            expanded.push(c);
        }
    }
    // The Windows OpenSSH client reports its system paths with the literal
    // `__PROGRAMDATA__` and expands that token itself; the capture resolves
    // it from the explicitly captured invocation environment, exactly as the
    // Windows system `ssh_config` location is resolved.
    if windows
        && let Some(resolved) = program_data(input)
            .and_then(|program_data| platform::expand_program_data(&expanded, program_data))
    {
        return Ok(resolved);
    }
    if expanded == "~" {
        return input
            .home
            .to_str()
            .map(str::to_owned)
            .ok_or("SSH home is not UTF-8");
    }
    if let Some(tail) = expanded.strip_prefix("~/") {
        return input
            .home
            .join(tail)
            .to_str()
            .map(str::to_owned)
            .ok_or("SSH path is not UTF-8");
    }
    if expanded.starts_with('~') {
        return Err("other-user SSH home expansion is not captured");
    }
    Ok(expanded)
}

/// Every known-hosts file the selected client's reported
/// `UserKnownHostsFile` / `GlobalKnownHostsFile` select, in the order it
/// reported them, each paired with the keyword that named it. `none`, the
/// Windows `NUL` and `/dev/null` are the spellings of "no trust from this
/// entry" and select nothing.
///
/// A **global** entry this capture cannot lower to one absolute file selects
/// nothing instead of refusing the whole context. Two real clients produce
/// such a value: Win32-OpenSSH prints `__PROGRAMDATA__\ssh/ssh_known_hosts`
/// and the caller's `PROGRAMDATA` may not have been captured, and the MSYS
/// client Git for Windows bundles prints its compiled-in
/// `/etc/ssh/ssh_known_hosts`, which carries no drive and so names a
/// different file on every drive the process may be on — what would resolve
/// it is that client's MSYS mount table, which this capture neither reads
/// nor witnesses, so any path chosen for it would be a guess rather than a
/// fact. Dropping a known-hosts file can only ever **narrow** trust: the
/// owned configuration offers the child fewer host keys, never more, so an
/// endpoint whose key lived only in the dropped file is refused at connect
/// time instead of being silently accepted; if nothing is left, the capture
/// still refuses outright below. A file that does not exist already
/// contributes nothing, and that is exactly what the client itself does with
/// a path it cannot open.
///
/// A **user** entry is still refused. It is the caller's own authored
/// setting rather than a compiled-in default, it is where this capture puts
/// the selected home's defaults, and an operator who wrote an unusable path
/// there has to be told, not have their configured trust quietly dropped.
fn trust_files(
    fields: &BTreeMap<String, Vec<String>>,
    input: &SshConfigSelection,
    endpoint: Option<&Endpoint<'_>>,
    windows: bool,
) -> Result<Vec<(&'static str, PathBuf)>, GitError> {
    let mut files = Vec::new();
    for kind in ["userknownhostsfile", "globalknownhostsfile"] {
        let global = kind == "globalknownhostsfile";
        for values in fields.get(kind).into_iter().flatten() {
            for value in inputs::words_for(values, windows)? {
                // `NUL` is the Windows spelling of `/dev/null`.
                if value == "none" || (windows && value.eq_ignore_ascii_case("NUL")) {
                    continue;
                }
                let lowered = match path(&value, input, endpoint, windows, kind) {
                    Ok(lowered) if platform::absolute_path(&lowered, windows) => lowered,
                    // A global default this capture cannot resolve selects no
                    // trust: that only ever narrows the host keys the child is
                    // offered, never widens them.
                    _ if global => continue,
                    Err(error) => return Err(error),
                    Ok(_) => {
                        return Err(refused_path(
                            input,
                            kind,
                            &value,
                            "it is not an absolute path, so the file it names depends on where \
                             the client happens to be invoked",
                            "write an absolute UserKnownHostsFile path in your SSH configuration \
                             (the selected home's `~/.ssh/known_hosts` is already captured), or \
                             set it to `none`",
                        ));
                    }
                };
                // `/dev/null` is the standard spelling of "no trust from this
                // list": it contributes nothing and is never opened. Every
                // other non-regular file is still refused by the witness.
                if lowered == Path::new("/dev/null") {
                    continue;
                }
                files.push((kind, lowered));
            }
        }
    }
    Ok(files)
}

fn scalar<'a>(fields: &'a BTreeMap<String, Vec<String>>, key: &str) -> Option<&'a str> {
    fields.get(key).and_then(|v| v.first()).map(String::as_str)
}
fn fields(output: &str) -> Result<BTreeMap<String, Vec<String>>, GitError> {
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in output.lines() {
        let (key, value) = line
            .split_once(' ')
            .ok_or_else(|| unsupported("unrecognized SSH evaluator record"))?;
        fields.entry(key.into()).or_default().push(value.into());
    }
    Ok(fields)
}
pub(super) struct ResolvedEndpoint {
    host: String,
    user: String,
    port: u16,
}
impl ResolvedEndpoint {
    pub(super) fn admit(&self, binding: &crate::HttpAdmission) -> Result<(), GitError> {
        let url = Url::parse(&format!("ssh://{}:{}/", self.host, self.port))
            .map_err(|_| unsupported("invalid effective SSH authority"))?;
        binding.admit_host(&url)?;
        Ok(())
    }
}
pub(super) fn endpoint(output: &str) -> Result<ResolvedEndpoint, GitError> {
    let fields = fields(output)?;
    let host = scalar(&fields, "hostname")
        .ok_or_else(|| unsupported("SSH evaluator omitted HostName"))?
        .to_owned();
    let user = scalar(&fields, "user")
        .ok_or_else(|| unsupported("SSH evaluator omitted User"))?
        .to_owned();
    let port = scalar(&fields, "port")
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .ok_or_else(|| unsupported("SSH evaluator returned invalid Port"))?;
    if !super::super::ssh::direct_host(&host) || !super::super::ssh::user_name(&user) {
        return Err(unsupported("captured SSH effective endpoint is not direct"));
    }
    Ok(ResolvedEndpoint { host, user, port })
}
/// The address-family, algorithm and RSA-size options the selected client
/// reported, replayed into the owned configuration under the keywords it
/// used.
fn client_options(
    fields: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<(String, String)>, GitError> {
    let family = scalar(fields, "addressfamily")
        .ok_or_else(|| unsupported("SSH evaluator omitted address family"))?;
    if !matches!(family, "any" | "inet" | "inet6") {
        return Err(unsupported("unrecognized selected SSH address family"));
    }
    let mut client_options: Vec<(String, String)> = vec![("addressfamily".into(), family.into())];
    // OpenSSH before 8.5 (the Windows 10 client) names the first one
    // `pubkeyacceptedkeytypes`; the owned configuration repeats each
    // keyword exactly as the selected client reported it.
    for names in [
        &["pubkeyacceptedalgorithms", "pubkeyacceptedkeytypes"][..],
        &["hostkeyalgorithms"],
        &["ciphers"],
        &["macs"],
        &["kexalgorithms"],
    ] {
        let (key, value) = names
            .iter()
            .find_map(|key| scalar(fields, key).map(|value| (*key, value)))
            .ok_or_else(|| unsupported("SSH evaluator omitted algorithm policy"))?;
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"@.,_-".contains(&b))
        {
            return Err(unsupported("unqualified SSH algorithm policy output"));
        }
        client_options.push((key.into(), value.into()));
    }
    // `RequiredRSASize` exists since OpenSSH 9.1; an older client reports
    // none and enforces its own compiled minimum, which is then kept.
    if let Some(reported) = scalar(fields, "requiredrsasize") {
        let rsa = reported
            .parse::<u32>()
            .ok()
            .filter(|v| *v >= 1024)
            .ok_or_else(|| unsupported("unqualified selected SSH RSA size policy"))?;
        client_options.push(("requiredrsasize".into(), rsa.to_string()));
    }
    Ok(client_options)
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW"
)]
pub(super) fn profile(
    owner: Arc<SshConfiguration>,
    authored: Authority,
    output: &str,
) -> Result<SshProfile, GitError> {
    let fields = fields(output)?;
    for key in [
        "proxycommand",
        "proxyjump",
        "knownhostscommand",
        "localcommand",
        "remotecommand",
        "pkcs11provider",
        "securitykeyprovider",
        "certificatefile",
        "revokedhostkeys",
        "setenv",
    ] {
        if fields.get(key).is_some_and(|v| {
            v.iter()
                .any(|s| !matches!(s.as_str(), "none" | "internal" | ""))
        }) {
            return Err(unsupported(
                "selected SSH command, proxy, provider or custom service setting is unsupported",
            ));
        }
    }
    if scalar(&fields, "pubkeyauthentication") != Some("true")
        && scalar(&fields, "pubkeyauthentication") != Some("yes")
    {
        return Err(unsupported(
            "selected SSH public-key authentication mode is not qualified",
        ));
    }
    if scalar(&fields, "preferredauthentications")
        .is_some_and(|s| !s.split(',').any(|part| part == "publickey"))
    {
        return Err(unsupported(
            "selected SSH authentication preference excludes publickey",
        ));
    }
    for key in ["bindaddress", "bindinterface"] {
        if scalar(&fields, key).is_some_and(|value| value != "none" && !value.is_empty()) {
            return Err(unsupported(
                "selected SSH source-address/interface binding is not qualified",
            ));
        }
    }
    let client_options = client_options(&fields)?;
    let ResolvedEndpoint { host, user, port } = endpoint(output)?;
    let endpoint = Endpoint {
        host: &host,
        original: &authored.host,
        user: &user,
        port,
    };
    let input = &owner.selected;
    let windows = cfg!(windows);
    let mut witness = Witness::default();
    let mut identities: Vec<String> = Vec::new();
    let mut any_identity = false;
    for value in fields.get("identityfile").into_iter().flatten() {
        if value == "none" {
            identities.push(value.clone());
            continue;
        }
        let file = path(value, input, Some(&endpoint), windows, "identityfile")?;
        if !platform::absolute_path(&file, windows) {
            return Err(refused_path(
                input,
                "identityfile",
                value,
                "a relative identity path requires an explicit invocation cwd, which a controlled \
                 child does not inherit",
                "write an absolute IdentityFile path in your SSH configuration, or remove the \
                 setting to use the client's `~/.ssh` defaults",
            ));
        }
        any_identity |= witness.identity(&file)?;
        let text = file
            .to_str()
            .ok_or_else(|| unsupported("SSH identity path is not UTF-8"))?;
        any_identity |= witness.identity(Path::new(&format!("{text}.pub")))?;
        if witness.identity(Path::new(&format!("{text}-cert.pub")))? {
            return Err(unsupported(
                "SSH certificate identities require separate qualification",
            ));
        }
        identities.push(text.into());
    }
    let raw_agent = scalar(&fields, "identityagent").unwrap_or("SSH_AUTH_SOCK");
    let agent: String = if raw_agent == "none" {
        "none".into()
    } else {
        let value = if raw_agent == "SSH_AUTH_SOCK" {
            input.environment.get("SSH_AUTH_SOCK").cloned().or_else(|| {
                input
                    .default_agent
                    .as_ref()
                    .filter(|endpoint| endpoint.exists())
                    .and_then(|endpoint| endpoint.to_str().map(str::to_owned))
            })
        } else if raw_agent.starts_with('$') && !raw_agent.starts_with("${") {
            input.environment.get(&raw_agent[1..]).cloned()
        } else {
            Some(raw_agent.into())
        };
        match value {
            None => "none".into(),
            Some(value) => {
                let p = path(&value, input, Some(&endpoint), windows, "identityagent")?;
                if !platform::absolute_path(&p, windows) && !platform::named_pipe(&p) {
                    return Err(refused_path(
                        input,
                        "identityagent",
                        &value,
                        "an agent endpoint must be an absolute socket path or a named pipe",
                        "set IdentityAgent to an absolute path, to a `\\\\.\\pipe\\` endpoint, or \
                         to `none`",
                    ));
                }
                p.to_str()
                    .ok_or_else(|| unsupported("SSH agent path is not UTF-8"))?
                    .into()
            }
        }
    };
    if !any_identity && agent == "none" {
        return Err(unsupported(
            "SSH identity files are unavailable and no agent is selected",
        ));
    }
    let mut trust: Vec<(String, Vec<u8>)> = Vec::new();
    for (kind, file) in trust_files(&fields, input, Some(&endpoint), windows)? {
        if let Some(bytes) = witness.content(&file)? {
            trust.push((kind.into(), bytes));
        }
    }
    if trust.iter().all(|(_, bytes)| bytes.is_empty()) {
        return Err(unsupported("selected SSH known-host trust is unavailable"));
    }
    let alias = scalar(&fields, "hostkeyalias")
        .filter(|v| *v != "none")
        .map(str::to_owned);
    if alias.as_ref().is_some_and(|s| {
        s.is_empty()
            || !s
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-:[]".contains(&b))
    }) {
        return Err(unsupported("unsupported SSH HostKeyAlias"));
    }
    let configured = Configured {
        owner: owner.clone(),
        authored,
        identities,
        agent,
        identities_only: scalar(&fields, "identitiesonly") == Some("yes"),
        client_options,
        trust,
        host_key_alias: alias,
        check_host_ip: scalar(&fields, "checkhostip") == Some("yes"),
        witness,
    };
    Ok(SshProfile::captured(host, user, port, configured))
}

#[cfg(test)]
#[path = "values_tests.rs"]
mod tests;
