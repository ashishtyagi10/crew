//! GETs to Gmail, run together on the worker's runtime and answered in the
//! order asked. A 401 is its own answer: the caller renews the token.
use std::time::Duration;

use serde_json::Value;

const HTTP: Duration = Duration::from_secs(20);

pub(crate) enum Fetch {
    Expired,
    Failed(String),
}

pub(crate) async fn all(
    http: reqwest::Client,
    token: String,
    urls: Vec<String>,
) -> Result<Vec<Value>, Fetch> {
    let mut set = tokio::task::JoinSet::new();
    for (i, url) in urls.into_iter().enumerate() {
        let (http, token) = (http.clone(), token.clone());
        set.spawn(async move { (i, one(&http, &token, &url).await) });
    }
    let mut out: Vec<Option<Value>> = vec![None; set.len()];
    while let Some(done) = set.join_next().await {
        let (i, got) = done.map_err(|_| Fetch::Failed("a request was dropped".into()))?;
        out[i] = Some(got?);
    }
    Ok(out.into_iter().map(Option::unwrap_or_default).collect())
}

async fn one(http: &reqwest::Client, token: &str, url: &str) -> Result<Value, Fetch> {
    let r = http
        .get(url)
        .bearer_auth(token)
        .timeout(HTTP)
        .send()
        .await
        .map_err(|e| Fetch::Failed(format!("could not reach Gmail: {}", e.without_url())))?;
    let status = r.status().as_u16();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    match status {
        200..=299 => Ok(v),
        401 => Err(Fetch::Expired),
        _ => {
            let msg = v["error"]["message"].as_str().unwrap_or("no reason given");
            Err(Fetch::Failed(crate::oauth::short_reason(&format!(
                "Gmail {status}: {msg}"
            ))))
        }
    }
}
