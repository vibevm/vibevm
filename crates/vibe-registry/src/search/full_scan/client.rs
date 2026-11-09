//! One bound HTTP execution owner for all full-scan GET families.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW");
use super::*;
use std::time::Duration;
pub(super) struct ScanClient {
    client: crate::http_admission::ManagedHttpClient,
    admission: Option<std::sync::Arc<crate::HttpAdmission>>,
}
impl ScanClient {
    pub(super) fn get(&self, url: &str) -> reqwest::blocking::RequestBuilder {
        self.client.get(url)
    }
    pub(super) fn send(
        &self,
        request: reqwest::blocking::RequestBuilder,
        url: &str,
    ) -> Result<crate::http_admission::ManagedResponse, FullScanError> {
        crate::http_admission::get(&self.client, request, url, self.admission.as_deref())
            .map_err(|e| scan_http_error(url, e))
    }
    pub(super) fn permits_token(&self, url: &str) -> bool {
        self.admission.is_none() || reqwest::Url::parse(url).is_ok_and(|u| u.scheme() == "https")
    }
}
pub(super) fn scan_http_error(
    url: &str,
    error: crate::http_admission::HttpGetError,
) -> FullScanError {
    use crate::http_admission::HttpGetError;
    match error {
        HttpGetError::Refused(reason) => FullScanError::ContactRefused(reason),
        HttpGetError::Transport(error) => FullScanError::Http {
            url: vibe_core::progress::transfer::safe_remote(url),
            message: error.to_string(),
        },
        HttpGetError::Io(error) => FullScanError::Http {
            url: vibe_core::progress::transfer::safe_remote(url),
            message: error.to_string(),
        },
    }
}
pub(super) fn build_client(
    admission: Option<std::sync::Arc<crate::HttpAdmission>>,
) -> Result<ScanClient, FullScanError> {
    let client = crate::http_admission::ManagedHttpClient::captured(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        Some(concat!("vibe-cli/", env!("CARGO_PKG_VERSION"))),
        reqwest::header::HeaderMap::new(),
        admission.clone(),
        None,
    )
    .map_err(|error| FullScanError::Http {
        url: "<client-build>".into(),
        message: match error {
            crate::http_admission::ClientBuildError::Http(error) => error.without_url().to_string(),
            error => error.to_string(),
        },
    })?;
    Ok(ScanClient { client, admission })
}
