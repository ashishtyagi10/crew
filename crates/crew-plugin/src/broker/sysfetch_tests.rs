use super::*;

use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn only_http_urls_are_fetched() {
    for bad in ["file:///etc/passwd", "ftp://example.com", "example.com", ""] {
        let err = check(bad).expect_err(&format!("{bad} was accepted"));
        assert!(err.contains("only http(s)"), "{err}");
    }
    assert!(check("https://example.com/a?b=1#c").is_ok());
}

#[test]
fn this_machine_and_this_network_are_refused() {
    for host in [
        "http://localhost:8080/x",
        "http://127.0.0.1/x",
        "https://10.0.0.5/x",
        "https://192.168.1.1/",
        "https://172.20.3.4/",
        "http://169.254.169.254/latest/meta-data/",
        "http://user@127.0.0.1/x",
        "http://[::1]/x",
        "http://db.internal/",
    ] {
        let err = check(host).expect_err(&format!("{host} was accepted"));
        assert!(err.contains("refused"), "{err}");
    }
    // …and a public address that merely looks similar is not.
    assert!(check("https://172.32.1.1/").is_ok());
    assert!(check("https://11.0.0.1/").is_ok());
}

/// A one-request HTTP server on a loopback port, so the fetch path is
/// exercised end to end without reaching the network. The private-address
/// guard is bypassed by calling [`get`] directly — which is the only place
/// in the crate that skips it, and it skips it on purpose, here.
fn serve(body: &'static str, content_type: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf);
            let res = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(res.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}/")
}

fn fetched(url: &str) -> Result<String, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(get(url))
}

#[test]
fn a_page_comes_back_as_text_with_its_markup_gone() {
    let url = serve(
        "<html><body><script>x()</script><p>Release 0.22.27 is out.</p></body></html>",
        "text/html; charset=utf-8",
    );
    let text = fetched(&url).expect("the page");
    assert_eq!(text, "Release 0.22.27 is out.");
}

#[test]
fn a_plain_text_body_is_returned_as_it_is() {
    let url = serve("line one\nline two", "text/plain");
    assert_eq!(fetched(&url).unwrap(), "line one\nline two");
}

#[test]
fn a_failure_status_is_an_error_the_agent_can_read() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf);
            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        }
    });
    let err = fetched(&format!("http://127.0.0.1:{port}/")).expect_err("404 is an error");
    assert!(err.contains("404"), "{err}");
}
