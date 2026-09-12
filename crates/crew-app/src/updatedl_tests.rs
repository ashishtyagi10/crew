//! The download's contract, driven against a raw `TcpListener` speaking HTTP/1.1
//! by hand — no mock-server dependency, and full control over *how slowly* the
//! body arrives, which is the whole point of these tests.
use std::io::{BufRead, BufReader, Write as IoWrite};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use super::*;

/// Read (and discard) the request head so the client's write completes, then
/// hand back the request line for assertions.
fn read_head(s: &mut BufReader<TcpStream>) -> Vec<String> {
    let mut head = Vec::new();
    loop {
        let mut line = String::new();
        if s.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            break;
        }
        head.push(line.trim_end().to_string());
    }
    head
}

/// Serve one request: `status` line, `len` bytes of body written in `chunks`
/// writes spaced `gap` apart. Returns the request head the client sent.
fn serve_once(
    status: &str,
    len: usize,
    chunks: usize,
    gap: Duration,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}/asset", listener.local_addr().unwrap());
    let status = status.to_string();
    let h = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let head = read_head(&mut reader);
        let mut w = stream;
        let _ = write!(
            w,
            "HTTP/1.1 {status}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
        );
        let _ = w.flush();
        let per = len / chunks.max(1);
        let mut sent = 0;
        for i in 0..chunks {
            let n = if i + 1 == chunks { len - sent } else { per };
            let _ = w.write_all(&vec![b'x'; n]);
            let _ = w.flush();
            sent += n;
            if i + 1 < chunks {
                std::thread::sleep(gap);
            }
        }
        head
    });
    (url, h)
}

/// The regression this module exists for: a body that takes **longer than 30
/// seconds** to arrive must still land. `self_update`'s downloader used
/// reqwest's blocking client, whose default 30 s timeout covers the body read,
/// so this exact shape failed with "error decoding response body: operation
/// timed out" — deterministically, on every retry, for every user whose link
/// could not carry the 11 MB archive inside half a minute.
#[test]
fn a_download_slower_than_thirty_seconds_still_completes() {
    let (url, server) = serve_once("200 OK", 3200, 8, Duration::from_secs(5));
    let started = Instant::now();
    let mut sink = Vec::new();
    let n = download_to(&url, "crew/test", &mut sink).expect("a slow download is not a failed one");
    let took = started.elapsed();
    server.join().unwrap();
    assert_eq!(n, 3200, "every byte of the archive");
    assert_eq!(sink.len(), 3200);
    assert!(
        took > Duration::from_secs(30),
        "the test only guards the 30 s ceiling if it actually crosses it (took {took:?})"
    );
}

#[test]
fn the_asset_headers_github_needs_are_sent() {
    let (url, server) = serve_once("200 OK", 8, 1, Duration::ZERO);
    let mut sink = Vec::new();
    download_to(&url, "crew/9.9.9", &mut sink).expect("download");
    let head = server.join().unwrap();
    let sent = head.join("\n").to_ascii_lowercase();
    assert!(
        sent.contains("accept: application/octet-stream"),
        "without it the API asset URL serves JSON metadata, not the archive: {sent}"
    );
    assert!(
        sent.contains("user-agent: crew/9.9.9"),
        "GitHub rejects an anonymous agent: {sent}"
    );
}

#[test]
fn a_failed_status_is_an_error_not_a_truncated_archive() {
    let (url, server) = serve_once("404 Not Found", 9, 1, Duration::ZERO);
    let mut sink = Vec::new();
    let err =
        download_to(&url, "crew/test", &mut sink).expect_err("404 must not look like success");
    let _ = server.join();
    assert!(
        err.to_string().contains("404"),
        "the status belongs in the message: {err}"
    );
}

/// A 200 with nothing behind it would otherwise be unpacked as an archive and
/// fail much later, with an error naming tar rather than the download.
#[test]
fn an_empty_body_is_an_error() {
    let (url, server) = serve_once("200 OK", 0, 1, Duration::ZERO);
    let mut sink = Vec::new();
    let err = download_to(&url, "crew/test", &mut sink).expect_err("an empty archive is a failure");
    let _ = server.join();
    assert!(err.to_string().contains("empty"), "{err}");
}
