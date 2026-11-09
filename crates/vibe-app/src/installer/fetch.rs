//! The fetch of one declared input (`PROP-065#sizes`): an HTTP download
//! through the transfer wrapper (`PROP-068#coverage`), or a package file
//! copied in place, verified against its SHA-256 either way.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#coverage");

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};
use vibe_core::progress::transfer::{self, Destination, Purpose, TransferRecord};

use super::{FetchWatch, hex, io, package_file};
use crate::error::InstallerError;

/// What a step's downloads tell the journal about themselves
/// (`PROP-068#transfer`): the package and step, the run they belong to
/// (`batch`) and the line that resumes it if it is interrupted (`#pause`).
#[derive(Debug, Clone, Default)]
pub(crate) struct TransferFacts {
    pub package: String,
    pub step: String,
    pub batch: Option<String>,
    pub resume: Option<String>,
}

impl TransferFacts {
    fn record(&self, purpose: Purpose, url: &str, total: Option<u64>) -> TransferRecord {
        let mut record = TransferRecord::http(purpose, url, Destination::Cache).total(total);
        if !self.package.is_empty() {
            record = record.package(&self.package);
        }
        if !self.step.is_empty() {
            record = record.step(&self.step);
        }
        if let Some(batch) = &self.batch {
            record = record.batch(batch);
        }
        if let Some(line) = &self.resume {
            record = record.resume(line);
        }
        record
    }
}

/// Copy a declared installer source into `destination`, verifying its
/// SHA-256 and its declared size. A source is an `https://` URL or a path
/// under `package_root`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#disk-writes")]
pub(crate) fn fetch_verified(
    source: &str,
    sha256: &str,
    package_root: &Path,
    destination: &Path,
    offline: bool,
    watch: &FetchWatch<'_>,
) -> Result<(), InstallerError> {
    let fetch_error = |reason: String| InstallerError::Fetch {
        source_ref: shown_source(source, watch),
        reason,
    };
    // An undeclared size still gives a bar when the server announces one
    // (`PROP-065#display`); only the declared size refuses a download.
    let remote = source.starts_with("https://") || source.starts_with("http://");
    let (input, announced): (Box<dyn Read>, Option<u64>) = if remote {
        if offline {
            return Err(InstallerError::Offline {
                source_ref: shown_source(source, watch),
            });
        }
        if let Some(client) = watch.http_client {
            let cancellation = watch
                .task
                .map(|task| task.progress().cancellation().clone())
                .unwrap_or_default();
            let response =
                client
                    .get_anonymous(source, &cancellation)
                    .map_err(|error| match error {
                        vibe_package_source::SourceHttpError::Policy(error) => {
                            InstallerError::SourcePolicy(error)
                        }
                        vibe_package_source::SourceHttpError::Transport(error) => {
                            managed_error(source, error)
                        }
                    })?;
            if !response.status().is_success() {
                return Err(fetch_error(format!("HTTP status {}", response.status())));
            }
            let announced = response.content_length();
            (Box::new(response), announced)
        } else {
            let builder = vibe_net::http::route(
                reqwest::blocking::Client::builder().timeout(Duration::from_secs(600)),
                vibe_net::http::Ambient::Inherit,
            )
            .map_err(|error| fetch_error(error.to_string()))?;
            let response = builder
                .build()
                .and_then(|client| client.get(source).send())
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|error| fetch_error(with_causes(&error)))?;
            let announced = response.content_length();
            (Box::new(response), announced)
        }
    } else {
        let path = package_file(package_root, source).map_err(fetch_error)?;
        let file = File::open(&path).map_err(|error| fetch_error(error.to_string()))?;
        (Box::new(file), None)
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
    }
    let output = File::create(destination).map_err(|source| io("creating", destination, source))?;
    let digest = if remote {
        // Every HTTP download goes through the one wrapper
        // (`PROP-068#coverage`); its samples are the step's measurements.
        let facts = watch.facts.cloned().unwrap_or_default();
        let purpose = watch.purpose.unwrap_or(Purpose::Other);
        let mut record = facts.record(purpose, source, watch.size.or(announced));
        if let Some(declared) = watch.remapped_from {
            record = record.remapped_from(declared);
        }
        let mut reader = transfer::begin(record, watch.task)
            .reader(input)
            .with_disk_writer(output);
        let result = {
            let (input, output) = reader.parts();
            copy_chunks(input, output, source, destination, watch)
        }
        .and_then(|result| {
            reader.sync_all().map_err(|error| {
                let _ = fs::remove_file(destination);
                io("writing", destination, error)
            })?;
            Ok(result)
        });
        match &result {
            Ok(_) => reader.finish(),
            Err(InstallerError::Interrupted { .. } | InstallerError::Cancelled(_)) => {
                reader.interrupted()
            }
            Err(error) => reader.fail(error.to_string()),
        }
        result?.1
    } else {
        // A package file is not a download (`PROP-068#transfer`): it is
        // copied and verified, with one measurement for the step's bar.
        let mut input = input;
        let mut output = output;
        let (copied, digest) = copy_chunks(&mut input, &mut output, source, destination, watch)?;
        output.sync_all().map_err(|error| {
            let _ = fs::remove_file(destination);
            io("writing", destination, error)
        })?;
        if let Some(task) = watch.task {
            task.set_progress(copied, watch.size, "bytes");
        }
        digest
    };
    if digest != sha256.to_ascii_lowercase() {
        let _ = fs::remove_file(destination);
        return Err(InstallerError::Digest {
            source_ref: shown_source(source, watch),
        });
    }
    Ok(())
}

/// The chunk loop both kinds of source share: the interrupt and the
/// declared size are checked between chunks, the digest is taken on the
/// way, and a refused or failed copy leaves no file behind. Answers the
/// bytes copied and their SHA-256.
fn copy_chunks(
    input: &mut dyn Read,
    output: &mut dyn Write,
    source: &str,
    destination: &Path,
    watch: &FetchWatch<'_>,
) -> Result<(u64, String), InstallerError> {
    let stop = |error: InstallerError| {
        let _ = fs::remove_file(destination);
        error
    };
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        if let Some(task) = watch.task {
            task.progress()
                .checkpoint()
                .map_err(|error| stop(InstallerError::Cancelled(error)))?;
        }
        if watch
            .interrupt
            .is_some_and(|interrupt| interrupt.poll(None))
        {
            return Err(stop(InstallerError::Interrupted {
                what: format!("fetching `{source}`"),
            }));
        }
        let read = input.read(&mut buffer).map_err(|error| {
            if watch.http_client.is_some()
                && (source.starts_with("https://") || source.starts_with("http://"))
            {
                stop(managed_error(
                    source,
                    vibe_registry::http_admission::HttpGetError::from_io(error),
                ))
            } else {
                stop(InstallerError::Fetch {
                    source_ref: shown_source(source, watch),
                    reason: error.to_string(),
                })
            }
        })?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if let Some(declared) = watch.size.filter(|declared| total > *declared) {
            return Err(stop(InstallerError::Oversize {
                source_ref: shown_source(source, watch),
                declared,
            }));
        }
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|error| stop(io("writing", destination, error)))?;
    }
    Ok((total, hex(&hash.finalize())))
}

fn managed_error(
    source: &str,
    error: vibe_registry::http_admission::HttpGetError,
) -> InstallerError {
    use vibe_registry::http_admission::HttpGetError;
    let reason = match error {
        HttpGetError::Refused(vibe_registry::ContactRefusal::OperationCancelled) => {
            return InstallerError::Cancelled(vibe_core::cancellation::Cancelled);
        }
        HttpGetError::Refused(refusal) => return InstallerError::ContactRefused(refusal),
        HttpGetError::Transport(error) => with_causes(&error.without_url()),
        HttpGetError::Io(error) => error.to_string(),
    };
    InstallerError::Fetch {
        source_ref: vibe_core::progress::transfer::safe_remote(source),
        reason,
    }
}

/// The error and every cause behind it, `: `-joined. An HTTP client's own
/// message is only the operation ("error sending request"); what went
/// wrong — a reset during the TLS handshake, a refused connection — is in
/// its causes (`PROP-059#installer-separation`: the failure names its
/// cause). A cause whose text the message already carries is not repeated.
fn with_causes(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        let part = next.to_string();
        if !part.is_empty() && !text.contains(&part) {
            text.push_str(": ");
            text.push_str(&part);
        }
        cause = next.source();
    }
    text
}

fn shown_source(source: &str, watch: &FetchWatch<'_>) -> String {
    if watch.http_client.is_some() {
        vibe_core::progress::transfer::safe_remote(source)
    } else {
        source.into()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use vibe_core::progress::Progress;
    use vibe_core::progress::transfer::{Outcome, TransferJournal, Transfers};
    use vibe_core::remap::{
        Artifact, ArtifactKind, Layer, LoadedSource, Locator, Provenance, RowDecl,
    };

    use super::super::{FetchWatch, Located, fetch_located};
    use super::*;

    /// Serve `body` once per connection on a loopback port.
    fn serve(body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().filter_map(Result::ok) {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream
                    .write_all(head.as_bytes())
                    .and_then(|()| stream.write_all(body));
            }
        });
        format!("http://{address}/mirror/payload.bin")
    }

    /// A connection the server closes without an answer is reported with
    /// the cause the HTTP client only carries in its error chain, not as
    /// the bare "error sending request".
    #[test]
    fn a_failed_request_names_its_cause() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().filter_map(Result::ok) {
                drop(stream);
            }
        });
        let home = tempfile::tempdir().unwrap();
        let source = format!("http://{address}/payload.bin");
        let error = fetch_verified(
            &source,
            &"0".repeat(64),
            home.path(),
            &home.path().join("out"),
            false,
            &FetchWatch::default(),
        )
        .unwrap_err();
        let InstallerError::Fetch { reason, .. } = error else {
            panic!("expected a fetch failure, got {error:?}");
        };
        let (_, after) = reason
            .split_once("error sending request")
            .unwrap_or_else(|| panic!("unexpected reason: {reason}"));
        assert!(after.contains(": "), "the cause is missing: {reason}");
    }

    /// A remapped input is journaled as fetched from the mirror, naming
    /// the URL the manifest declared (`PROP-068#redaction`), with the
    /// step's package, step, batch and purpose.
    #[test]
    fn a_remapped_download_is_journaled_with_the_declared_url() {
        let home = tempfile::tempdir().unwrap();
        let journal = TransferJournal::new(home.path());
        let mirror = serve(b"payload");
        let digest = hex(&Sha256::digest(b"payload"));
        let declared = "https://upstream.invalid/payload.bin?token=secret";
        let row: RowDecl =
            toml::from_str(&format!("sha256 = \"{digest}\"\nurl = \"{mirror}\"")).unwrap();
        let locator = Locator::from_sources(
            vec![LoadedSource {
                layer: Layer::Local,
                source: "settings.toml".into(),
                rows: vec![
                    row.into_row(Provenance::new(Layer::Local, "settings.toml", 0))
                        .unwrap(),
                ],
                policy: Default::default(),
            }],
            Vec::new(),
        );
        let artifact = Artifact::new(ArtifactKind::AppInput)
            .with_url(declared)
            .with_digest(&digest);
        let transfers = Transfers::new(Some(journal.clone()), None, "install -g");
        let task = Progress::default()
            .with_transfers(Some(transfers))
            .task("fetch");
        let facts = TransferFacts {
            package: "org.example/tool@1.0.0".into(),
            step: "fetch".into(),
            batch: Some("20261101T100000.000Z".into()),
            resume: None,
        };
        let watch = FetchWatch {
            task: Some(&task),
            facts: Some(&facts),
            purpose: Some(Purpose::Archive),
            ..FetchWatch::default()
        };
        let out = home.path().join("out");
        let located = Located {
            locator: &locator,
            artifact: &artifact,
            declared,
            sha256: &digest,
            filename: None,
        };
        fetch_located(&located, home.path(), &out, false, &watch).unwrap();
        task.finish();
        let entry = &journal.history()[0];
        assert_eq!(entry.outcome, Some(Outcome::Finished));
        assert_eq!(entry.record.url, mirror);
        assert_eq!(
            entry.record.remapped_from.as_deref(),
            Some("https://upstream.invalid/payload.bin")
        );
        assert_eq!(entry.record.purpose, Purpose::Archive);
        assert_eq!(entry.record.batch, "20261101T100000.000Z");
        assert_eq!(entry.record.step.as_deref(), Some("fetch"));
        assert_eq!(entry.bytes, 7);
        assert_eq!(entry.disk_written_bytes, Some(7));
    }
}
