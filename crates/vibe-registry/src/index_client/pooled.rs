//! Invocation-local HTTP pools for a resolved index client.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http");

use std::time::Duration;

use super::{FETCH_TIMEOUT_SECS, IndexClient};

impl std::fmt::Debug for IndexClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IndexClient")
            .field("file_base", &self.file_base)
            .field("server_base", &self.server_base)
            .field("auth", &self.auth)
            .field("file_client_ready", &self.file_client.get().is_some())
            .field("server_client_ready", &self.server_client.get().is_some())
            .finish()
    }
}

impl IndexClient {
    pub(super) fn file_client(&self) -> Result<&crate::http_admission::ManagedHttpClient, &str> {
        self.file_client
            .get_or_init(|| {
                Self::build_bound_client(
                    Duration::from_secs(FETCH_TIMEOUT_SECS),
                    &self.auth,
                    &self.file_base,
                    self.admission.as_deref(),
                )
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(String::as_str)
    }

    pub(super) fn server_client(&self) -> Result<&crate::http_admission::ManagedHttpClient, &str> {
        self.server_client
            .get_or_init(|| {
                Self::build_bound_client(
                    Duration::from_secs(FETCH_TIMEOUT_SECS),
                    &self.auth,
                    &self.server_base,
                    self.admission.as_deref(),
                )
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(String::as_str)
    }
}
