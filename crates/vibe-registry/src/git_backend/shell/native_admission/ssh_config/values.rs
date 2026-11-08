//! Lower selected OpenSSH values, preserving ordered identities and exact routing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
pub(super) struct Endpoint<'a> {
    pub host: &'a str,
    pub original: &'a str,
    pub user: &'a str,
    pub port: u16,
}
pub(super) fn path(
    value: &str,
    input: &SshConfigSelection,
    endpoint: Option<&Endpoint<'_>>,
) -> Result<PathBuf, GitError> {
    expand_path(value, input, endpoint, false).map(PathBuf::from)
}
pub(super) fn deferred_path(value: &str, input: &SshConfigSelection) -> Result<String, GitError> {
    expand_path(value, input, None, true)
}
fn expand_path(
    value: &str,
    input: &SshConfigSelection,
    endpoint: Option<&Endpoint<'_>>,
    deferred: bool,
) -> Result<String, GitError> {
    let mut expanded = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.next() != Some('{') {
                return Err(unsupported("unsupported SSH path environment syntax"));
            }
            let mut name = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(c) => name.push(c),
                    None => return Err(unsupported("unterminated SSH path variable")),
                }
            }
            if name == "HOME" {
                expanded.push_str(
                    input
                        .home
                        .to_str()
                        .ok_or_else(|| unsupported("SSH home is not UTF-8"))?,
                );
            } else {
                expanded.push_str(
                    input
                        .environment
                        .get(&name)
                        .ok_or_else(|| unsupported("SSH path expansion variable is unavailable"))?,
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
                Some('d') => expanded.push_str(
                    input
                        .home
                        .to_str()
                        .ok_or_else(|| unsupported("SSH home is not UTF-8"))?,
                ),
                Some(token) => {
                    if deferred && matches!(token, 'h' | 'n' | 'r' | 'p') {
                        expanded.push('%');
                        expanded.push(token);
                        continue;
                    }
                    let endpoint = endpoint.ok_or_else(|| {
                        unsupported("context-dependent SSH Include token is not captured")
                    })?;
                    match token {
                        'h' => expanded.push_str(endpoint.host),
                        'n' => expanded.push_str(endpoint.original),
                        'r' => expanded.push_str(endpoint.user),
                        'p' => expanded.push_str(&endpoint.port.to_string()),
                        _ => return Err(unsupported("unsupported SSH path token")),
                    }
                }
                None => return Err(unsupported("incomplete SSH path token")),
            }
        } else {
            expanded.push(c);
        }
    }
    if expanded == "~" {
        return input
            .home
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| unsupported("SSH home is not UTF-8"));
    }
    if let Some(tail) = expanded.strip_prefix("~/") {
        return input
            .home
            .join(tail)
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| unsupported("SSH path is not UTF-8"));
    }
    if expanded.starts_with('~') {
        return Err(unsupported("other-user SSH home expansion is not captured"));
    }
    Ok(expanded)
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
    let family = scalar(&fields, "addressfamily")
        .ok_or_else(|| unsupported("SSH evaluator omitted address family"))?;
    if !matches!(family, "any" | "inet" | "inet6") {
        return Err(unsupported("unrecognized selected SSH address family"));
    }
    let mut client_options = vec![("addressfamily".into(), family.into())];
    for key in [
        "pubkeyacceptedalgorithms",
        "hostkeyalgorithms",
        "ciphers",
        "macs",
        "kexalgorithms",
    ] {
        let value = scalar(&fields, key)
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
    let rsa = scalar(&fields, "requiredrsasize")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| *v >= 1024)
        .ok_or_else(|| unsupported("unqualified selected SSH RSA size policy"))?;
    client_options.push(("requiredrsasize".into(), rsa.to_string()));
    let ResolvedEndpoint { host, user, port } = endpoint(output)?;
    let endpoint = Endpoint {
        host: &host,
        original: &authored.host,
        user: &user,
        port,
    };
    let input = &owner.selected;
    let mut witness = Witness::default();
    let mut identities = Vec::new();
    let mut any_identity = false;
    for value in fields.get("identityfile").into_iter().flatten() {
        if value == "none" {
            identities.push(value.clone());
            continue;
        }
        let file = path(value, input, Some(&endpoint))?;
        if !file.is_absolute() {
            return Err(unsupported(
                "relative SSH identity paths require an explicit invocation cwd",
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
    let agent = if raw_agent == "none" {
        "none".into()
    } else {
        let value = if raw_agent == "SSH_AUTH_SOCK" {
            input.environment.get("SSH_AUTH_SOCK").cloned()
        } else if raw_agent.starts_with('$') && !raw_agent.starts_with("${") {
            input.environment.get(&raw_agent[1..]).cloned()
        } else {
            Some(raw_agent.into())
        };
        match value {
            None => "none".into(),
            Some(value) => {
                let p = path(&value, input, Some(&endpoint))?;
                if !p.is_absolute() {
                    return Err(unsupported("captured SSH agent path must be absolute"));
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
    let mut trust = Vec::new();
    for kind in ["userknownhostsfile", "globalknownhostsfile"] {
        for values in fields.get(kind).into_iter().flatten() {
            for value in inputs::words(values)? {
                if value == "none" {
                    continue;
                }
                let p = path(&value, input, Some(&endpoint))?;
                if !p.is_absolute() {
                    return Err(unsupported("relative SSH trust path is unsupported"));
                }
                // `/dev/null` is the standard spelling of "no trust from this
                // list": it contributes nothing and is never opened. Every
                // other non-regular file is still refused by the witness.
                if p == Path::new("/dev/null") {
                    continue;
                }
                if let Some(bytes) = witness.content(&p)? {
                    trust.push((kind.into(), bytes));
                }
            }
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
    Ok(SshProfile::captured(
        input.executable.clone(),
        host,
        user,
        port,
        configured,
    ))
}
