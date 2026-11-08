//! Bounded owned numeric-loopback fixture; no detached listener survives a test.
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
enum Reply {
    Body(Vec<u8>),
    Missing,
    #[cfg(unix)]
    Redirect(String),
}
pub(super) struct Server {
    pub(super) url: String,
    contacts: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Server {
    pub(super) fn new(host: &str, body: &[u8]) -> Self {
        let body = body.to_vec();
        Self::respond(host, move |_| Reply::Body(body.clone()))
    }
    /// Answer every request with a 302 to `target` plus the requested path.
    #[cfg(unix)]
    pub(super) fn redirect(host: &str, target: String) -> Self {
        Self::respond(host, move |request| {
            Reply::Redirect(format!("{target}{request}"))
        })
    }
    pub(super) fn directory(host: &str, root: std::path::PathBuf) -> Self {
        Self::respond(host, move |request| {
            let path = request
                .split('?')
                .next()
                .unwrap_or_default()
                .trim_start_matches('/');
            if path.split('/').any(|segment| segment == "..") {
                return Reply::Missing;
            }
            fs::read(root.join(path)).map_or(Reply::Missing, Reply::Body)
        })
    }
    fn respond(host: &str, response: impl Fn(&str) -> Reply + Send + 'static) -> Self {
        let listener = TcpListener::bind((host, 0)).unwrap();
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
                        socket
                            .set_read_timeout(Some(Duration::from_millis(200)))
                            .unwrap();
                        let mut request = [0u8; 8192];
                        let count = socket.read(&mut request).unwrap_or_default();
                        let line = String::from_utf8_lossy(&request[..count]);
                        let requested = line.split_whitespace().nth(1).unwrap_or("");
                        let (status, location, body) = match response(requested) {
                            Reply::Body(body) => ("200 OK", String::new(), body),
                            Reply::Missing => ("404 Not Found", String::new(), Vec::new()),
                            #[cfg(unix)]
                            Reply::Redirect(target) => {
                                ("302 Found", format!("Location: {target}\r\n"), Vec::new())
                            }
                        };
                        let _ = write!(
                            socket,
                            "HTTP/1.1 {status}\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = socket.write_all(&body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
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
    pub(super) fn count(&self) -> usize {
        self.contacts.load(Ordering::SeqCst)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}
