//! Endpoint admission for a native request: the selected address is checked
//! against the operation binding and captured as one [`AdmittedRemote`].
//! Split from `native_admission.rs` along the address-capture seam for the
//! file-length budget; same items, same behaviour.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::{GitError, NativeGitAdmission, NativeProtocol, ssh, unsupported};
use reqwest::Url;
use std::sync::Arc;

pub(super) enum AdmittedRemote {
    /// A direct `http://` or `https://` endpoint; the URL scheme selects TLS.
    Https {
        url: Url,
        authorization: Option<String>,
    },
    Ssh(ssh::Remote),
    /// A repository on this machine: a `file://` url or an absolute path.
    /// No host is contacted, so there is nothing for source policy to admit
    /// or exclude; Git may use only its `file` transport for it.
    Local {
        address: String,
    },
}
impl AdmittedRemote {
    pub(super) fn address(&self) -> &str {
        match self {
            Self::Https { url, .. } => url.as_str(),
            Self::Ssh(remote) => &remote.address,
            Self::Local { address } => address,
        }
    }
    pub(super) fn protocol(&self) -> NativeProtocol {
        match self {
            Self::Https { url, .. } if url.scheme() == "http" => NativeProtocol::Http,
            Self::Https { .. } => NativeProtocol::Https,
            Self::Ssh(_) => NativeProtocol::Ssh,
            Self::Local { .. } => NativeProtocol::File,
        }
    }
}

/// A repository Git reads through the filesystem rather than a network
/// transport (PROP-002 `#offline-local`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalRepository {
    /// On this machine's disk: no host is contacted, so source policy has
    /// nothing to admit or exclude.
    Disk(String),
    /// A Windows share (`\\server\share\repo`, `//server/share/repo`,
    /// `file://server/share/repo`): reading it contacts `host`, which source
    /// policy checks before Git runs. `address` is the spelling Git is given.
    Share { host: String, address: String },
}
impl LocalRepository {
    /// The spelling Git is given.
    pub fn address(&self) -> &str {
        match self {
            Self::Disk(address) | Self::Share { address, .. } => address,
        }
    }
    /// Refuse a share whose host source policy excludes; a disk path
    /// contacts nothing.
    pub fn admit(&self, binding: &crate::HttpAdmission) -> Result<(), GitError> {
        let Self::Share { host, .. } = self else {
            return Ok(());
        };
        // Not `file://`: a URL parser drops a `localhost` file host.
        let url = Url::parse(&format!("smb://{host}/"))
            .map_err(|_| unsupported("a share's server name is not a host name"))?;
        binding.admit_host(&url)?;
        Ok(())
    }
}

/// The local-repository spelling of `address`, if it is one, on this
/// platform. `Some(Err)` is a `file:` spelling vibe cannot pin to one
/// directory. A relative path is not taken as local: it would be read
/// against the operation's private working directory, and its `host:path`
/// look-alike is the scp form of SSH.
pub fn local_repository(address: &str) -> Option<Result<LocalRepository, GitError>> {
    local_repository_for(address, cfg!(windows))
}

/// [`local_repository`] for a platform. Everywhere: a `file:` url without a
/// remote host and an absolute path are [`LocalRepository::Disk`]. On
/// Windows also a drive path (`C:\x`, `C:/x`) and a share — a UNC path, its
/// `\\?\UNC\` form, `file://server/...` or `file:////server/...` — which is
/// [`LocalRepository::Share`]; elsewhere a `file:` url naming another
/// machine is refused, as Git cannot reach it.
pub fn local_repository_for(
    address: &str,
    windows: bool,
) -> Option<Result<LocalRepository, GitError>> {
    let raw = address.strip_prefix("git+").unwrap_or(address);
    if raw
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("file:"))
    {
        // `file:////server/share` is a share on Windows; URL parsers differ
        // on its path, so it is read as text.
        if windows && let Some(unc) = raw.get(5..).filter(|rest| rest.starts_with("////")) {
            return share(&unc[2..]).map(|found| {
                found.map(|(host, rest)| LocalRepository::Share {
                    address: format!("//{host}/{rest}"),
                    host,
                })
            });
        }
        return Some(match Url::parse(raw) {
            Ok(url) if url.query().is_some() || url.fragment().is_some() => Err(unsupported(
                "a local repository url must be `file:///<absolute path>`",
            )),
            Ok(url) if url.host_str().is_none_or(|host| host == "localhost") => {
                Ok(LocalRepository::Disk(raw.to_owned()))
            }
            Ok(url) if windows => match url.host_str() {
                Some(host) => Ok(LocalRepository::Share {
                    address: format!("//{host}{}", url.path()),
                    host: host.to_owned(),
                }),
                None => Err(unsupported("a share url names no server")),
            },
            _ => Err(unsupported(
                "a local repository url must be `file:///<absolute path>`",
            )),
        });
    }
    if raw.contains("://") {
        return None;
    }
    if windows {
        if let Some(found) = share(raw) {
            return Some(found.map(|(host, _)| LocalRepository::Share {
                host,
                address: raw.to_owned(),
            }));
        }
        let bytes = raw.as_bytes();
        let drive = bytes.len() > 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/');
        let verbatim_drive = raw.strip_prefix(r"\\?\").is_some_and(|rest| {
            let rest = rest.as_bytes();
            rest.len() > 2 && rest[0].is_ascii_alphabetic() && rest[1] == b':'
        });
        return (drive || verbatim_drive).then(|| Ok(LocalRepository::Disk(raw.to_owned())));
    }
    std::path::Path::new(raw)
        .is_absolute()
        .then(|| Ok(LocalRepository::Disk(raw.to_owned())))
}

/// The server and the rest of a Windows share spelling (`\\server\share\x`,
/// `//server/share/x`, `\\?\UNC\server\share\x`), if `path` is one. A
/// device path (`\\.\`, `\\?\` other than `UNC`) is refused: it is not a
/// repository directory.
fn share(path: &str) -> Option<Result<(String, String), GitError>> {
    let rest = if let Some(rest) = path
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| path.strip_prefix("//?/UNC/"))
    {
        rest
    } else if path.starts_with(r"\\?\") || path.starts_with(r"\\.\") || path.starts_with("//./") {
        return (!path.starts_with(r"\\?\") || !path.as_bytes().get(5).is_some_and(|b| *b == b':'))
            .then(|| Err(unsupported("a device path is not a repository directory")));
    } else {
        path.strip_prefix(r"\\")
            .or_else(|| path.strip_prefix("//"))?
    };
    let (host, tail) = rest.split_once(['\\', '/'])?;
    if host.is_empty() || tail.is_empty() {
        return Some(Err(unsupported("a share path names no server and share")));
    }
    Some(Ok((host.to_owned(), tail.to_owned())))
}
impl NativeGitAdmission {
    // Metadata/address capture never grants use of an SSH credential.
    pub(in crate::git_backend::shell) fn preflight_selected_address(
        &self,
        address: &str,
    ) -> Result<(), GitError> {
        let raw = address.strip_prefix("git+").unwrap_or(address);
        if raw.contains("://") {
            let url = Url::parse(raw).map_err(|_| unsupported("invalid selected endpoint"))?;
            if matches!(url.scheme(), "http" | "https" | "ssh") {
                self.binding.admit_host(&url)?;
            }
        } else if crate::registry_auth::ssh_source(raw).map_err(unsupported)? {
            let authority = if let Some(end) = raw.find("]:") {
                &raw[..=end]
            } else {
                raw.split_once(':').map(|(host, _)| host).unwrap_or(raw)
            };
            let host = authority
                .rsplit_once('@')
                .map_or(authority, |(_, host)| host);
            let url = Url::parse(&format!("ssh://{host}/"))
                .map_err(|_| unsupported("invalid selected SSH endpoint"))?;
            self.binding.admit_host(&url)?;
        }
        if crate::registry_auth::ssh_source(raw).map_err(unsupported)?
            && let Some(configuration) = self.selected_ssh_configuration()?
        {
            configuration.preflight_address(raw, &self.binding)?;
        }
        Ok(())
    }

    // Metadata/address capture never grants use of an SSH credential.
    pub(super) fn capture_address(
        &self,
        address: &str,
        reject_https_credentials: bool,
    ) -> Result<AdmittedRemote, GitError> {
        if address.chars().any(char::is_control) {
            return Err(unsupported("control character in endpoint"));
        }
        if let Some(local) = local_repository(address) {
            let local = local?;
            local.admit(&self.binding)?;
            return Ok(AdmittedRemote::Local {
                address: local.address().to_owned(),
            });
        }
        if address.starts_with("ssh://") || (!address.contains("://") && address.contains(':')) {
            let mut selected = None;
            for (index, profile) in self.ssh.iter().enumerate() {
                if let Some(remote) = profile.capture_candidate(address, &self.binding, index)?
                    && selected.replace(remote).is_some()
                {
                    return Err(unsupported("ambiguous exact SSH grants for this address"));
                }
            }
            if let Some(configuration) = self.selected_ssh_configuration()? {
                let profile = Arc::new(configuration.profile(address, &self.binding)?);
                let remote = ssh::captured_remote(profile, address, &self.binding)?;
                if selected.replace(remote).is_some() {
                    return Err(unsupported("explicit and captured SSH grants overlap"));
                }
            }
            return selected
                .map(AdmittedRemote::Ssh)
                .ok_or_else(|| unsupported("SSH endpoint requires one exact selected profile"));
        }
        let mut url =
            Url::parse(address).map_err(|_| unsupported("direct HTTP(S) endpoint required"))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(unsupported(
                "only direct HTTP(S) without query or fragment is supported",
            ));
        }
        let authorization = if !url.username().is_empty() || url.password().is_some() {
            // A credential never crosses plaintext: plain HTTP is credential-free.
            if reject_https_credentials
                || url.scheme() != "https"
                || url.username() != "x-access-token"
            {
                return Err(unsupported("unsupported credential route"));
            }
            let password = url
                .password()
                .ok_or_else(|| unsupported("token credential is incomplete"))?;
            // Existing token-env injection uses literal tokens. Encoded/ambiguous
            // userinfo is not silently reinterpreted as a different password.
            if password.contains('%') {
                return Err(unsupported("encoded token credential is unsupported"));
            }
            let client = reqwest::blocking::Client::new();
            let request = client
                .get("https://127.0.0.1/")
                .basic_auth("x-access-token", Some(password))
                .build()
                .map_err(|_| unsupported("invalid token credential"))?;
            Some(
                request.headers()[reqwest::header::AUTHORIZATION]
                    .to_str()
                    .map_err(|_| unsupported("invalid token header"))?
                    .to_owned(),
            )
        } else {
            None
        };
        url.set_username("")
            .map_err(|_| unsupported("invalid endpoint authority"))?;
        url.set_password(None)
            .map_err(|_| unsupported("invalid endpoint authority"))?;
        self.binding.admit_address(url.as_str())?;
        Ok(AdmittedRemote::Https { url, authorization })
    }
    pub(super) fn capture_for_request(
        &self,
        address: &str,
        anonymous: bool,
        registry_auth: Option<vibe_core::manifest::AuthKind>,
    ) -> Result<AdmittedRemote, GitError> {
        use vibe_core::manifest::AuthKind;
        // Address/host checks remain shared. Registry auth narrows credential
        // use; it does not forbid credential-free HTTPS metadata reads.
        let reject_https_credentials =
            anonymous || registry_auth.is_some_and(|auth| !matches!(auth, AuthKind::TokenEnv));
        // Ordinary factory selection does not supersede active Git transport
        // commands or proxy/auth/TLS configuration. Explicit-direct profiles
        // retain their existing, separately declared override authority.
        if self.ssh_provider.is_some() && self.selection.is_some() {
            self.preflight_selected_address(address)?;
            let selected_ssh = address.starts_with("ssh://")
                || (!address.contains("://") && address.contains(':'));
            self.capture_context(None)?
                .validate_warm_transport(selected_ssh, false)?;
        }
        let remote = self.capture_address(address, reject_https_credentials)?;
        if matches!(remote, AdmittedRemote::Ssh(_))
            && (anonymous || registry_auth.is_some_and(|auth| !matches!(auth, AuthKind::Ssh)))
        {
            return Err(unsupported(
                "registry credential binding does not permit the selected SSH key or agent",
            ));
        }
        Ok(remote)
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalRepository, local_repository_for};

    fn disk(address: &str, windows: bool) -> String {
        match local_repository_for(address, windows).unwrap().unwrap() {
            LocalRepository::Disk(address) => address,
            other => panic!("{address}: {other:?}"),
        }
    }
    fn share(address: &str) -> (String, String) {
        match local_repository_for(address, true).unwrap().unwrap() {
            LocalRepository::Share { host, address } => (host, address),
            other => panic!("{address}: {other:?}"),
        }
    }

    #[test]
    fn local_spellings_are_file_urls_and_absolute_paths_only() {
        for windows in [false, true] {
            for local in [
                "file:///srv/vibe.git",
                "FILE:///srv/vibe.git",
                "file://localhost/srv/v.git",
            ] {
                assert_eq!(disk(local, windows), local);
            }
            assert!(
                local_repository_for("file:///srv/vibe.git?x=1", windows)
                    .unwrap()
                    .is_err()
            );
            // Network spellings, scp form and relative paths are not local.
            for remote in [
                "https://host/r.git",
                "ssh://host/r.git",
                "git@host:r.git",
                "corp-git:r.git",
                "relative/r.git",
            ] {
                assert!(local_repository_for(remote, windows).is_none(), "{remote}");
            }
        }
        #[cfg(unix)]
        assert_eq!(disk("/srv/vibe.git", false), "/srv/vibe.git");
        // Off Windows a `file:` url naming another machine is unreachable.
        assert!(
            local_repository_for("file://fileserver/share/vibe.git", false)
                .unwrap()
                .is_err()
        );
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn windows_drive_paths_are_disk_and_shares_name_their_server() {
        for drive in [
            r"C:\Work\corp.git",
            "C:/Work/corp.git",
            r"\\?\C:\Work\corp.git",
        ] {
            assert_eq!(disk(drive, true), drive);
        }
        assert_eq!(
            share(r"\\fileserver\vibe\corp.git"),
            ("fileserver".into(), r"\\fileserver\vibe\corp.git".into())
        );
        assert_eq!(
            share("//fileserver/vibe/corp.git"),
            ("fileserver".into(), "//fileserver/vibe/corp.git".into())
        );
        assert_eq!(share(r"\\?\UNC\fileserver\vibe\corp.git").0, "fileserver");
        // Both file-url spellings of a share reach Git as a UNC path.
        assert_eq!(
            share("file://fileserver/vibe/corp.git"),
            ("fileserver".into(), "//fileserver/vibe/corp.git".into())
        );
        assert_eq!(
            share("file:////fileserver/vibe/corp.git"),
            ("fileserver".into(), "//fileserver/vibe/corp.git".into())
        );
        // A device path or a server without a share is not a repository.
        for refused in [r"\\.\pipe\x", r"\\fileserver", r"\\fileserver\"] {
            assert!(
                local_repository_for(refused, true).is_none_or(|found| found.is_err()),
                "{refused}"
            );
        }
        // A drive-relative or root-relative path is not local.
        for relative in ["C:corp.git", r"\Work\corp.git"] {
            assert!(local_repository_for(relative, true).is_none(), "{relative}");
        }
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn an_excluded_share_server_is_refused_before_contact() {
        let binding = crate::HttpAdmission::new(["fileserver.corp.example"]).unwrap();
        let excluded = share(r"\\fileserver.corp.example\vibe\corp.git");
        let admitted = local_repository_for(r"\\other\vibe\corp.git", true)
            .unwrap()
            .unwrap();
        assert!(matches!(
            (LocalRepository::Share {
                host: excluded.0,
                address: excluded.1
            })
            .admit(&binding),
            Err(super::GitError::ContactRefused(
                crate::ContactRefusal::ExcludedHost { .. }
            ))
        ));
        admitted.admit(&binding).unwrap();
        LocalRepository::Disk("C:/Work/corp.git".into())
            .admit(&binding)
            .unwrap();
        // `localhost` is a host like any other here (a URL parser would drop
        // it from a `file:` url).
        let local = local_repository_for(r"\\localhost\C$\Work\corp.git", true)
            .unwrap()
            .unwrap();
        local.admit(&binding).unwrap();
        let localhost_excluded = crate::HttpAdmission::new(["localhost"]).unwrap();
        assert!(local.admit(&localhost_excluded).is_err());
    }
}
