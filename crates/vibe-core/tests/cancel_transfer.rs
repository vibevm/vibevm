//! Cancellation on real slow loopback input, independent of progress rendering.
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
use vibe_core::{
    cancellation::CancellationToken,
    progress::{
        Progress,
        transfer::{Destination, Purpose, Transfer, TransferRecord},
    },
};
#[test]
fn slow_loopback_reader_stops_between_chunks_without_an_observer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let token = CancellationToken::new();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        for _ in 0..100 {
            if stream.write_all(&[7; 1024]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let task = Progress::default()
        .with_cancellation(token.clone())
        .task("download");
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let record = TransferRecord::http(
        Purpose::Archive,
        "http://127.0.0.1/body",
        Destination::Cache,
    );
    let mut reader = Transfer::begin(record, Some(&task), None).reader(stream);
    let mut buffer = [0; 1024];
    assert!(reader.read(&mut buffer).unwrap() > 0);
    token.cancel();
    let start = Instant::now();
    assert!(reader.read(&mut buffer).is_err());
    assert!(token.acknowledged());
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(reader);
    server.join().unwrap();
}
