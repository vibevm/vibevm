//! Admitted initial Git HTTP relocation; native follow-up requests never follow.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;
use std::io::{BufRead, Read};
const MAX_SERVICE_PREFIX: u64 = 64 * 1024;

/// HTTP discovery trust selected independently of Git's system TLS backend.
/// Native PEM trust is reused automatically when no override is selected.
pub enum HttpDiscoveryTrust {
    Pem(Vec<u8>),
    /// Explicit WebPKI bundled roots, not a claim of Git-System equivalence.
    BundledWebPkiRoots,
}
impl std::fmt::Debug for HttpDiscoveryTrust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Pem(_) => "HttpDiscoveryTrust::Pem(..)",
            Self::BundledWebPkiRoots => "HttpDiscoveryTrust::BundledWebPkiRoots",
        })
    }
}
impl NativeGitAdmission {
    /// Enable initial advertisement discovery with explicit HTTP trust. Direct
    /// System Git remains useful without this selection; redirects then refuse.
    pub fn with_http_discovery_trust(mut self, trust: HttpDiscoveryTrust) -> Self {
        self.discovery = Some(trust);
        self
    }

    pub(super) fn relocate(&self, remote: AdmittedRemote) -> Result<AdmittedRemote, GitError> {
        let AdmittedRemote::Https { url, authorization } = remote else {
            return Ok(remote);
        };
        if url.query().is_some() || url.fragment().is_some() {
            return Err(unsupported(
                "native HTTPS base query/fragment is not a relocatable repository identity",
            ));
        }
        let pem = match (&self.discovery, &self.trust) {
            (Some(HttpDiscoveryTrust::Pem(bytes)), _) | (None, TlsTrust::Pem(bytes)) => {
                Some(bytes.as_slice())
            }
            (Some(HttpDiscoveryTrust::BundledWebPkiRoots), _) => None,
            (None, TlsTrust::System) => return Ok(AdmittedRemote::Https { url, authorization }),
        };
        let mut builder =
            crate::http_admission::bound_builder(reqwest::blocking::Client::builder())
                .timeout(std::time::Duration::from_secs(30));
        if let Some(pem) = pem {
            builder = builder.tls_built_in_root_certs(false);
            for certificate in reqwest::Certificate::from_pem_bundle(pem)
                .map_err(|_| unsupported("invalid captured HTTP discovery trust"))?
            {
                builder = builder.add_root_certificate(certificate);
            }
        }
        let client = builder
            .build()
            .map_err(|_| unsupported("HTTP discovery client could not be constructed"))?;
        let mut advertisement = url.clone();
        advertisement.set_path(&format!("{}/info/refs", url.path().trim_end_matches('/')));
        advertisement.set_query(Some("service=git-upload-pack"));
        let mut request = client
            .get(advertisement.clone())
            .header("Git-Protocol", "version=2");
        if let Some(auth) = &authorization {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }
        let observed = crate::http_admission::get_observed(
            &client,
            request,
            advertisement.as_str(),
            &self.binding,
        )
        .map_err(|error| match error {
            crate::http_admission::HttpGetError::Refused(refusal) => {
                GitError::ContactRefused(refusal)
            }
            crate::http_admission::HttpGetError::Transport(_) => GitError::NetworkUnreachable {
                url: url.to_string(),
            },
        })?;
        match observed.response.status().as_u16() {
            401 | 403 => {
                return Err(GitError::AuthFailed {
                    url: url.to_string(),
                });
            }
            404 => {
                return Err(GitError::RepoNotFound {
                    url: url.to_string(),
                });
            }
            200 => {}
            _ => {
                return Err(GitError::NetworkUnreachable {
                    url: url.to_string(),
                });
            }
        }
        if observed.redirects_followed == 0 {
            return Ok(AdmittedRemote::Https { url, authorization });
        }
        let final_url = observed.response.url().clone();
        let base_path = final_url
            .path()
            .strip_suffix("/info/refs")
            .ok_or_else(|| unsupported("initial Git redirect has no service-preserving base"))?
            .to_owned();
        if final_url.scheme() != "https"
            || final_url.query() != Some("service=git-upload-pack")
            || final_url.fragment().is_some()
        {
            return Err(unsupported(
                "initial Git redirect changed the service request",
            ));
        }
        let smart = observed
            .response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                value.split(';').next().is_some_and(|kind| {
                    kind.trim()
                        .eq_ignore_ascii_case("application/x-git-upload-pack-advertisement")
                })
            });
        // Identify the service, not buffer its potentially enormous ref set.
        // Native Git subsequently validates the full advertisement and objects.
        let mut reader = std::io::BufReader::new(observed.response.take(MAX_SERVICE_PREFIX + 1));
        let mut body = Vec::new();
        reader
            .read_until(b'\n', &mut body)
            .map_err(|_| GitError::NetworkUnreachable {
                url: url.to_string(),
            })?;
        if body.len() as u64 > MAX_SERVICE_PREFIX {
            return Err(unsupported(
                "Git service advertisement metadata prefix exceeds its bound",
            ));
        }
        if smart && body == b"001e# service=git-upload-pack\n" {
            let mut flush = [0; 4];
            reader
                .read_exact(&mut flush)
                .map_err(|_| unsupported("Git service advertisement is incomplete"))?;
            body.extend_from_slice(&flush);
        }
        if !valid_advertisement(&body, smart) {
            return Err(unsupported(
                "initial redirect did not return a Git upload-pack advertisement",
            ));
        }
        let mut target = final_url;
        target.set_path(&base_path);
        target.set_query(None);
        self.binding.admit_address(target.as_str())?;
        Ok(AdmittedRemote::Https {
            url: target,
            authorization: if observed.crossed_origin {
                None
            } else {
                authorization
            },
        })
    }
}

fn valid_advertisement(bytes: &[u8], smart: bool) -> bool {
    if smart {
        return bytes.starts_with(b"001e# service=git-upload-pack\n0000")
            || bytes.starts_with(b"000eversion 2\n");
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    !text.is_empty()
        && text.lines().all(|line| {
            line.split_once('\t').is_some_and(|(oid, name)| {
                full_oid(oid) && name.starts_with("refs/") && !name.chars().any(char::is_control)
            })
        })
}
