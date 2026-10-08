//! How the updater asks GitHub for the newest release.
use std::io::{BufRead, BufReader, Write};

/// One request served by a stand-in for GitHub's API: the path it was asked
/// for, and the release it answers with.
fn serve_once(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
        // Drain the headers before answering.
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).unwrap() <= 2 {
                break;
            }
        }
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(resp.as_bytes()).unwrap();
        path
    });
    (url, handle)
}

/// The newest release is asked for BY NAME — one request to
/// `/releases/latest` — never by paging the whole list: GitHub stops a list
/// at 1000 entries (422 past it), and the 1001st release broke every update.
#[test]
fn the_newest_release_is_asked_for_by_name() {
    let (api, server) = serve_once(
        r#"{"tag_name":"v9.8.7","created_at":"2026-10-08T00:00:00Z","name":"v9.8.7","assets":[{"url":"https://example.invalid/a","name":"crew-aarch64-apple-darwin.tar.gz"}]}"#,
    );
    let release = super::latest_release(&api).expect("the release");
    assert_eq!(release.version, "9.8.7");
    assert_eq!(release.assets.len(), 1);
    assert_eq!(
        server.join().unwrap(),
        "/repos/ashishtyagi10/crew/releases/latest"
    );
}
