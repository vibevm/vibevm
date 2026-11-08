//! Owned async request/body bridge. Cancellation drops network futures, then
//! joins the worker before returning; no blocking HTTP work is detached.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use super::*;
use std::{
    io::{self, Read},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use vibe_core::cancellation::CancellationToken;

pub enum ManagedResponse {
    Blocking(reqwest::blocking::Response),
    Controlled(ControlledResponse),
}
pub struct ControlledResponse {
    status: reqwest::StatusCode,
    headers: reqwest::header::HeaderMap,
    url: Url,
    length: Option<u64>,
    body: tokio::sync::mpsc::Receiver<Result<Vec<u8>, HttpGetError>>,
    pending: std::io::Cursor<Vec<u8>>,
    worker: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}
impl ManagedResponse {
    pub fn status(&self) -> reqwest::StatusCode {
        match self {
            Self::Blocking(r) => r.status(),
            Self::Controlled(r) => r.status,
        }
    }
    pub fn headers(&self) -> &reqwest::header::HeaderMap {
        match self {
            Self::Blocking(r) => r.headers(),
            Self::Controlled(r) => &r.headers,
        }
    }
    pub fn url(&self) -> &Url {
        match self {
            Self::Blocking(r) => r.url(),
            Self::Controlled(r) => &r.url,
        }
    }
    pub fn content_length(&self) -> Option<u64> {
        match self {
            Self::Blocking(r) => r.content_length(),
            Self::Controlled(r) => r.length,
        }
    }
    pub fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, HttpGetError> {
        serde_json::from_slice(&self.bytes()?)
            .map_err(|error| HttpGetError::Io(io::Error::other(error)))
    }
    pub fn text(self) -> Result<String, HttpGetError> {
        match self {
            Self::Blocking(response) => response.text().map_err(HttpGetError::Transport),
            response => {
                let bytes = response.bytes()?;
                let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
                Ok(String::from_utf8_lossy(bytes).into_owned())
            }
        }
    }
    pub fn bytes(mut self) -> Result<Vec<u8>, HttpGetError> {
        let mut bytes = Vec::new();
        self.read_to_end(&mut bytes)
            .map_err(HttpGetError::from_io)?;
        Ok(bytes)
    }
}
impl Read for ManagedResponse {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Blocking(r) => r.read(buffer),
            Self::Controlled(r) => r.read(buffer),
        }
    }
}
impl ControlledResponse {
    fn join(&mut self) -> io::Result<()> {
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| io::Error::other("controlled HTTP worker panicked"))?;
        }
        Ok(())
    }
}
impl Read for ControlledResponse {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            let count = self.pending.read(buffer)?;
            if count > 0 {
                return Ok(count);
            }
            match self.body.blocking_recv() {
                Some(Ok(bytes)) => self.pending = std::io::Cursor::new(bytes),
                Some(Err(error)) => {
                    self.join()?;
                    return Err(io::Error::other(error));
                }
                None => {
                    self.join()?;
                    return Ok(0);
                }
            }
        }
    }
}
impl Drop for ControlledResponse {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.body.close();
        let _ = self.join();
    }
}
async fn stopped(token: &CancellationToken, local: &AtomicBool) {
    loop {
        if local.load(Ordering::SeqCst) || token.checkpoint().is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
fn cancelled() -> HttpGetError {
    ContactRefusal::OperationCancelled.into()
}

pub(super) fn send(
    client: &reqwest::Client,
    request: reqwest::blocking::Request,
    token: &CancellationToken,
) -> Result<ManagedResponse, HttpGetError> {
    token.checkpoint().map_err(|_| cancelled())?;
    let body = request
        .body()
        .map(|body| {
            body.as_bytes()
                .map(Vec::from)
                .ok_or(ContactRefusal::UnsupportedControlledRequest)
        })
        .transpose()?;
    let mut outgoing = reqwest::Request::new(request.method().clone(), request.url().clone());
    *outgoing.headers_mut() = request.headers().clone();
    if let Some(body) = body {
        *outgoing.body_mut() = Some(body.into());
    }
    *outgoing.timeout_mut() = request.timeout().copied();
    let client = client.clone();
    let cancellation = token.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let local = stop.clone();
    let (headers_tx, headers_rx) = std::sync::mpsc::sync_channel(1);
    let (body_tx, body) = tokio::sync::mpsc::channel(2);
    let worker = std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = headers_tx.send(Err(HttpGetError::Io(error)));
                return;
            }
        };
        runtime.block_on(async move {
            let result = tokio::select! {
                _=stopped(&cancellation,&local)=>Err(cancelled()),
                result=client.execute(outgoing)=>result.map_err(HttpGetError::Transport),
            };
            let mut response = match result {
                Ok(response) => response,
                Err(error) => {
                    let _ = headers_tx.send(Err(error));
                    return;
                }
            };
            let headers = (
                response.status(),
                response.headers().clone(),
                response.url().clone(),
                response.content_length(),
            );
            if headers_tx.send(Ok(headers)).is_err() {
                return;
            }
            loop {
                let next = tokio::select! {
                    _=stopped(&cancellation,&local)=>Err(cancelled()),
                    next=response.chunk()=>next.map_err(HttpGetError::Transport),
                };
                match next {
                    Ok(Some(bytes)) => {
                        let delivered = tokio::select! {
                            _=stopped(&cancellation,&local)=>false,
                            result=body_tx.send(Ok(bytes.to_vec()))=>result.is_ok(),
                        };
                        if !delivered {
                            drop(response);
                            if !local.load(Ordering::SeqCst) {
                                let _ = body_tx.send(Err(cancelled())).await;
                            }
                            return;
                        }
                    }
                    Ok(None) => return,
                    Err(error) => {
                        drop(response);
                        let _ = body_tx.send(Err(error)).await;
                        return;
                    }
                }
            }
        });
        // Dropping the runtime waits for its owned work. No task or HTTP
        // blocking thread is allowed to outlive the operation's join.
    });
    match headers_rx.recv() {
        Ok(Ok((status, headers, url, length))) => {
            Ok(ManagedResponse::Controlled(ControlledResponse {
                status,
                headers,
                url,
                length,
                body,
                pending: std::io::Cursor::new(Vec::new()),
                worker: Some(worker),
                stop,
            }))
        }
        result => {
            stop.store(true, Ordering::SeqCst);
            drop(body);
            worker.join().map_err(|_| {
                HttpGetError::Io(io::Error::other("controlled HTTP worker panicked"))
            })?;
            match result {
                Ok(Err(error)) => Err(error),
                _ => Err(HttpGetError::Io(io::Error::other(
                    "controlled HTTP header channel closed",
                ))),
            }
        }
    }
}
