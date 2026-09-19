use super::*;

#[test]
fn host_is_taken_from_the_endpoint_url() {
    assert_eq!(
        host_of("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"),
        "dashscope.aliyuncs.com"
    );
    assert_eq!(host_of("http://127.0.0.1:8080/v1"), "127.0.0.1:8080");
    // Never panics on junk: the diagnostic path must not have a failure mode.
    assert_eq!(host_of("not a url"), "not a url");
}

/// The regression this module exists for: a read timeout used to reach the
/// user as `error decoding response body`, which names neither the timeout
/// nor the endpoint.
#[tokio::test]
async fn a_read_timeout_says_timeout_and_names_the_host() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock); // accept, then never answer
        }
    });
    let endpoint = format!("http://{addr}/v1/chat/completions");
    let client = reqwest::Client::builder()
        .read_timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();
    let err = client.get(&endpoint).send().await.unwrap_err();
    let said = wire_error(&err, &endpoint);
    assert!(said.contains("timed out"), "{said}");
    assert!(said.contains(&addr.to_string()), "{said}");
    assert!(!said.contains("error decoding response body"), "{said}");
}

#[tokio::test]
async fn an_unreachable_endpoint_says_so() {
    // Bind then drop: the port is free again, so connecting is refused fast.
    let addr = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let endpoint = format!("http://{addr}/v1/chat/completions");
    let client = reqwest::Client::new();
    let err = client.get(&endpoint).send().await.unwrap_err();
    let said = wire_error(&err, &endpoint);
    assert!(said.contains("connect"), "{said}");
    assert!(said.contains(&addr.to_string()), "{said}");
}
