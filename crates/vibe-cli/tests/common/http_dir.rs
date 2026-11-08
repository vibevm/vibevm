//! A bounded, owned plain-HTTP server over one directory on numeric loopback.
//!
//! Enough to serve a dumb-HTTP Git repository (a bare repository after
//! `git update-server-info`), which the controlled native transport admits:
//! PROP-073 `#NATIVE-PREPARATION` admits HTTP, HTTPS and SSH, never a local
//! `file://` Git endpoint. The listener thread stops and is joined on drop, so
//! no detached listener survives a test.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct DirectoryServer {
    /// `http://127.0.0.1:<port>` — no trailing slash.
    pub url: String,
    contacts: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl DirectoryServer {
    /// Serve `root`: `GET /<rel>` answers the file at `root/<rel>`, anything
    /// else (a directory, a missing file, a `..` segment) is a 404.
    pub fn new(root: PathBuf) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let contacts = Arc::new(AtomicUsize::new(0));
        let counter = contacts.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        counter.fetch_add(1, Ordering::SeqCst);
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_millis(500)))
                            .unwrap();
                        let mut request = [0u8; 8192];
                        let count = socket.read(&mut request).unwrap_or_default();
                        let head = String::from_utf8_lossy(&request[..count]);
                        let target = head.split_whitespace().nth(1).unwrap_or("");
                        let path = target
                            .split('?')
                            .next()
                            .unwrap_or_default()
                            .trim_start_matches('/');
                        let body = if path.split('/').any(|segment| segment == "..") {
                            None
                        } else {
                            fs::read(root.join(path)).ok()
                        };
                        let (status, body) = match body {
                            Some(body) => ("200 OK", body),
                            None => ("404 Not Found", Vec::new()),
                        };
                        let _ = write!(
                            socket,
                            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = socket.write_all(&body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
        });
        Self {
            url,
            contacts,
            stop,
            worker: Some(worker),
        }
    }

    /// How many connections the server has accepted so far.
    pub fn count(&self) -> usize {
        self.contacts.load(Ordering::SeqCst)
    }
}

impl Drop for DirectoryServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
