//! Requests to Gmail, run together on the worker's runtime and answered in
//! the order asked: a GET, or a POST of JSON when the request carries a body
//! (and a lone PATCH, for Google Calendar's `events.patch`). A 401 is its
//! own answer: the caller renews the token.
use std::time::Duration;

use serde_json::Value;

const HTTP: Duration = Duration::from_secs(20);

pub(crate) enum Fetch {
    Expired,
    Failed(String),
}

/// A URL, and the JSON to POST to it (`None`: a GET).
pub(crate) type Req = (String, Option<Value>);

pub(crate) async fn all(
    http: reqwest::Client,
    token: String,
    reqs: Vec<Req>,
) -> Result<Vec<Value>, Fetch> {
    let mut set = tokio::task::JoinSet::new();
    for (i, (url, body)) in reqs.into_iter().enumerate() {
        let (http, token) = (http.clone(), token.clone());
        set.spawn(async move { (i, one(&http, &token, &url, body).await) });
    }
    let mut out: Vec<Option<Value>> = vec![None; set.len()];
    while let Some(done) = set.join_next().await {
        let (i, got) = done.map_err(|_| Fetch::Failed("a request was dropped".into()))?;
        out[i] = Some(got?);
    }
    Ok(out.into_iter().map(Option::unwrap_or_default).collect())
}

async fn one(
    http: &reqwest::Client,
    token: &str,
    url: &str,
    body: Option<Value>,
) -> Result<Value, Fetch> {
    let req = match body {
        Some(b) => http.post(url).json(&b),
        None => http.get(url),
    };
    answer(req, token, service(url)).await
}

/// A PATCH of `body` to `url`.
pub(crate) async fn patch(
    http: reqwest::Client,
    token: String,
    url: String,
    body: Value,
) -> Result<Value, Fetch> {
    let who = service(&url);
    answer(http.patch(url).json(&body), &token, who).await
}

/// Whose answer it is, by the address asked: Google Calendar's failures were
/// all worded as Gmail's ("Gmail 409" for an event already there).
fn service(url: &str) -> &'static str {
    if url.contains("/calendar/") {
        "Google Calendar"
    } else {
        "Gmail"
    }
}

async fn answer(req: reqwest::RequestBuilder, token: &str, who: &str) -> Result<Value, Fetch> {
    let r = req
        .bearer_auth(token)
        .timeout(HTTP)
        .send()
        .await
        .map_err(|e| Fetch::Failed(format!("could not reach {who}: {}", e.without_url())))?;
    let status = r.status().as_u16();
    let body = r.bytes().await;
    let v: Option<Value> = body
        .as_ref()
        .ok()
        .and_then(|b| serde_json::from_slice(b).ok());
    match status {
        // An answer that broke off (the timeout reaches into the body, and a
        // big attachment takes a while) read as an empty one: a list of
        // nothing, a thread with no mails, a file of no bytes.
        200..=299 => v.ok_or_else(|| Fetch::Failed(format!("{who}\u{2019}s answer was cut off"))),
        401 => Err(Fetch::Expired),
        _ => {
            let v = v.unwrap_or(Value::Null);
            let msg = v["error"]["message"].as_str().unwrap_or("no reason given");
            Err(Fetch::Failed(crate::oauth::short_reason(&format!(
                "{who} {status}: {msg}"
            ))))
        }
    }
}
