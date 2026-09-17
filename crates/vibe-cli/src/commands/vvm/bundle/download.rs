//! Bounded release downloads with byte progress.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use specmark::spec;
#[cfg(test)]
use vibe_core::progress::Progress;
use vibe_core::progress::ProgressTask;

use super::super::store::VersionStore;

static REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);
pub(super) const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
pub(super) const TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub(super) fn download_path(store: &VersionStore, name: &str) -> PathBuf {
    store.data_dir().join(format!(
        ".download-{}-{}-{name}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

pub(super) fn cache_busted(url: &str) -> String {
    let nonce = REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{url}?vvm_nonce={}-{}-{nonce}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

pub(super) struct DownloadCleanup(pub(super) PathBuf);

impl Drop for DownloadCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) trait Downloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        maximum_bytes: u64,
        expected_bytes: Option<u64>,
        progress: &ProgressTask,
    ) -> Result<()>;
}

pub(super) struct HttpDownloader;

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-060#SELF-STAGES")]
impl Downloader for HttpDownloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        maximum_bytes: u64,
        expected_bytes: Option<u64>,
        progress: &ProgressTask,
    ) -> Result<()> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating download directory `{}`", parent.display()))?;
        }
        let client = reqwest::blocking::Client::builder()
            .user_agent(format!("vibevm/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .context("building anonymous GitHub release client")?;
        let mut response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/octet-stream")
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .header(reqwest::header::PRAGMA, "no-cache")
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .with_context(|| format!("downloading `{url}`"))?;
        let total = expected_bytes.or_else(|| response.content_length());
        progress.set_progress(0, total, "bytes");
        write_download_observed(destination, maximum_bytes, &mut response, progress, total)
            .with_context(|| format!("writing download `{}`", destination.display()))?;
        Ok(())
    }
}

pub(super) fn safe_url(raw: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return "remote release endpoint".to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
pub(super) fn write_download(
    destination: &Path,
    maximum_bytes: u64,
    reader: &mut impl Read,
) -> Result<()> {
    let task = Progress::default().task("download");
    write_download_observed(destination, maximum_bytes, reader, &task, None)
}

fn write_download_observed(
    destination: &Path,
    maximum_bytes: u64,
    reader: &mut impl Read,
    progress: &ProgressTask,
    total: Option<u64>,
) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| format!("creating `{}`", destination.display()))?;
    let result = copy_download_observed(reader, &mut file, maximum_bytes, |copied| {
        progress.set_progress(copied, total, "bytes");
    });
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result.map(|_| ())
}

pub(super) fn copy_download(
    reader: &mut impl Read,
    writer: &mut impl io::Write,
    maximum: u64,
) -> Result<u64> {
    copy_download_observed(reader, writer, maximum, |_| {})
}

fn copy_download_observed(
    reader: &mut impl Read,
    writer: &mut impl io::Write,
    maximum: u64,
    mut observed: impl FnMut(u64),
) -> Result<u64> {
    let mut reader = reader.take(maximum.saturating_add(1));
    let mut buffer = [0_u8; 64 * 1024];
    let mut copied = 0_u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        writer.write_all(&buffer[..read])?;
        copied += read as u64;
        observed(copied);
    }
    if copied > maximum {
        bail!("download exceeds its {maximum}-byte limit");
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver};

    use super::*;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<ProgressEvent>>);

    impl ProgressObserver for Recorder {
        fn observe(&self, event: ProgressEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[test]
    fn bounded_download_reports_real_bytes_under_the_callers_known_total() {
        let recorder = Arc::new(Recorder::default());
        let progress = Progress::new(recorder.clone());
        let task = progress.task("bundle");
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("bundle.zip");
        let mut source = std::io::Cursor::new(vec![7_u8; 150_000]);

        write_download_observed(&destination, 150_000, &mut source, &task, Some(150_000)).unwrap();
        task.finish();

        let events = recorder.0.lock().unwrap();
        assert!(matches!(events[0].kind, ProgressEventKind::Started { .. }));
        assert!(events.iter().any(|event| matches!(
            event.kind,
            ProgressEventKind::Progress {
                completed: 150_000,
                total: Some(150_000),
                ref unit,
            } if unit == "bytes"
        )));
        assert!(matches!(
            events.last().unwrap().kind,
            ProgressEventKind::Finished
        ));
        assert_eq!(std::fs::metadata(destination).unwrap().len(), 150_000);
    }

    #[test]
    fn verbose_url_detail_cannot_reveal_userinfo_query_or_fragment() {
        assert_eq!(
            safe_url("https://name:secret@example.test/file?token=secret#fragment"),
            "https://example.test/file"
        );
    }
}
