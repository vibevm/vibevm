use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use vibe_llm::{
    ChatInput, ChatMessage, ChatRole, Endpoint, LLMProvider, MAX_CHAT_RESPONSE_BYTES,
    OpenAiCompatibleProvider, ProviderError, TransportError,
};

const BODY_CANARY: &str = "oversize-body-canary-must-never-appear";

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

fn error_chain_text(error: &(dyn Error + 'static)) -> String {
    let mut out = format!("{error:?}\n{error}");
    let mut source = error.source();
    while let Some(next) = source {
        out.push('\n');
        out.push_str(&format!("{next:?}\n{next}"));
        source = next.source();
    }
    out
}

#[test]
fn production_transport_refuses_a_limit_plus_one_success_body_without_echo() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_request(&mut stream);
        let mut body = vec![b'x'; MAX_CHAT_RESPONSE_BYTES + 1];
        body[..BODY_CANARY.len()].copy_from_slice(BODY_CANARY.as_bytes());
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
    });

    let provider = OpenAiCompatibleProvider::with_reqwest(
        "demo-model",
        Endpoint::parse(&format!("http://{address}/v1/chat/completions"), false).unwrap(),
        None,
    )
    .unwrap();
    let input = ChatInput::new(vec![ChatMessage::new(ChatRole::User, "hello")]).unwrap();
    let error = provider.chat(&input).unwrap_err();
    assert!(matches!(
        &error,
        ProviderError::Transport(TransportError::ResponseTooLarge { limit })
            if *limit == MAX_CHAT_RESPONSE_BYTES
    ));
    let rendered = error_chain_text(&error);
    assert!(!rendered.contains(BODY_CANARY), "{rendered}");
    server.join().unwrap();
}
