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
}
impl AdmittedRemote {
    pub(super) fn address(&self) -> &str {
        match self {
            Self::Https { url, .. } => url.as_str(),
            Self::Ssh(remote) => &remote.address,
        }
    }
    pub(super) fn protocol(&self) -> NativeProtocol {
        match self {
            Self::Https { url, .. } if url.scheme() == "http" => NativeProtocol::Http,
            Self::Https { .. } => NativeProtocol::Https,
            Self::Ssh(_) => NativeProtocol::Ssh,
        }
    }
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
