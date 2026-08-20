//! Worker stdio plumbing: feed the prompt in, tee stdout through the
//! stream parser into the transcript file, drain stderr into its own
//! file. Split out of `main.rs` at the module-grain file budget
//! (surface-form discipline) — these three tasks are the pod's stdio
//! responsibility seam and run alongside the supervision loop.

use camino::Utf8PathBuf;
use fractality_backend_claude_code::stream::{StreamParser, StreamSummary};
use fractality_core::run::UsageTotals;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Writes the spec's stdin payload to the child and closes the pipe —
/// EOF is part of the contract (CC's print mode reads stdin to EOF, F14).
/// A payload can exceed the pipe buffer, so this runs as its own task
/// alongside the pumps rather than blocking the supervision loop.
pub(crate) fn feed_stdin(
    stdin: Option<tokio::process::ChildStdin>,
    payload: Option<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let (Some(mut stdin), Some(payload)) = (stdin, payload) else {
            return;
        };
        if let Err(e) = stdin.write_all(payload.as_bytes()).await {
            // A worker that exits before reading its prompt surfaces its
            // own error; the broken pipe here is the symptom, not the story.
            tracing::warn!(error = %e, "stdin feed ended early");
        }
        let _ = stdin.shutdown().await;
    })
}

/// Streams the worker's stdout into the transcript file while teeing
/// every line through the stream parser (Phase 3 metering, D14 tolerant
/// — a malformed line is counted, never fatal). File-write failures are
/// loud but do not stop parsing: losing the transcript must not also
/// lose the metering, and vice versa. Publishes running totals into the
/// watch channel; returns the end-of-stream summary.
pub(crate) fn pump_transcript(
    reader: Option<tokio::process::ChildStdout>,
    path: Utf8PathBuf,
    usage_tx: tokio::sync::watch::Sender<UsageTotals>,
) -> tokio::task::JoinHandle<Option<StreamSummary>> {
    tokio::spawn(async move {
        let reader = reader?;
        let mut parser = StreamParser::new();
        let mut file = match tokio::fs::File::create(path.as_std_path()).await {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::error!(%path, error = %e, "cannot open transcript file");
                None
            }
        };
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if let Some(f) = file.as_mut() {
                        let wrote = async {
                            f.write_all(line.as_bytes()).await?;
                            f.write_all(b"\n").await
                        }
                        .await;
                        if let Err(e) = wrote {
                            tracing::error!(%path, error = %e, "transcript write failed; parsing continues");
                            file = None;
                        }
                    }
                    parser.feed_line(&line);
                    usage_tx.send_replace(parser.totals());
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(%path, error = %e, "transcript stream ended with an error");
                    break;
                }
            }
        }
        if let Some(mut f) = file {
            let _ = f.flush().await;
        }
        Some(parser.finish())
    })
}

/// Streams a child pipe into a run-dir file.
pub(crate) fn pump<R>(reader: Option<R>, path: Utf8PathBuf) -> tokio::task::JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let Some(mut reader) = reader else { return };
        let mut file = match tokio::fs::File::create(path.as_std_path()).await {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(%path, error = %e, "cannot open transcript file");
                return;
            }
        };
        if let Err(e) = tokio::io::copy(&mut reader, &mut file).await {
            tracing::warn!(%path, error = %e, "transcript pump ended with an error");
        }
        let _ = file.flush().await;
    })
}
