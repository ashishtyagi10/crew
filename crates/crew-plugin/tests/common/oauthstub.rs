//! A scriptable loopback HTTP server for the OAuth / zero-key e2e tests:
//! the broker child under test reaches it through `CREW_OAUTH_BASE` (the
//! device and token endpoints) and `CREW_DASHSCOPE_BASE_URL` (the OpenAI-wire
//! chat endpoint), so a full sign-in and a full model call both happen with no
//! network and no key. The handler decides every response from the request
//! path + body; every request is recorded for assertions.
#![allow(dead_code)]
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// One recorded request: (path, body).
pub type Seen = Arc<Mutex<Vec<(String, String)>>>;

pub struct StubServer {
    /// `http://127.0.0.1:<port>` — hand it to the env seams.
    pub base: String,
    pub seen: Seen,
}

/// Serve until the test process exits (the accept thread is deliberately
/// leaked — e2e processes are short-lived). `handler(path, body)` returns
/// `(status, json_body)` per request.
pub fn serve(handler: impl Fn(&str, &str) -> (u16, String) + Send + 'static) -> StubServer {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    std::thread::spawn(move || {
        for sock in listener.incoming() {
            let Ok(mut sock) = sock else { continue };
            let Some((path, body)) = read_request(&mut sock) else {
                continue;
            };
            record.lock().unwrap().push((path.clone(), body.clone()));
            let (status, json) = handler(&path, &body);
            let reply = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                 content-length: {}\r\nconnection: close\r\n\r\n{json}",
                json.len()
            );
            let _ = sock.write_all(reply.as_bytes());
        }
    });
    StubServer { base, seen }
}

/// Minimal HTTP/1.1 request read: request line for the path, headers for
/// Content-Length, then exactly that many body bytes.
fn read_request(sock: &mut std::net::TcpStream) -> Option<(String, String)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        let n = sock.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = find(&buf, b"\r\n\r\n") {
            break i;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let path = head.lines().next()?.split_whitespace().nth(1)?.to_string();
    let want: usize = head
        .to_lowercase()
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + 4 + want {
        let n = sock.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Some((
        path,
        String::from_utf8_lossy(&buf[head_end + 4..]).to_string(),
    ))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// An OpenAI-wire chat completion reply carrying `text`.
pub fn chat_reply(text: &str) -> String {
    serde_json::json!({
        "choices": [{"message": {"role": "assistant", "content": text}}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 10}
    })
    .to_string()
}

/// Grep `root` recursively (plus any extra strings) for `needle`; returns
/// every hit as `path: line` (or `<transcript>` for extras), with `allowed`
/// path substrings excluded — the token STORE legitimately holds the token;
/// log sinks never may.
pub fn sweep(
    root: &std::path::Path,
    extras: &[(&str, &str)],
    needle: &str,
    allowed: &[&str],
) -> Vec<String> {
    let mut hits = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let name = p.to_string_lossy().into_owned();
                if allowed.iter().any(|a| name.contains(a)) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue; // binary file: token material is text
                };
                if text.contains(needle) {
                    hits.push(format!("{name}: contains the secret"));
                }
            }
        }
    }
    for (label, text) in extras {
        if text.contains(needle) {
            hits.push(format!("<{label}>: contains the secret"));
        }
    }
    hits
}
