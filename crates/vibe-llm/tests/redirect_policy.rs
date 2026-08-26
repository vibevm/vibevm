use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use vibe_llm::{
    ChatInput, ChatMessage, ChatRole, Endpoint, LLMProvider, OpenAiCompatibleProvider,
    ProviderError,
};

fn read_request(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
}

#[test]
fn production_transport_does_not_follow_a_loopback_redirect() {
    const BODY_CANARY: &str = "redirect-body-canary-must-never-appear";
    let redirect_target = TcpListener::bind("127.0.0.1:0").unwrap();
    redirect_target.set_nonblocking(true).unwrap();
    let target_address = redirect_target.local_addr().unwrap();
    let (stop_tx, stop_rx) = mpsc::channel();
    let (hit_tx, hit_rx) = mpsc::channel();
    let target = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if stop_rx.try_recv().is_ok() || Instant::now() >= deadline {
                return;
            }
            match redirect_target.accept() {
                Ok((mut stream, _)) => {
                    read_request(&mut stream);
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        )
                        .unwrap();
                    hit_tx.send(()).unwrap();
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("redirect target accept failed: {error}"),
            }
        }
    });

    let redirect_source = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_address = redirect_source.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = redirect_source.accept().unwrap();
        read_request(&mut stream);
        write!(
            stream,
            "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/captured\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{BODY_CANARY}",
            BODY_CANARY.len()
        )
        .unwrap();
    });

    let provider = OpenAiCompatibleProvider::with_reqwest(
        "demo-model",
        Endpoint::parse(
            &format!("http://{source_address}/v1/chat/completions"),
            false,
        )
        .unwrap(),
        None,
    )
    .unwrap();
    let input = ChatInput::new(vec![ChatMessage::new(ChatRole::User, "hello")]).unwrap();
    let error = provider.chat(&input).unwrap_err();
    assert!(matches!(
        &error,
        ProviderError::HttpStatus { status: 302, .. }
    ));
    assert!(!format!("{error:?}\n{error}").contains(BODY_CANARY));
    server.join().unwrap();
    stop_tx.send(()).unwrap();
    target.join().unwrap();
    assert!(
        hit_rx.try_recv().is_err(),
        "redirect target observed a request"
    );
}
