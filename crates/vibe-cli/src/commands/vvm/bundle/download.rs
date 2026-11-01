//! Bounded release downloads with byte progress, through the transfer
//! wrapper (`PROP-068#coverage`): a `release-bundle` transfer whose bytes
//! land in the vibe home.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#provenance");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#coverage");

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
use vibe_core::progress::transfer::{self, Destination, Purpose, TransferReader, TransferRecord};

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
        let record =
            TransferRecord::http(Purpose::ReleaseBundle, url, Destination::Home).total(total);
        let reader = transfer::begin(record, Some(progress)).reader(&mut response);
        write_download_observed(destination, maximum_bytes, reader)
            .with_context(|| format!("writing download `{}`", destination.display()))?;
        Ok(())
    }
}

pub(super) use transfer::safe_url;

#[cfg(test)]
pub(super) fn write_download(
    destination: &Path,
    maximum_bytes: u64,
    reader: &mut impl Read,
) -> Result<()> {
    let task = Progress::default().task("download");
    let record = TransferRecord::http(Purpose::ReleaseBundle, "file:///test", Destination::Home);
    write_download_observed(
        destination,
        maximum_bytes,
        transfer::begin(record, Some(&task)).reader(reader),
    )
}

/// Write a counted download to a new file and end its transfer with the
/// outcome; a failed write leaves no file behind.
fn write_download_observed<R: Read>(
    destination: &Path,
    maximum_bytes: u64,
    mut reader: TransferReader<R>,
) -> Result<()> {
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| format!("creating `{}`", destination.display()))
    {
        Ok(file) => file,
        Err(error) => {
            // Someone else's file stands there: it is never touched.
            reader.fail(format!("{error:#}"));
            return Err(error);
        }
    };
    let result = copy_download(&mut reader, &mut file, maximum_bytes);
    drop(file);
    match &result {
        Ok(_) => reader.finish(),
        Err(error) => {
            reader.fail(format!("{error:#}"));
            let _ = fs::remove_file(destination);
        }
    }
    result.map(|_| ())
}

pub(super) fn copy_download(
    reader: &mut impl Read,
    writer: &mut impl io::Write,
    maximum: u64,
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
        let source = std::io::Cursor::new(vec![7_u8; 150_000]);
        let record = TransferRecord::http(
            Purpose::ReleaseBundle,
            "https://example.test/bundle.zip",
            Destination::Home,
        )
        .total(Some(150_000));
        let reader = transfer::begin(record, Some(&task)).reader(source);

        write_download_observed(&destination, 150_000, reader).unwrap();
        task.finish();

        let events = recorder.0.lock().unwrap();
        assert!(matches!(events[0].kind, ProgressEventKind::Started { .. }));
        assert!(matches!(
            &events[1].kind,
            ProgressEventKind::Started { transfer: Some(record), .. }
                if record.purpose == Purpose::ReleaseBundle
        ));
        assert!(events.iter().any(|event| matches!(
            event.kind,
            ProgressEventKind::Progress {
                completed: 150_000,
                total: Some(150_000),
                ref unit,
                transfer: Some(_),
                ..
            } if unit == "bytes"
        )));
        assert!(matches!(
            events.last().unwrap().kind,
            ProgressEventKind::Finished { .. }
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
