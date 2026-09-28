use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::provider::{AnthropicProvider, CompletionRequest, OpenRouterProvider, Provider};

const REPLY: &str = r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;

/// A loopback host that reports every request head it reads, with the index
/// of the connection it arrived on (0 = the first one opened). HEAD gets a
/// bodyless 404 — the status a real endpoint gives a warm — and anything else
/// a one-line completion.
fn host() -> (SocketAddr, Receiver<(usize, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for (i, conn) in listener.incoming().enumerate() {
            let Ok(conn) = conn else { return };
            let tx = tx.clone();
            thread::spawn(move || serve(i, conn, tx));
        }
    });
    (addr, rx)
}

/// Answer requests on one connection until the client closes it.
fn serve(i: usize, conn: TcpStream, tx: Sender<(usize, String)>) {
    let mut out = conn.try_clone().unwrap();
    let mut r = BufReader::new(conn);
    loop {
        let mut head = String::new();
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let len = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        let mut body = vec![0; len];
        let _ = r.read_exact(&mut body);
        let answer = if head.starts_with("HEAD ") {
            "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n".to_string()
        } else {
            format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{REPLY}",
                REPLY.len()
            )
        };
        let _ = tx.send((i, head));
        let _ = out.write_all(answer.as_bytes());
        let _ = out.flush();
    }
}

fn next(rx: &Receiver<(usize, String)>) -> (usize, String) {
    rx.recv_timeout(Duration::from_secs(5))
        .expect("no request reached the host")
}

#[test]
fn a_warm_opens_a_connection_to_the_endpoint_without_the_key() {
    let (addr, rx) = host();
    let p = OpenRouterProvider::new("sk-or-secret".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    p.warm();
    let (_, head) = next(&rx);
    assert!(head.starts_with("HEAD /v1/chat/completions "), "{head}");
    assert!(
        !head.to_ascii_lowercase().contains("authorization"),
        "{head}"
    );
    assert!(!head.contains("sk-or-secret"), "{head}");
}

#[test]
fn an_anthropic_warm_carries_neither_the_key_nor_the_bearer() {
    let (addr, rx) = host();
    let base = format!("http://{addr}");
    AnthropicProvider::new("sk-ant-secret".into())
        .with_base_url(&base)
        .warm();
    AnthropicProvider::with_oauth("sk-ant-oat01-secret".into())
        .with_base_url(&base)
        .warm();
    for _ in 0..2 {
        let (_, head) = next(&rx);
        let low = head.to_ascii_lowercase();
        assert!(head.starts_with("HEAD /v1/messages "), "{head}");
        assert!(!low.contains("x-api-key"), "{head}");
        assert!(!low.contains("authorization"), "{head}");
        assert!(!head.contains("secret"), "{head}");
    }
}

/// The point of the warm: the call after it pays no handshake, because it
/// checks out the connection the warm left in the pool. A warm on any other
/// client would open a socket nothing ever reuses.
#[test]
fn the_call_after_a_warm_rides_the_warmed_connection() {
    let (addr, rx) = host();
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    p.warm();
    let (warmed, head) = next(&rx);
    assert!(head.starts_with("HEAD "), "{head}");
    // The client returns the socket to the pool as it reads the 404, just
    // after the host has written it.
    thread::sleep(Duration::from_millis(250));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let req = CompletionRequest {
        model: "m".into(),
        prompt: "hi".into(),
        max_tokens: 8,
        ..Default::default()
    };
    let reply = rt.block_on(p.complete(req)).unwrap();
    assert_eq!(reply.text, "ok");
    let (called, head) = next(&rx);
    assert!(head.starts_with("POST "), "{head}");
    assert_eq!(called, warmed, "the call opened a second connection");
}

/// Typing must never wait on the network: a host that accepts and then says
/// nothing leaves the warm pending on the provider runtime, not the caller.
#[test]
fn a_warm_returns_at_once_even_when_the_host_never_answers() {
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let p = OpenRouterProvider::new("k".into()).with_endpoint(format!(
        "http://{}/v1/chat/completions",
        silent.local_addr().unwrap()
    ));
    let t = Instant::now();
    p.warm();
    assert!(
        t.elapsed() < Duration::from_millis(500),
        "{:?}",
        t.elapsed()
    );
    drop(silent);
}

/// The broker holds its provider as `Arc<dyn Provider>`, and a generic
/// consumer sees the `Arc` as the provider: it must warm what it wraps, not
/// fall back to the trait's no-op.
#[test]
fn a_shared_provider_warms_the_one_it_wraps() {
    fn warm_it<P: Provider>(p: &P) {
        p.warm();
    }
    let (addr, rx) = host();
    let p: std::sync::Arc<dyn Provider> = std::sync::Arc::new(
        OpenRouterProvider::new("k".into())
            .with_endpoint(format!("http://{addr}/v1/chat/completions")),
    );
    warm_it(&p);
    let (_, head) = next(&rx);
    assert!(head.starts_with("HEAD "), "{head}");
}
